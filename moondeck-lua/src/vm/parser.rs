use super::ast::*;
use super::error::VmError;
use super::lexer::{Span, SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> SpannedToken {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(SpannedToken {
            token: Token::Eof,
            span: Span { line: 0, col: 0 },
        });
        self.pos += 1;
        t
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<SpannedToken, VmError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            Err(self.error(format!("expected {:?}, got {:?}", expected, self.peek())))
        }
    }

    fn expect_name(&mut self) -> Result<String, VmError> {
        match self.peek().clone() {
            Token::Name(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.error(format!("expected name, got {:?}", other))),
        }
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or(Span { line: 0, col: 0 })
    }

    fn error(&self, msg: impl Into<String>) -> VmError {
        let span = self.current_span();
        VmError::Compile {
            message: msg.into(),
            line: span.line,
            col: span.col,
        }
    }
}

// ─── Public API ─────────────────────────────────────────────────────────────

impl Parser {
    pub fn parse(&mut self) -> Result<Chunk, VmError> {
        let body = self.parse_block()?;
        if !self.check(&Token::Eof) {
            return Err(self.error(format!("unexpected token {:?}", self.peek())));
        }
        Ok(Chunk { body })
    }
}

// ─── Block / Statement parsing ──────────────────────────────────────────────

impl Parser {
    fn parse_block(&mut self) -> Result<Block, VmError> {
        let mut stmts = Vec::new();
        let mut ret = None;

        loop {
            match self.peek() {
                Token::End | Token::Else | Token::ElseIf | Token::Eof => break,
                Token::Return => {
                    self.advance();
                    ret = Some(self.parse_expr_list_optional()?);
                    self.match_token(&Token::Semi);
                    break;
                }
                Token::Semi => {
                    self.advance();
                    continue;
                }
                _ => {
                    if let Some(stmt) = self.parse_stmt()? {
                        stmts.push(stmt);
                    }
                }
            }
        }

        Ok(Block { stmts, ret })
    }

    fn parse_stmt(&mut self) -> Result<Option<Stmt>, VmError> {
        match self.peek().clone() {
            Token::Local => self.parse_local().map(Some),
            Token::If => self.parse_if().map(Some),
            Token::While => self.parse_while().map(Some),
            Token::For => self.parse_for().map(Some),
            Token::Function => self.parse_function_def().map(Some),
            Token::Break => {
                self.advance();
                Ok(Some(Stmt::Break))
            }
            _ => self.parse_expr_stat().map(Some),
        }
    }

