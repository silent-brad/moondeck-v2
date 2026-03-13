use crate::bindings::{get_draw_commands, lua_serde::json_to_lua, set_draw_offset, DrawCommand};
use crate::vm::value::{LuaString, LuaTable, TableKey, Value};
use crate::vm::Fuel;
use crate::LuaRuntime;
use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::DrawTarget;
use moondeck_core::gfx::DrawContext;
use moondeck_core::ui::{Event, Gesture, WidgetContext};
use moondeck_core::TtfFont;

include!(concat!(env!("OUT_DIR"), "/embedded_widgets.rs"));

pub struct WidgetPlugin {
    pub module: String,
    module_table: Option<LuaTable>,
    widget_state: Option<LuaTable>,
    initialized: bool,
}

impl WidgetPlugin {
    pub fn new(module: &str, _instance_id: usize) -> Self {
        Self { module: module.to_string(), module_table: None, widget_state: None, initialized: false }
    }

    fn get_source(&self) -> Option<&'static str> {
        EMBEDDED_WIDGETS.iter().find(|(n, _)| *n == self.module).map(|(_, s)| *s)
    }

    pub fn init(&mut self, runtime: &mut LuaRuntime, ctx: &WidgetContext) -> Result<()> {
        let source = match self.get_source() {
            Some(s) => s,
            None => { log::warn!("Widget not found: {}", self.module); self.initialized = true; return Ok(()); }
        };

        let vm = runtime.vm();
        let module_name = self.module.clone();

        // Test: can we compile and get the proto to inspect it?
        let proto = vm.compile(Some(&module_name), source)
            .map_err(|e| anyhow::anyhow!("Failed to compile widget {}: {:?}", module_name, e))?;
        log::info!("Widget {} compiled: {} instructions, {} constants, {} nested protos, {} locals",
            module_name, proto.code.len(), proto.constants.len(), proto.nested.len(), proto.num_locals);
        for (i, c) in proto.constants.iter().enumerate() {
            match c {
                crate::vm::Constant::Str(sym) => log::info!("  const[{}] = str:{}", i, vm.symbols.resolve(*sym)),
                crate::vm::Constant::Int(n) => log::info!("  const[{}] = int:{}", i, n),
                _ => log::info!("  const[{}] = {:?}", i, c),
            }
        }

        let mut fuel = Fuel::with(100000);
        let results = vm
            .execute(&proto, &mut fuel)
            .map_err(|e| anyhow::anyhow!("Failed to load widget {}: {:?}", module_name, e))?;

        let first_result = results.into_iter().next();
        log::info!("Widget {} exec result: {:?}", module_name, first_result.as_ref().map(|v| v.type_name()));

        let module_table = match first_result {
            Some(Value::Table(t)) => {
                let entries = t.iter();
                log::info!("Widget {} table has {} entries:", module_name, entries.len());
                for (k, v) in &entries {
                    let key_str = match k {
                        TableKey::Sym(s) => vm.symbols.resolve(*s).to_string(),
                        TableKey::Int(i) => format!("[{}]", i),
                        TableKey::Str(s) => s.as_ref().clone(),
                    };
                    log::info!("  {} = {}", key_str, v.type_name());
                }
                t
            }
            other => {
                log::warn!("Widget {} did not return a table, got: {:?}", module_name,
                    other.as_ref().map(|v| v.type_name()));
                self.initialized = true;
                return Ok(());
            }
        };

        // Call init if exists
        let init_fn = module_table.get_str("init", &vm.symbols);
        log::info!("Widget {} init_fn type: {}", module_name, init_fn.type_name());
        let widget_state = if !init_fn.is_nil() {
            let ctx_table = build_context_table(vm, ctx);
            let mut fuel = Fuel::with(100000);
            let results = vm
                .call_function(&init_fn, &[Value::Table(ctx_table)], &mut fuel)
                .map_err(|e| anyhow::anyhow!("Widget {} init error: {:?}", module_name, e))?;
            match results.into_iter().next() {
                Some(Value::Table(t)) => Some(t),
                _ => None,
            }
        } else {
            None
        };

        log::info!("Widget {} loaded", module_name);
        self.module_table = Some(module_table);
        self.widget_state = widget_state;
        self.initialized = true;
        Ok(())
    }

    pub fn update(&self, runtime: &mut LuaRuntime, delta_ms: u32) -> Result<()> {
        let module_table = match &self.module_table { Some(m) => m, None => return Ok(()) };

        let vm = runtime.vm();
        let update_fn = module_table.get_str("update", &vm.symbols);
        if update_fn.is_nil() { return Ok(()); }

        let state = get_state_value(&self.widget_state);
        let mut fuel = Fuel::with(100000);
        if let Err(e) = vm.call_function(&update_fn, &[state, Value::Int(delta_ms as i64)], &mut fuel) {
            log::error!("Widget {} update error: {:?}", self.module, e);
        }
        Ok(())
    }

    pub fn render<T: DrawTarget<Color = Rgb565>>(
        &self, runtime: &mut LuaRuntime, ctx: &WidgetContext, draw_ctx: &mut DrawContext<'_, T>,
    ) -> Result<()> {
        if !self.initialized { return Ok(()); }
        let module_table = match &self.module_table { Some(m) => m, None => return Ok(()) };

        let draw_cmds = get_draw_commands();
        draw_cmds.clear_commands();
        set_draw_offset(ctx.x, ctx.y);

        let vm = runtime.vm();
        let render_fn = module_table.get_str("render", &vm.symbols);
        if !render_fn.is_nil() {
            let state = get_state_value(&self.widget_state);
            let gfx = vm.get_global("gfx");
            let mut fuel = Fuel::with(100000);
            if let Err(e) = vm.call_function(&render_fn, &[state, gfx], &mut fuel) {
                log::error!("Widget {} render error: {:?}", self.module, e);
            }
        }

        execute_draw_commands(draw_cmds.take_commands(), ctx, draw_ctx);
        Ok(())
    }

    pub fn on_event(&self, runtime: &mut LuaRuntime, event: &Event) -> Result<bool> {
        let module_table = match &self.module_table { Some(m) => m, None => return Ok(false) };

        let vm = runtime.vm();
        let on_event_fn = module_table.get_str("on_event", &vm.symbols);
        if on_event_fn.is_nil() { return Ok(false); }

        let state = get_state_value(&self.widget_state);
        let event_table = build_event_table(vm, event);
        let mut fuel = Fuel::with(100000);
        match vm.call_function(&on_event_fn, &[state, Value::Table(event_table)], &mut fuel) {
            Ok(results) => {
                if let Some(Value::Bool(b)) = results.first() {
                    Ok(*b)
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                log::error!("Widget {} on_event error: {:?}", self.module, e);
                Ok(false)
            }
        }
    }
}

