use anyhow::{Context, Result};
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};
use moondeck_core::{
    gfx::{DrawContext, ImageCache},
    ui::{Event, Gesture, WidgetContext},
    TtfFont,
};
use piccolo::{Closure, Executor, Fuel, StashedTable, Table, Value};

use crate::{
    bindings::{get_draw_commands, lua_serde::json_to_lua, set_draw_offset, DrawCommand},
    LuaRuntime,
};

pub fn embedded_widget_sources() -> &'static [(&'static str, &'static str)] {
    crate::bindings::embedded_lua_modules()
}

pub struct WidgetPlugin {
    pub module: String,
    module_table: Option<StashedTable>,
    widget_state: Option<StashedTable>,
    initialized: bool,
}

impl WidgetPlugin {
    pub fn new(module: &str, _instance_id: usize) -> Self {
        Self {
            module: module.to_string(),
            module_table: None,
            widget_state: None,
            initialized: false,
        }
    }

    fn get_source(&self) -> Option<&'static str> {
        crate::bindings::embedded_lua_modules()
            .iter()
            .find(|(n, _)| *n == self.module)
            .map(|(_, s)| *s)
    }

    pub fn init(&mut self, runtime: &mut LuaRuntime, ctx: &WidgetContext) -> Result<()> {
        // Prefer embedded source (always in sync with the binary) over SPIFFS,
        // falling back to SPIFFS for user-uploaded widgets not in the embedded set.
        let fs_source = runtime.read_widget_source(&self.module);
        let source: std::borrow::Cow<'_, str> = if let Some(s) = self.get_source() {
            s.into()
        } else if let Some(ref s) = fs_source {
            s.as_str().into()
        } else {
            log::warn!("Widget not found: {}", self.module);
            self.initialized = true;
            return Ok(());
        };

        let lua = runtime.lua();
        let module_name = self.module.clone();

        let (module_table, widget_state) = lua
            .try_enter(|lctx| {
                // Load module
                let closure = Closure::load(lctx, Some(&module_name), source.as_bytes())?;
                let exec = run_executor(
                    lctx,
                    Executor::start(lctx, closure.into(), ()),
                    &module_name,
                    "load",
                )?;

                let module = match exec.take_result::<Value>(lctx)? {
                    Ok(Value::Table(t)) => t,
                    Ok(_) | Err(_) => return Ok((None, None)),
                };

                // Call init if exists
                let state = if let Value::Function(f) = module.get(lctx, "init") {
                    let ctx_table = build_context_table(lctx, ctx)?;
                    let exec = run_executor(
                        lctx,
                        Executor::start(lctx, f, (ctx_table,)),
                        &module_name,
                        "init",
                    )?;
                    match exec.take_result::<Value>(lctx)? {
                        Ok(Value::Table(t)) => Some(lctx.stash(t)),
                        Ok(other) => {
                            log::warn!("Widget {} init returned non-table: {:?}", module_name, other);
                            None
                        }
                        Err(e) => {
                            log::error!("Widget {} init error: {:?}", module_name, e);
                            None
                        }
                    }
                } else {
                    None
                };

                log::info!("Widget {} loaded", module_name);
                Ok((Some(lctx.stash(module)), state))
            })
            .context("Failed to initialize widget")?;

        self.module_table = module_table;
        self.widget_state = widget_state;
        self.initialized = true;
        Ok(())
    }

    pub fn update(&self, runtime: &mut LuaRuntime, delta_ms: u32) -> Result<()> {
        let module_table = match &self.module_table {
            Some(m) => m,
            None => return Ok(()),
        };
        let module_name = &self.module;
        let widget_state = &self.widget_state;

        runtime.lua().try_enter(|lctx| {
            let module = lctx.fetch(module_table);
            if let Value::Function(f) = module.get(lctx, "update") {
                let state = get_state_value(lctx, widget_state);
                let exec = run_executor(
                    lctx,
                    Executor::start(lctx, f, (state, delta_ms as i64)),
                    module_name,
                    "update",
                )?;
                if let Ok(Err(e)) = exec.take_result::<Value>(lctx) {
                    log::error!("Widget {} update error: {:?}", module_name, e);
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn render<T: DrawTarget<Color = Rgb565>>(
        &self,
        runtime: &mut LuaRuntime,
        ctx: &WidgetContext,
        draw_ctx: &mut DrawContext<'_, T>,
        image_cache: &mut ImageCache,
    ) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        let module_table = match &self.module_table {
            Some(m) => m,
            None => return Ok(()),
        };

        let draw_cmds = get_draw_commands();
        draw_cmds.clear_commands();
        set_draw_offset(ctx.x, ctx.y);

        let widget_state = &self.widget_state;
        runtime.lua().try_enter(|lctx| {
            let module = lctx.fetch(module_table);
            if let Value::Function(f) = module.get(lctx, "render") {
                let state = get_state_value(lctx, widget_state);
                let gfx = lctx.globals().get(lctx, "gfx");
                let exec = run_executor(
                    lctx,
                    Executor::start(lctx, f, (state, gfx)),
                    &self.module,
                    "render",
                )?;
                if let Ok(Err(e)) = exec.take_result::<Value>(lctx) {
                    log::error!("Widget {} render error: {:?}", self.module, e);
                }
            }
            Ok(())
        })?;

        execute_draw_commands(draw_cmds.take_commands(), ctx, draw_ctx, image_cache);
        Ok(())
    }

    pub fn on_event(&self, runtime: &mut LuaRuntime, event: &Event) -> Result<bool> {
        let module_table = match &self.module_table {
            Some(m) => m,
            None => return Ok(false),
        };
        let widget_state = &self.widget_state;

        Ok(runtime.lua().try_enter(|lctx| {
            let module = lctx.fetch(module_table);
            if let Value::Function(f) = module.get(lctx, "on_event") {
                let state = get_state_value(lctx, widget_state);
                let event_table = build_event_table(lctx, event)?;
                let exec = run_executor(
                    lctx,
                    Executor::start(lctx, f, (state, event_table)),
                    &self.module,
                    "on_event",
                )?;
                if let Ok(Ok(Value::Boolean(b))) = exec.take_result::<Value>(lctx) {
                    return Ok(b);
                }
            }
            Ok(false)
        })?)
    }
}

// Helper to run executor with fuel
fn run_executor<'gc>(
    ctx: piccolo::Context<'gc>,
    executor: Executor<'gc>,
    module: &str,
    method: &str,
) -> Result<Executor<'gc>, anyhow::Error> {
    let stashed = ctx.stash(executor);
    let mut fuel = Fuel::with(100000);
    let exec = ctx.fetch(&stashed);
    while !exec.step(ctx, &mut fuel) {
        if fuel.remaining() <= 0 {
            log::warn!("Widget {} {} ran out of fuel", module, method);
            break;
        }
    }
    Ok(exec)
}

fn get_state_value<'gc>(ctx: piccolo::Context<'gc>, state: &Option<StashedTable>) -> Value<'gc> {
    state
        .as_ref()
        .map(|s| ctx.fetch(s).into())
        .unwrap_or_else(|| Table::new(&ctx).into())
}

