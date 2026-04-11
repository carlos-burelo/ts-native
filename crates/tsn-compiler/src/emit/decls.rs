use std::collections::HashMap;

use super::Compiler;
use tsn_core::ast::{
    ArrayPatternEl, ClassDecl, ClassMember, Decl, Decorator, ExportDecl, ExportDefaultDecl,
    ExtensionDecl, FunctionDecl, ImportDecl, ImportSpecifier, ObjPatternProp, Pattern, Stmt,
};
use tsn_core::{OpCode, SourceRange};

impl Compiler {
    pub(super) fn compile_fn_decl(&mut self, decl: &FunctionDecl) -> Result<(), String> {
        if self.scope.depth == 0 && !self.is_function {
            // Global scope: compile then define global
            let (proto, upvalues) = super::compile_function_with_parent(
                &decl.id,
                &decl.params,
                &decl.body,
                decl.modifiers.is_async,
                decl.modifiers.is_generator,
                false,
                self,
            )?;
            self.emit_closure(proto, upvalues);
            if decl.decorators.is_empty() {
                self.emit_define_global(&decl.id);
            } else {
                // Apply decorators to the function on the stack, then define
                self.emit_apply_entity_decorators(&decl.decorators)?;
                self.emit_define_global(&decl.id);
            }
        } else {
            // Local function: pre-declare slot with null so body can capture for self-recursion
            self.emit(OpCode::OpPushNull);
            let fn_slot = self.scope.declare_local(&decl.id);
            let (proto, upvalues) = super::compile_function_with_parent(
                &decl.id,
                &decl.params,
                &decl.body,
                decl.modifiers.is_async,
                decl.modifiers.is_generator,
                false,
                self,
            )?;
            self.emit_closure(proto, upvalues);
            if !decl.decorators.is_empty() {
                self.emit_apply_entity_decorators(&decl.decorators)?;
            }
            // OpSetLocal peeks, so pop the extra closure copy
            self.emit1(OpCode::OpSetLocal, fn_slot);
            self.emit(OpCode::OpPop);
        }
        Ok(())
    }

