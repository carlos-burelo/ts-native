use super::Compiler;
use crate::chunk::{Literal, PoolEntry};
use crate::scope::LoopContext;
use tsn_core::ast::{
    Decl, ExportDecl, ForInit, NamespaceDecl, Pattern, Program, Stmt, SumTypeDecl,
};
use tsn_core::OpCode;

impl Compiler {
    pub fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        for stmt in &program.body {
            self.compile_stmt(stmt)?;
        }
        self.emit(OpCode::OpPushNull);
        self.emit(OpCode::OpReturn);
        Ok(())
    }

    pub(super) fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        self.line = stmt.range().start.line;
        match stmt {
            Stmt::Empty { .. } => {}

            Stmt::Expr { expression, .. } => {
                self.compile_expr(expression)?;
                self.emit_smart_pop();
            }

            Stmt::Block { stmts, .. } => {
                self.scope.push_block();
                for s in stmts {
                    self.compile_stmt(s)?;
                }
                self.emit_dispose_cleanup()?;
                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }
            }

            Stmt::Decl(decl) => self.compile_decl(decl)?,

            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.compile_expr(test)?;
                let then_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.compile_stmt(consequent)?;

                if let Some(alt) = alternate {
                    let else_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(then_jump);
                    self.emit(OpCode::OpPop);
                    self.compile_stmt(alt)?;
                    self.patch_jump(else_jump);
                } else {
                    let over_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(then_jump);
                    self.emit(OpCode::OpPop);
                    self.patch_jump(over_jump);
                }
            }

            Stmt::While { test, body, .. } => {
                let loop_start = self.chunk_len();
                self.compile_expr(test)?;
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);

                let locals_base = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_base,
                    locals_at_body_start: locals_base,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::DoWhile { body, test, .. } => {
                let loop_start = self.chunk_len();
                let locals_base = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_base,
                    locals_at_body_start: locals_base,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }
                self.compile_expr(test)?;
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                let locals_before_for = self.scope.local_count();
                self.scope.push_block();

                if let Some(init) = init {
                    match init.as_ref() {
                        ForInit::Var {
                            kind: _,
                            declarators,
                        } => {
                            for d in declarators {
                                if let Some(init_expr) = &d.init {
                                    self.compile_expr(init_expr)?;
                                } else {
                                    self.emit(OpCode::OpPushNull);
                                }
                                self.declare_pattern_local(&d.id)?;
                            }
                        }
                        ForInit::Expr(expr) => {
                            self.compile_expr(expr)?;
                            self.emit(OpCode::OpPop);
                        }
                    }
                }

                let loop_start = self.chunk_len();
                let mut exit_jump = None;
                if let Some(test_expr) = test {
                    self.compile_expr(test_expr)?;
                    let j = self.emit_jump(OpCode::OpJumpIfFalse);
                    exit_jump = Some(j);
                    self.emit(OpCode::OpPop);
                }

                let locals_after_init = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_before_for,
                    locals_at_body_start: locals_after_init,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                if let Some(update_expr) = update {
                    self.compile_expr(update_expr)?;
                    self.emit(OpCode::OpPop);
                }

                self.emit_loop(loop_start);

                if let Some(j) = exit_jump {
                    self.patch_jump(j);
                    self.emit(OpCode::OpPop);
                }

                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::ForIn {
                kind: _,
                left,
                right,
                body,
                ..
            } => {
                self.compile_expr(right)?;
                self.emit(OpCode::OpObjectKeys);

                let arr_slot = self.scope.declare_local("__for_keys__");

                let zero = self.add_const(PoolEntry::Literal(Literal::Int(0)));
                self.emit1(OpCode::OpPushConst, zero);
                let idx_slot = self.scope.declare_local("__for_kidx__");

                let loop_start = self.chunk_len();
                self.emit1(OpCode::OpGetLocal, idx_slot);
                self.emit1(OpCode::OpGetLocal, arr_slot);
                let length_key = self.add_str("length");
                let cs = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, length_key, cs);
                self.emit(OpCode::OpLt);
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);

                self.emit1(OpCode::OpGetLocal, arr_slot);
                self.emit1(OpCode::OpGetLocal, idx_slot);
                self.emit(OpCode::OpGetIndex);

                let scope_was = self.scope.local_count();
                self.assign_pattern(left)?;

                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],

                    locals_before_hidden: arr_slot as usize,

                    locals_at_body_start: scope_was,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let added = self.scope.local_count() - scope_was;
                for _ in 0..added {
                    self.emit(OpCode::OpPop);
                }

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                self.emit1(OpCode::OpGetLocal, idx_slot);
                let one = self.add_const(PoolEntry::Literal(Literal::Int(1)));
                self.emit1(OpCode::OpPushConst, one);
                self.emit(OpCode::OpAdd);
                self.emit1(OpCode::OpSetLocalDrop, idx_slot);

                self.emit_loop(loop_start);

                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                self.scope.locals.truncate(arr_slot as usize);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::ForOf {
                kind: _,
                left,
                right,
                body,
                is_await,
                ..
            } => {
                use tsn_types::value::SymbolKind;

                self.compile_expr(right)?;
                let iterable_slot = self.scope.declare_local("__for_iterable__");

                self.emit1(OpCode::OpGetLocal, iterable_slot);
                self.emit1(OpCode::OpGetLocal, iterable_slot);
                let symbol_kind = if *is_await {
                    SymbolKind::AsyncIterator
                } else {
                    SymbolKind::Iterator
                };
                let sym_idx = self.chunk.add_symbol(symbol_kind);
                self.emit1(OpCode::OpGetSymbol, sym_idx);
                self.emit(OpCode::OpSwap);
                self.emit1(OpCode::OpCall, 1);
                let iterator_slot = self.scope.declare_local("__for_iterator__");

                let loop_start = self.chunk_len();

                self.emit1(OpCode::OpGetLocal, iterator_slot);
                self.emit1(OpCode::OpGetLocal, iterator_slot);
                let next_key = self.add_str("next");
                let cs_next = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, next_key, cs_next);
                self.emit(OpCode::OpSwap);
                self.emit1(OpCode::OpCall, 1);

                if *is_await {
                    self.emit(OpCode::OpAwait);
                }

                self.emit(OpCode::OpDup);

                let done_key = self.add_str("done");
                let cs_done = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, done_key, cs_done);

                let exit_jump = self.emit_jump(OpCode::OpJumpIfTrue);

                self.emit(OpCode::OpPop);

                let value_key = self.add_str("value");
                let cs_val = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, value_key, cs_val);

                self.scope.push_block();
                let scope_was = self.scope.local_count();
                self.assign_pattern(left)?;

                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: iterable_slot as usize,
                    locals_at_body_start: scope_was,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                self.emit_loop(loop_start);

                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                self.scope.locals.truncate(iterable_slot as usize);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.compile_expr(discriminant)?;
                let exit_jumps: Vec<usize> = vec![];
                let mut next_jumps: Vec<usize> = vec![];

                for case in cases {
                    for p in next_jumps.drain(..) {
                        self.patch_jump(p);
                    }

                    if let Some(test) = &case.test {
                        self.emit(OpCode::OpDup);
                        self.compile_expr(test)?;
                        self.emit(OpCode::OpEq);
                        let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        next_jumps.push(skip);
                    }

                    for s in &case.body {
                        self.compile_stmt(s)?;
                    }
                }

                for p in next_jumps {
                    self.patch_jump(p);
                }
                self.emit(OpCode::OpPop);

                for p in exit_jumps {
                    self.patch_jump(p);
                }
            }

            Stmt::Return { argument, .. } => {
                if let Some(val) = argument {
                    self.compile_expr(val)?;
                } else {
                    self.emit(OpCode::OpPushNull);
                }
                // Inline pending finally blocks (innermost → outermost) and pop
                // try handlers before leaving the function frame early.
                let try_depth = self.try_stack.len();
                let stashed: Vec<Option<Stmt>> = self.try_stack.clone();
                for entry in stashed.iter().rev() {
                    if let Some(fin_stmt) = entry {
                        self.compile_stmt(fin_stmt)?;
                    }
                }
                for _ in 0..try_depth {
                    self.emit(OpCode::OpPopTry);
                }
                self.emit(OpCode::OpReturn);
            }

            Stmt::Break { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    let to_pop = self.scope.local_count() - ctx.locals_before_hidden;
                    for _ in 0..to_pop {
                        self.emit(OpCode::OpPop);
                    }
                    let p = self.chunk.emit_jump(OpCode::OpJump, self.line);
                    self.loop_stack.last_mut().unwrap().break_patches.push(p);
                }
            }

            Stmt::Continue { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    let to_pop = self.scope.local_count() - ctx.locals_at_body_start;
                    for _ in 0..to_pop {
                        self.emit(OpCode::OpPop);
                    }
                    let p = self.chunk.emit_jump(OpCode::OpJump, self.line);
                    self.loop_stack.last_mut().unwrap().continue_patches.push(p);
                }
            }

            Stmt::Throw { argument, .. } => {
                self.compile_expr(argument)?;
                self.emit(OpCode::OpThrow);
            }

            Stmt::Try {
                block,
                catch,
                finally,
                ..
            } => {
                // Push the finally body (if any) so that early returns inside the
                // try block can inline it before the OpReturn.
                self.try_stack.push(finally.as_deref().cloned());
                let try_start = self.emit_jump(OpCode::OpTry);
                self.compile_stmt(block)?;
                self.try_stack.pop();
                self.emit(OpCode::OpPopTry);
                let finally_jump = self.emit_jump(OpCode::OpJump);
                self.patch_jump(try_start);

                if let Some(catch_clause) = catch {
                    self.scope.push_block();
                    if let Some(param) = &catch_clause.param {
                        self.declare_pattern_local(param)?;
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                    self.compile_stmt(&catch_clause.body)?;
                    let (_count, captured) = self.scope.pop_block();
                    for is_cap in captured {
                        if is_cap {
                            self.emit(OpCode::OpCloseUpvalue);
                        } else {
                            self.emit(OpCode::OpPop);
                        }
                    }
                } else {
                    self.emit(OpCode::OpPop);
                }

                self.patch_jump(finally_jump);

                if let Some(fin) = finally {
                    self.compile_stmt(fin)?;
                }
            }

            Stmt::Labeled { body, .. } => {
                self.compile_stmt(body)?;
            }

            Stmt::Using {
                declarations,
                is_await,
                ..
            } => {
                for d in declarations {
                    if let Some(init) = &d.init {
                        self.compile_expr(init)?;
                    } else {
                        self.emit(OpCode::OpPushNull);
                    }
                    let name = match &d.id {
                        Pattern::Identifier { name, .. } => name.clone(),
                        _ => {
                            return Err("using only supports simple identifier patterns".to_owned())
                        }
                    };
                    let slot = self.scope.declare_local(&name);
                    self.scope
                        .disposables
                        .push((slot, *is_await, self.scope.depth));
                }
            }

            Stmt::Debugger { .. } => {}
        }
        Ok(())
    }

    pub(super) fn emit_dispose_cleanup(&mut self) -> Result<(), String> {
        let disposables = self.scope.disposables_at_current_depth();
        for (slot, _is_async) in disposables {
            self.emit1(OpCode::OpGetLocal, slot);
            let idx = self.add_str("dispose");
            let cs = self.alloc_cache_slot();
            self.emit2(OpCode::OpGetProperty, idx, cs);
            self.emit1(OpCode::OpCall, 0);
            self.emit(OpCode::OpPop);
        }
        Ok(())
    }

    pub(super) fn compile_decl(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::Variable(v) => {
                if v.is_declare {
                    Ok(())
                } else {
                    self.compile_var_decl(v)
                }
            }
            Decl::Function(f) => {
                if f.modifiers.is_declare {
                    Ok(())
                } else {
                    self.compile_fn_decl(f)
                }
            }
            Decl::Class(c) => {
                if c.modifiers.is_declare {
                    Ok(())
                } else {
                    self.compile_class_decl(c)
                }
            }
            Decl::Import(i) => self.compile_import(i),
            Decl::Export(e) => self.compile_export(e),
            Decl::Interface(_) | Decl::TypeAlias(_) => Ok(()),
            Decl::Enum(en) => self.compile_enum(en),
            Decl::Namespace(ns) => {
                let name_idx = self.add_str(&ns.id);
                self.compile_namespace_as_expr(ns)?;
                self.emit1(OpCode::OpDefineGlobal, name_idx);
                Ok(())
            }
            Decl::Struct(_) => Ok(()),
            Decl::Extension(e) => self.compile_extension_decl(e),
            Decl::SumType(st) => self.compile_sum_type(st),
        }
    }

    fn compile_sum_type(&mut self, st: &SumTypeDecl) -> Result<(), String> {
        for variant in &st.variants {
            if variant.fields.is_empty() {
                let tag_key_idx = self.add_str("__tag");
                self.emit1(OpCode::OpPushConst, tag_key_idx);
                let tag_val_idx = self.add_str(&variant.name);
                self.emit1(OpCode::OpPushConst, tag_val_idx);
                self.emit1(OpCode::OpBuildObject, 1);
                if self.scope.depth == 0 {
                    let name_idx = self.add_str(&variant.name);
                    self.emit1(OpCode::OpDefineGlobal, name_idx);
                } else {
                    self.scope.declare_local(&variant.name);
                }
            } else {
                let proto = self.compile_sum_variant_ctor(&variant.name, &variant.fields)?;
                let upvalues = vec![];
                self.emit_closure(proto, upvalues);
                if self.scope.depth == 0 {
                    let name_idx = self.add_str(&variant.name);
                    self.emit1(OpCode::OpDefineGlobal, name_idx);
                } else {
                    self.scope.declare_local(&variant.name);
                }
            }
        }
        Ok(())
    }

    fn compile_sum_variant_ctor(
        &mut self,
        variant_name: &str,
        fields: &[tsn_core::ast::SumField],
    ) -> Result<crate::chunk::FunctionProto, String> {
        let arity = fields.len();
        let mut c = Compiler::new(variant_name.to_owned(), arity, false, false);
        c.is_function = true;
        c.parent = self as *mut Compiler;
        c.type_annotations = self.type_annotations;
        c.extension_calls = self.extension_calls;
        c.extension_members = self.extension_members;
        c.extension_set_members = self.extension_set_members;
        c.class_slot_registry = self.class_slot_registry.clone();

        for field in fields {
            c.scope.declare_local(&field.name);
        }

        let pair_count = 1 + fields.len() as u16;

        let tag_key_idx = c.add_str("__tag");
        c.emit1(OpCode::OpPushConst, tag_key_idx);
        let tag_val_idx = c.add_str(variant_name);
        c.emit1(OpCode::OpPushConst, tag_val_idx);

        for (i, field) in fields.iter().enumerate() {
            let key_idx = c.add_str(&field.name);
            c.emit1(OpCode::OpPushConst, key_idx);
            c.emit1(OpCode::OpGetLocal, i as u16);
        }

        c.emit1(OpCode::OpBuildObject, pair_count);
        c.emit(OpCode::OpReturn);

        let (proto, _upvalues) = c.finish();
        Ok(proto)
    }
    pub fn compile_namespace_as_expr(&mut self, ns: &NamespaceDecl) -> Result<(), String> {
        self.scope.push_block();
        let base_local_idx = self.scope.local_count() as u16;
        self.emit(OpCode::OpPushNull);
        self.scope.declare_local("__ns_exports__");

        let mut export_count = 0u16;

        for member in &ns.body {
            let inner = if let Decl::Export(ExportDecl::Decl { declaration, .. }) = member {
                declaration.as_ref()
            } else {
                member
            };
            match inner {
                Decl::Function(f) => {
                    self.emit(OpCode::OpPushNull);
                    self.scope.declare_local(&f.id);
                }
                Decl::Class(c) => {
                    if let Some(id) = &c.id {
                        self.emit(OpCode::OpPushNull);
                        self.scope.declare_local(id);
                    }
                }
                Decl::Namespace(n) => {
                    self.emit(OpCode::OpPushNull);
                    self.scope.declare_local(&n.id);
                }
                Decl::Enum(e) => {
                    self.emit(OpCode::OpPushNull);
                    self.scope.declare_local(&e.id);
                }
                Decl::Variable(v) => {
                    for decl in &v.declarators {
                        if let Pattern::Identifier { name, .. } = &decl.id {
                            self.emit(OpCode::OpPushNull);
                            self.scope.declare_local(name);
                        }
                    }
                }
                _ => {}
            }
        }

        for member in &ns.body {
            let inner = if let Decl::Export(ExportDecl::Decl { declaration, .. }) = member {
                declaration.as_ref()
            } else {
                member
            };
            match inner {
                Decl::Function(f) => {
                    let (proto, upvalues) = super::compile_function_with_parent(
                        &f.id,
                        &f.params,
                        &f.body,
                        f.modifiers.is_async,
                        f.modifiers.is_generator,
                        false,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.emit_set_var(&f.id);
                    self.emit(OpCode::OpPop);
                }
                Decl::Class(c) => {
                    self.compile_class_as_expr(c)?;
                    if let Some(id) = &c.id {
                        self.emit_set_var(id);
                        self.emit(OpCode::OpPop);
                    }
                }
                Decl::Namespace(n) => {
                    self.compile_namespace_as_expr(n)?;
                    self.emit_set_var(&n.id);
                    self.emit(OpCode::OpPop);
                }
                Decl::Enum(e) => {
                    self.compile_enum_as_expr(e)?;
                    self.emit_set_var(&e.id);
                    self.emit(OpCode::OpPop);
                }
                Decl::Variable(v) => {
                    for decl in &v.declarators {
                        if let Some(init) = &decl.init {
                            self.compile_expr(init)?;
                        } else {
                            self.emit(OpCode::OpPushNull);
                        }
                        if let Pattern::Identifier { name, .. } = &decl.id {
                            self.emit_set_var(name);
                            self.emit(OpCode::OpPop);
                        }
                    }
                }
                _ => {}
            }
        }

        for member in &ns.body {
            if let Decl::Export(ExportDecl::Decl { declaration, .. }) = member {
                match declaration.as_ref() {
                    Decl::Function(f) => {
                        let key_idx = self.add_str(&f.id);
                        self.emit1(OpCode::OpPushConst, key_idx);
                        self.emit_get_var(&f.id);
                        export_count += 1;
                    }
                    Decl::Class(c) => {
                        if let Some(id) = &c.id {
                            let key_idx = self.add_str(id);
                            self.emit1(OpCode::OpPushConst, key_idx);
                            self.emit_get_var(id);
                            export_count += 1;
                        }
                    }
                    Decl::Namespace(n) => {
                        let key_idx = self.add_str(&n.id);
                        self.emit1(OpCode::OpPushConst, key_idx);
                        self.emit_get_var(&n.id);
                        export_count += 1;
                    }
                    Decl::Enum(e) => {
                        let key_idx = self.add_str(&e.id);
                        self.emit1(OpCode::OpPushConst, key_idx);
                        self.emit_get_var(&e.id);
                        export_count += 1;
                    }
                    Decl::Variable(v) => {
                        for decl in &v.declarators {
                            if let Pattern::Identifier { name, .. } = &decl.id {
                                let key_idx = self.add_str(name);
                                self.emit1(OpCode::OpPushConst, key_idx);
                                self.emit_get_var(name);
                                export_count += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        self.emit1(OpCode::OpBuildObject, export_count);
        self.emit1(OpCode::OpSetLocalDrop, base_local_idx);

        let (_count, captured) = self.scope.pop_block();
        for is_cap in captured.iter().take(_count - 1) {
            if *is_cap {
                self.emit(OpCode::OpCloseUpvalue);
            } else {
                self.emit(OpCode::OpPop);
            }
        }

        Ok(())
    }
}
