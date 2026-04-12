use super::Compiler;
use tsn_core::ast::{Decl, ExportDecl, NamespaceDecl, Pattern, SumTypeDecl};
use tsn_core::OpCode;

impl Compiler {
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

    pub(crate) fn compile_decl(&mut self, decl: &Decl) -> Result<(), String> {
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
                    let (proto, upvalues) = super::super::compile_function_with_parent(
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
