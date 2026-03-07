use anyhow::Result;
use moondeck_core::gfx::{Color, Font};
use piccolo::{Callback, CallbackReturn, Lua, String as LuaString, Table, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Clear { color: Color },
    FillRect { x: i32, y: i32, w: u32, h: u32, color: Color },
    StrokeRect { x: i32, y: i32, w: u32, h: u32, color: Color, thickness: u32 },
    FillRoundedRect { x: i32, y: i32, w: u32, h: u32, radius: u32, color: Color },
    FillCircle { cx: i32, cy: i32, radius: u32, color: Color },
    StrokeCircle { cx: i32, cy: i32, radius: u32, color: Color, thickness: u32 },
    Line { x1: i32, y1: i32, x2: i32, y2: i32, color: Color, thickness: u32 },
    Text { x: i32, y: i32, text: String, color: Color, font: Font },
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
        let mut guard = self.commands.lock().unwrap();
        guard.drain(..).collect()
    }

    pub fn clear_commands(&self) {
        self.commands.lock().unwrap().clear();
    }

    pub fn push(&self, cmd: DrawCommand) {
        self.commands.lock().unwrap().push(cmd);
    }

    fn get_offset(&self) -> (i32, i32) {
        (*self.offset_x.lock().unwrap(), *self.offset_y.lock().unwrap())
    }

    pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: Color) {
        self.push(DrawCommand::FillRect {
            x: x as i32,
            y: y as i32,
            w,
            h,
            color,
        });
    }

    pub fn stroke_rect(&self, x: u32, y: u32, w: u32, h: u32, color: Color, thickness: u32) {
        self.push(DrawCommand::StrokeRect {
            x: x as i32,
            y: y as i32,
            w,
            h,
            color,
            thickness,
        });
    }

    pub fn text(&self, x: i32, y: i32, text: &str, color: Color, font: Font) {
        self.push(DrawCommand::Text {
            x,
            y,
            text: text.to_string(),
            color,
            font,
        });
    }

    pub fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color, thickness: u32) {
        self.push(DrawCommand::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            thickness,
        });
    }
}

fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::BLACK,
        "white" => Color::WHITE,
        "red" => Color::RED,
        "green" => Color::GREEN,
        "blue" => Color::BLUE,
        "cyan" => Color::CYAN,
        "magenta" => Color::MAGENTA,
        "yellow" => Color::YELLOW,
        "gray" | "grey" => Color::GRAY,
        _ => Color::from_hex(s).unwrap_or(Color::WHITE),
    }
}

fn parse_font(size_name: &str) -> Font {
    match size_name.to_lowercase().as_str() {
        "small" => Font::Small,
        "medium" => Font::Medium,
        "large" => Font::Large,
        "xlarge" | "xxlarge" => Font::XLarge,
        _ => Font::Medium,
    }
}

fn color_from_value(val: Value) -> Color {
    match val {
        Value::String(s) => parse_color(s.to_str().unwrap_or("#FFFFFF")),
        Value::Integer(i) => {
            let r = ((i >> 16) & 0xFF) as u8;
            let g = ((i >> 8) & 0xFF) as u8;
            let b = (i & 0xFF) as u8;
            Color::new(r, g, b)
        }
        _ => Color::WHITE,
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
        let gfx_table = Table::new(&ctx);

        // gfx:fill_rect(x, y, w, h, color)
        gfx_table.set(
            ctx,
            "fill_rect",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, x, y, w, h, color): (Value, i64, i64, i64, i64, Value) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::FillRect {
                        x: x as i32 + ox,
                        y: y as i32 + oy,
                        w: w as u32,
                        h: h as u32,
                        color: color_from_value(color),
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:stroke_rect(x, y, w, h, color, thickness)
        gfx_table.set(
            ctx,
            "stroke_rect",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, x, y, w, h, color, thickness): (Value, i64, i64, i64, i64, Value, i64) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::StrokeRect {
                        x: x as i32 + ox,
                        y: y as i32 + oy,
                        w: w as u32,
                        h: h as u32,
                        color: color_from_value(color),
                        thickness: thickness as u32,
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:fill_rounded_rect(x, y, w, h, radius, color)
        gfx_table.set(
            ctx,
            "fill_rounded_rect",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, x, y, w, h, radius, color): (Value, i64, i64, i64, i64, i64, Value) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::FillRoundedRect {
                        x: x as i32 + ox,
                        y: y as i32 + oy,
                        w: w as u32,
                        h: h as u32,
                        radius: radius as u32,
                        color: color_from_value(color),
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:circle(cx, cy, radius, color)
        gfx_table.set(
            ctx,
            "circle",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, cx, cy, radius, color): (Value, i64, i64, i64, Value) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::FillCircle {
                        cx: cx as i32 + ox,
                        cy: cy as i32 + oy,
                        radius: radius as u32,
                        color: color_from_value(color),
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:line(x1, y1, x2, y2, color, thickness)
        gfx_table.set(
            ctx,
            "line",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, x1, y1, x2, y2, color, thickness): (Value, i64, i64, i64, i64, Value, i64) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::Line {
                        x1: x1 as i32 + ox,
                        y1: y1 as i32 + oy,
                        x2: x2 as i32 + ox,
                        y2: y2 as i32 + oy,
                        color: color_from_value(color),
                        thickness: thickness as u32,
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:text(x, y, text, color, font_size)
        gfx_table.set(
            ctx,
            "text",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, x, y, text, color, font_size): (Value, i64, i64, LuaString, Value, LuaString) =
                    stack.consume(ctx)?;
                let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::Text {
                        x: x as i32 + ox,
                        y: y as i32 + oy,
                        text: text.to_str().unwrap_or("").to_string(),
                        color: color_from_value(color),
                        font: parse_font(font_size.to_str().unwrap_or("medium")),
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // gfx:clear(color)
        gfx_table.set(
            ctx,
            "clear",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_self, color): (Value, Value) = stack.consume(ctx)?;
                DRAW_COMMANDS.with(|dc| {
                    dc.push(DrawCommand::Clear {
                        color: color_from_value(color),
                    })
                });
                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("gfx", gfx_table)?;
        Ok(())
    })?;

    Ok(())
}