fn build_context_table<'gc>(
    ctx: piccolo::Context<'gc>,
    wctx: &WidgetContext,
) -> Result<Table<'gc>, piccolo::Error<'gc>> {
    let t = Table::new(&ctx);
    t.set(ctx, "x", wctx.x as i64)?;
    t.set(ctx, "y", wctx.y as i64)?;
    t.set(ctx, "width", wctx.width as i64)?;
    t.set(ctx, "height", wctx.height as i64)?;

    let opts = Table::new(&ctx);
    for (key, value) in &wctx.opts {
        let k = ctx.intern(key.as_bytes());
        opts.set(ctx, k, json_to_lua(ctx, value))?;
    }
    t.set(ctx, "opts", opts)?;
    Ok(t)
}

fn build_event_table<'gc>(
    ctx: piccolo::Context<'gc>,
    event: &Event,
) -> Result<Table<'gc>, piccolo::Error<'gc>> {
    let t = Table::new(&ctx);
    match event {
        Event::Gesture(Gesture::Tap { x, y }) => {
            t.set(ctx, "type", "tap")?;
            t.set(ctx, "x", *x as i64)?;
            t.set(ctx, "y", *y as i64)?;
        }
        Event::Gesture(g) => {
            t.set(ctx, "type", "swipe")?;
            t.set(
                ctx,
                "direction",
                match g {
                    Gesture::SwipeLeft => "left",
                    Gesture::SwipeRight => "right",
                    Gesture::SwipeUp => "up",
                    Gesture::SwipeDown => "down",
                    _ => "unknown",
                },
            )?;
        }
        _ => {
            t.set(ctx, "type", "unknown")?;
        }
    }
    Ok(t)
}

fn execute_draw_commands<T: DrawTarget<Color = Rgb565>>(
    commands: Vec<DrawCommand>,
    ctx: &WidgetContext,
    draw_ctx: &mut DrawContext<'_, T>,
    image_cache: &mut ImageCache,
) {
    for cmd in commands {
        match cmd {
            DrawCommand::Clear { color } => {
                draw_ctx.fill_rect(ctx.x, ctx.y, ctx.width, ctx.height, color)
            }
            DrawCommand::FillRoundedRect {
                x,
                y,
                w,
                h,
                radius,
                color,
            } => draw_ctx.fill_rounded_rect(x, y, w, h, radius, color),
            DrawCommand::StrokeRoundedRect {
                x,
                y,
                w,
                h,
                radius,
                color,
                thickness,
            } => draw_ctx.stroke_rounded_rect(x, y, w, h, radius, color, thickness),
            DrawCommand::FillCircle {
                cx,
                cy,
                radius,
                color,
            } => draw_ctx.fill_circle(cx, cy, radius, color),
            DrawCommand::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                thickness,
            } => draw_ctx.line(x1, y1, x2, y2, color, thickness),
            DrawCommand::Text {
                x,
                y,
                text,
                color,
                family,
                size,
            } => {
                let ttf = match family.as_str() {
                    "ebgaramond" | "garamond" => TtfFont::ebgaramond(size),
                    _ => TtfFont::inter(size),
                };
                draw_ctx.text_ttf(x, y, &text, color, ttf);
            }
            DrawCommand::Image { x, y, w, h, path } => {
                if !image_cache.contains(&path) {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            let ext = path.rsplit('.').next().unwrap_or("");
                            let result = match ext {
                                "jpg" | "jpeg" => image_cache.decode_jpeg_to_rgb565(&path, &bytes),
                                "rgb565" => image_cache.load_rgb565(&path, &bytes, w, h),
                                _ => Err(anyhow::anyhow!("Unsupported image format: {}", ext)),
                            };
                            if let Err(e) = result {
                                log::error!("Failed to load image {}: {}", path, e);
                            }
                        }
                        Err(e) => log::error!("Failed to read image file {}: {}", path, e),
                    }
                }
                if let Some(img) = image_cache.get(&path) {
                    draw_ctx.draw_image_scaled(x, y, &img.pixels, img.width, img.height, w, h);
                }
            }
        }
    }
}
