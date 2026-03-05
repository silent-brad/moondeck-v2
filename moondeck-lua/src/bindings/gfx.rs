use anyhow::Result;
use moondeck_core::gfx::{Color, Font};
use piccolo::Lua;
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
}

impl LuaDrawCommands {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
        }
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
}

#[allow(dead_code)]
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

pub fn register_gfx(_lua: &mut Lua) -> Result<()> {
    Ok(())
}

thread_local! {
    pub static DRAW_COMMANDS: LuaDrawCommands = LuaDrawCommands::new();
}

#[allow(dead_code)]
pub fn get_draw_commands() -> LuaDrawCommands {
    DRAW_COMMANDS.with(|dc| dc.clone())
}

#[allow(dead_code)]
pub fn push_draw_command(cmd: DrawCommand) {
    DRAW_COMMANDS.with(|dc| dc.push(cmd));
}