    pub(super) fn compile_class_as_expr(&mut self, decl: &ClassDecl) -> Result<(), String> {
        let name = decl.id.as_deref().unwrap_or("<anonymous>");
        let name_idx = self.add_str(name);

        if let Some(super_expr) = &decl.super_class {
            self.compile_expr(super_expr)?;
        }

        self.emit1(OpCode::OpClass, name_idx);

        if decl.super_class.is_some() {
            self.emit(OpCode::OpInherit);
        }

        self.class_vtable_layout = Some(HashMap::new());
        self.class_field_layout = Some(HashMap::new());
        self.class_field_count = 0;
        self.class_field_layout_valid = true;
        self.pending_field_inits.clear();

        if let Some(tsn_core::ast::Expr::Identifier {
            name: parent_name, ..
        }) = &decl.super_class
        {
            if let Some((parent_fields, parent_count)) =
                self.class_slot_registry.get(parent_name.as_str()).cloned()
            {
                *self.class_field_layout.as_mut().unwrap() = parent_fields;
                self.class_field_count = parent_count;
            } else {
                // Parent class is external and not in registry — we cannot determine
                // the inherited field count, so own-field slot numbers would be wrong.
                // Disable the fixed-field optimization; fall back to SET/GET_PROPERTY.
                self.class_field_layout_valid = false;
            }
        }

        for member in &decl.body {
            if let ClassMember::Property {
                key,
                init,
                modifiers,
                ..
            } = member
            {
                let key_idx = self.add_str(key);
                if modifiers.is_static {
                    if let Some(expr) = init {
                        self.compile_expr(expr)?;
                    } else {
                        self.emit(OpCode::OpPushNull);
                    }
                    self.emit1(OpCode::OpDefineStatic, key_idx);
                } else {
                    let slot = self.class_field_count;
                    self.class_field_count += 1;
                    self.class_field_layout
                        .as_mut()
                        .unwrap()
                        .insert(key.clone(), slot);
                    self.emit1(OpCode::OpDeclareField, key_idx);
                    if let Some(expr) = init {
                        self.pending_field_inits
                            .push((key.clone(), slot, expr.clone()));
                    }
                }
            }
        }

        for member in &decl.body {
            match member {
                ClassMember::Method {
                    key,
                    params,
                    body: Some(body),
                    modifiers,
                    decorators,
                    ..
                } => {
                    let (fn_proto, upvalues) = super::compile_function_with_parent(
                        key,
                        params,
                        body,
                        modifiers.is_async,
                        modifiers.is_generator,
                        !modifiers.is_static,
                        self,
                    )?;
                    let key_idx = self.add_str(key);

                    if !modifiers.is_static {
                        if let Some(vtable) = self.class_vtable_layout.as_mut() {
                            if !vtable.contains_key(key) {
                                let next_idx = vtable.len();
                                vtable.insert(key.clone(), next_idx);
                            }
                        }
                    }

                    self.emit_closure(fn_proto, upvalues);
                    if !decorators.is_empty() {
                        use tsn_core::ast::operators::Visibility;
                        let is_private = matches!(
                            modifiers.visibility,
                            Some(Visibility::Private)
                        );
                        self.emit_apply_method_decorators(
                            decorators,
                            key,
                            "method",
                            modifiers.is_static,
                            is_private,
                        )?;
                    }
                    if modifiers.is_static {
                        self.emit1(OpCode::OpDefineStatic, key_idx);
                    } else {
                        self.emit1(OpCode::OpMethod, key_idx);
                    }
                }
                ClassMember::Constructor { params, body, .. } => {
                    let (fn_proto, upvalues) = super::compile_function_with_parent(
                        "constructor",
                        params,
                        body,
                        false,
                        false,
                        true,
                        self,
                    )?;
                    let key_idx = self.add_str("constructor");
                    self.emit_closure(fn_proto, upvalues);
                    self.emit1(OpCode::OpMethod, key_idx);
                }
                ClassMember::Getter {
                    key,
                    body: Some(body),
                    modifiers,
                    ..
                } => {
                    let (fn_proto, upvalues) = super::compile_function_with_parent(
                        key,
                        &[],
                        body,
                        false,
                        false,
                        !modifiers.is_static,
                        self,
                    )?;
                    let key_idx = self.add_str(key);
                    self.emit_closure(fn_proto, upvalues);
                    if modifiers.is_static {
                        self.emit1(OpCode::OpDefineStaticGetter, key_idx);
                    } else {
                        self.emit1(OpCode::OpDefineGetter, key_idx);
                    }
                }
                ClassMember::Setter {
                    key,
                    param,
                    body: Some(body),
                    modifiers,
                    ..
                } => {
                    let (fn_proto, upvalues) = super::compile_function_with_parent(
                        key,
                        std::slice::from_ref(param),
                        body,
                        false,
                        false,
                        !modifiers.is_static,
                        self,
                    )?;
                    let key_idx = self.add_str(key);
                    self.emit_closure(fn_proto, upvalues);
                    if modifiers.is_static {
                        self.emit1(OpCode::OpDefineStaticSetter, key_idx);
                    } else {
                        self.emit1(OpCode::OpDefineSetter, key_idx);
                    }
                }
                ClassMember::Property { .. } => {}
                _ => {}
            }
        }

        if !self.pending_field_inits.is_empty() {
            let empty_body = Stmt::Block {
                stmts: vec![],
                range: SourceRange::default(),
            };
            let (fn_proto, upvalues) = super::compile_function_with_parent(
                "constructor",
                &[],
                &empty_body,
                false,
                false,
                true,
                self,
            )?;
            let key_idx = self.add_str("constructor");
            self.emit_closure(fn_proto, upvalues);
            self.emit1(OpCode::OpMethod, key_idx);
        }

        if let (Some(id), Some(layout)) = (&decl.id, self.class_field_layout.take()) {
            self.class_slot_registry
                .insert(id.clone(), (layout, self.class_field_count));
        }

        self.class_vtable_layout = None;

        self.class_field_count = 0;

        Ok(())
    }

