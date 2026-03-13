use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Interned string ID for fast comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

/// Lua value type - the central type of the VM
#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(LuaString),
    Table(LuaTable),
    Function(LuaFunction),
    NativeFn(NativeFnId),
}

/// A Lua string - either interned (Symbol) or heap-allocated
#[derive(Clone, Debug)]
pub enum LuaString {
    Interned(Symbol),
    Heap(Arc<String>),
}

/// Reference to a Lua table
#[derive(Clone, Debug)]
pub struct LuaTable(pub Arc<Mutex<TableInner>>);

/// Native function identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeFnId(pub u32);

/// Lua closure reference
#[derive(Clone, Debug)]
pub struct LuaFunction(pub Arc<ClosureObj>);

/// A compiled function prototype
#[derive(Clone, Debug)]
pub struct Proto {
    pub name: Option<String>,
    pub code: Vec<u32>,
    pub constants: Vec<Constant>,
    pub nested: Vec<Proto>,
    pub upvalue_desc: Vec<UpvalueDesc>,
    pub num_locals: u8,
    pub num_params: u8,
}

/// Constant pool entry
#[derive(Clone, Debug)]
pub enum Constant {
    Nil,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(Symbol),
}

/// Upvalue descriptor
#[derive(Clone, Debug, Copy)]
pub struct UpvalueDesc {
    pub is_local: bool,
    pub index: u8,
}

/// Runtime closure object
#[derive(Debug, Clone)]
pub struct ClosureObj {
    pub proto: Arc<Proto>,
    pub upvalues: Vec<Upvalue>,
}

/// Runtime upvalue - either still on the stack or closed over
#[derive(Clone, Debug)]
pub enum Upvalue {
    Open { frame: usize, slot: usize },
    Closed(Value),
}

/// Table key type (restricted - no metatables needed)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TableKey {
    Int(i64),
    Sym(Symbol),
    Str(Arc<String>),
}

/// Internal table storage
#[derive(Debug)]
pub struct TableInner {
    pub array: Vec<Value>,
    pub map: Vec<(TableKey, Value)>,
}

/// Interned string table for fast symbol lookup
pub struct SymbolTable {
    to_string: Vec<String>,
    from_string: HashMap<String, Symbol>,
}

// ---------------------------------------------------------------------------
// Value impls
// ---------------------------------------------------------------------------

