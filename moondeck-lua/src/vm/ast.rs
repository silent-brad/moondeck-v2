/// AST node types for the Lua parser.

/// A complete Lua chunk (file or function body)
#[derive(Debug, Clone)]
pub struct Chunk {
    pub body: Block,
}

/// A block of statements with optional return
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub ret: Option<Vec<Expr>>,
}

/// Statement types
#[derive(Debug, Clone)]
pub enum Stmt {
    /// local x, y = expr1, expr2
    LocalAssign {
        names: Vec<String>,
        exprs: Vec<Expr>,
    },
    /// x, t.y = expr1, expr2
    Assign {
        targets: Vec<LValue>,
        exprs: Vec<Expr>,
    },
    /// function call as statement (e.g., `print("hi")`)
    ExprStmt(Expr),
    /// if cond then block [elseif cond then block]* [else block] end
    If {
        cond: Expr,
        then_block: Block,
        elseif_clauses: Vec<(Expr, Block)>,
        else_block: Option<Block>,
    },
    /// while cond do block end
    While {
        cond: Expr,
        block: Block,
    },
    /// for name = start, limit [, step] do block end
    NumericFor {
        name: String,
        start: Expr,
        limit: Expr,
        step: Option<Expr>,
        block: Block,
    },
    /// for names in exprs do block end
    GenericFor {
        names: Vec<String>,
        iterators: Vec<Expr>,
        block: Block,
    },
    /// local function name(...) block end
    LocalFunction {
        name: String,
        params: Vec<String>,
        body: Block,
    },
    /// function name.field(...) block end (or function name:method(...) block end)
    FunctionDef {
        target: LValue,
        params: Vec<String>,
        is_method: bool,
        body: Block,
    },
    /// return exprs
    Return(Vec<Expr>),
    /// break
    Break,
}

/// Left-value for assignment
#[derive(Debug, Clone)]
pub enum LValue {
    Name(String),
    /// table[key]
    Index(Box<Expr>, Box<Expr>),
    /// table.field
    Field(Box<Expr>, String),
}

/// Expression types
#[derive(Debug, Clone)]
pub enum Expr {
    Nil,
    True,
    False,
    Integer(i64),
    Number(f64),
    Str(String),
    Name(String),

    /// Binary operation
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operation
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
    },
    /// Table constructor { [key=val, ...] }
    Table(Vec<TableField>),
    /// Function call: expr(args)
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    /// Method call: expr:name(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    /// Field access: expr.name
    Field {
        object: Box<Expr>,
        name: String,
    },
    /// Index access: expr[key]
    Index {
        object: Box<Expr>,
        key: Box<Expr>,
    },
    /// Anonymous function: function(params) block end
    Function {
        params: Vec<String>,
        body: Block,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Len,
}

/// Table constructor field
#[derive(Debug, Clone)]
pub enum TableField {
    /// { expr, expr, ... } (array-style, sequential integer keys)
    Value(Expr),
    /// { name = expr } (record-style)
    NameValue(String, Expr),
    /// { [expr] = expr } (general key)
    IndexValue(Expr, Expr),
}
