use std::collections::HashMap;

use super::super::Compiler;
use tsn_core::ast::{ClassDecl, ClassMember, ExtensionDecl, FunctionDecl, Stmt};
use tsn_core::{OpCode, SourceRange};

impl Compiler {
    pub(crate) fn compile_fn_decl(&mut self, decl: &FunctionDecl) -> Result<(), String> {
        if self.scope.depth == 0 && !self.is_function {
            // Global scope: compile then define global
            let (proto, upvalues) = super::super::compile_function_with_parent(
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
            let (proto, upvalues) = super::super::compile_function_with_parent(
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

    pub(crate) fn compile_class_as_expr(&mut self, decl: &ClassDecl) -> Result<(), String> {
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
                    let (fn_proto, upvalues) = super::super::compile_function_with_parent(
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
                        let is_private = matches!(modifiers.visibility, Some(Visibility::Private));
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
                    let (fn_proto, upvalues) = super::super::compile_function_with_parent(
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
                    let (fn_proto, upvalues) = super::super::compile_function_with_parent(
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
                    let (fn_proto, upvalues) = super::super::compile_function_with_parent(
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
            let (fn_proto, upvalues) = super::super::compile_function_with_parent(
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

    pub(crate) fn compile_extension_decl(&mut self, decl: &ExtensionDecl) -> Result<(), String> {
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
                    let (proto, upvalues) = super::super::compile_function_with_parent(
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
                    let (proto, upvalues) = super::super::compile_function_with_parent(
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
                    let (proto, upvalues) = super::super::compile_function_with_parent(
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

    pub(crate) fn compile_class_decl(&mut self, decl: &ClassDecl) -> Result<(), String> {
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
}
