use super::super::Compiler;
use tsn_core::ast::Decorator;
use tsn_core::OpCode;

impl Compiler {
    fn emit_is_null_check(&mut self) {
        self.emit(OpCode::OpDup);
        self.emit(OpCode::OpPushNull);
        self.emit(OpCode::OpEq);
    }

    /// Apply decorators to an entity already on the stack (class or function).
    /// Decorators are applied bottom-to-top (last in source = applied first).
    /// Calling convention: `decorator(entity)` — single arg.
    /// If decorator returns non-null, that replaces the entity on the stack.
    pub(crate) fn emit_apply_entity_decorators(
        &mut self,
        decorators: &[Decorator],
    ) -> Result<(), String> {
        for deco in decorators.iter().rev() {
            // Stack: [..., entity]
            self.emit(OpCode::OpDup);
            // Stack: [..., entity, entity_copy]
            self.compile_expr(&deco.expression)?;
            // Stack: [..., entity, entity_copy, deco]
            self.emit(OpCode::OpSwap);
            // Stack: [..., entity, deco, entity_copy]
            self.emit1(OpCode::OpCall, 1);
            // Stack: [..., entity, result]
            // If result is null/void, keep original entity; else use result
            self.emit_is_null_check();
            // Stack: [..., entity, result, bool(result===null)]
            let use_original = self.emit_jump(OpCode::OpJumpIfFalse);
            // result IS null — pop bool (peeked), pop null result, keep entity
            self.emit(OpCode::OpPop); // pop peeked bool
            self.emit(OpCode::OpPop); // pop null result
            let end_jump = self.emit_jump(OpCode::OpJump);
            self.patch_jump(use_original);
            // result is NOT null — pop bool (peeked), pop original entity, keep result
            self.emit(OpCode::OpPop); // pop peeked bool
            self.emit(OpCode::OpSwap); // [..., result, entity]
            self.emit(OpCode::OpPop); // [..., result]
            self.patch_jump(end_jump);
            // Stack: [..., entity_or_replacement]
        }
        Ok(())
    }

    /// Apply decorators to a method closure already on the stack.
    /// Calling convention: `decorator(originalFn, ctx)` — two args.
    /// ctx = `{ name, kind, isStatic, isPrivate }`.
    /// If decorator returns non-null, that replaces the function on the stack.
    pub(crate) fn emit_apply_method_decorators(
        &mut self,
        decorators: &[Decorator],
        method_name: &str,
        kind: &str,
        is_static: bool,
        is_private: bool,
    ) -> Result<(), String> {
        for deco in decorators.iter().rev() {
            // Stack: [..., fn_orig]
            self.emit(OpCode::OpDup);
            // Stack: [..., fn_orig, fn_copy]
            self.compile_expr(&deco.expression)?;
            // Stack: [..., fn_orig, fn_copy, deco]
            self.emit(OpCode::OpSwap);
            // Stack: [..., fn_orig, deco, fn_copy]

            // Build context object: { name, kind, isStatic, isPrivate }
            let k_name = self.add_str("name");
            let v_name = self.add_str(method_name);
            let k_kind = self.add_str("kind");
            let v_kind = self.add_str(kind);
            let k_static = self.add_str("isStatic");
            let k_private = self.add_str("isPrivate");
            self.emit1(OpCode::OpPushConst, k_name);
            self.emit1(OpCode::OpPushConst, v_name);
            self.emit1(OpCode::OpPushConst, k_kind);
            self.emit1(OpCode::OpPushConst, v_kind);
            self.emit1(OpCode::OpPushConst, k_static);
            if is_static {
                self.emit(OpCode::OpPushTrue);
            } else {
                self.emit(OpCode::OpPushFalse);
            }
            self.emit1(OpCode::OpPushConst, k_private);
            if is_private {
                self.emit(OpCode::OpPushTrue);
            } else {
                self.emit(OpCode::OpPushFalse);
            }
            self.emit1(OpCode::OpBuildObject, 4);
            // Stack: [..., fn_orig, deco, fn_copy, ctx]
            self.emit1(OpCode::OpCall, 2);
            // Stack: [..., fn_orig, result]
            self.emit_is_null_check();
            // Stack: [..., fn_orig, result, bool(result===null)]
            let use_original = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit(OpCode::OpPop); // pop peeked bool
            self.emit(OpCode::OpPop); // pop null result, keep fn_orig
            let end_jump = self.emit_jump(OpCode::OpJump);
            self.patch_jump(use_original);
            self.emit(OpCode::OpPop); // pop peeked bool
            self.emit(OpCode::OpSwap); // [..., result, fn_orig]
            self.emit(OpCode::OpPop); // [..., result]
            self.patch_jump(end_jump);
            // Stack: [..., fn_or_replacement]
        }
        Ok(())
    }
}
