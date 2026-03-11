use anyhow::Result;
use piccolo::{Callback, CallbackReturn, Lua, Table, Value};

pub fn register_util(lua: &mut Lua) -> Result<()> {
    lua.try_enter(|ctx| {
        let util = Table::new(&ctx);

        // util.word_wrap(text, max_chars) -> table of lines
        util.set(ctx, "word_wrap", Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
            let (a1, a2, a3): (Value, Value, Value) = stack.consume(ctx)?;
            let (text, max) = if matches!(a1, Value::Table(_)) { (a2, a3) } else { (a1, a2) };

            let text_str = match text {
                Value::String(s) => s.to_str().unwrap_or("").to_string(),
                _ => String::new(),
            };
            let max_chars = match max {
                Value::Integer(n) => n.max(1) as usize,
                Value::Number(n) => (n as i64).max(1) as usize,
                _ => 80,
            };

            let result = Table::new(&ctx);
            for (i, line) in word_wrap(&text_str, max_chars).iter().enumerate() {
                result.set(ctx, (i + 1) as i64, ctx.intern(line.as_bytes()))?;
            }
            stack.replace(ctx, result);
            Ok(CallbackReturn::Return)
        }))?;

        // util.format(fmt, ...) -> string
        util.set(ctx, "format", Callback::from_fn(&ctx, |ctx, _exec, mut stack| {
            let (a1, a2, a3, a4, a5): (Value, Value, Value, Value, Value) = stack.consume(ctx)?;
            let (fmt, args) = if matches!(a1, Value::Table(_)) {
                (a2, vec![a3, a4, a5])
            } else {
                (a1, vec![a2, a3, a4])
            };

            let fmt_str = match fmt {
                Value::String(s) => s.to_str().unwrap_or("").to_string(),
                _ => String::new(),
            };

            stack.replace(ctx, ctx.intern(format_string(&fmt_str, &args).as_bytes()));
            Ok(CallbackReturn::Return)
        }))?;

        ctx.set_global("util", util)?;
        Ok(())
    })?;
    Ok(())
}

fn word_wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        if line.is_empty() {
            if word.len() > max {
                for chunk in word.as_bytes().chunks(max) {
                    lines.push(String::from_utf8_lossy(chunk).into());
                }
            } else {
                line = word.into();
            }
        } else if line.len() + 1 + word.len() <= max {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(line);
            line = if word.len() > max {
                for chunk in word.as_bytes().chunks(max) {
                    lines.push(String::from_utf8_lossy(chunk).into());
                }
                String::new()
            } else {
                word.into()
            };
        }
    }
    if !line.is_empty() { lines.push(line); }
    lines
}

fn format_string(fmt: &str, args: &[Value]) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' { result.push(c); continue; }
        match chars.peek() {
            Some('%') => { result.push('%'); chars.next(); }
            Some(_) => {
                let mut spec = String::from("%");
                while chars.peek().map_or(false, |c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+') {
                    spec.push(chars.next().unwrap());
                }
                if let Some(t) = chars.next() {
                    spec.push(t);
                    if arg_idx < args.len() {
                        result.push_str(&format_value(&spec, &args[arg_idx]));
                        arg_idx += 1;
                    }
                }
            }
            None => result.push('%'),
        }
    }
    result
}

fn format_value(spec: &str, value: &Value) -> String {
    let num = match value {
        Value::Integer(i) => *i as f64,
        Value::Number(f) => *f,
        Value::String(s) if spec.ends_with('s') => return s.to_str().unwrap_or("").into(),
        Value::String(s) => s.to_str().unwrap_or("0").parse().unwrap_or(0.0),
        Value::Nil => return String::new(),
        _ => 0.0,
    };

    let inner = &spec[1..spec.len() - 1];
    let zero_pad = inner.starts_with('0');
    let width: usize = inner.trim_start_matches(|c: char| !c.is_ascii_digit()).split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0);

    match spec.chars().last() {
        Some('f') => inner.find('.').and_then(|p| inner[p + 1..].parse().ok())
            .map(|prec| format!("{:.prec$}", num, prec = prec))
            .unwrap_or_else(|| format!("{:.2}", num)),
        Some('d') => {
            let i = num as i64;
            if zero_pad && width > 0 { format!("{:0>w$}", i, w = width) }
            else if width > 0 { format!("{:>w$}", i, w = width) }
            else { i.to_string() }
        }
        _ => num.to_string(),
    }
}