    fn parse_local(&mut self) -> Result<Stmt, VmError> {
        self.expect(&Token::Local)?;

        // local function name(...)
        if self.check(&Token::Function) {
            self.advance();
            let name = self.expect_name()?;
            let (params, body) = self.parse_function_body()?;
            return Ok(Stmt::LocalFunction { name, params, body });
        }

        // local name1, name2, ... = expr1, expr2, ...
        let mut names = vec![self.expect_name()?];
        while self.match_token(&Token::Comma) {
            names.push(self.expect_name()?);
        }

        let exprs = if self.match_token(&Token::Assign) {
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        Ok(Stmt::LocalAssign { names, exprs })
    }

    fn parse_if(&mut self) -> Result<Stmt, VmError> {
        self.expect(&Token::If)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::Then)?;
        let then_block = self.parse_block()?;

        let mut elseif_clauses = Vec::new();
        while self.match_token(&Token::ElseIf) {
            let cond = self.parse_expr()?;
            self.expect(&Token::Then)?;
            let block = self.parse_block()?;
            elseif_clauses.push((cond, block));
        }

        let else_block = if self.match_token(&Token::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        self.expect(&Token::End)?;
        Ok(Stmt::If {
            cond,
            then_block,
            elseif_clauses,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, VmError> {
        self.expect(&Token::While)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::Do)?;
        let block = self.parse_block()?;
        self.expect(&Token::End)?;
        Ok(Stmt::While { cond, block })
    }

    fn parse_for(&mut self) -> Result<Stmt, VmError> {
        self.expect(&Token::For)?;
        let first_name = self.expect_name()?;

        if self.match_token(&Token::Assign) {
            // Numeric for: for name = start, limit [, step] do block end
            let start = self.parse_expr()?;
            self.expect(&Token::Comma)?;
            let limit = self.parse_expr()?;
            let step = if self.match_token(&Token::Comma) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(&Token::Do)?;
            let block = self.parse_block()?;
            self.expect(&Token::End)?;
            Ok(Stmt::NumericFor {
                name: first_name,
                start,
                limit,
                step,
                block,
            })
        } else {
            // Generic for: for name1, name2 in expr_list do block end
            let mut names = vec![first_name];
            while self.match_token(&Token::Comma) {
                names.push(self.expect_name()?);
            }
            self.expect(&Token::In)?;
            let iterators = self.parse_expr_list()?;
            self.expect(&Token::Do)?;
            let block = self.parse_block()?;
            self.expect(&Token::End)?;
            Ok(Stmt::GenericFor {
                names,
                iterators,
                block,
            })
        }
    }

    fn parse_function_def(&mut self) -> Result<Stmt, VmError> {
        self.expect(&Token::Function)?;
        let base_name = self.expect_name()?;
        let mut is_method = false;

        // Build dotted path: M.init, M.foo.bar
        let mut target_expr = Expr::Name(base_name);
        loop {
            if self.match_token(&Token::Dot) {
                let field = self.expect_name()?;
                target_expr = Expr::Field {
                    object: Box::new(target_expr),
                    name: field,
                };
            } else if self.match_token(&Token::Colon) {
                let method = self.expect_name()?;
                target_expr = Expr::Field {
                    object: Box::new(target_expr),
                    name: method,
                };
                is_method = true;
                break;
            } else {
                break;
            }
        }

        let target = expr_to_lvalue(&target_expr)?;
        let (mut params, body) = self.parse_function_body()?;
        if is_method {
            params.insert(0, "self".to_string());
        }

        Ok(Stmt::FunctionDef {
            target,
            params,
            is_method,
            body,
        })
    }

    fn parse_function_body(&mut self) -> Result<(Vec<String>, Block), VmError> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            params.push(self.expect_name()?);
            while self.match_token(&Token::Comma) {
                params.push(self.expect_name()?);
            }
        }
        self.expect(&Token::RParen)?;
        let body = self.parse_block()?;
        self.expect(&Token::End)?;
        Ok((params, body))
    }

    fn parse_expr_stat(&mut self) -> Result<Stmt, VmError> {
        let expr = self.parse_suffixed_expr()?;

        // Check for assignment: expr = ... or expr, expr = ...
        if self.check(&Token::Assign) || self.check(&Token::Comma) {
            let mut targets = vec![expr_to_lvalue(&expr)?];
            while self.match_token(&Token::Comma) {
                let e = self.parse_suffixed_expr()?;
                targets.push(expr_to_lvalue(&e)?);
            }
            self.expect(&Token::Assign)?;
            let exprs = self.parse_expr_list()?;
            Ok(Stmt::Assign { targets, exprs })
        } else {
            // Must be a function call statement
            match &expr {
                Expr::Call { .. } | Expr::MethodCall { .. } => Ok(Stmt::ExprStmt(expr)),
                _ => Err(self.error("expected assignment or function call")),
            }
        }
    }
}

// ─── Expression parsing (Pratt-style precedence climbing) ───────────────────

impl Parser {
    fn parse_expr(&mut self) -> Result<Expr, VmError> {
        self.parse_or()
    }

