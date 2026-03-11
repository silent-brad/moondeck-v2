use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Lua, Table, Value};

pub fn register_util(lua: &mut Lua) -> Result<()> {
    lua.try_enter(|ctx| {
        let util_table = Table::new(&ctx);

        // util.word_wrap(text, max_chars) -> table of lines
        util_table.set(
            ctx,
            "word_wrap",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (arg1, arg2, arg3): (Value, Value, Value) = stack.consume(ctx)?;
                let (text_val, max_val) = match arg1 {
                    Value::Table(_) => (arg2, arg3),
                    _ => (arg1, arg2),
                };

                let text_str = match text_val {
                    Value::String(s) => s.to_str().unwrap_or("").to_string(),
                    Value::Nil => String::new(),
                    _ => String::new(),
                };

                let max_chars = match max_val {
                    Value::Integer(n) => n.max(1) as usize,
                    Value::Number(n) => (n as i64).max(1) as usize,
                    _ => 80,
                };

                let lines = word_wrap(&text_str, max_chars);

                let result_table = Table::new(&ctx);
                for (i, line) in lines.iter().enumerate() {
                    result_table.set(ctx, (i + 1) as i64, ctx.intern(line.as_bytes()))?;
                }

                stack.replace(ctx, result_table);
                Ok(CallbackReturn::Return)
            }),
        )?;

        // util.format(fmt, ...) -> string
        // Supports: %.0f, %.1f, %.2f, %.4f, %d, %s, %%
        util_table.set(
            ctx,
            "format",
            Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
                let (arg1, arg2, arg3, arg4, arg5): (Value, Value, Value, Value, Value) =
                    stack.consume(ctx)?;

                let (fmt_val, args) = match arg1 {
                    Value::Table(_) => (arg2, vec![arg3, arg4, arg5]),
                    _ => (arg1, vec![arg2, arg3, arg4]),
                };

                let fmt = match fmt_val {
                    Value::String(s) => s.to_str().unwrap_or("").to_string(),
                    _ => String::new(),
                };

                let result = format_string(&fmt, &args);
                stack.replace(ctx, ctx.intern(result.as_bytes()));
                Ok(CallbackReturn::Return)
            }),
        )?;

        ctx.set_global("util", util_table)?;
        Ok(())
    })?;

    Ok(())
}

fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > max_chars {
                let mut remaining = word;
                while remaining.len() > max_chars {
                    lines.push(remaining[..max_chars].to_string());
                    remaining = &remaining[max_chars..];
                }
                if !remaining.is_empty() {
                    current_line = remaining.to_string();
                }
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            if word.len() > max_chars {
                let mut remaining = word;
                while remaining.len() > max_chars {
                    lines.push(remaining[..max_chars].to_string());
                    remaining = &remaining[max_chars..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn format_string(fmt: &str, args: &[Value]) -> String {
    let mut result = String::new();
    let mut arg_index = 0;
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                if next == '%' {
                    result.push('%');
                    chars.next();
                    continue;
                }

                // Parse format specifier
                let mut spec = String::new();
                spec.push('%');

                // Collect digits and format chars
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
                        spec.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Get format type
                if let Some(fmt_type) = chars.next() {
                    spec.push(fmt_type);

                    if arg_index < args.len() {
                        let formatted = format_value(&spec, &args[arg_index]);
                        result.push_str(&formatted);
                        arg_index += 1;
                    }
                }
            } else {
                result.push('%');
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn format_value(spec: &str, value: &Value) -> String {
    let num = match value {
        Value::Integer(i) => *i as f64,
        Value::Number(f) => *f,
        Value::String(s) => {
            if spec.ends_with('s') {
                return s.to_str().unwrap_or("").to_string();
            }
            s.to_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)
        }
        Value::Nil => return String::new(),
        _ => 0.0,
    };

    // Parse width for zero-padding (e.g., %02d, %03d)
    let inner = &spec[1..spec.len() - 1]; // Remove % and format char
    let zero_pad = inner.starts_with('0');
    let width: usize = inner
        .trim_start_matches('0')
        .trim_start_matches('-')
        .trim_start_matches('+')
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if spec.ends_with('f') {
        // Float formatting
        if let Some(dot_pos) = inner.find('.') {
            let prec_str = &inner[dot_pos + 1..];
            if let Ok(precision) = prec_str.parse::<usize>() {
                return format!("{:.prec$}", num, prec = precision);
            }
        }
        return format!("{:.2}", num);
    } else if spec.ends_with('d') {
        // Integer formatting with optional zero-padding
        let int_val = num as i64;
        if zero_pad && width > 0 {
            return format!("{:0>width$}", int_val, width = width);
        } else if width > 0 {
            return format!("{:>width$}", int_val, width = width);
        }
        return format!("{}", int_val);
    } else if spec.ends_with('s') {
        return format!("{}", num);
    }

    format!("{}", num)
}
