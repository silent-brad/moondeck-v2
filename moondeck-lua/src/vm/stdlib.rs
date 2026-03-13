use super::error::VmError;
use super::runtime::VmState;
use super::value::*;

/// Register all standard library functions into the VM
pub fn register_all(vm: &mut VmState) {
    register_base(vm);
    register_math(vm);
    register_string(vm);
}

// ─── Base library ───────────────────────────────────────────────────────────

fn register_base(vm: &mut VmState) {
    vm.register_native("print", |vm, args| {
        let parts: Vec<String> = args.iter().map(|v| vm.value_to_string(v)).collect();
        log::info!("{}", parts.join("\t"));
        Ok(vec![])
    });

    vm.register_native("tostring", |vm, args| {
        let s = if args.is_empty() {
            "nil".to_string()
        } else {
            vm.value_to_string(&args[0])
        };
        let sym = vm.symbols.intern(&s);
        Ok(vec![Value::Str(LuaString::Interned(sym))])
    });

    vm.register_native("tonumber", |_vm, args| {
        if args.is_empty() {
            return Ok(vec![Value::Nil]);
        }
        match &args[0] {
            Value::Int(i) => Ok(vec![Value::Int(*i)]),
            Value::Num(n) => Ok(vec![Value::Num(*n)]),
            Value::Str(s) => {
                let text = match s {
                    LuaString::Heap(arc) => arc.as_ref().clone(),
                    LuaString::Interned(_) => return Ok(vec![Value::Nil]),
                };
                if let Ok(i) = text.parse::<i64>() {
                    Ok(vec![Value::Int(i)])
                } else if let Ok(n) = text.parse::<f64>() {
                    Ok(vec![Value::Num(n)])
                } else {
                    Ok(vec![Value::Nil])
                }
            }
            _ => Ok(vec![Value::Nil]),
        }
    });

    vm.register_native("type", |_vm, args| {
        let name = if args.is_empty() {
            "nil"
        } else {
            args[0].type_name()
        };
        Ok(vec![Value::Str(LuaString::Heap(std::sync::Arc::new(
            name.to_string(),
        )))])
    });

    vm.register_native("ipairs", |vm, args| {
        if args.is_empty() {
            return Err(VmError::Runtime("bad argument to 'ipairs'".into()));
        }
        let table = args[0].clone();

        // Return: iterator function, table, 0
        let iter_id = vm.register_native_id(move |_vm, iter_args| {
            // iter_args[0] = table, iter_args[1] = index
            let tbl = &iter_args[0];
            let idx = iter_args
                .get(1)
                .and_then(|v| v.as_int())
                .unwrap_or(0)
                + 1;

            match tbl {
                Value::Table(t) => {
                    let val = t.get_int(idx);
                    if val.is_nil() {
                        Ok(vec![Value::Nil])
                    } else {
                        Ok(vec![Value::Int(idx), val])
                    }
                }
                _ => Ok(vec![Value::Nil]),
            }
        });

        Ok(vec![Value::NativeFn(iter_id), table, Value::Int(0)])
    });

    vm.register_native("pairs", |vm, args| {
        if args.is_empty() {
            return Err(VmError::Runtime("bad argument to 'pairs'".into()));
        }
        // Simple pairs: iterate over all table entries
        // For now, just return ipairs behavior (sufficient for the widget use case)
        let table = args[0].clone();
        let iter_id = vm.register_native_id(move |_vm, iter_args| {
            let tbl = &iter_args[0];
            let idx = iter_args
                .get(1)
                .and_then(|v| v.as_int())
                .unwrap_or(0)
                + 1;
            match tbl {
                Value::Table(t) => {
                    let val = t.get_int(idx);
                    if val.is_nil() {
                        Ok(vec![Value::Nil])
                    } else {
                        Ok(vec![Value::Int(idx), val])
                    }
                }
                _ => Ok(vec![Value::Nil]),
            }
        });
        Ok(vec![Value::NativeFn(iter_id), table, Value::Int(0)])
    });

    vm.register_native("error", |_vm, args| {
        let msg = if args.is_empty() {
            "error".to_string()
        } else {
            match &args[0] {
                Value::Str(s) => match s {
                    LuaString::Heap(arc) => arc.as_ref().clone(),
                    LuaString::Interned(sym) => format!("error(sym:{})", sym.0),
                },
                v => format!("{}", v),
            }
        };
        Err(VmError::Runtime(msg))
    });
}

// ─── Math library ───────────────────────────────────────────────────────────