    // or
    fn parse_or(&mut self) -> Result<Expr, VmError> {
        let mut left = self.parse_and()?;
        while self.match_token(&Token::Or) {
            let right = self.parse_and()?;
            left = Expr::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // and
    fn parse_and(&mut self) -> Result<Expr, VmError> {
        let mut left = self.parse_comparison()?;
        while self.match_token(&Token::And) {
            let right = self.parse_comparison()?;
            left = Expr::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // == ~= < <= > >=
    fn parse_comparison(&mut self) -> Result<Expr, VmError> {
        let mut left = self.parse_concat()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // .. (right-associative)
    fn parse_concat(&mut self) -> Result<Expr, VmError> {
        let left = self.parse_add()?;
        if self.match_token(&Token::DotDot) {
            let right = self.parse_concat()?; // right-recursive for right-associativity
            Ok(Expr::BinOp {
                op: BinOp::Concat,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    // + -
    fn parse_add(&mut self) -> Result<Expr, VmError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // * / %
    fn parse_mul(&mut self) -> Result<Expr, VmError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // unary: - not #
    fn parse_unary(&mut self) -> Result<Expr, VmError> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnOp {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnOp {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Hash => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnOp {
                    op: UnOp::Len,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    // Postfix: field access, indexing, calls
    fn parse_postfix(&mut self) -> Result<Expr, VmError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek().clone() {
                Token::Dot => {
                    self.advance();
                    let name = self.expect_name()?;
                    expr = Expr::Field {
                        object: Box::new(expr),
                        name,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let key = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        key: Box::new(key),
                    };
                }
                Token::LParen => {
                    let args = self.parse_call_args()?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                    };
                }
                Token::Colon => {
                    self.advance();
                    let method = self.expect_name()?;
                    let args = self.parse_call_args()?;
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method,
                        args,
                    };
                }
                Token::LBrace => {
                    // f{...} sugar → f({...})
                    let tbl = self.parse_table_constructor()?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args: vec![tbl],
                    };
                }
                Token::String(s) => {
                    // f"str" sugar → f("str")
                    let s = s.clone();
                    self.advance();
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args: vec![Expr::Str(s)],
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    // Also used for assignment LHS parsing (without suffix call)
    fn parse_suffixed_expr(&mut self) -> Result<Expr, VmError> {
        self.parse_postfix()
    }

    fn parse_primary(&mut self) -> Result<Expr, VmError> {
        match self.peek().clone() {
            Token::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            Token::True => {
                self.advance();
                Ok(Expr::True)
            }
            Token::False => {
                self.advance();
                Ok(Expr::False)
            }
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Str(s))
            }
            Token::Name(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Name(s))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBrace => self.parse_table_constructor(),
            Token::Function => {
                self.advance();
                let (params, body) = self.parse_function_body()?;
                Ok(Expr::Function { params, body })
            }
            other => Err(self.error(format!("unexpected token in expression: {:?}", other))),
        }
    }

    fn parse_table_constructor(&mut self) -> Result<Expr, VmError> {
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();

        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            if self.match_token(&Token::LBracket) {
                // [expr] = expr
                let key = self.parse_expr()?;
                self.expect(&Token::RBracket)?;
                self.expect(&Token::Assign)?;
                let value = self.parse_expr()?;
                fields.push(TableField::IndexValue(key, value));
            } else if matches!(self.peek(), Token::Name(_))
                && matches!(self.peek_at(1), Token::Assign)
            {
                // name = expr
                let name = self.expect_name()?;
                self.expect(&Token::Assign)?;
                let value = self.parse_expr()?;
                fields.push(TableField::NameValue(name, value));
            } else {
                // expr (array-style)
                let value = self.parse_expr()?;
                fields.push(TableField::Value(value));
            }

            // Optional separator
            if !self.match_token(&Token::Comma) {
                self.match_token(&Token::Semi);
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Expr::Table(fields))
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, VmError> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            args.push(self.parse_expr()?);
            while self.match_token(&Token::Comma) {
                args.push(self.parse_expr()?);
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, VmError> {
        let mut exprs = vec![self.parse_expr()?];
        while self.match_token(&Token::Comma) {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    fn parse_expr_list_optional(&mut self) -> Result<Vec<Expr>, VmError> {
        // Return statement may have no expressions
        match self.peek() {
            Token::End | Token::Else | Token::ElseIf | Token::Eof | Token::Semi => {
                Ok(Vec::new())
            }
            _ => self.parse_expr_list(),
        }
    }
}

// ─── Utility ────────────────────────────────────────────────────────────────

fn expr_to_lvalue(expr: &Expr) -> Result<LValue, VmError> {
    match expr {
        Expr::Name(name) => Ok(LValue::Name(name.clone())),
        Expr::Field { object, name } => Ok(LValue::Field(Box::new(*object.clone()), name.clone())),
        Expr::Index { object, key } => {
            Ok(LValue::Index(Box::new(*object.clone()), Box::new(*key.clone())))
        }
        _ => Err(VmError::Compile {
            message: "invalid assignment target".to_string(),
            line: 0,
            col: 0,
        }),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::lexer::Lexer;

    fn parse_str(src: &str) -> Result<Chunk, VmError> {
        let tokens = Lexer::new(src).tokenize()?;
        Parser::new(tokens).parse()
    }

    #[test]
    fn local_assign() {
        let chunk = parse_str("local x = 42").unwrap();
        assert_eq!(chunk.body.stmts.len(), 1);
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { names, exprs } => {
                assert_eq!(names, &["x"]);
                assert!(matches!(exprs[0], Expr::Integer(42)));
            }
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn local_multi_assign() {
        let chunk = parse_str("local a, b = 1, 2").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { names, exprs } => {
                assert_eq!(names.len(), 2);
                assert_eq!(exprs.len(), 2);
            }
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn function_def() {
        let chunk = parse_str("function M.init(ctx)\n  return {}\nend").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::FunctionDef { target, params, .. } => {
                assert!(matches!(target, LValue::Field(_, name) if name == "init"));
                assert_eq!(params, &["ctx"]);
            }
            s => panic!("expected FunctionDef, got {:?}", s),
        }
    }

    #[test]
    fn if_elseif_else() {
        let src = "if x then y = 1 elseif z then y = 2 else y = 3 end";
        let chunk = parse_str(src).unwrap();
        match &chunk.body.stmts[0] {
            Stmt::If {
                elseif_clauses,
                else_block,
                ..
            } => {
                assert_eq!(elseif_clauses.len(), 1);
                assert!(else_block.is_some());
            }
            s => panic!("expected If, got {:?}", s),
        }
    }

    #[test]
    fn numeric_for() {
        let chunk = parse_str("for i = 1, 10 do end").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::NumericFor { name, step, .. } => {
                assert_eq!(name, "i");
                assert!(step.is_none());
            }
            s => panic!("expected NumericFor, got {:?}", s),
        }
    }

    #[test]
    fn numeric_for_with_step() {
        let chunk = parse_str("for i = 1, 10, 2 do end").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::NumericFor { step, .. } => assert!(step.is_some()),
            s => panic!("expected NumericFor, got {:?}", s),
        }
    }

    #[test]
    fn generic_for() {
        let chunk = parse_str("for i, v in ipairs(t) do end").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::GenericFor { names, .. } => {
                assert_eq!(names, &["i", "v"]);
            }
            s => panic!("expected GenericFor, got {:?}", s),
        }
    }

    #[test]
    fn table_constructor() {
        let chunk = parse_str("local t = { x = 1, y = 2, \"hello\" }").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => match &exprs[0] {
                Expr::Table(fields) => assert_eq!(fields.len(), 3),
                e => panic!("expected Table, got {:?}", e),
            },
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn method_call() {
        let chunk = parse_str("gfx:text(10, 20, \"hi\", \"white\", \"large\")").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::ExprStmt(Expr::MethodCall { method, args, .. }) => {
                assert_eq!(method, "text");
                assert_eq!(args.len(), 5);
            }
            s => panic!("expected MethodCall, got {:?}", s),
        }
    }

    #[test]
    fn operator_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let chunk = parse_str("local x = 1 + 2 * 3").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => match &exprs[0] {
                Expr::BinOp {
                    op: BinOp::Add,
                    right,
                    ..
                } => {
                    assert!(matches!(**right, Expr::BinOp { op: BinOp::Mul, .. }));
                }
                e => panic!("expected Add(_, Mul(..)), got {:?}", e),
            },
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn concat_right_assoc() {
        // a .. b .. c should parse as a .. (b .. c)
        let chunk = parse_str("local x = a .. b .. c").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => match &exprs[0] {
                Expr::BinOp {
                    op: BinOp::Concat,
                    right,
                    ..
                } => {
                    assert!(matches!(**right, Expr::BinOp { op: BinOp::Concat, .. }));
                }
                e => panic!("expected Concat(_, Concat(..)), got {:?}", e),
            },
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn while_loop() {
        let chunk = parse_str("while x > 0 do x = x - 1 end").unwrap();
        assert!(matches!(&chunk.body.stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn local_function() {
        let chunk = parse_str("local function pad2(n) return n end").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalFunction { name, params, .. } => {
                assert_eq!(name, "pad2");
                assert_eq!(params, &["n"]);
            }
            s => panic!("expected LocalFunction, got {:?}", s),
        }
    }

    #[test]
    fn return_value() {
        let chunk = parse_str("return M").unwrap();
        assert!(chunk.body.ret.is_some());
        let ret = chunk.body.ret.unwrap();
        assert_eq!(ret.len(), 1);
        assert!(matches!(&ret[0], Expr::Name(n) if n == "M"));
    }

    #[test]
    fn assignment() {
        let chunk = parse_str("x = 1").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::Assign { targets, exprs } => {
                assert!(matches!(&targets[0], LValue::Name(n) if n == "x"));
                assert_eq!(exprs.len(), 1);
            }
            s => panic!("expected Assign, got {:?}", s),
        }
    }

    #[test]
    fn field_assignment() {
        let chunk = parse_str("state.x = 42").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::Assign { targets, .. } => {
                assert!(matches!(&targets[0], LValue::Field(_, name) if name == "x"));
            }
            s => panic!("expected Assign with field target, got {:?}", s),
        }
    }

    #[test]
    fn index_access() {
        let chunk = parse_str("local x = t[1]").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => {
                assert!(matches!(&exprs[0], Expr::Index { .. }));
            }
            s => panic!("expected LocalAssign with Index, got {:?}", s),
        }
    }

    #[test]
    fn unary_ops() {
        let chunk = parse_str("local x = -a + #t").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => match &exprs[0] {
                Expr::BinOp { op: BinOp::Add, left, right } => {
                    assert!(matches!(**left, Expr::UnOp { op: UnOp::Neg, .. }));
                    assert!(matches!(**right, Expr::UnOp { op: UnOp::Len, .. }));
                }
                e => panic!("expected Add(Neg, Len), got {:?}", e),
            },
            s => panic!("expected LocalAssign, got {:?}", s),
        }
    }

    #[test]
    fn anon_function() {
        let chunk = parse_str("local f = function(x) return x end").unwrap();
        match &chunk.body.stmts[0] {
            Stmt::LocalAssign { exprs, .. } => {
                assert!(matches!(&exprs[0], Expr::Function { .. }));
            }
            s => panic!("expected LocalAssign with Function, got {:?}", s),
        }
    }

    #[test]
    fn widget_snippet() {
        let src = r##"
local M = {}

function M.init(ctx)
    return {
        x = ctx.x,
        y = ctx.y,
        width = ctx.width,
        height = ctx.height,
    }
end

function M.render(state, gfx)
    gfx:text(state.x, state.y, "Hello", "#ffffff", "large")
end

return M
"##;
        let chunk = parse_str(src).unwrap();
        assert_eq!(chunk.body.stmts.len(), 3); // local M, M.init def, M.render def
        assert!(chunk.body.ret.is_some());
    }
}
