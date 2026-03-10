use crate::bindings::{get_draw_commands, set_draw_offset, DrawCommand};
use crate::LuaRuntime;
use anyhow::{Context, Result};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::DrawTarget;
use moondeck_core::gfx::{DrawContext};
use moondeck_core::ui::{Event, Gesture, WidgetContext};
use moondeck_core::TtfFont;
use piccolo::{Closure, Executor, Fuel, StashedTable, Table, Value};

// Auto-generated from config/widgets/*.lua by build.rs
include!(concat!(env!("OUT_DIR"), "/embedded_widgets.rs"));

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

    fn get_widget_source(&self) -> Option<&'static str> {
        EMBEDDED_WIDGETS
            .iter()
            .find(|(name, _)| *name == self.module)
            .map(|(_, src)| *src)
    }

    pub fn init(&mut self, runtime: &mut LuaRuntime, ctx: &WidgetContext) -> Result<()> {
        let source = match self.get_widget_source() {
            Some(s) => s,
            None => {
                log::warn!("Widget module not found: {}", self.module);
                self.initialized = true;
                return Ok(());
            }
        };

        let lua = runtime.lua();

        // Load the widget module
        let (module_table, widget_state) = lua.try_enter(|lctx| {
            // Load and execute the widget Lua file
            let closure = Closure::load(lctx, Some(self.module.as_str()), source.as_bytes())?;

            let executor = Executor::start(lctx, closure.into(), ());
            let stashed = lctx.stash(executor);

            // Run to get the module table
            let mut fuel = Fuel::with(100000);
            let exec = lctx.fetch(&stashed);
            while !exec.step(lctx, &mut fuel) {
                if fuel.remaining() <= 0 {
                    log::warn!("Widget {} ran out of fuel", self.module);
                    break;
                }
            }

            let result = exec.take_result::<Value>(lctx)?;

            let module = match result {
                Ok(Value::Table(t)) => {
                    log::debug!("Widget {} module loaded successfully", self.module);
                    t
                }
                Ok(other) => {
                    log::warn!("Widget {} returned {:?} instead of table", self.module, other);
                    return Ok((None, None));
                }
                Err(e) => {
                    log::error!("Widget {} load error (likely require/stdlib issue): {:?}", self.module, e);
                    return Ok((None, None));
                }
            };

            // Call init function if it exists
            let init_fn = module.get(lctx, "init");
            let state = if let Value::Function(f) = init_fn {
                // Create context table for init
                let ctx_table = Table::new(&lctx);
                ctx_table.set(lctx, "x", ctx.x as i64)?;
                ctx_table.set(lctx, "y", ctx.y as i64)?;
                ctx_table.set(lctx, "width", ctx.width as i64)?;
                ctx_table.set(lctx, "height", ctx.height as i64)?;

                // Create opts table - intern keys to avoid lifetime issues
                let opts_table = Table::new(&lctx);
                let opts_clone: Vec<_> = ctx.opts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                for (key, value) in opts_clone {
                    let key_str = lctx.intern(key.as_bytes());
                    match value {
                        serde_json::Value::Bool(b) => { opts_table.set(lctx, key_str, b)?; }
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                opts_table.set(lctx, key_str, i)?;
                            } else if let Some(f) = n.as_f64() {
                                opts_table.set(lctx, key_str, f)?;
                            }
                        }
                        serde_json::Value::String(s) => { opts_table.set(lctx, key_str, lctx.intern(s.as_bytes()))?; }
                        _ => {}
                    }
                }
                ctx_table.set(lctx, "opts", opts_table)?;

                // Call init(ctx)
                let init_exec = Executor::start(lctx, f, (ctx_table,));
                let init_stashed = lctx.stash(init_exec);
                let mut fuel = Fuel::with(100000);
                let exec = lctx.fetch(&init_stashed);
                while !exec.step(lctx, &mut fuel) {
                    if fuel.remaining() <= 0 {
                        log::warn!("Widget {} init ran out of fuel", self.module);
                        break;
                    }
                }
                let init_result = exec.take_result::<Value>(lctx)?;

                match init_result {
                    Ok(Value::Table(t)) => {
                        log::debug!("Widget {} init returned state table successfully", self.module);
                        Some(lctx.stash(t))
                    }
                    Ok(other) => {
                        log::warn!("Widget {} init returned {:?} instead of table", self.module, other);
                        None
                    }
                    Err(e) => {
                        log::error!("Widget {} init error: {:?}", self.module, e);
                        None
                    }
                }
            } else {
                log::debug!("Widget {} has no init function", self.module);
                None
            };

            log::info!("Widget {} loaded successfully", self.module);
            Ok((Some(lctx.stash(module)), state))
        }).context("Failed to initialize widget")?;

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

        let lua = runtime.lua();
        let widget_state = &self.widget_state;
        let module_name = &self.module;
        Ok(lua.try_enter(|lctx| {
            let module = lctx.fetch(module_table);

            let update_fn = module.get(lctx, "update");
            if let Value::Function(f) = update_fn {
                let state_val: Value = if let Some(ref state_stash) = widget_state {
                    lctx.fetch(state_stash).into()
                } else {
                    log::warn!("Widget {} has no state, using empty table", module_name);
                    Table::new(&lctx).into()
                };

                let exec = Executor::start(lctx, f, (state_val, delta_ms as i64));
                let stashed = lctx.stash(exec);
                let mut fuel = Fuel::with(100000);
                let exec = lctx.fetch(&stashed);
                while !exec.step(lctx, &mut fuel) {
                    if fuel.remaining() <= 0 {
                        log::warn!("Widget {} update ran out of fuel", module_name);
                        break;
                    }
                }

                if let Ok(Err(e)) = exec.take_result::<Value>(lctx) {
                    log::error!("Widget {} update error: {:?}", module_name, e);
                }
            }
            Ok(())
        })?)
    }

    pub fn render<T: DrawTarget<Color = Rgb565>>(
        &self,
        runtime: &mut LuaRuntime,
        ctx: &WidgetContext,
        draw_ctx: &mut DrawContext<'_, T>,
    ) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // If no module loaded, show placeholder
        let module_table = match &self.module_table {
            Some(m) => m,
            None => {
                log::warn!("Widget {} not found", self.module);
                return Ok(());
            }
        };

        // Clear draw commands and set offset for relative positioning
        let draw_cmds = get_draw_commands();
        draw_cmds.clear_commands();
        set_draw_offset(ctx.x, ctx.y);

        // Call the widget's render function
        let lua = runtime.lua();
        let widget_state = &self.widget_state;
        lua.try_enter(|lctx| {
            let module = lctx.fetch(module_table);

            let render_fn = module.get(lctx, "render");
            if let Value::Function(f) = render_fn {
                let gfx = lctx.globals().get(lctx, "gfx");

                // Pass state if available, otherwise pass an empty table
                let state_val: Value = if let Some(ref state_stash) = widget_state {
                    lctx.fetch(state_stash).into()
                } else {
                    Table::new(&lctx).into()
                };

                let exec = Executor::start(lctx, f, (state_val, gfx));
                let stashed = lctx.stash(exec);
                let mut fuel = Fuel::with(100000);
                let exec = lctx.fetch(&stashed);
                while !exec.step(lctx, &mut fuel) {
                    if fuel.remaining() <= 0 {
                        log::warn!("Widget {} render ran out of fuel", self.module);
                        break;
                    }
                }

                // Check for execution errors
                if let Ok(Err(e)) = exec.take_result::<Value>(lctx) {
                    log::error!("Widget {} render error: {:?}", self.module, e);
                }
            }
            Ok(())
        })?;

        // Execute the collected draw commands
        let commands = draw_cmds.take_commands();
        for cmd in commands {
            match cmd {
                DrawCommand::Clear { color } => {
                    draw_ctx.fill_rect(ctx.x, ctx.y, ctx.width, ctx.height, color);
                }
                DrawCommand::FillRect { x, y, w, h, color } => {
                    draw_ctx.fill_rect(x, y, w, h, color);
                }
                DrawCommand::StrokeRect { x, y, w, h, color, thickness } => {
                    draw_ctx.stroke_rect(x, y, w, h, color, thickness);
                }
                DrawCommand::FillRoundedRect { x, y, w, h, radius, color } => {
                    draw_ctx.fill_rounded_rect(x, y, w, h, radius, color);
                }
                DrawCommand::StrokeRoundedRect { x, y, w, h, radius, color, thickness } => {
                    draw_ctx.stroke_rounded_rect(x, y, w, h, radius, color, thickness);
                }
                DrawCommand::FillCircle { cx, cy, radius, color } => {
                    draw_ctx.fill_circle(cx, cy, radius, color);
                }
                DrawCommand::StrokeCircle { cx, cy, radius, color, thickness } => {
                    draw_ctx.stroke_circle(cx, cy, radius, color, thickness);
                }
                DrawCommand::Line { x1, y1, x2, y2, color, thickness } => {
                    draw_ctx.line(x1, y1, x2, y2, color, thickness);
                }
                DrawCommand::Text { x, y, text, color, font } => {
                    use moondeck_core::gfx::Font;
                    let ttf_font = match font {
                        Font::Small => TtfFont::inter(12),
                        Font::Medium => TtfFont::inter(16),
                        Font::Large => TtfFont::inter(24),
                        Font::XLarge => TtfFont::inter(32),
                    };
                    draw_ctx.text_ttf(x, y, &text, color, ttf_font);
                }
            }
        }

        Ok(())
    }

    pub fn on_event(&self, runtime: &mut LuaRuntime, event: &Event) -> Result<bool> {
        let module_table = match &self.module_table {
            Some(m) => m,
            None => return Ok(false),
        };

        let lua = runtime.lua();
        let widget_state = &self.widget_state;
        Ok(lua.try_enter(|lctx| {
            let module = lctx.fetch(module_table);

            let event_fn = module.get(lctx, "on_event");
            if let Value::Function(f) = event_fn {
                let state_val: Value = if let Some(ref state_stash) = widget_state {
                    lctx.fetch(state_stash).into()
                } else {
                    Table::new(&lctx).into()
                };

                let event_table = Table::new(&lctx);
                match event {
                    Event::Gesture(Gesture::Tap { x, y }) => {
                        event_table.set(lctx, "type", "tap")?;
                        event_table.set(lctx, "x", *x as i64)?;
                        event_table.set(lctx, "y", *y as i64)?;
                    }
                    Event::Gesture(Gesture::SwipeLeft) => {
                        event_table.set(lctx, "type", "swipe")?;
                        event_table.set(lctx, "direction", "left")?;
                    }
                    Event::Gesture(Gesture::SwipeRight) => {
                        event_table.set(lctx, "type", "swipe")?;
                        event_table.set(lctx, "direction", "right")?;
                    }
                    Event::Gesture(Gesture::SwipeUp) => {
                        event_table.set(lctx, "type", "swipe")?;
                        event_table.set(lctx, "direction", "up")?;
                    }
                    Event::Gesture(Gesture::SwipeDown) => {
                        event_table.set(lctx, "type", "swipe")?;
                        event_table.set(lctx, "direction", "down")?;
                    }
                    _ => {
                        event_table.set(lctx, "type", "unknown")?;
                    }
                }

                let exec = Executor::start(lctx, f, (state_val, event_table));
                let stashed = lctx.stash(exec);
                let mut fuel = Fuel::with(100000);
                let exec = lctx.fetch(&stashed);
                while !exec.step(lctx, &mut fuel) {
                    if fuel.remaining() <= 0 {
                        break;
                    }
                }

                let result = exec.take_result::<Value>(lctx);
                if let Ok(Ok(Value::Boolean(b))) = result {
                    return Ok(b);
                }
            }
            Ok(false)
        })?)
    }
}
