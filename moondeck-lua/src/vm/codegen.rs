use super::ast::*;
use super::bytecode::*;
use super::error::VmError;
use super::value::{Constant, Proto, Symbol, SymbolTable, UpvalueDesc};

/// Compile a parsed Lua chunk into a Proto (bytecode function prototype).
pub fn compile(chunk: &Chunk, symbols: &mut SymbolTable) -> Result<Proto, VmError> {
    let mut compiler = FnCompiler::new(None, 0, symbols);
    compiler.compile_block(&chunk.body)?;
    // Ensure the function returns
    if chunk.body.ret.is_none() {
        let r = compiler.alloc_reg();
        compiler.emit_abc(Op::LoadNil, r, 0, 0);
        compiler.emit_abc(Op::Return, compiler.next_reg - 1, 2, 0);
    }
    Ok(compiler.finish())
}

// ─── Scope / Local tracking ────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Local {
    name: String,
    slot: u8,
    depth: usize,
    captured: bool,
}

#[derive(Debug, Clone)]
struct UpvalDesc {
    is_local: bool,
    index: u8,
}

// ─── Function Compiler ─────────────────────────────────────────────────────

struct FnCompiler<'s> {
    name: Option<String>,
    code: Vec<u32>,
    constants: Vec<Constant>,
    nested: Vec<Proto>,
    locals: Vec<Local>,
    upvalues: Vec<UpvalDesc>,
    scope_depth: usize,
    next_reg: u8,
    max_reg: u8,
    num_params: u8,
    symbols: &'s mut SymbolTable,
    loop_breaks: Vec<Vec<usize>>, // stack of break patch lists per loop
}

impl<'s> FnCompiler<'s> {
    fn new(name: Option<String>, num_params: u8, symbols: &'s mut SymbolTable) -> Self {
        Self {
            name,
            code: Vec::new(),
            constants: Vec::new(),
            nested: Vec::new(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            next_reg: 0,
            max_reg: 0,
            num_params,
            symbols,
            loop_breaks: Vec::new(),
        }
    }

    fn finish(self) -> Proto {
        Proto {
            name: self.name,
            code: self.code,
            constants: self.constants,
            nested: self.nested,
            upvalue_desc: self
                .upvalues
                .into_iter()
                .map(|u| UpvalueDesc {
                    is_local: u.is_local,
                    index: u.index,
                })
                .collect(),
            num_locals: self.max_reg,
            num_params: self.num_params,
        }
    }

    // ── Register allocation ─────────────────────────────────────────────

    fn alloc_reg(&mut self) -> u8 {
        let r = self.next_reg;
        self.next_reg += 1;
        if self.next_reg > self.max_reg {
            self.max_reg = self.next_reg;
        }
        r
    }

    fn free_reg(&mut self) {
        if self.next_reg > 0 {
            self.next_reg -= 1;
        }
    }

    fn save_top(&self) -> u8 {
        self.next_reg
    }

    fn restore_top(&mut self, top: u8) {
        self.next_reg = top;
    }

    // ── Constant pool ───────────────────────────────────────────────────

    fn add_const(&mut self, c: Constant) -> u32 {
        // Check for duplicates
        for (i, existing) in self.constants.iter().enumerate() {
            let eq = match (existing, &c) {
                (Constant::Nil, Constant::Nil) => true,
                (Constant::Bool(a), Constant::Bool(b)) => a == b,
                (Constant::Int(a), Constant::Int(b)) => a == b,
                (Constant::Num(a), Constant::Num(b)) => a == b,
                (Constant::Str(a), Constant::Str(b)) => a == b,
                _ => false,
            };
            if eq {
                return i as u32;
            }
        }
        let idx = self.constants.len() as u32;
        self.constants.push(c);
        idx
    }

    fn add_str_const(&mut self, s: &str) -> u32 {
        let sym = self.symbols.intern(s);
        self.add_const(Constant::Str(sym))
    }

    fn symbol(&mut self, s: &str) -> Symbol {
        self.symbols.intern(s)
    }

    // ── Scope management ────────────────────────────────────────────────

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        while let Some(local) = self.locals.last() {
            if local.depth <= self.scope_depth {
                break;
            }
            if local.captured {
                self.emit_abc(Op::Close, local.slot, 0, 0);
            }
            self.locals.pop();
            self.free_reg();
        }
    }

