use std::sync::Arc;

use super::bytecode::*;
use super::codegen;
use super::error::VmError;
use super::fuel::Fuel;
use super::lexer::Lexer;
use super::parser::Parser;
use super::value::*;

/// Type alias for native function callbacks
pub type NativeFn = Box<dyn Fn(&mut VmState, &[Value]) -> Result<Vec<Value>, VmError> + Send + Sync>;

/// Call frame on the VM stack
#[derive(Debug)]
struct CallFrame {
    closure: LuaFunction,
    pc: usize,
    base: usize, // base index into the value stack
    num_results: u8,
}

/// The main VM state
pub struct VmState {
    pub symbols: SymbolTable,
    pub globals: LuaTable,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    native_fns: Vec<NativeFn>,
    open_upvalues: Vec<(usize, Arc<std::sync::Mutex<Upvalue>>)>,
    max_stack: usize,
    max_frames: usize,
}

impl VmState {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            globals: LuaTable::new(),
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(16),
            native_fns: Vec::new(),
            open_upvalues: Vec::new(),
            max_stack: 1024,
            max_frames: 64,
        }
    }

    // ── Native function registration ────────────────────────────────────

    pub fn register_native<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&mut VmState, &[Value]) -> Result<Vec<Value>, VmError> + Send + Sync + 'static,
    {
        let id = NativeFnId(self.native_fns.len() as u32);
        self.native_fns.push(Box::new(f));
        let sym = self.symbols.intern(name);
        self.globals
            .set(TableKey::Sym(sym), Value::NativeFn(id));
    }

    pub fn register_native_id<F>(&mut self, f: F) -> NativeFnId
    where
        F: Fn(&mut VmState, &[Value]) -> Result<Vec<Value>, VmError> + Send + Sync + 'static,
    {
        let id = NativeFnId(self.native_fns.len() as u32);
        self.native_fns.push(Box::new(f));
        id
    }

    // ── Global access ───────────────────────────────────────────────────

    pub fn set_global(&mut self, name: &str, value: Value) {
        let sym = self.symbols.intern(name);
        self.globals.set(TableKey::Sym(sym), value);
    }

    pub fn get_global(&self, name: &str) -> Value {
        if let Some(sym) = self.symbols.lookup(name) {
            self.globals.get(&TableKey::Sym(sym))
        } else {
            Value::Nil
        }
    }

    // ── Compilation ─────────────────────────────────────────────────────

    pub fn compile(&mut self, name: Option<&str>, source: &str) -> Result<Proto, VmError> {
        let tokens = Lexer::new(source).tokenize()?;
        let chunk = Parser::new(tokens).parse()?;
        let mut proto = codegen::compile(&chunk, &mut self.symbols)?;
        if name.is_some() {
            proto.name = name.map(|s| s.to_string());
        }
        Ok(proto)
    }

    // ── Execution ───────────────────────────────────────────────────────

    /// Execute a compiled proto as a top-level chunk. Returns the return value(s).
    pub fn execute(&mut self, proto: &Proto, fuel: &mut Fuel) -> Result<Vec<Value>, VmError> {
        let closure = LuaFunction(Arc::new(ClosureObj {
            proto: Arc::new(proto.clone()),
            upvalues: Vec::new(),
        }));

        // Set up stack: base for this call frame
        let base = self.stack.len();

        // Ensure stack has enough room for locals
        let needed = proto.num_locals as usize;
        self.stack.resize(base + needed + 16, Value::Nil);

        self.frames.push(CallFrame {
            closure,
            pc: 0,
            base,
            num_results: 0,
        });

        self.run(fuel)
    }

    /// Execute a Lua source string. Compiles and runs it.
    pub fn exec_string(
        &mut self,
        name: Option<&str>,
        source: &str,
        fuel: &mut Fuel,
    ) -> Result<Vec<Value>, VmError> {
        let proto = self.compile(name, source)?;
        self.execute(&proto, fuel)
    }

    /// Call a Lua function value with the given arguments
    pub fn call_function(
        &mut self,
        func: &Value,
        args: &[Value],
        fuel: &mut Fuel,
    ) -> Result<Vec<Value>, VmError> {
        match func {
            Value::Function(closure) => {
                let base = self.stack.len();
                let proto = &closure.0.proto;
                let needed = proto.num_locals as usize;

                // Push args, padding with nil if needed
                for i in 0..needed.max(args.len()) {
                    if i < args.len() {
                        self.stack.push(args[i].clone());
                    } else {
                        self.stack.push(Value::Nil);
                    }
                }
                // Ensure minimum stack space
                while self.stack.len() < base + needed + 16 {
                    self.stack.push(Value::Nil);
                }

                self.frames.push(CallFrame {
                    closure: closure.clone(),
                    pc: 0,
                    base,
                    num_results: 0,
                });

                self.run(fuel)
            }
            Value::NativeFn(id) => {
                let idx = id.0 as usize;
                if idx >= self.native_fns.len() {
                    return Err(VmError::Runtime("invalid native function".into()));
                }
                // We need to call the native fn. Since it borrows &mut self,
                // we temporarily take it out.
                let f = unsafe {
                    let ptr = &self.native_fns[idx] as *const NativeFn;
                    &*ptr
                };
                f(self, args)
            }
            _ => Err(VmError::Type {
                expected: "function",
                got: func.type_name(),
            }),
        }
    }

    // ── Main interpreter loop ───────────────────────────────────────────

    fn run(&mut self, fuel: &mut Fuel) -> Result<Vec<Value>, VmError> {
        loop {
            if fuel.consume(1) {
                return Err(VmError::OutOfFuel);
            }

            let frame = match self.frames.last() {
                Some(f) => f,
                None => return Ok(vec![Value::Nil]),
            };

            let pc = frame.pc;
            let base = frame.base;
            let proto = frame.closure.0.proto.clone();

            if pc >= proto.code.len() {
                // Implicit return nil at end of function
                self.frames.pop();
                self.stack.truncate(base);
                return Ok(vec![Value::Nil]);
            }

            let instr = proto.code[pc];
            let op = decode_op(instr);
            let a = decode_a(instr) as usize;
            let b = decode_b(instr) as usize;
            let c = decode_c(instr) as usize;
            let bx = decode_bx(instr) as usize;
            let sbx = decode_sbx(instr);

            // Advance PC
            self.frames.last_mut().unwrap().pc = pc + 1;

            // Ensure stack is large enough
            let min_len = base + a + 4;
            if self.stack.len() <= min_len {
                self.stack.resize(min_len + 16, Value::Nil);
            }

            match op {
                Op::Move => {
                    let v = self.stack[base + b].clone();
                    self.stack[base + a] = v;
                }
                Op::LoadNil => {
                    self.stack[base + a] = Value::Nil;
                }
                Op::LoadTrue => {
                    self.stack[base + a] = Value::Bool(true);
                }
                Op::LoadFalse => {
                    self.stack[base + a] = Value::Bool(false);
                }
                Op::LoadConst => {
                    let val = self.const_to_value(&proto.constants[bx]);
                    self.stack[base + a] = val;
                }
                Op::NewTable => {
                    self.stack[base + a] = Value::Table(LuaTable::with_capacity(b, c));
                }
                Op::GetTable => {
                    let tbl = self.stack[base + b].clone();
                    let key = self.stack[base + c].clone();
                    let val = self.table_get(&tbl, &key)?;
                    self.stack[base + a] = val;
                }
                Op::SetTable => {
                    let key = self.stack[base + b].clone();
                    let val = self.stack[base + c].clone();
                    let tbl = self.stack[base + a].clone();
                    self.table_set(&tbl, key, val)?;
                }
                Op::GetField => {
                    let tbl = self.stack[base + b].clone();
                    let key_const = &proto.constants[c];
                    let key = self.const_to_value(key_const);
                    let val = self.table_get(&tbl, &key)?;
                    self.stack[base + a] = val;
                }
                Op::SetField => {
                    // SetField A B C: R[A][constants[C]] = R[B]
                    let val = self.stack[base + b].clone();
                    let key_const = &proto.constants[c];
                    let key = self.const_to_value(key_const);
                    let tbl = self.stack[base + a].clone();
                    self.table_set(&tbl, key, val)?;
                }
                Op::GetIndex => {
                    // R[A] = R[B][C as integer]
                    let tbl = self.stack[base + b].clone();
                    let key = Value::Int(c as i64);
                    let val = self.table_get(&tbl, &key)?;
                    self.stack[base + a] = val;
                }
                Op::SetIndex => {
                    // R[A][B as integer] = R[C]
                    let val = self.stack[base + c].clone();
                    let key = Value::Int(b as i64);
                    let tbl = self.stack[base + a].clone();
                    self.table_set(&tbl, key, val)?;
                }
                Op::GetGlobal => {
                    let key = self.const_to_value(&proto.constants[bx]);
                    if let Some(tk) = key.to_table_key() {
                        let val = self.globals.get(&tk);
                        self.stack[base + a] = val;
                    } else {
                        self.stack[base + a] = Value::Nil;
                    }
                }
                Op::SetGlobal => {
                    let val = self.stack[base + a].clone();
                    let key = self.const_to_value(&proto.constants[bx]);
                    if let Some(tk) = key.to_table_key() {
                        self.globals.set(tk, val);
                    }
                }
                Op::GetUpval => {
                    let uv = &self.frames.last().unwrap().closure.0.upvalues[b];
                    let val = match uv {
                        Upvalue::Closed(v) => v.clone(),
                        Upvalue::Open { frame, slot } => {
                            let f = &self.frames[*frame];
                            self.stack[f.base + *slot].clone()
                        }
                    };
                    self.stack[base + a] = val;
                }
                Op::SetUpval => {
                    let val = self.stack[base + a].clone();
                    let frame_idx = self.frames.len() - 1;
                    let uv = &self.frames[frame_idx].closure.0.upvalues[b];
                    match uv {
                        Upvalue::Closed(_) => {
                            let uv = &mut Arc::make_mut(
                                &mut self.frames[frame_idx].closure.0,
                            )
                            .upvalues[b];
                            if let Upvalue::Closed(ref mut v) = uv {
                                *v = val;
                            }
                        }
                        Upvalue::Open { frame, slot } => {
                            let f_base = self.frames[*frame].base;
                            let s = *slot;
                            self.stack[f_base + s] = val;
                        }
                    }
                }
                Op::Add => self.arith_op(base, a, b, c, |x, y| x + y, |x, y| x + y)?,
                Op::Sub => self.arith_op(base, a, b, c, |x, y| x - y, |x, y| x - y)?,
                Op::Mul => self.arith_op(base, a, b, c, |x, y| x * y, |x, y| x * y)?,
                Op::Div => {
                    let lv = self.stack[base + b].as_num();
                    let rv = self.stack[base + c].as_num();
                    match (lv, rv) {
                        (Some(l), Some(r)) => {
                            self.stack[base + a] = Value::Num(l / r);
                        }
                        _ => {
                            return Err(VmError::Type {
                                expected: "number",
                                got: "non-number",
                            })
                        }
                    }
                }
                Op::Mod => self.arith_op(base, a, b, c, |x, y| x % y, |x, y| x % y)?,
                Op::Unm => {
                    let v = &self.stack[base + b];
                    self.stack[base + a] = match v {
                        Value::Int(i) => Value::Int(-i),
                        Value::Num(n) => Value::Num(-n),
                        _ => {
                            return Err(VmError::Type {
                                expected: "number",
                                got: v.type_name(),
                            })
                        }
                    };
                }
                Op::Concat => {
                    let ls = self.value_to_string(&self.stack[base + b]);
                    let rs = self.value_to_string(&self.stack[base + c]);
                    let result = format!("{}{}", ls, rs);
                    let sym = self.symbols.intern(&result);
                    self.stack[base + a] = Value::Str(LuaString::Interned(sym));
                }
                Op::Len => {
                    let v = &self.stack[base + b];
                    self.stack[base + a] = match v {
                        Value::Table(t) => Value::Int(t.array_len() as i64),
                        Value::Str(s) => {
                            let len = s.as_str(&self.symbols).len();
                            Value::Int(len as i64)
                        }
                        _ => {
                            return Err(VmError::Type {
                                expected: "table or string",
                                got: v.type_name(),
                            })
                        }
                    };
                }
                Op::Eq => {
                    let result = self.stack[base + b] == self.stack[base + c];
                    self.stack[base + a] = Value::Bool(result);
                }
                Op::Ne => {
                    let result = self.stack[base + b] != self.stack[base + c];
                    self.stack[base + a] = Value::Bool(result);
                }
                Op::Lt => self.compare_op(base, a, b, c, |o| matches!(o, std::cmp::Ordering::Less))?,
                Op::Le => self.compare_op(base, a, b, c, |o| {
                    matches!(o, std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                })?,
                Op::Gt => self.compare_op(base, a, b, c, |o| {
                    matches!(o, std::cmp::Ordering::Greater)
                })?,
                Op::Ge => self.compare_op(base, a, b, c, |o| {
                    matches!(o, std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                })?,
                Op::Not => {
                    let v = self.stack[base + b].is_truthy();
                    self.stack[base + a] = Value::Bool(!v);
                }
                Op::TestSet => {
                    // TestSet A B C: if (bool(R[B]) == C) then R[A] = R[B] else pc++
                    let test = self.stack[base + b].is_truthy();
                    if test == (c != 0) {
                        self.stack[base + a] = self.stack[base + b].clone();
                    } else {
                        self.frames.last_mut().unwrap().pc += 1;
                    }
                }
                Op::Jmp => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.pc = (frame.pc as i32 + sbx) as usize;
                }
                Op::JmpIf => {
                    if self.stack[base + a].is_truthy() {
                        let frame = self.frames.last_mut().unwrap();
                        frame.pc = (frame.pc as i32 + sbx) as usize;
                    }
                }
                Op::JmpIfNot => {
                    if !self.stack[base + a].is_truthy() {
                        let frame = self.frames.last_mut().unwrap();
                        frame.pc = (frame.pc as i32 + sbx) as usize;
                    }
                }
                Op::Call => {
                    // Call A B C: call R[A] with B-1 args, expect C-1 results
                    let func_val = self.stack[base + a].clone();
                    let nargs = if b == 0 {
                        0
                    } else {
                        b - 1
                    };
                    let nresults = if c == 0 { 255u8 } else { (c - 1) as u8 };

                    let mut args = Vec::with_capacity(nargs);
                    for i in 0..nargs {
                        let idx = base + a + 1 + i;
                        if idx < self.stack.len() {
                            args.push(self.stack[idx].clone());
                        } else {
                            args.push(Value::Nil);
                        }
                    }

                    let results = self.call_value(&func_val, &args, fuel)?;

                    // Store results
                    let num_to_store = if nresults == 255 {
                        results.len()
                    } else {
                        nresults as usize
                    };

                    for i in 0..num_to_store {
                        let idx = base + a + i;
                        if idx >= self.stack.len() {
                            self.stack.resize(idx + 1, Value::Nil);
                        }
                        self.stack[idx] = if i < results.len() {
                            results[i].clone()
                        } else {
                            Value::Nil
                        };
                    }
                }
                Op::Return => {
                    // Return A B: return B-1 values from R[A..A+B-2]
                    let nvals = if b == 0 { 0 } else { b - 1 };
                    let mut results = Vec::with_capacity(nvals);
                    for i in 0..nvals {
                        let idx = base + a + i;
                        if idx < self.stack.len() {
                            results.push(self.stack[idx].clone());
                        } else {
                            results.push(Value::Nil);
                        }
                    }

                    // Pop frame and clean up stack
                    self.frames.pop();
                    self.stack.truncate(base);

                    // Always return — the caller (Op::Call via call_value)
                    // handles storing results in its own registers.
                    return Ok(results);
                }
                Op::ForPrep => {
                    // R[A] -= R[A+2]; pc += sBx
                    let init = self.stack[base + a].as_num().unwrap_or(0.0);
                    let step = self.stack[base + a + 2].as_num().unwrap_or(1.0);
                    self.stack[base + a] = Value::Num(init - step);
                    let frame = self.frames.last_mut().unwrap();
                    frame.pc = (frame.pc as i32 + sbx) as usize;
                }
                Op::ForLoop => {
                    // R[A] += R[A+2]; if R[A] <= R[A+1] then { pc += sBx; R[A+3] = R[A] }
                    let idx = base + a;
                    let val = self.stack[idx].as_num().unwrap_or(0.0);
                    let step = self.stack[idx + 2].as_num().unwrap_or(1.0);
                    let limit = self.stack[idx + 1].as_num().unwrap_or(0.0);
                    let new_val = val + step;

                    self.stack[idx] = Value::Num(new_val);

                    let continue_loop = if step >= 0.0 {
                        new_val <= limit
                    } else {
                        new_val >= limit
                    };

                    if continue_loop {
                        // Set loop variable
                        if idx + 3 >= self.stack.len() {
                            self.stack.resize(idx + 4, Value::Nil);
                        }
                        self.stack[idx + 3] = if new_val == (new_val as i64 as f64) {
                            Value::Int(new_val as i64)
                        } else {
                            Value::Num(new_val)
                        };
                        let frame = self.frames.last_mut().unwrap();
                        frame.pc = (frame.pc as i32 + sbx) as usize;
                    }
                }
                Op::Closure => {
                    let proto_ref = &self.frames.last().unwrap().closure.0.proto;
                    let nested_proto = proto_ref.nested[bx].clone();

                    // Resolve upvalues
                    let mut upvals = Vec::new();
                    for desc in &nested_proto.upvalue_desc {
                        if desc.is_local {
                            upvals.push(Upvalue::Open {
                                frame: self.frames.len() - 1,
                                slot: desc.index as usize,
                            });
                        } else {
                            let parent_uv =
                                &self.frames.last().unwrap().closure.0.upvalues[desc.index as usize];
                            upvals.push(parent_uv.clone());
                        }
                    }

                    let closure = LuaFunction(Arc::new(ClosureObj {
                        proto: Arc::new(nested_proto),
                        upvalues: upvals,
                    }));
                    self.stack[base + a] = Value::Function(closure);
                }
                Op::Close => {
                    // Close upvalues >= R[A] (not fully implemented yet)
                }
                Op::Nop => {}
            }
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn call_value(
        &mut self,
        func: &Value,
        args: &[Value],
        fuel: &mut Fuel,
    ) -> Result<Vec<Value>, VmError> {
        match func {
            Value::Function(closure) => {
                if self.frames.len() >= self.max_frames {
                    return Err(VmError::StackOverflow);
                }

                let base = self.stack.len();
                let proto = &closure.0.proto;
                let needed = proto.num_locals as usize;

                // Push args into new frame's registers
                for i in 0..needed.max(args.len()).max(proto.num_params as usize) {
                    if i < args.len() {
                        self.stack.push(args[i].clone());
                    } else {
                        self.stack.push(Value::Nil);
                    }
                }
                while self.stack.len() < base + needed + 16 {
                    self.stack.push(Value::Nil);
                }

                self.frames.push(CallFrame {
                    closure: closure.clone(),
                    pc: 0,
                    base,
                    num_results: 0,
                });

                self.run(fuel)
            }
            Value::NativeFn(id) => {
                let idx = id.0 as usize;
                if idx >= self.native_fns.len() {
                    return Err(VmError::Runtime("invalid native function id".into()));
                }
                let f = unsafe {
                    let ptr = &self.native_fns[idx] as *const NativeFn;
                    &*ptr
                };
                f(self, args)
            }
            _ => Err(VmError::Type {
                expected: "function",
                got: func.type_name(),
            }),
        }
    }

    fn const_to_value(&self, c: &Constant) -> Value {
        match c {
            Constant::Nil => Value::Nil,
            Constant::Bool(b) => Value::Bool(*b),
            Constant::Int(i) => Value::Int(*i),
            Constant::Num(n) => Value::Num(*n),
            Constant::Str(sym) => Value::Str(LuaString::Interned(*sym)),
        }
    }

    fn table_get(&self, table: &Value, key: &Value) -> Result<Value, VmError> {
        match table {
            Value::Table(t) => {
                if let Some(tk) = key.to_table_key() {
                    Ok(t.get(&tk))
                } else {
                    Ok(Value::Nil)
                }
            }
            _ => {
                // Try to be lenient — nil indexing returns nil
                if table.is_nil() {
                    return Err(VmError::Runtime("attempt to index a nil value".into()));
                }
                Err(VmError::Type {
                    expected: "table",
                    got: table.type_name(),
                })
            }
        }
    }

    fn table_set(&self, table: &Value, key: Value, value: Value) -> Result<(), VmError> {
        match table {
            Value::Table(t) => {
                if let Some(tk) = key.to_table_key() {
                    t.set(tk, value);
                    Ok(())
                } else {
                    Err(VmError::Runtime("invalid table key".into()))
                }
            }
            _ => Err(VmError::Type {
                expected: "table",
                got: table.type_name(),
            }),
        }
    }

    fn arith_op(
        &mut self,
        base: usize,
        a: usize,
        b: usize,
        c: usize,
        int_op: fn(i64, i64) -> i64,
        num_op: fn(f64, f64) -> f64,
    ) -> Result<(), VmError> {
        let lv = &self.stack[base + b];
        let rv = &self.stack[base + c];

        let result = match (lv, rv) {
            (Value::Int(l), Value::Int(r)) => Value::Int(int_op(*l, *r)),
            (Value::Num(l), Value::Num(r)) => Value::Num(num_op(*l, *r)),
            (Value::Int(l), Value::Num(r)) => Value::Num(num_op(*l as f64, *r)),
            (Value::Num(l), Value::Int(r)) => Value::Num(num_op(*l, *r as f64)),
            _ => {
                return Err(VmError::Runtime(format!(
                    "attempt to perform arithmetic on {} and {}",
                    lv.type_name(),
                    rv.type_name()
                )))
            }
        };

        self.stack[base + a] = result;
        Ok(())
    }

    fn compare_op(
        &mut self,
        base: usize,
        a: usize,
        b: usize,
        c: usize,
        pred: fn(std::cmp::Ordering) -> bool,
    ) -> Result<(), VmError> {
        let lv = &self.stack[base + b];
        let rv = &self.stack[base + c];

        let ord = match (lv, rv) {
            (Value::Int(l), Value::Int(r)) => l.cmp(r),
            (Value::Num(l), Value::Num(r)) => l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Int(l), Value::Num(r)) => (*l as f64)
                .partial_cmp(r)
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Num(l), Value::Int(r)) => l
                .partial_cmp(&(*r as f64))
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(l), Value::Str(r)) => {
                let ls = l.as_str(&self.symbols);
                let rs = r.as_str(&self.symbols);
                ls.cmp(&rs)
            }
            _ => {
                return Err(VmError::Runtime(format!(
                    "attempt to compare {} with {}",
                    lv.type_name(),
                    rv.type_name()
                )))
            }
        };

        self.stack[base + a] = Value::Bool(pred(ord));
        Ok(())
    }

    pub fn value_to_string(&self, v: &Value) -> String {
        match v {
            Value::Nil => "nil".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Num(n) => {
                if *n == (*n as i64 as f64) && n.is_finite() {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Str(s) => s.as_str(&self.symbols),
            Value::Table(t) => format!("table: {:p}", Arc::as_ptr(&t.0)),
            Value::Function(f) => format!("function: {:p}", Arc::as_ptr(&f.0)),
            Value::NativeFn(id) => format!("function: native#{}", id.0),
        }
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_lua(src: &str) -> Vec<Value> {
        let mut vm = VmState::new();
        super::super::stdlib::register_all(&mut vm);
        let mut fuel = Fuel::with(100000);
        vm.exec_string(None, src, &mut fuel).unwrap()
    }

    #[test]
    fn return_int() {
        let r = run_lua("return 42");
        assert_eq!(r.len(), 1);
        assert!(matches!(r[0], Value::Int(42)));
    }

    #[test]
    fn return_string() {
        let r = run_lua("return \"hello\"");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn arithmetic() {
        let r = run_lua("return 2 + 3 * 4");
        assert_eq!(r[0].as_num(), Some(14.0));
    }

    #[test]
    fn local_variable() {
        let r = run_lua("local x = 10\nreturn x");
        assert!(matches!(r[0], Value::Int(10)));
    }

    #[test]
    fn table_field() {
        let r = run_lua("local t = { x = 42 }\nreturn t.x");
        assert!(matches!(r[0], Value::Int(42)));
    }

    #[test]
    fn table_array() {
        let r = run_lua("local t = { 10, 20, 30 }\nreturn t[2]");
        assert!(matches!(r[0], Value::Int(20)));
    }

    #[test]
    fn if_true() {
        let r = run_lua("local x = 1\nif x then\n  x = 2\nend\nreturn x");
        assert!(matches!(r[0], Value::Int(2)));
    }

    #[test]
    fn if_false() {
        let r = run_lua("local x = false\nlocal y = 1\nif x then\n  y = 2\nend\nreturn y");
        assert!(matches!(r[0], Value::Int(1)));
    }

    #[test]
    fn for_loop() {
        let r = run_lua("local s = 0\nfor i = 1, 5 do\n  s = s + i\nend\nreturn s");
        assert_eq!(r[0].as_num(), Some(15.0));
    }

    #[test]
    fn function_call() {
        let r = run_lua("local function add(a, b) return a + b end\nreturn add(3, 4)");
        assert_eq!(r[0].as_num(), Some(7.0));
    }

    #[test]
    fn string_concat() {
        let r = run_lua("return \"hello\" .. \" \" .. \"world\"");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn table_length() {
        let r = run_lua("local t = { 1, 2, 3, 4, 5 }\nreturn #t");
        assert!(matches!(r[0], Value::Int(5)));
    }

    #[test]
    fn boolean_ops() {
        let r = run_lua("return true and false");
        assert!(matches!(r[0], Value::Bool(false)));
    }

    #[test]
    fn or_short_circuit() {
        let r = run_lua("return nil or 42");
        assert!(matches!(r[0], Value::Int(42)));
    }

    #[test]
    fn comparison() {
        let r = run_lua("return 3 > 2");
        assert!(matches!(r[0], Value::Bool(true)));
    }

    #[test]
    fn module_pattern() {
        let r = run_lua(
            "local M = {}\n\
             function M.add(a, b) return a + b end\n\
             return M",
        );
        assert!(matches!(r[0], Value::Table(_)));
    }

    #[test]
    fn module_call_method() {
        // Mimic widget pattern: exec returns table, then call method on it
        let mut vm = VmState::new();
        super::super::stdlib::register_all(&mut vm);
        let mut fuel = Fuel::with(100000);

        let results = vm.exec_string(None,
            "local M = {}\n\
             function M.add(a, b) return a + b end\n\
             function M.greet() return 42 end\n\
             return M",
            &mut fuel,
        ).unwrap();

        let table = match &results[0] {
            Value::Table(t) => t.clone(),
            other => panic!("Expected table, got {:?}", other),
        };

        // Look up "add" function
        let add_fn = table.get_str("add", &vm.symbols);
        assert!(!add_fn.is_nil(), "add function should not be nil");
        assert_eq!(add_fn.type_name(), "function");

        // Look up "greet" function
        let greet_fn = table.get_str("greet", &vm.symbols);
        assert!(!greet_fn.is_nil(), "greet function should not be nil");

        // Call the function
        let mut fuel = Fuel::with(100000);
        let result = vm.call_function(&add_fn, &[Value::Int(3), Value::Int(4)], &mut fuel).unwrap();
        assert_eq!(result[0].as_num(), Some(7.0));
    }

    #[test]
    fn nested_function_call() {
        // Test calling a function within a function (like set_quote in quote.lua)
        let r = run_lua(
            "local function double(x) return x * 2 end\n\
             local function apply(x) return double(x) end\n\
             return apply(21)",
        );
        assert_eq!(r[0].as_num(), Some(42.0));
    }

    #[test]
    fn module_with_local_function() {
        // Test the quote.lua pattern: local function used inside module methods
        let mut vm = VmState::new();
        super::super::stdlib::register_all(&mut vm);
        let mut fuel = Fuel::with(100000);

        let results = vm.exec_string(None,
            "local M = {}\n\
             local data = { 10, 20, 30 }\n\
             local function get_item(i) return data[i] end\n\
             function M.init() return { value = get_item(2) } end\n\
             return M",
            &mut fuel,
        ).unwrap();

        let table = match &results[0] {
            Value::Table(t) => t.clone(),
            other => panic!("Expected table, got {:?}", other),
        };

        let init_fn = table.get_str("init", &vm.symbols);
        assert!(!init_fn.is_nil(), "init should not be nil");

        let mut fuel = Fuel::with(100000);
        let result = vm.call_function(&init_fn, &[], &mut fuel).unwrap();
        match &result[0] {
            Value::Table(t) => {
                let val = t.get_str("value", &vm.symbols);
                assert!(matches!(val, Value::Int(20)), "Expected 20, got {:?}", val);
            }
            other => panic!("Expected table, got {:?}", other),
        }
    }

    #[test]
    fn global_function_call() {
        // Test calling a globally-registered native function from Lua
        let mut vm = VmState::new();
        super::super::stdlib::register_all(&mut vm);

        // Register a native function on a table (mimics theme:get)
        let tbl = LuaTable::new();
        let id = vm.register_native_id(|_vm, _args| {
            Ok(vec![Value::Int(99)])
        });
        let sym = vm.symbols.intern("get");
        tbl.set_sym(sym, Value::NativeFn(id));
        vm.set_global("mytable", Value::Table(tbl));

        let mut fuel = Fuel::with(100000);
        let r = vm.exec_string(None, "return mytable:get()", &mut fuel).unwrap();
        assert!(matches!(r[0], Value::Int(99)), "Expected 99, got {:?}", r[0]);
    }
}
