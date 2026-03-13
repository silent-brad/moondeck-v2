use anyhow::Result;
use moondeck_core::gfx::{Color, Font};
use crate::vm::{LuaTable, Value, VmState};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Clear { color: Color },
    FillRoundedRect { x: i32, y: i32, w: u32, h: u32, radius: u32, color: Color },
    StrokeRoundedRect { x: i32, y: i32, w: u32, h: u32, radius: u32, color: Color, thickness: u32 },
    FillCircle { cx: i32, cy: i32, radius: u32, color: Color },
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
        std::mem::take(&mut *self.commands.lock().unwrap())
    }

    pub fn clear_commands(&self) {
        self.commands.lock().unwrap().clear();
    }

    pub fn push(&self, cmd: DrawCommand) {
        self.commands.lock().unwrap().push(cmd);
    }

    pub fn get_offset(&self) -> (i32, i32) {
        (*self.offset_x.lock().unwrap(), *self.offset_y.lock().unwrap())
    }
}

// Value conversion helpers
fn val_i32(val: &Value) -> i32 {
    match val {
        Value::Int(i) => *i as i32,
        Value::Num(n) => *n as i32,
        _ => 0,
    }
}

fn val_u32(val: &Value) -> u32 {
    match val {
        Value::Int(i) => (*i).max(0) as u32,
        Value::Num(n) => n.max(0.0) as u32,
        _ => 0,
    }
}

fn color(vm: &VmState, val: &Value) -> Color {
    match val {
        Value::Str(s) => {
            let s = s.as_str(&vm.symbols);
            Color::from_hex(&s).unwrap()
        }
        Value::Int(i) => Color::new(
            ((i >> 16) & 0xFF) as u8,
            ((i >> 8) & 0xFF) as u8,
            (i & 0xFF) as u8,
        ),
        _ => Color::from_hex("#FFFFFF").unwrap(),
    }
}

fn font(vm: &VmState, val: &Value) -> Font {
    let name = match val {
        Value::Str(s) => s.as_str(&vm.symbols).to_lowercase(),
        _ => "medium".to_string(),
    };
    match name.as_str() {
        "small" => Font::Small,
        "large" => Font::Large,
        "xlarge" | "xxlarge" => Font::XLarge,
        _ => Font::Medium,
    }
}

fn text_val(vm: &VmState, val: &Value) -> String {
    match val {
        Value::Str(s) => s.as_str(&vm.symbols),
        Value::Int(i) => i.to_string(),
        Value::Num(n) => n.to_string(),
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

pub fn register_gfx(vm: &mut VmState) -> Result<()> {
    let gfx = LuaTable::new();

    // fill_rounded_rect(x, y, w, h, r, c)
    // args[0]=self, args[1..6]=x,y,w,h,r,c
    let id = vm.register_native_id(|vm, args| {
        let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::FillRoundedRect {
            x: val_i32(&args[1]) + ox,
            y: val_i32(&args[2]) + oy,
            w: val_u32(&args[3]),
            h: val_u32(&args[4]),
            radius: val_u32(&args[5]),
            color: color(vm, &args[6]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("fill_rounded_rect");
    gfx.set_sym(sym, Value::NativeFn(id));

    // stroke_rounded_rect(x, y, w, h, r, c, t)
    // args[0]=self, args[1..7]=x,y,w,h,r,c,t
    let id = vm.register_native_id(|vm, args| {
        let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::StrokeRoundedRect {
            x: val_i32(&args[1]) + ox,
            y: val_i32(&args[2]) + oy,
            w: val_u32(&args[3]),
            h: val_u32(&args[4]),
            radius: val_u32(&args[5]),
            color: color(vm, &args[6]),
            thickness: val_u32(&args[7]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("stroke_rounded_rect");
    gfx.set_sym(sym, Value::NativeFn(id));

    // fill_circle(cx, cy, r, c)
    // args[0]=self, args[1..4]=cx,cy,r,c
    let id = vm.register_native_id(|vm, args| {
        let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::FillCircle {
            cx: val_i32(&args[1]) + ox,
            cy: val_i32(&args[2]) + oy,
            radius: val_u32(&args[3]),
            color: color(vm, &args[4]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("fill_circle");
    gfx.set_sym(sym, Value::NativeFn(id));

    // line(x1, y1, x2, y2, c, t)
    // args[0]=self, args[1..6]=x1,y1,x2,y2,c,t
    let id = vm.register_native_id(|vm, args| {
        let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::Line {
            x1: val_i32(&args[1]) + ox,
            y1: val_i32(&args[2]) + oy,
            x2: val_i32(&args[3]) + ox,
            y2: val_i32(&args[4]) + oy,
            color: color(vm, &args[5]),
            thickness: val_u32(&args[6]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("line");
    gfx.set_sym(sym, Value::NativeFn(id));

    // text(x, y, txt, c, f)
    // args[0]=self, args[1..5]=x,y,txt,c,f
    let id = vm.register_native_id(|vm, args| {
        let (ox, oy) = DRAW_COMMANDS.with(|dc| dc.get_offset());
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::Text {
            x: val_i32(&args[1]) + ox,
            y: val_i32(&args[2]) + oy,
            text: text_val(vm, &args[3]),
            color: color(vm, &args[4]),
            font: font(vm, &args[5]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("text");
    gfx.set_sym(sym, Value::NativeFn(id));

    // clear(c)
    // args[0]=self, args[1]=c
    let id = vm.register_native_id(|vm, args| {
        DRAW_COMMANDS.with(|dc| dc.push(DrawCommand::Clear {
            color: color(vm, &args[1]),
        }));
        Ok(vec![Value::Nil])
    });
    let sym = vm.symbols.intern("clear");
    gfx.set_sym(sym, Value::NativeFn(id));

    vm.set_global("gfx", Value::Table(gfx));
    Ok(())
}
