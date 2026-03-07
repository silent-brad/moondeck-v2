use anyhow::Result;
use moondeck_core::gfx::Color;
use piccolo::{Callback, CallbackReturn, Lua, String as LuaString, Table, Value};
use std::sync::{Arc, Mutex};

use crate::bindings::gfx::{get_draw_commands, get_draw_offset};

fn create_theme_colors_table<'gc>(ctx: piccolo::Context<'gc>, theme_name: &str) -> Table<'gc> {
    let colors = Table::new(&ctx);

    let (bg_card, bg_surface, border_primary, text_primary, text_muted, text_accent, accent_primary, accent_success, accent_error, accent_warning) = match theme_name {
        "light" => (
            "#f5f5f5", "#ffffff", "#e0e0e0",
            "#1a1a1a", "#666666", "#0066cc",
            "#0066cc", "#28a745", "#dc3545", "#ffc107",
        ),
        _ => (
            "#1a1a2e", "#16213e", "#3a3a5e",
            "#ffffff", "#a0a0a0", "#00d4ff",
            "#00d4ff", "#00ff88", "#ff4466", "#ffaa00",
        ),
    };

    let _ = colors.set(ctx, "bg_card", ctx.intern(bg_card.as_bytes()));
    let _ = colors.set(ctx, "bg_surface", ctx.intern(bg_surface.as_bytes()));
    let _ = colors.set(ctx, "border_primary", ctx.intern(border_primary.as_bytes()));
    let _ = colors.set(ctx, "text_primary", ctx.intern(text_primary.as_bytes()));
    let _ = colors.set(ctx, "text_muted", ctx.intern(text_muted.as_bytes()));
    let _ = colors.set(ctx, "text_accent", ctx.intern(text_accent.as_bytes()));
    let _ = colors.set(ctx, "accent_primary", ctx.intern(accent_primary.as_bytes()));
    let _ = colors.set(ctx, "accent_success", ctx.intern(accent_success.as_bytes()));
    let _ = colors.set(ctx, "accent_error", ctx.intern(accent_error.as_bytes()));
    let _ = colors.set(ctx, "accent_warning", ctx.intern(accent_warning.as_bytes()));

    colors
}

