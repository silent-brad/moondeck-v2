use anyhow::Result;
use moondeck_core::gfx::{Color, Font};
use piccolo::{Lua, Table, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Clear {
        color: Color,
    },
    FillRoundedRect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: Color,
    },
    StrokeRoundedRect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: Color,
        thickness: u32,
    },
    FillCircle {
        cx: i32,
        cy: i32,
        radius: u32,
        color: Color,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color,
        thickness: u32,
    },
    Text {
        x: i32,
        y: i32,
        text: String,
        color: Color,
        font: Font,
    },
}

#[derive(Clone, Default)]
pub struct LuaDrawCommands {
    commands: Arc<Mutex<Vec<DrawCommand>>>,
    offset_x: Arc<Mutex<i32>>,
    offset_y: Arc<Mutex<i32>>,
}

impl LuaDrawCommands {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
            offset_x: Arc::new(Mutex::new(0)),
            offset_y: Arc::new(Mutex::new(0)),
        }
    }

    pub fn set_offset(&self, x: i32, y: i32) {
        *self.offset_x.lock().unwrap() = x;
        *self.offset_y.lock().unwrap() = y;
    }

    pub fn take_commands(&self) -> Vec<DrawCommand> {
        std::mem::take(&mut *self.commands.lock().unwrap())
    }

    pub fn clear_commands(&self) {
        self.commands.lock().unwrap().clear();
    }

    pub fn push(&self, cmd: DrawCommand) {
        self.commands.lock().unwrap().push(cmd);
    }

    pub fn get_offset(&self) -> (i32, i32) {
        (
            *self.offset_x.lock().unwrap(),
            *self.offset_y.lock().unwrap(),
        )
    }
}

// Value conversion helpers
fn i32(val: Value) -> i32 {
    match val {
        Value::Integer(i) => i as i32,
        Value::Number(n) => n as i32,
        _ => 0,
    }
}

fn u32(val: Value) -> u32 {
    match val {
        Value::Integer(i) => i.max(0) as u32,
        Value::Number(n) => n.max(0.0) as u32,
        _ => 0,
    }
}

fn color(val: Value) -> Color {
    match val {
        Value::String(s) => Color::from_hex(s.to_str().unwrap_or("#FFFFFF")).unwrap(),
        Value::Integer(i) => Color::new(
            ((i >> 16) & 0xFF) as u8,
            ((i >> 8) & 0xFF) as u8,
            (i & 0xFF) as u8,
        ),
        _ => Color::from_hex("#FFFFFF").unwrap(),
    }
}

fn font(val: Value) -> Font {
    let name = match val {
        Value::String(s) => s.to_str().unwrap_or("medium").to_lowercase(),
        _ => "medium".to_string(),
    };
    match name.as_str() {
        "small" => Font::Small,
        "large" => Font::Large,
        "xlarge" | "xxlarge" => Font::XLarge,
        _ => Font::Medium,
    }
}

fn text_val(val: Value) -> String {
    match val {
        Value::String(s) => s.to_str().unwrap_or("").to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

thread_local! {
    pub static DRAW_COMMANDS: LuaDrawCommands = LuaDrawCommands::new();
}

pub fn get_draw_commands() -> LuaDrawCommands {
    DRAW_COMMANDS.with(|dc| dc.clone())
}

pub fn set_draw_offset(x: i32, y: i32) {
    DRAW_COMMANDS.with(|dc| dc.set_offset(x, y));
}

pub fn get_draw_offset() -> (u32, u32) {
    DRAW_COMMANDS.with(|dc| {
        let (x, y) = dc.get_offset();
        (x.max(0) as u32, y.max(0) as u32)
    })
}

pub fn register_gfx(lua: &mut Lua) -> Result<()> {
    lua.try_enter(|ctx| {
        let gfx = Table::new(&ctx);

        // Rectangle commands
        gfx_draw!(gfx, ctx, "fill_rounded_rect", (x, y, w, h, r, c) => |ox, oy| DrawCommand::FillRoundedRect {
            x: i32(x) + ox, y: i32(y) + oy, w: u32(w), h: u32(h), radius: u32(r), color: color(c)
        });

        gfx_draw!(gfx, ctx, "stroke_rounded_rect", (x, y, w, h, r, c, t) => |ox, oy| DrawCommand::StrokeRoundedRect {
            x: i32(x) + ox, y: i32(y) + oy, w: u32(w), h: u32(h), radius: u32(r), color: color(c), thickness: u32(t)
        });

        // Circle commands
        gfx_draw!(gfx, ctx, "fill_circle", (a, b, r, c) => |ox, oy| DrawCommand::FillCircle {
            cx: i32(a) + ox, cy: i32(b) + oy, radius: u32(r), color: color(c)
        });

        // Line command
        gfx_draw!(gfx, ctx, "line", (x1, y1, x2, y2, c, t) => |ox, oy| DrawCommand::Line {
            x1: i32(x1) + ox, y1: i32(y1) + oy, x2: i32(x2) + ox, y2: i32(y2) + oy,
            color: color(c), thickness: u32(t)
        });

        // Text command
        gfx_draw!(gfx, ctx, "text", (x, y, txt, c, f) => |ox, oy| DrawCommand::Text {
            x: i32(x) + ox, y: i32(y) + oy, text: text_val(txt), color: color(c), font: font(f)
        });

        // Clear command (no offset needed but macro requires it)
        gfx_draw!(gfx, ctx, "clear", (c) => |_ox, _oy| DrawCommand::Clear { color: color(c) });

        ctx.set_global("gfx", gfx)?;
        Ok(())
    })?;

    Ok(())
}