impl Value {
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Num(n) => {
                let trunc = *n as i64;
                if (trunc as f64) == *n {
                    Some(trunc)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<String> {
        match self {
            Value::Str(s) => match s {
                LuaString::Interned(_) => None,
                LuaString::Heap(arc) => Some(arc.as_ref().clone()),
            },
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&LuaTable> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&LuaFunction> {
        match self {
            Value::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "boolean",
            Value::Int(_) => "number",
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Table(_) => "table",
            Value::Function(_) => "function",
            Value::NativeFn(_) => "function",
        }
    }

    pub fn to_table_key(&self) -> Option<TableKey> {
        match self {
            Value::Int(i) => Some(TableKey::Int(*i)),
            Value::Num(n) => {
                let trunc = *n as i64;
                if (trunc as f64) == *n {
                    Some(TableKey::Int(trunc))
                } else {
                    None
                }
            }
            Value::Str(LuaString::Interned(sym)) => Some(TableKey::Sym(*sym)),
            Value::Str(LuaString::Heap(arc)) => Some(TableKey::Str(arc.clone())),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Num(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Table(t) => write!(f, "table: {:p}", Arc::as_ptr(&t.0)),
            Value::Function(func) => write!(f, "function: {:p}", Arc::as_ptr(&func.0)),
            Value::NativeFn(id) => write!(f, "function: native#{}", id.0),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Int(a), Value::Num(b)) => (*a as f64) == *b,
            (Value::Num(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Table(a), Value::Table(b)) => Arc::ptr_eq(&a.0, &b.0),
            (Value::Function(a), Value::Function(b)) => Arc::ptr_eq(&a.0, &b.0),
            (Value::NativeFn(a), Value::NativeFn(b)) => a == b,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// LuaString impls
// ---------------------------------------------------------------------------

impl LuaString {
    pub fn as_str(&self, symbols: &SymbolTable) -> String {
        match self {
            LuaString::Interned(sym) => symbols.resolve(*sym).to_owned(),
            LuaString::Heap(arc) => arc.as_ref().clone(),
        }
    }
}

impl PartialEq for LuaString {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LuaString::Interned(a), LuaString::Interned(b)) => a == b,
            (LuaString::Heap(a), LuaString::Heap(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for LuaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaString::Interned(sym) => write!(f, "<sym:{}>", sym.0),
            LuaString::Heap(arc) => write!(f, "{}", arc),
        }
    }
}

// ---------------------------------------------------------------------------
// LuaTable impls
// ---------------------------------------------------------------------------

impl LuaTable {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(TableInner {
            array: Vec::new(),
            map: Vec::new(),
        })))
    }

    pub fn with_capacity(arr: usize, map: usize) -> Self {
        Self(Arc::new(Mutex::new(TableInner {
            array: Vec::with_capacity(arr),
            map: Vec::with_capacity(map),
        })))
    }

    pub fn get(&self, key: &TableKey) -> Value {
        let inner = self.0.lock().unwrap();
        inner.get(key)
    }

    pub fn set(&self, key: TableKey, value: Value) {
        let mut inner = self.0.lock().unwrap();
        inner.set(key, value);
    }

    pub fn get_int(&self, i: i64) -> Value {
        let inner = self.0.lock().unwrap();
        inner.get_int(i)
    }

    pub fn set_int(&self, i: i64, value: Value) {
        let mut inner = self.0.lock().unwrap();
        inner.set_int(i, value);
    }

    pub fn get_sym(&self, s: Symbol) -> Value {
        self.get(&TableKey::Sym(s))
    }

    pub fn set_sym(&self, s: Symbol, value: Value) {
        self.set(TableKey::Sym(s), value);
    }

    pub fn get_str(&self, s: &str, symbols: &SymbolTable) -> Value {
        if let Some(sym) = symbols.lookup(s) {
            self.get(&TableKey::Sym(sym))
        } else {
            let inner = self.0.lock().unwrap();
            for (k, v) in &inner.map {
                if let TableKey::Str(arc) = k {
                    if arc.as_ref() == s {
                        return v.clone();
                    }
                }
            }
            Value::Nil
        }
    }

    pub fn array_len(&self) -> usize {
        let inner = self.0.lock().unwrap();
        inner.array.len()
    }

    pub fn iter(&self) -> Vec<(TableKey, Value)> {
        let inner = self.0.lock().unwrap();
        let mut entries = Vec::new();
        for (i, v) in inner.array.iter().enumerate() {
            if !v.is_nil() {
                entries.push((TableKey::Int(i as i64 + 1), v.clone()));
            }
        }
        for (k, v) in &inner.map {
            if !v.is_nil() {
                entries.push((k.clone(), v.clone()));
            }
        }
        entries
    }
}

impl Default for LuaTable {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TableInner impls
// ---------------------------------------------------------------------------

impl TableInner {
    fn get(&self, key: &TableKey) -> Value {
        match key {
            TableKey::Int(i) => self.get_int(*i),
            _ => {
                for (k, v) in &self.map {
                    if k == key {
                        return v.clone();
                    }
                }
                Value::Nil
            }
        }
    }

    fn set(&mut self, key: TableKey, value: Value) {
        match &key {
            TableKey::Int(i) => {
                self.set_int(*i, value);
            }
            _ => {
                for entry in &mut self.map {
                    if entry.0 == key {
                        if value.is_nil() {
                            // Don't bother removing; just set to Nil
                        }
                        entry.1 = value;
                        return;
                    }
                }
                if !value.is_nil() {
                    self.map.push((key, value));
                }
            }
        }
    }

    fn get_int(&self, i: i64) -> Value {
        if i >= 1 {
            let idx = (i - 1) as usize;
            if idx < self.array.len() {
                return self.array[idx].clone();
            }
        }
        // Fall back to map lookup
        for (k, v) in &self.map {
            if let TableKey::Int(ki) = k {
                if *ki == i {
                    return v.clone();
                }
            }
        }
        Value::Nil
    }

    fn set_int(&mut self, i: i64, value: Value) {
        if i >= 1 {
            let idx = (i - 1) as usize;
            if idx < self.array.len() {
                self.array[idx] = value;
                return;
            }
            // Extend array if index is exactly the next slot
            if idx == self.array.len() {
                self.array.push(value);
                return;
            }
        }
        // Store in map for non-contiguous or non-positive indices
        for entry in &mut self.map {
            if let TableKey::Int(ki) = &entry.0 {
                if *ki == i {
                    entry.1 = value;
                    return;
                }
            }
        }
        if !value.is_nil() {
            self.map.push((TableKey::Int(i), value));
        }
    }
}

// ---------------------------------------------------------------------------
// SymbolTable impls
// ---------------------------------------------------------------------------

impl SymbolTable {
    pub fn new() -> Self {
        let mut st = Self {
            to_string: Vec::new(),
            from_string: HashMap::new(),
        };

        // Pre-intern common Lua keywords and builtins
        let prelude = [
            // Keywords
            "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
            "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
            "until", "while",
            // Common builtins / field names
            "print", "type", "tostring", "tonumber", "error", "pairs", "ipairs", "next",
            "select", "unpack", "require", "pcall", "xpcall", "setmetatable",
            "getmetatable", "rawget", "rawset", "rawlen",
            // Common widget lifecycle
            "init", "update", "render", "on_event",
            // Common table fields
            "n", "insert", "remove", "sort", "concat",
            // String lib
            "string", "format", "sub", "len", "byte", "char", "find", "rep", "lower",
            "upper",
            // Math lib
            "math", "floor", "ceil", "abs", "max", "min", "sqrt", "random",
            // Table lib
            "table",
            // Misc
            "_ENV", "_G", "__index", "__newindex", "__call", "__len", "__tostring",
        ];

        for s in &prelude {
            st.intern(s);
        }

        st
    }

    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.from_string.get(s) {
            return sym;
        }
        let id = self.to_string.len() as u32;
        let sym = Symbol(id);
        self.to_string.push(s.to_owned());
        self.from_string.insert(s.to_owned(), sym);
        sym
    }

    pub fn resolve(&self, s: Symbol) -> &str {
        &self.to_string[s.0 as usize]
    }

    /// Look up a symbol without interning
    pub fn lookup(&self, s: &str) -> Option<Symbol> {
        self.from_string.get(s).copied()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
