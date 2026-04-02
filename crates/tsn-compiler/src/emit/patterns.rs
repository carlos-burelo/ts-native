use super::Compiler;
use crate::chunk::{Literal, PoolEntry};
use tsn_core::ast::{Pattern, VarKind, VariableDecl};
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_var_decl(&mut self, decl: &VariableDecl) -> Result<(), String> {
        for d in &decl.declarators {
            if let Some(init) = &d.init {
                self.compile_expr(init)?;
            } else {
                self.emit(OpCode::OpPushNull);
            }

            match decl.kind {
                VarKind::Let | VarKind::Const => {
                    if self.scope.depth > 0 || self.is_function {
                        self.declare_pattern_local(&d.id)?;
                    } else {
                        self.declare_pattern_global(&d.id)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn declare_pattern_local(&mut self, pattern: &Pattern) -> Result<(), String> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                self.scope.declare_local(name);
            }
            Pattern::Array { elements, rest, .. } => {
                let src = self.scope.declare_local("") as u16;
                for (i, el) in elements.iter().enumerate() {
                    if let Some(elem) = el {
                        let idx = self.add_const(PoolEntry::Literal(Literal::Int(i as i64)));
                        self.emit1(OpCode::OpGetLocal, src);
                        self.emit1(OpCode::OpPushConst, idx);
                        self.emit(OpCode::OpGetIndex);
                        self.declare_pattern_local(&elem.pattern)?;
                    }
                }
                if let Some(rest_pat) = rest {
                    let start =
                        self.add_const(PoolEntry::Literal(Literal::Int(elements.len() as i64)));
                    let slice_key = self.add_str("slice");
                    let cs = self.alloc_cache_slot();
                    self.emit1(OpCode::OpGetLocal, src);
                    self.emit2(OpCode::OpGetProperty, slice_key, cs);
                    self.emit1(OpCode::OpPushConst, start);
                    self.emit1(OpCode::OpCall, 1);
                    self.declare_pattern_local(rest_pat)?;
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                let src = self.scope.declare_local("") as u16;
                for prop in properties {
                    let key_idx = self.add_str(&prop.key);
                    let cs = self.alloc_cache_slot();
                    self.emit1(OpCode::OpGetLocal, src);
                    self.emit2(OpCode::OpGetPropertyMaybe, key_idx, cs);
                    self.declare_pattern_local(&prop.value)?;
                }
                if let Some(rest_pat) = rest {
                    let _ = rest_pat;
                    self.emit(OpCode::OpPushNull);
                    self.declare_pattern_local(rest_pat)?;
                }
            }
            Pattern::Assignment { left, right, .. } => {
                self.emit(OpCode::OpDup);
                self.emit(OpCode::OpIsNull);
                let not_null = self.emit_jump(OpCode::OpJumpIfFalse);

                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);
                self.compile_expr(right)?;
                let end = self.emit_jump(OpCode::OpJump);
                self.patch_jump(not_null);

                self.emit(OpCode::OpPop);
                self.patch_jump(end);
                self.declare_pattern_local(left)?;
            }
            Pattern::Rest { argument, .. } => {
                self.declare_pattern_local(argument)?;
            }
        }
        Ok(())
    }

    pub(super) fn declare_pattern_global(&mut self, pattern: &Pattern) -> Result<(), String> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                self.emit_define_global(name);
            }
            Pattern::Array { elements, rest, .. } => {
                let tmp = self.add_str("$__destr__$");
                self.emit1(OpCode::OpDefineGlobal, tmp);
                for (i, el) in elements.iter().enumerate() {
                    if let Some(elem) = el {
                        let idx = self.add_const(PoolEntry::Literal(Literal::Int(i as i64)));
                        self.emit1(OpCode::OpGetGlobal, tmp);
                        self.emit1(OpCode::OpPushConst, idx);
                        self.emit(OpCode::OpGetIndex);
                        self.declare_pattern_global(&elem.pattern)?;
                    }
                }
                if let Some(rest_pat) = rest {
                    let start =
                        self.add_const(PoolEntry::Literal(Literal::Int(elements.len() as i64)));
                    let slice_key = self.add_str("slice");
                    let cs = self.alloc_cache_slot();
                    self.emit1(OpCode::OpGetGlobal, tmp);
                    self.emit2(OpCode::OpGetProperty, slice_key, cs);
                    self.emit1(OpCode::OpPushConst, start);
                    self.emit1(OpCode::OpCall, 1);
                    self.declare_pattern_global(rest_pat)?;
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                let tmp = self.add_str("$__destr__$");
                self.emit1(OpCode::OpDefineGlobal, tmp);
                for prop in properties {
                    let key_idx = self.add_str(&prop.key);
                    let cs = self.alloc_cache_slot();
                    self.emit1(OpCode::OpGetGlobal, tmp);
                    self.emit2(OpCode::OpGetPropertyMaybe, key_idx, cs);
                    self.declare_pattern_global(&prop.value)?;
                }
                if let Some(rest_pat) = rest {
                    let _ = rest_pat;
                    self.emit(OpCode::OpPushNull);
                    self.declare_pattern_global(rest_pat)?;
                }
            }
            Pattern::Assignment { left, right, .. } => {
                self.emit(OpCode::OpDup);
                self.emit(OpCode::OpIsNull);
                let not_null = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);
                self.compile_expr(right)?;
                let end = self.emit_jump(OpCode::OpJump);
                self.patch_jump(not_null);
                self.emit(OpCode::OpPop);
                self.patch_jump(end);
                self.declare_pattern_global(left)?;
            }
            Pattern::Rest { argument, .. } => {
                self.declare_pattern_global(argument)?;
            }
        }
        Ok(())
    }

    pub(super) fn assign_pattern(&mut self, pattern: &Pattern) -> Result<(), String> {
        self.declare_pattern_local(pattern)
    }

    pub(super) fn compile_enum(&mut self, en: &tsn_core::ast::EnumDecl) -> Result<(), String> {
        self.compile_enum_as_expr(en)?;
        let name_idx = self.add_str(&en.id);
        self.emit1(OpCode::OpDefineGlobal, name_idx);
        Ok(())
    }

    pub(super) fn compile_enum_as_expr(
        &mut self,
        en: &tsn_core::ast::EnumDecl,
    ) -> Result<(), String> {
        let mut count = 0u16;
        for member in &en.members {
            let key_idx = self.add_str(&member.id);
            self.emit1(OpCode::OpPushConst, key_idx);
            if let Some(init) = &member.init {
                self.compile_expr(init)?;
            } else {
                let v = self.add_const(PoolEntry::Literal(Literal::Int(count as i64)));
                self.emit1(OpCode::OpPushConst, v);
            }
            count += 1;
        }
        self.emit1(OpCode::OpBuildObject, count);
        Ok(())
    }
}