fn register_math(vm: &mut VmState) {
    let math = LuaTable::new();

    let floor_id = vm.register_native_id(|_vm, args| {
        let n = args.first().and_then(|v| v.as_num()).unwrap_or(0.0);
        Ok(vec![Value::Int(n.floor() as i64)])
    });
    let sym = vm.symbols.intern("floor");
    math.set(TableKey::Sym(sym), Value::NativeFn(floor_id));

    let ceil_id = vm.register_native_id(|_vm, args| {
        let n = args.first().and_then(|v| v.as_num()).unwrap_or(0.0);
        Ok(vec![Value::Int(n.ceil() as i64)])
    });
    let sym = vm.symbols.intern("ceil");
    math.set(TableKey::Sym(sym), Value::NativeFn(ceil_id));

    let abs_id = vm.register_native_id(|_vm, args| {
        match args.first() {
            Some(Value::Int(i)) => Ok(vec![Value::Int(i.abs())]),
            Some(Value::Num(n)) => Ok(vec![Value::Num(n.abs())]),
            _ => Ok(vec![Value::Int(0)]),
        }
    });
    let sym = vm.symbols.intern("abs");
    math.set(TableKey::Sym(sym), Value::NativeFn(abs_id));

    let max_id = vm.register_native_id(|_vm, args| {
        if args.is_empty() {
            return Ok(vec![Value::Nil]);
        }
        let mut best = args[0].clone();
        let mut best_n = best.as_num().unwrap_or(f64::NEG_INFINITY);
        for arg in &args[1..] {
            if let Some(n) = arg.as_num() {
                if n > best_n {
                    best = arg.clone();
                    best_n = n;
                }
            }
        }
        Ok(vec![best])
    });
    let sym = vm.symbols.intern("max");
    math.set(TableKey::Sym(sym), Value::NativeFn(max_id));

    let min_id = vm.register_native_id(|_vm, args| {
        if args.is_empty() {
            return Ok(vec![Value::Nil]);
        }
        let mut best = args[0].clone();
        let mut best_n = best.as_num().unwrap_or(f64::INFINITY);
        for arg in &args[1..] {
            if let Some(n) = arg.as_num() {
                if n < best_n {
                    best = arg.clone();
                    best_n = n;
                }
            }
        }
        Ok(vec![best])
    });
    let sym = vm.symbols.intern("min");
    math.set(TableKey::Sym(sym), Value::NativeFn(min_id));

    let sqrt_id = vm.register_native_id(|_vm, args| {
        let n = args.first().and_then(|v| v.as_num()).unwrap_or(0.0);
        Ok(vec![Value::Num(n.sqrt())])
    });
    let sym = vm.symbols.intern("sqrt");
    math.set(TableKey::Sym(sym), Value::NativeFn(sqrt_id));

    vm.set_global("math", Value::Table(math));
}

// ─── String library ─────────────────────────────────────────────────────────

fn register_string(vm: &mut VmState) {
    let string_tbl = LuaTable::new();

    let sub_id = vm.register_native_id(|vm, args| {
        let s = match args.first() {
            Some(v) => vm.value_to_string(v),
            None => return Ok(vec![Value::Str(LuaString::Heap(std::sync::Arc::new(String::new())))]),
        };
        let i = args.get(1).and_then(|v| v.as_int()).unwrap_or(1);
        let j = args.get(2).and_then(|v| v.as_int()).unwrap_or(-1);

        let len = s.len() as i64;
        let start = if i >= 0 {
            (i - 1).max(0) as usize
        } else {
            (len + i).max(0) as usize
        };
        let end = if j >= 0 {
            (j as usize).min(s.len())
        } else {
            (len + j + 1).max(0) as usize
        };

        let result = if start < end && start < s.len() {
            s[start..end.min(s.len())].to_string()
        } else {
            String::new()
        };

        let sym = vm.symbols.intern(&result);
        Ok(vec![Value::Str(LuaString::Interned(sym))])
    });
    let sym = vm.symbols.intern("sub");
    string_tbl.set(TableKey::Sym(sym), Value::NativeFn(sub_id));

    let format_id = vm.register_native_id(|vm, args| {
        let fmt = match args.first() {
            Some(v) => vm.value_to_string(v),
            None => return Ok(vec![Value::Str(LuaString::Heap(std::sync::Arc::new(String::new())))]),
        };

        let result = lua_format(&fmt, &args[1..], vm);
        let sym = vm.symbols.intern(&result);
        Ok(vec![Value::Str(LuaString::Interned(sym))])
    });
    let sym = vm.symbols.intern("format");
    string_tbl.set(TableKey::Sym(sym), Value::NativeFn(format_id));

    let len_id = vm.register_native_id(|vm, args| {
        let s = match args.first() {
            Some(v) => vm.value_to_string(v),
            None => return Ok(vec![Value::Int(0)]),
        };
        Ok(vec![Value::Int(s.len() as i64)])
    });
    let sym = vm.symbols.intern("len");
    string_tbl.set(TableKey::Sym(sym), Value::NativeFn(len_id));

    vm.set_global("string", Value::Table(string_tbl));
}

/// Minimal Lua string.format implementation
fn lua_format(fmt: &str, args: &[Value], vm: &VmState) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            result.push(c);
            continue;
        }

        match chars.peek() {
            Some('%') => {
                result.push('%');
                chars.next();
            }
            Some(_) => {
                let mut spec = String::from("%");
                while chars
                    .peek()
                    .map_or(false, |c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' || *c == '0')
                {
                    spec.push(chars.next().unwrap());
                }
                if let Some(t) = chars.next() {
                    spec.push(t);
                    if arg_idx < args.len() {
                        result.push_str(&format_value(&spec, &args[arg_idx], vm));
                        arg_idx += 1;
                    }
                }
            }
            None => result.push('%'),
        }
    }
    result
}

fn format_value(spec: &str, value: &Value, vm: &VmState) -> String {
    let num = match value {
        Value::Int(i) => *i as f64,
        Value::Num(f) => *f,
        Value::Str(_) if spec.ends_with('s') => return vm.value_to_string(value),
        Value::Str(_) => vm.value_to_string(value).parse().unwrap_or(0.0),
        Value::Nil => return String::new(),
        _ => 0.0,
    };

    let inner = &spec[1..spec.len() - 1];
    let zero_pad = inner.starts_with('0');
    let width: usize = inner
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    match spec.chars().last() {
        Some('f') => inner
            .find('.')
            .and_then(|p| inner[p + 1..].parse().ok())
            .map(|prec: usize| format!("{:.prec$}", num))
            .unwrap_or_else(|| format!("{:.2}", num)),
        Some('d') => {
            let i = num as i64;
            if zero_pad && width > 0 {
                format!("{:0>w$}", i, w = width)
            } else if width > 0 {
                format!("{:>w$}", i, w = width)
            } else {
                i.to_string()
            }
        }
        Some('s') => vm.value_to_string(value),
        _ => num.to_string(),
    }
}