fn get_state_value(state: &Option<LuaTable>) -> Value {
    state.as_ref()
        .map(|s| Value::Table(s.clone()))
        .unwrap_or_else(|| Value::Table(LuaTable::new()))
}

fn build_context_table(vm: &mut crate::vm::VmState, wctx: &WidgetContext) -> LuaTable {
    let t = LuaTable::new();
    let sym_x = vm.symbols.intern("x");
    let sym_y = vm.symbols.intern("y");
    let sym_width = vm.symbols.intern("width");
    let sym_height = vm.symbols.intern("height");
    let sym_opts = vm.symbols.intern("opts");

    t.set_sym(sym_x, Value::Int(wctx.x as i64));
    t.set_sym(sym_y, Value::Int(wctx.y as i64));
    t.set_sym(sym_width, Value::Int(wctx.width as i64));
    t.set_sym(sym_height, Value::Int(wctx.height as i64));

    let opts = LuaTable::new();
    for (key, value) in &wctx.opts {
        let lua_val = json_to_lua(&mut vm.symbols, value);
        let sym = vm.symbols.intern(key);
        opts.set_sym(sym, lua_val);
    }
    t.set_sym(sym_opts, Value::Table(opts));

    t
}

fn build_event_table(vm: &mut crate::vm::VmState, event: &Event) -> LuaTable {
    let t = LuaTable::new();
    let sym_type = vm.symbols.intern("type");
    let sym_x = vm.symbols.intern("x");
    let sym_y = vm.symbols.intern("y");
    let sym_direction = vm.symbols.intern("direction");

    match event {
        Event::Gesture(Gesture::Tap { x, y }) => {
            let s = vm.symbols.intern("tap");
            t.set_sym(sym_type, Value::Str(LuaString::Interned(s)));
            t.set_sym(sym_x, Value::Int(*x as i64));
            t.set_sym(sym_y, Value::Int(*y as i64));
        }
        Event::Gesture(g) => {
            let s = vm.symbols.intern("swipe");
            t.set_sym(sym_type, Value::Str(LuaString::Interned(s)));
            let dir = match g {
                Gesture::SwipeLeft => "left",
                Gesture::SwipeRight => "right",
                Gesture::SwipeUp => "up",
                Gesture::SwipeDown => "down",
                _ => "unknown",
            };
            let ds = vm.symbols.intern(dir);
            t.set_sym(sym_direction, Value::Str(LuaString::Interned(ds)));
        }
        _ => {
            let s = vm.symbols.intern("unknown");
            t.set_sym(sym_type, Value::Str(LuaString::Interned(s)));
        }
    }
    t
}

fn execute_draw_commands<T: DrawTarget<Color = Rgb565>>(
    commands: Vec<DrawCommand>, ctx: &WidgetContext, draw_ctx: &mut DrawContext<'_, T>,
) {
    use moondeck_core::gfx::Font;
    for cmd in commands {
        match cmd {
            DrawCommand::Clear { color } => draw_ctx.fill_rect(ctx.x, ctx.y, ctx.width, ctx.height, color),
            DrawCommand::FillRoundedRect { x, y, w, h, radius, color } => draw_ctx.fill_rounded_rect(x, y, w, h, radius, color),
            DrawCommand::StrokeRoundedRect { x, y, w, h, radius, color, thickness } => draw_ctx.stroke_rounded_rect(x, y, w, h, radius, color, thickness),
            DrawCommand::FillCircle { cx, cy, radius, color } => draw_ctx.fill_circle(cx, cy, radius, color),
            DrawCommand::Line { x1, y1, x2, y2, color, thickness } => draw_ctx.line(x1, y1, x2, y2, color, thickness),
            DrawCommand::Text { x, y, text, color, font } => {
                let ttf = match font {
                    Font::Small => TtfFont::inter(12),
                    Font::Medium => TtfFont::inter(16),
                    Font::Large => TtfFont::inter(24),
                    Font::XLarge => TtfFont::inter(32),
                };
                draw_ctx.text_ttf(x, y, &text, color, ttf);
            }
        }
    }
}