    fn declare_local(&mut self, name: &str) -> u8 {
        let slot = self.alloc_reg();
        self.locals.push(Local {
            name: name.to_string(),
            slot,
            depth: self.scope_depth,
            captured: false,
        });
        slot
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        for local in self.locals.iter().rev() {
            if local.name == name {
                return Some(local.slot);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, _name: &str) -> Option<u8> {
        // For now, upvalues are not resolved across compiler boundaries in this
        // single-pass design. They will be handled when we have nested compilers.
        // Return None to fall back to global lookup.
        None
    }

    fn mark_local_captured(&mut self, name: &str) {
        for local in self.locals.iter_mut().rev() {
            if local.name == name {
                local.captured = true;
                return;
            }
        }
    }

    // ── Instruction emission ────────────────────────────────────────────

    fn emit(&mut self, instr: u32) -> usize {
        let pos = self.code.len();
        self.code.push(instr);
        pos
    }

    fn emit_abc(&mut self, op: Op, a: u8, b: u16, c: u16) -> usize {
        self.emit(encode_abc(op, a, b, c))
    }

    fn emit_abx(&mut self, op: Op, a: u8, bx: u32) -> usize {
        self.emit(encode_abx(op, a, bx))
    }

    fn emit_asbx(&mut self, op: Op, a: u8, sbx: i32) -> usize {
        self.emit(encode_asbx(op, a, sbx))
    }

    fn emit_jmp(&mut self, offset: i32) -> usize {
        self.emit(encode_sbx(Op::Jmp, offset))
    }

    fn emit_jmp_placeholder(&mut self) -> usize {
        self.emit_jmp(0) // will be patched
    }

    fn patch_jmp(&mut self, instr_pos: usize, target: usize) {
        let offset = target as i32 - instr_pos as i32 - 1;
        self.code[instr_pos] = encode_sbx(Op::Jmp, offset);
    }

    fn patch_jmp_if_not(&mut self, instr_pos: usize, cond_reg: u8, target: usize) {
        let offset = target as i32 - instr_pos as i32 - 1;
        self.code[instr_pos] = encode_asbx(Op::JmpIfNot, cond_reg, offset);
    }

    fn current_pos(&self) -> usize {
        self.code.len()
    }
}

// ─── Block / Statement compilation ─────────────────────────────────────────

impl<'s> FnCompiler<'s> {
    fn compile_block(&mut self, block: &Block) -> Result<(), VmError> {
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        if let Some(ref ret_exprs) = block.ret {
            self.compile_return(ret_exprs)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), VmError> {
        match stmt {
            Stmt::LocalAssign { names, exprs } => self.compile_local_assign(names, exprs),
            Stmt::Assign { targets, exprs } => self.compile_assign(targets, exprs),
            Stmt::ExprStmt(expr) => {
                let top = self.save_top();
                self.compile_expr_discard(expr)?;
                self.restore_top(top);
                Ok(())
            }
            Stmt::If {
                cond,
                then_block,
                elseif_clauses,
                else_block,
            } => self.compile_if(cond, then_block, elseif_clauses, else_block),
            Stmt::While { cond, block } => self.compile_while(cond, block),
            Stmt::NumericFor {
                name,
                start,
                limit,
                step,
                block,
            } => self.compile_numeric_for(name, start, limit, step.as_ref(), block),
            Stmt::GenericFor {
                names,
                iterators,
                block,
            } => self.compile_generic_for(names, iterators, block),
            Stmt::LocalFunction { name, params, body } => {
                self.compile_local_function(name, params, body)
            }
            Stmt::FunctionDef {
                target,
                params,
                is_method,
                body,
            } => self.compile_function_def(target, params, *is_method, body),
            Stmt::Return(exprs) => self.compile_return(exprs),
            Stmt::Break => self.compile_break(),
        }
    }

    fn compile_local_assign(
        &mut self,
        names: &[String],
        exprs: &[Expr],
    ) -> Result<(), VmError> {
        let base = self.next_reg;

        // Compile expressions into consecutive registers
        for (i, expr) in exprs.iter().enumerate() {
            if i < names.len() {
                let reg = self.alloc_reg();
                self.compile_expr_into(expr, reg)?;
            }
        }

        // Fill remaining names with nil
        for _i in exprs.len()..names.len() {
            let reg = self.alloc_reg();
            self.emit_abc(Op::LoadNil, reg, 0, 0);
        }

        // Register locals (pointing to the registers we just allocated)
        for (i, name) in names.iter().enumerate() {
            let slot = base + i as u8;
            self.locals.push(Local {
                name: name.clone(),
                slot,
                depth: self.scope_depth,
                captured: false,
            });
        }

        Ok(())
    }

    fn compile_assign(
        &mut self,
        targets: &[LValue],
        exprs: &[Expr],
    ) -> Result<(), VmError> {
        let top = self.save_top();

        // Compile all RHS exprs into temp registers
        let mut value_regs = Vec::new();
        for expr in exprs {
            let reg = self.alloc_reg();
            self.compile_expr_into(expr, reg)?;
            value_regs.push(reg);
        }

        // Assign each target
        for (i, target) in targets.iter().enumerate() {
            let val_reg = if i < value_regs.len() {
                value_regs[i]
            } else {
                // Extra targets get nil
                let r = self.alloc_reg();
                self.emit_abc(Op::LoadNil, r, 0, 0);
                r
            };

            match target {
                LValue::Name(name) => {
                    if let Some(slot) = self.resolve_local(name) {
                        if slot != val_reg {
                            self.emit_abc(Op::Move, slot, val_reg as u16, 0);
                        }
                    } else {
                        let ki = self.add_str_const(name);
                        self.emit_abx(Op::SetGlobal, val_reg, ki);
                    }
                }
                LValue::Field(obj, field) => {
                    let obj_reg = self.alloc_reg();
                    self.compile_expr_into(obj, obj_reg)?;
                    let ki = self.add_str_const(field);
                    self.emit_abc(Op::SetField, obj_reg, val_reg as u16, ki as u16);
                    self.free_reg(); // obj_reg
                }
                LValue::Index(obj, key) => {
                    let obj_reg = self.alloc_reg();
                    self.compile_expr_into(obj, obj_reg)?;
                    let key_reg = self.alloc_reg();
                    self.compile_expr_into(key, key_reg)?;
                    self.emit_abc(Op::SetTable, obj_reg, key_reg as u16, val_reg as u16);
                    self.free_reg(); // key_reg
                    self.free_reg(); // obj_reg
                }
            }
        }

        self.restore_top(top);
        Ok(())
    }

    fn compile_if(
        &mut self,
        cond: &Expr,
        then_block: &Block,
        elseif_clauses: &[(Expr, Block)],
        else_block: &Option<Block>,
    ) -> Result<(), VmError> {
        let top = self.save_top();

        // Compile condition
        let cond_reg = self.alloc_reg();
        self.compile_expr_into(cond, cond_reg)?;

        // Jump over then-block if false
        let false_jmp = self.current_pos();
        self.emit_asbx(Op::JmpIfNot, cond_reg, 0); // placeholder

        self.restore_top(top);

        // Then block
        self.begin_scope();
        self.compile_block(then_block)?;
        self.end_scope();

        // Collect end-jumps to patch after all branches
        let mut end_jumps = Vec::new();

        if !elseif_clauses.is_empty() || else_block.is_some() {
            let end_jmp = self.emit_jmp_placeholder();
            end_jumps.push(end_jmp);
        }

        // Patch false jump to here
        let here = self.current_pos();
        self.patch_jmp_if_not(false_jmp, cond_reg, here);

        // Elseif clauses
        for (elif_cond, elif_block) in elseif_clauses {
            let top2 = self.save_top();
            let cr = self.alloc_reg();
            self.compile_expr_into(elif_cond, cr)?;

            let elif_false = self.current_pos();
            self.emit_asbx(Op::JmpIfNot, cr, 0);
            self.restore_top(top2);

            self.begin_scope();
            self.compile_block(elif_block)?;
            self.end_scope();

            let end_jmp = self.emit_jmp_placeholder();
            end_jumps.push(end_jmp);

            let here2 = self.current_pos();
            self.patch_jmp_if_not(elif_false, cr, here2);
        }

        // Else block
        if let Some(eb) = else_block {
            self.begin_scope();
            self.compile_block(eb)?;
            self.end_scope();
        }

        // Patch all end jumps
        let end_pos = self.current_pos();
        for jmp in end_jumps {
            self.patch_jmp(jmp, end_pos);
        }

        Ok(())
    }

    fn compile_while(&mut self, cond: &Expr, block: &Block) -> Result<(), VmError> {
        let loop_start = self.current_pos();
        self.loop_breaks.push(Vec::new());

        let top = self.save_top();
        let cond_reg = self.alloc_reg();
        self.compile_expr_into(cond, cond_reg)?;

        let exit_jmp = self.current_pos();
        self.emit_asbx(Op::JmpIfNot, cond_reg, 0);
        self.restore_top(top);

        self.begin_scope();
        self.compile_block(block)?;
        self.end_scope();

        // Jump back to loop start
        let back_offset = loop_start as i32 - self.current_pos() as i32 - 1;
        self.emit_jmp(back_offset);

        let exit_pos = self.current_pos();
        self.patch_jmp_if_not(exit_jmp, cond_reg, exit_pos);

        // Patch breaks
        let breaks = self.loop_breaks.pop().unwrap_or_default();
        for brk in breaks {
            self.patch_jmp(brk, exit_pos);
        }

        Ok(())
    }

    fn compile_numeric_for(
        &mut self,
        name: &str,
        start: &Expr,
        limit: &Expr,
        step: Option<&Expr>,
        block: &Block,
    ) -> Result<(), VmError> {
        self.begin_scope();
        self.loop_breaks.push(Vec::new());

        // Allocate 4 consecutive registers: init, limit, step, loop_var
        let base = self.next_reg;
        let init_reg = self.alloc_reg();
        let limit_reg = self.alloc_reg();
        let step_reg = self.alloc_reg();
        let var_reg = self.alloc_reg(); // exposed as the loop variable

        self.compile_expr_into(start, init_reg)?;
        self.compile_expr_into(limit, limit_reg)?;
        if let Some(s) = step {
            self.compile_expr_into(s, step_reg)?;
        } else {
            let ki = self.add_const(Constant::Int(1));
            self.emit_abx(Op::LoadConst, step_reg, ki);
        }

        // Register the loop variable as a local
        self.locals.push(Local {
            name: name.to_string(),
            slot: var_reg,
            depth: self.scope_depth,
            captured: false,
        });

        // ForPrep: R[A] -= R[A+2], then jump to ForLoop check
        let prep_pos = self.current_pos();
        self.emit_asbx(Op::ForPrep, base, 0); // placeholder

        let loop_body = self.current_pos();

        // Compile body
        self.begin_scope();
        self.compile_block(block)?;
        self.end_scope();

        // ForLoop: R[A] += R[A+2]; if R[A] <= R[A+1] { R[A+3] = R[A]; jump to body }
        let loop_pos = self.current_pos();
        let back_offset = loop_body as i32 - loop_pos as i32 - 1;
        self.emit_asbx(Op::ForLoop, base, back_offset);

        // Patch ForPrep to jump to ForLoop
        let prep_offset = loop_pos as i32 - prep_pos as i32 - 1;
        self.code[prep_pos] = encode_asbx(Op::ForPrep, base, prep_offset);

        let exit_pos = self.current_pos();
        let breaks = self.loop_breaks.pop().unwrap_or_default();
        for brk in breaks {
            self.patch_jmp(brk, exit_pos);
        }

        self.end_scope();
        Ok(())
    }

    fn compile_generic_for(
        &mut self,
        names: &[String],
        iterators: &[Expr],
        block: &Block,
    ) -> Result<(), VmError> {
        // Compile `for k, v in ipairs(t) do ... end` as:
        //   local __iter, __state, __ctrl = <iterators>
        //   while true do
        //     local k, v = __iter(__state, __ctrl)
        //     if k == nil then break end
        //     __ctrl = k
        //     <body>
        //   end
        // For simplicity, we just expand into the equivalent bytecode.

        self.begin_scope();
        self.loop_breaks.push(Vec::new());

        // Allocate 3 hidden registers for iterator state
        let iter_reg = self.alloc_reg();
        let state_reg = self.alloc_reg();
        let ctrl_reg = self.alloc_reg();

        // Compile iterator expressions (typically just `ipairs(t)`)
        // ipairs returns: iterator_fn, table, 0
        if let Some(first) = iterators.first() {
            self.compile_expr_into(first, iter_reg)?;
            // If the iterator is a call, it may return multiple values
            // For now, just handle single-expression iterators
        }
        // Initialize state and ctrl to nil (they'll be set by the call)
        self.emit_abc(Op::LoadNil, state_reg, 0, 0);
        self.emit_abc(Op::LoadNil, ctrl_reg, 0, 0);

        let loop_start = self.current_pos();

        // Call iterator: results go into name registers
        let name_base = self.next_reg;
        for name in names {
            self.declare_local(name);
        }

        // CALL iter_reg with (state, ctrl) -> names.len() results
        // We need to set up: R[iter_reg] already has func,
        // copy state and ctrl as args
        let call_base = self.alloc_reg();
        self.emit_abc(Op::Move, call_base, iter_reg as u16, 0);
        let arg1 = self.alloc_reg();
        self.emit_abc(Op::Move, arg1, state_reg as u16, 0);
        let arg2 = self.alloc_reg();
        self.emit_abc(Op::Move, arg2, ctrl_reg as u16, 0);
        self.emit_abc(
            Op::Call,
            call_base,
            3, // 2 args + 1
            names.len() as u16 + 1,
        );

        // Move results into name locals
        for (i, _) in names.iter().enumerate() {
            if call_base + i as u8 != name_base + i as u8 {
                self.emit_abc(
                    Op::Move,
                    name_base + i as u8,
                    (call_base + i as u8) as u16,
                    0,
                );
            }
        }
        self.restore_top(name_base + names.len() as u8);

        // Check if first result is nil → break
        let nil_check = self.alloc_reg();
        self.emit_abc(Op::LoadNil, nil_check, 0, 0);
        let cmp_reg = self.alloc_reg();
        self.emit_abc(Op::Eq, cmp_reg, name_base as u16, nil_check as u16);
        let exit_jmp = self.current_pos();
        self.emit_asbx(Op::JmpIf, cmp_reg, 0); // if first == nil, exit
        self.free_reg(); // cmp_reg
        self.free_reg(); // nil_check

        // Update control variable
        self.emit_abc(Op::Move, ctrl_reg, name_base as u16, 0);

        // Compile body
        self.begin_scope();
        self.compile_block(block)?;
        self.end_scope();

        // Jump back to loop start
        let back = loop_start as i32 - self.current_pos() as i32 - 1;
        self.emit_jmp(back);

        let exit_pos = self.current_pos();
        // Patch exit jump
        self.code[exit_jmp] = encode_asbx(Op::JmpIf, cmp_reg, exit_pos as i32 - exit_jmp as i32 - 1);

        let breaks = self.loop_breaks.pop().unwrap_or_default();
        for brk in breaks {
            self.patch_jmp(brk, exit_pos);
        }

        self.end_scope();
        Ok(())
    }

    fn compile_local_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &Block,
    ) -> Result<(), VmError> {
        let slot = self.declare_local(name);
        let proto = self.compile_nested_function(Some(name), params, body)?;
        let proto_idx = self.nested.len();
        self.nested.push(proto);
        self.emit_abx(Op::Closure, slot, proto_idx as u32);
        Ok(())
    }

    fn compile_function_def(
        &mut self,
        target: &LValue,
        params: &[String],
        _is_method: bool,
        body: &Block,
    ) -> Result<(), VmError> {
        let fname = match target {
            LValue::Name(n) => Some(n.as_str()),
            LValue::Field(_, n) => Some(n.as_str()),
            _ => None,
        };

        let proto = self.compile_nested_function(fname, params, body)?;
        let proto_idx = self.nested.len();
        self.nested.push(proto);

        let top = self.save_top();
        let fn_reg = self.alloc_reg();
        self.emit_abx(Op::Closure, fn_reg, proto_idx as u32);

        match target {
            LValue::Name(name) => {
                if let Some(slot) = self.resolve_local(name) {
                    self.emit_abc(Op::Move, slot, fn_reg as u16, 0);
                } else {
                    let ki = self.add_str_const(name);
                    self.emit_abx(Op::SetGlobal, fn_reg, ki);
                }
            }
            LValue::Field(obj, field) => {
                let obj_reg = self.alloc_reg();
                self.compile_expr_into(obj, obj_reg)?;
                let ki = self.add_str_const(field);
                self.emit_abc(Op::SetField, obj_reg, fn_reg as u16, ki as u16);
            }
            LValue::Index(obj, key) => {
                let obj_reg = self.alloc_reg();
                self.compile_expr_into(obj, obj_reg)?;
                let key_reg = self.alloc_reg();
                self.compile_expr_into(key, key_reg)?;
                self.emit_abc(Op::SetTable, obj_reg, key_reg as u16, fn_reg as u16);
            }
        }

        self.restore_top(top);
        Ok(())
    }

    fn compile_nested_function(
        &mut self,
        name: Option<&str>,
        params: &[String],
        body: &Block,
    ) -> Result<Proto, VmError> {
        // We need to create a new compiler for the nested function.
        // However, we can't easily borrow self.symbols mutably twice.
        // So we temporarily take it out.
        let symbols = self.symbols as *mut SymbolTable;
        // SAFETY: We don't use self.symbols while the nested compiler is alive,
        // and we restore it before returning.
        let sym_ref = unsafe { &mut *symbols };

        let mut nested = FnCompiler::new(
            name.map(|n| n.to_string()),
            params.len() as u8,
            sym_ref,
        );

        // Declare parameters as locals
        for param in params {
            nested.declare_local(param);
        }

        nested.compile_block(body)?;

        // Ensure return
        if body.ret.is_none() {
            let r = nested.alloc_reg();
            nested.emit_abc(Op::LoadNil, r, 0, 0);
            nested.emit_abc(Op::Return, nested.next_reg - 1, 2, 0);
        }

        Ok(nested.finish())
    }

    fn compile_return(&mut self, exprs: &[Expr]) -> Result<(), VmError> {
        if exprs.is_empty() {
            let r = self.alloc_reg();
            self.emit_abc(Op::LoadNil, r, 0, 0);
            self.emit_abc(Op::Return, r, 1, 0);
            self.free_reg();
        } else {
            let base = self.next_reg;
            for expr in exprs {
                let reg = self.alloc_reg();
                self.compile_expr_into(expr, reg)?;
            }
            self.emit_abc(Op::Return, base, exprs.len() as u16 + 1, 0);
        }
        Ok(())
    }

    fn compile_break(&mut self) -> Result<(), VmError> {
        if self.loop_breaks.is_empty() {
            return Err(VmError::Compile {
                message: "break outside loop".to_string(),
                line: 0,
                col: 0,
            });
        }
        let jmp = self.emit_jmp_placeholder();
        self.loop_breaks.last_mut().unwrap().push(jmp);
        Ok(())
    }
}

// ─── Expression compilation ────────────────────────────────────────────────

impl<'s> FnCompiler<'s> {
    /// Compile expression and discard result (for statement expressions like function calls)
    fn compile_expr_discard(&mut self, expr: &Expr) -> Result<(), VmError> {
        let top = self.save_top();
        match expr {
            Expr::Call { func, args } => {
                let base = self.next_reg;
                let fn_reg = self.alloc_reg();
                self.compile_expr_into(func, fn_reg)?;
                for arg in args {
                    let ar = self.alloc_reg();
                    self.compile_expr_into(arg, ar)?;
                }
                self.emit_abc(Op::Call, base, args.len() as u16 + 1, 1); // 0 results
                self.restore_top(top);
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                self.compile_method_call_into(object, method, args, None)?;
                self.restore_top(top);
            }
            _ => {
                let reg = self.alloc_reg();
                self.compile_expr_into(expr, reg)?;
                self.restore_top(top);
            }
        }
        Ok(())
    }

    /// Compile an expression, placing the result in register `dst`
    fn compile_expr_into(&mut self, expr: &Expr, dst: u8) -> Result<(), VmError> {
        match expr {
            Expr::Nil => {
                self.emit_abc(Op::LoadNil, dst, 0, 0);
            }
            Expr::True => {
                self.emit_abc(Op::LoadTrue, dst, 0, 0);
            }
            Expr::False => {
                self.emit_abc(Op::LoadFalse, dst, 0, 0);
            }
            Expr::Integer(n) => {
                let ki = self.add_const(Constant::Int(*n));
                self.emit_abx(Op::LoadConst, dst, ki);
            }
            Expr::Number(n) => {
                let ki = self.add_const(Constant::Num(*n));
                self.emit_abx(Op::LoadConst, dst, ki);
            }
            Expr::Str(s) => {
                let ki = self.add_str_const(s);
                self.emit_abx(Op::LoadConst, dst, ki);
            }
            Expr::Name(name) => {
                if let Some(slot) = self.resolve_local(name) {
                    if slot != dst {
                        self.emit_abc(Op::Move, dst, slot as u16, 0);
                    }
                } else if let Some(uv) = self.resolve_upvalue(name) {
                    self.emit_abc(Op::GetUpval, dst, uv as u16, 0);
                } else {
                    let ki = self.add_str_const(name);
                    self.emit_abx(Op::GetGlobal, dst, ki);
                }
            }
            Expr::BinOp { op, left, right } => {
                self.compile_binop(*op, left, right, dst)?;
            }
            Expr::UnOp { op, operand } => {
                self.compile_unop(*op, operand, dst)?;
            }
            Expr::Table(fields) => {
                self.compile_table(fields, dst)?;
            }
            Expr::Call { func, args } => {
                self.compile_call(func, args, dst)?;
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                self.compile_method_call_into(object, method, args, Some(dst))?;
            }
            Expr::Field { object, name } => {
                let top = self.save_top();
                let obj_reg = if dst == self.next_reg {
                    self.alloc_reg()
                } else {
                    let r = self.alloc_reg();
                    r
                };
                self.compile_expr_into(object, obj_reg)?;
                let ki = self.add_str_const(name);
                self.emit_abc(Op::GetField, dst, obj_reg as u16, ki as u16);
                self.restore_top(top);
            }
            Expr::Index { object, key } => {
                let top = self.save_top();
                let obj_reg = self.alloc_reg();
                self.compile_expr_into(object, obj_reg)?;
                let key_reg = self.alloc_reg();
                self.compile_expr_into(key, key_reg)?;
                self.emit_abc(Op::GetTable, dst, obj_reg as u16, key_reg as u16);
                self.restore_top(top);
            }
            Expr::Function { params, body } => {
                let proto = self.compile_nested_function(None, params, body)?;
                let idx = self.nested.len();
                self.nested.push(proto);
                self.emit_abx(Op::Closure, dst, idx as u32);
            }
        }
        Ok(())
    }

    fn compile_binop(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        dst: u8,
    ) -> Result<(), VmError> {
        // Short-circuit: and / or
        match op {
            BinOp::And => return self.compile_and(left, right, dst),
            BinOp::Or => return self.compile_or(left, right, dst),
            _ => {}
        }

        let top = self.save_top();
        let left_reg = self.alloc_reg();
        self.compile_expr_into(left, left_reg)?;
        let right_reg = self.alloc_reg();
        self.compile_expr_into(right, right_reg)?;

        let bc_op = match op {
            BinOp::Add => Op::Add,
            BinOp::Sub => Op::Sub,
            BinOp::Mul => Op::Mul,
            BinOp::Div => Op::Div,
            BinOp::Mod => Op::Mod,
            BinOp::Concat => Op::Concat,
            BinOp::Eq => Op::Eq,
            BinOp::Ne => Op::Ne,
            BinOp::Lt => Op::Lt,
            BinOp::Le => Op::Le,
            BinOp::Gt => Op::Gt,
            BinOp::Ge => Op::Ge,
            BinOp::And | BinOp::Or => unreachable!(),
        };

        self.emit_abc(bc_op, dst, left_reg as u16, right_reg as u16);
        self.restore_top(top);
        Ok(())
    }

    fn compile_and(&mut self, left: &Expr, right: &Expr, dst: u8) -> Result<(), VmError> {
        // `a and b`: if a is falsy, result is a; otherwise result is b
        self.compile_expr_into(left, dst)?;
        let skip = self.current_pos();
        self.emit_asbx(Op::JmpIfNot, dst, 0); // if falsy, skip right
        self.compile_expr_into(right, dst)?;
        let end = self.current_pos();
        self.code[skip] = encode_asbx(Op::JmpIfNot, dst, end as i32 - skip as i32 - 1);
        Ok(())
    }

    fn compile_or(&mut self, left: &Expr, right: &Expr, dst: u8) -> Result<(), VmError> {
        // `a or b`: if a is truthy, result is a; otherwise result is b
        self.compile_expr_into(left, dst)?;
        let skip = self.current_pos();
        self.emit_asbx(Op::JmpIf, dst, 0); // if truthy, skip right
        self.compile_expr_into(right, dst)?;
        let end = self.current_pos();
        self.code[skip] = encode_asbx(Op::JmpIf, dst, end as i32 - skip as i32 - 1);
        Ok(())
    }

    fn compile_unop(&mut self, op: UnOp, operand: &Expr, dst: u8) -> Result<(), VmError> {
        let top = self.save_top();
        let src = self.alloc_reg();
        self.compile_expr_into(operand, src)?;

        match op {
            UnOp::Neg => self.emit_abc(Op::Unm, dst, src as u16, 0),
            UnOp::Not => self.emit_abc(Op::Not, dst, src as u16, 0),
            UnOp::Len => self.emit_abc(Op::Len, dst, src as u16, 0),
        };

        self.restore_top(top);
        Ok(())
    }

    fn compile_table(&mut self, fields: &[TableField], dst: u8) -> Result<(), VmError> {
        let arr_count = fields
            .iter()
            .filter(|f| matches!(f, TableField::Value(_)))
            .count();
        let hash_count = fields.len() - arr_count;

        self.emit_abc(Op::NewTable, dst, arr_count as u16, hash_count as u16);

        let mut array_idx = 1i64;

        for field in fields {
            match field {
                TableField::Value(expr) => {
                    let top = self.save_top();
                    let val_reg = self.alloc_reg();
                    self.compile_expr_into(expr, val_reg)?;
                    // SetIndex: R[dst][array_idx] = R[val_reg]
                    self.emit_abc(Op::SetIndex, dst, array_idx as u16, val_reg as u16);
                    array_idx += 1;
                    self.restore_top(top);
                }
                TableField::NameValue(name, expr) => {
                    let top = self.save_top();
                    let val_reg = self.alloc_reg();
                    self.compile_expr_into(expr, val_reg)?;
                    let ki = self.add_str_const(name);
                    self.emit_abc(Op::SetField, dst, val_reg as u16, ki as u16);
                    self.restore_top(top);
                }
                TableField::IndexValue(key, value) => {
                    let top = self.save_top();
                    let key_reg = self.alloc_reg();
                    self.compile_expr_into(key, key_reg)?;
                    let val_reg = self.alloc_reg();
                    self.compile_expr_into(value, val_reg)?;
                    self.emit_abc(Op::SetTable, dst, key_reg as u16, val_reg as u16);
                    self.restore_top(top);
                }
            }
        }

        Ok(())
    }

    fn compile_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
        dst: u8,
    ) -> Result<(), VmError> {
        let top = self.save_top();

        // Place function and args in consecutive registers starting at a temporary base
        let base = self.next_reg;
        let fn_reg = self.alloc_reg();
        self.compile_expr_into(func, fn_reg)?;

        for arg in args {
            let ar = self.alloc_reg();
            self.compile_expr_into(arg, ar)?;
        }

        // B = nargs + 1, C = nresults + 1 (C=2 means 1 result)
        self.emit_abc(Op::Call, base, args.len() as u16 + 1, 2);

        // Move result to dst if needed
        if base != dst {
            self.emit_abc(Op::Move, dst, base as u16, 0);
        }

        self.restore_top(top);
        Ok(())
    }

    fn compile_method_call_into(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        dst: Option<u8>,
    ) -> Result<(), VmError> {
        let top = self.save_top();

        // Method call: obj:method(args) → obj.method(obj, args)
        let base = self.next_reg;

        // Get the method function
        let fn_reg = self.alloc_reg();
        let obj_reg = self.alloc_reg();
        self.compile_expr_into(object, obj_reg)?;

        let ki = self.add_str_const(method);
        self.emit_abc(Op::GetField, fn_reg, obj_reg as u16, ki as u16);

        // First arg is the object itself (self)
        // obj_reg is already allocated, we need it as first arg after fn
        // Layout: [fn_reg] [obj_reg] [arg1] [arg2] ...
        // But fn_reg and obj_reg may not be adjacent to base, let me restructure.
        // Actually we need: R[base] = method_fn, R[base+1] = obj, R[base+2..] = args

        // Move fn to base if needed
        if fn_reg != base {
            self.emit_abc(Op::Move, base, fn_reg as u16, 0);
        }
        // Place obj as first arg
        let self_reg = base + 1;
        if obj_reg != self_reg {
            self.emit_abc(Op::Move, self_reg, obj_reg as u16, 0);
        }

        // Adjust next_reg to be after self
        self.next_reg = base + 2;

        for arg in args {
            let ar = self.alloc_reg();
            self.compile_expr_into(arg, ar)?;
        }

        let nargs = args.len() as u16 + 1 + 1; // +1 for self, +1 for Call encoding
        let nresults = if dst.is_some() { 2u16 } else { 1 };
        self.emit_abc(Op::Call, base, nargs, nresults);

        if let Some(d) = dst {
            if base != d {
                self.emit_abc(Op::Move, d, base as u16, 0);
            }
        }

        self.restore_top(top);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::lexer::Lexer;
    use super::super::parser::Parser;

    fn compile_str(src: &str) -> Proto {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let chunk = Parser::new(tokens).parse().unwrap();
        let mut symbols = SymbolTable::new();
        compile(&chunk, &mut symbols).unwrap()
    }

    #[test]
    fn compile_return_42() {
        let proto = compile_str("return 42");
        assert!(!proto.code.is_empty());
        assert!(proto.constants.iter().any(|c| matches!(c, Constant::Int(42))));
    }

    #[test]
    fn compile_local_and_return() {
        let proto = compile_str("local x = 10\nreturn x");
        assert!(!proto.code.is_empty());
    }

    #[test]
    fn compile_table() {
        let proto = compile_str("return { x = 1, y = 2 }");
        assert!(!proto.code.is_empty());
    }

    #[test]
    fn compile_if() {
        let proto = compile_str("local x = 1\nif x then\n  x = 2\nend");
        assert!(!proto.code.is_empty());
    }

    #[test]
    fn compile_function() {
        let proto = compile_str("local M = {}\nfunction M.init(ctx)\n  return {}\nend\nreturn M");
        assert!(!proto.code.is_empty());
        assert!(!proto.nested.is_empty());
    }

    #[test]
    fn compile_for_loop() {
        let proto = compile_str("local s = 0\nfor i = 1, 10 do\n  s = s + i\nend\nreturn s");
        assert!(!proto.code.is_empty());
    }

    #[test]
    fn compile_method_call() {
        let proto = compile_str("gfx:text(10, 20, \"hello\")");
        assert!(!proto.code.is_empty());
    }
}