pub fn register_modules(lua: &mut Lua) -> Result<()> {
    let current_theme = Arc::new(Mutex::new("dark".to_string()));
    let current_theme_get = current_theme.clone();
    let current_theme_set = current_theme;

    lua.try_enter(|ctx| {
        // Create theme module
        let theme_table = Table::new(&ctx);

        theme_table.set(
            ctx,
            "set",
            Callback::from_fn(&ctx, move |ctx, _exec, mut stack| {
                let (_self_table, theme_name): (Value, LuaString) = stack.consume(ctx)?;
                let theme_str = theme_name.to_str().unwrap_or("dark").to_string();
                *current_theme_set.lock().unwrap() = theme_str;
                stack.replace(ctx, true);
                Ok(CallbackReturn::Return)
            }),
        )?;

        theme_table.set(
            ctx,
            "get",
            Callback::from_fn(&ctx, move |ctx, _exec, mut stack| {
                let theme = current_theme_get.lock().unwrap().clone();
                let colors = create_theme_colors_table(ctx, &theme);
                stack.replace(ctx, colors);
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("__theme_module", theme_table)?;

        // Create layout module (stub)
        let layout_table = Table::new(&ctx);
        layout_table.set(
            ctx,
            "grid",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                stack.replace(ctx, Table::new(&ctx));
                Ok(CallbackReturn::Return)
            }),
        )?;
        ctx.set_global("__layout_module", layout_table)?;

        // Create components module
        let components_table = Table::new(&ctx);

        // components.new - stub
        components_table.set(
            ctx,
            "new",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                stack.replace(ctx, Table::new(&ctx));
                Ok(CallbackReturn::Return)
            }),
        )?;

        // components.card(gfx, x, y, w, h, opts)
        components_table.set(
            ctx,
            "card",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (gfx, x, y, w, h, opts): (Value, i64, i64, i64, i64, Value) = stack.consume(ctx)?;

                let (offset_x, offset_y) = get_draw_offset();
                let abs_x = offset_x + x as u32;
                let abs_y = offset_y + y as u32;

                let mut bg_color = Color::from_hex("#1a1a2e").unwrap_or(Color::BLACK);
                let mut border_color = Color::from_hex("#3a3a5e").unwrap_or(Color::GRAY);

                if let Value::Table(opts_table) = opts {
                    if let Value::String(bg) = opts_table.get(ctx, "bg") {
                        if let Some(c) = Color::from_hex(bg.to_str().unwrap_or("#1a1a2e")) {
                            bg_color = c;
                        }
                    }
                    if let Value::String(border) = opts_table.get(ctx, "border") {
                        if let Some(c) = Color::from_hex(border.to_str().unwrap_or("#3a3a5e")) {
                            border_color = c;
                        }
                    }
                }

                let draw_cmds = get_draw_commands();
                draw_cmds.fill_rect(abs_x, abs_y, w as u32, h as u32, bg_color);
                draw_cmds.stroke_rect(abs_x, abs_y, w as u32, h as u32, border_color, 1);

                stack.replace(ctx, gfx);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // components.title_bar(gfx, x, y, width, title, opts) -> returns height
        components_table.set(
            ctx,
            "title_bar",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_gfx, x, y, _width, title, opts): (Value, i64, i64, i64, LuaString, Value) = stack.consume(ctx)?;

                let (offset_x, offset_y) = get_draw_offset();
                let abs_x = (offset_x as i64 + x) as i32;
                let abs_y = (offset_y as i64 + y) as i32;

                let mut accent_color = Color::from_hex("#00d4ff").unwrap_or(Color::CYAN);

                if let Value::Table(opts_table) = opts {
                    if let Value::String(accent) = opts_table.get(ctx, "accent") {
                        if let Some(c) = Color::from_hex(accent.to_str().unwrap_or("#00d4ff")) {
                            accent_color = c;
                        }
                    }
                }

                let draw_cmds = get_draw_commands();
                let title_str = title.to_str().unwrap_or("Widget");
                draw_cmds.text(abs_x, abs_y, title_str, Color::WHITE, moondeck_core::gfx::Font::Large);
                draw_cmds.line(abs_x, abs_y + 22, abs_x + 60, abs_y + 22, accent_color, 2);

                stack.replace(ctx, 30i64);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // components.loading(gfx, x, y)
        components_table.set(
            ctx,
            "loading",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_gfx, x, y): (Value, i64, i64) = stack.consume(ctx)?;

                let (offset_x, offset_y) = get_draw_offset();
                let abs_x = (offset_x as i64 + x) as i32;
                let abs_y = (offset_y as i64 + y) as i32;

                let draw_cmds = get_draw_commands();
                draw_cmds.text(abs_x, abs_y, "Loading...", Color::from_hex("#a0a0a0").unwrap_or(Color::GRAY), moondeck_core::gfx::Font::Medium);

                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // components.error(gfx, x, y, width, message)
        components_table.set(
            ctx,
            "error",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_gfx, x, y, _width, message): (Value, i64, i64, i64, LuaString) = stack.consume(ctx)?;

                let (offset_x, offset_y) = get_draw_offset();
                let abs_x = (offset_x as i64 + x) as i32;
                let abs_y = (offset_y as i64 + y) as i32;

                let draw_cmds = get_draw_commands();
                let msg = message.to_str().unwrap_or("Error");
                draw_cmds.text(abs_x, abs_y, msg, Color::from_hex("#ff4466").unwrap_or(Color::RED), moondeck_core::gfx::Font::Medium);

                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // components.item_row(gfx, x, y, width, label, value, opts)
        components_table.set(
            ctx,
            "item_row",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (_gfx, x, y, _width, label, value, _opts): (Value, i64, i64, i64, LuaString, LuaString, Value) = stack.consume(ctx)?;

                let (offset_x, offset_y) = get_draw_offset();
                let abs_x = (offset_x as i64 + x) as i32;
                let abs_y = (offset_y as i64 + y) as i32;

                let draw_cmds = get_draw_commands();
                let label_str = label.to_str().unwrap_or("");
                let value_str = value.to_str().unwrap_or("");

                draw_cmds.text(abs_x, abs_y, label_str, Color::from_hex("#a0a0a0").unwrap_or(Color::GRAY), moondeck_core::gfx::Font::Small);
                draw_cmds.text(abs_x + 80, abs_y, value_str, Color::WHITE, moondeck_core::gfx::Font::Small);

                stack.replace(ctx, Value::Nil);
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("__components_module", components_table)?;

        // Create require function
        ctx.set_global(
            "require",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let module_name: LuaString = stack.consume(ctx)?;
                let module_str = module_name.to_str().unwrap_or("");

                let result = match module_str {
                    "theme" => ctx.globals().get(ctx, "__theme_module"),
                    "layout" => ctx.globals().get(ctx, "__layout_module"),
                    "components" => ctx.globals().get(ctx, "__components_module"),
                    _ => Value::Nil,
                };

                if matches!(result, Value::Nil) {
                    let msg = format!("module '{}' not found", module_str);
                    let err_val: Value = ctx.intern(msg.as_bytes()).into();
                    return Err(piccolo::Error::from_value(err_val));
                }

                stack.replace(ctx, result);
                Ok(CallbackReturn::Return)
            }),
        )?;

        Ok(())
    })?;

    Ok(())
}