    pub(super) fn compile_extension_decl(&mut self, decl: &ExtensionDecl) -> Result<(), String> {
        use tsn_core::{well_known, TypeKind};
        let type_name = match &decl.target.kind {
            TypeKind::Int => well_known::INT.to_owned(),
            TypeKind::Float => well_known::FLOAT.to_owned(),
            TypeKind::Str => well_known::STR.to_owned(),
            TypeKind::Bool => well_known::BOOL.to_owned(),
            TypeKind::Char => well_known::CHAR.to_owned(),
            TypeKind::Named(n, _) => n.clone(),
            TypeKind::Generic(n, _, _) => n.clone(),
            TypeKind::Array(_) => well_known::ARRAY.to_owned(),
            _ => well_known::DYNAMIC.to_owned(),
        };

        for member in &decl.members {
            match member {
                tsn_core::ast::ExtensionMember::Method(method) => {
                    let mangled = format!("__ext_{}_{}", type_name, method.id);
                    let (proto, upvalues) = super::compile_function_with_parent(
                        &mangled,
                        &method.params,
                        &method.body,
                        method.modifiers.is_async,
                        method.modifiers.is_generator,
                        true,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.emit_define_global(&mangled);
                }
                tsn_core::ast::ExtensionMember::Getter { key, body, .. } => {
                    let mangled = format!("__extget_{}_{}", type_name, key);
                    let (proto, upvalues) = super::compile_function_with_parent(
                        &mangled,
                        &[],
                        body,
                        false,
                        false,
                        true,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.emit_define_global(&mangled);
                }
                tsn_core::ast::ExtensionMember::Setter {
                    key, param, body, ..
                } => {
                    let mangled = format!("__extset_{}_{}", type_name, key);
                    let (proto, upvalues) = super::compile_function_with_parent(
                        &mangled,
                        std::slice::from_ref(param),
                        body,
                        false,
                        false,
                        true,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.emit_define_global(&mangled);
                }
            }
        }

        Ok(())
    }

    pub(super) fn compile_class_decl(&mut self, decl: &ClassDecl) -> Result<(), String> {
        self.compile_class_as_expr(decl)?;

        if !decl.decorators.is_empty() {
            self.emit_apply_entity_decorators(&decl.decorators)?;
        }

        if let Some(id) = &decl.id {
            if self.scope.depth == 0 {
                self.emit_define_global(id);
            } else {
                self.scope.declare_local(id);
            }
        }

        Ok(())
    }

    /// Emit a null check on the top-of-stack value, leaving a bool on top.
    /// Stack before: [..., val]
    /// Stack after:  [..., val, bool(val === null)]
    fn emit_is_null_check(&mut self) {
        self.emit(OpCode::OpDup);
        self.emit(OpCode::OpPushNull);
        self.emit(OpCode::OpEq);
    }

    /// Apply decorators to an entity already on the stack (class or function).
    /// Decorators are applied bottom-to-top (last in source = applied first).
    /// Calling convention: `decorator(entity)` — single arg.
    /// If decorator returns non-null, that replaces the entity on the stack.
    fn emit_apply_entity_decorators(
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
            self.emit(OpCode::OpPop);  // pop peeked bool
            self.emit(OpCode::OpSwap); // [..., result, entity]
            self.emit(OpCode::OpPop);  // [..., result]
            self.patch_jump(end_jump);
            // Stack: [..., entity_or_replacement]
        }
        Ok(())
    }

    /// Apply decorators to a method closure already on the stack.
    /// Calling convention: `decorator(originalFn, ctx)` — two args.
    /// ctx = `{ name, kind, isStatic, isPrivate }`.
    /// If decorator returns non-null, that replaces the function on the stack.
    fn emit_apply_method_decorators(
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
            self.emit(OpCode::OpPop);  // pop peeked bool
            self.emit(OpCode::OpPop);  // pop null result, keep fn_orig
            let end_jump = self.emit_jump(OpCode::OpJump);
            self.patch_jump(use_original);
            self.emit(OpCode::OpPop);  // pop peeked bool
            self.emit(OpCode::OpSwap); // [..., result, fn_orig]
            self.emit(OpCode::OpPop);  // [..., result]
            self.patch_jump(end_jump);
            // Stack: [..., fn_or_replacement]
        }
        Ok(())
    }

    pub(super) fn compile_import(&mut self, decl: &ImportDecl) -> Result<(), String> {
        let src_idx = self.add_str(&decl.source);
        self.emit1(OpCode::OpImport, src_idx);

        for spec in &decl.specifiers {
            if let Some(ann_ptr) = self.type_annotations {
                let ann = unsafe { &*ann_ptr };
                if ann.contains_type_only(spec.range().start.offset) {
                    continue;
                }
            }

            self.emit(OpCode::OpDup);
            match spec {
                ImportSpecifier::Default { local, .. } => {
                    let key_idx = self.add_str("default");
                    let cs = self.alloc_cache_slot();
                    self.emit2(OpCode::OpGetProperty, key_idx, cs);
                    let local_idx = self.add_str(local);
                    self.emit1(OpCode::OpDefineGlobal, local_idx);
                }
                ImportSpecifier::Named {
                    local, imported, ..
                } => {
                    let key_idx = self.add_str(imported);
                    let cs = self.alloc_cache_slot();
                    self.emit2(OpCode::OpGetProperty, key_idx, cs);
                    let local_idx = self.add_str(local);
                    self.emit1(OpCode::OpDefineGlobal, local_idx);
                }
                ImportSpecifier::Namespace { local, .. } => {
                    let local_idx = self.add_str(local);
                    self.emit1(OpCode::OpDefineGlobal, local_idx);
                }
            }
        }
        self.emit(OpCode::OpPop);
        Ok(())
    }

    pub(super) fn compile_export(&mut self, decl: &ExportDecl) -> Result<(), String> {
        match decl {
            ExportDecl::Decl { declaration, .. } => {
                self.compile_decl(declaration)?;

                for name in runtime_export_names(declaration) {
                    self.emit_get_var(&name);
                    let key = self.add_str(&name);
                    self.emit1(OpCode::OpMergeExports, key);
                }
            }
            ExportDecl::Default { declaration, .. } => match declaration.as_ref() {
                ExportDefaultDecl::Function(f) => {
                    if f.modifiers.is_declare {
                        return Ok(());
                    }
                    self.compile_fn_decl(f)?;
                    self.emit_get_var(&f.id);
                    let key = self.add_str("default");
                    self.emit1(OpCode::OpMergeExports, key);
                }
                ExportDefaultDecl::Class(c) => {
                    if c.modifiers.is_declare {
                        return Ok(());
                    }
                    self.compile_class_decl(c)?;
                    if let Some(id) = &c.id {
                        self.emit_get_var(id);
                    }

                    let key = self.add_str("default");
                    self.emit1(OpCode::OpMergeExports, key);
                }
                ExportDefaultDecl::Expr(e) => {
                    self.compile_expr(e)?;
                    let key = self.add_str("default");
                    self.emit1(OpCode::OpMergeExports, key);
                }
            },
            ExportDecl::Named {
                specifiers, source, ..
            } => match source {
                Some(src) if !specifiers.is_empty() => {
                    let src_idx = self.add_str(src);
                    self.emit1(OpCode::OpImport, src_idx);
                    for spec in specifiers {
                        self.emit(OpCode::OpDup);
                        let imported_idx = self.add_str(&spec.local);
                        let cs = self.alloc_cache_slot();
                        self.emit2(OpCode::OpGetProperty, imported_idx, cs);
                        let exported_idx = self.add_str(&spec.exported);
                        self.emit1(OpCode::OpMergeExports, exported_idx);
                    }
                    self.emit(OpCode::OpPop);
                }
                Some(_) => {}
                None => {
                    for spec in specifiers {
                        self.emit_get_var(&spec.local);
                        let key = self.add_str(&spec.exported);
                        self.emit1(OpCode::OpMergeExports, key);
                    }
                }
            },
            ExportDecl::All { source, alias, .. } => {
                let src_idx = self.add_str(source);
                match alias {
                    Some(alias_name) => {
                        self.emit1(OpCode::OpImport, src_idx);
                        let key = self.add_str(alias_name);
                        self.emit1(OpCode::OpMergeExports, key);
                    }
                    None => {
                        self.emit1(OpCode::OpReexport, src_idx);
                    }
                }
            }
        }
        Ok(())
    }
}

fn runtime_export_names(decl: &Decl) -> Vec<String> {
    match decl {
        Decl::Variable(v) => v
            .declarators
            .iter()
            .flat_map(|d| {
                if v.is_declare {
                    Vec::new()
                } else {
                    pattern_binding_names(&d.id)
                }
            })
            .collect(),
        Decl::Function(f) => {
            if f.modifiers.is_declare {
                vec![]
            } else {
                vec![f.id.clone()]
            }
        }
        Decl::Class(c) => {
            if c.modifiers.is_declare {
                vec![]
            } else {
                c.id.iter().cloned().collect()
            }
        }
        Decl::Enum(e) => vec![e.id.clone()],
        Decl::Namespace(n) => vec![n.id.clone()],
        Decl::SumType(st) => st.variants.iter().map(|v| v.name.clone()).collect(),
        Decl::Interface(_)
        | Decl::TypeAlias(_)
        | Decl::Struct(_)
        | Decl::Import(_)
        | Decl::Export(_)
        | Decl::Extension(_) => vec![],
    }
}

fn pattern_binding_names(pat: &Pattern) -> Vec<String> {
    match pat {
        Pattern::Identifier { name, .. } => vec![name.clone()],
        Pattern::Array { elements, rest, .. } => {
            let mut names: Vec<String> = elements
                .iter()
                .flatten()
                .flat_map(|el: &ArrayPatternEl| pattern_binding_names(&el.pattern))
                .collect();
            if let Some(r) = rest {
                names.extend(pattern_binding_names(r));
            }
            names
        }
        Pattern::Object {
            properties, rest, ..
        } => {
            let mut names: Vec<String> = properties
                .iter()
                .flat_map(|p: &ObjPatternProp| pattern_binding_names(&p.value))
                .collect();
            if let Some(r) = rest {
                names.extend(pattern_binding_names(r));
            }
            names
        }
        Pattern::Assignment { left, .. } => pattern_binding_names(left),
        Pattern::Rest { argument, .. } => pattern_binding_names(argument),
    }
}
