use super::super::Compiler;
use tsn_core::ast::{
    ArrayPatternEl, Decl, ExportDecl, ExportDefaultDecl, ImportDecl, ImportSpecifier,
    ObjPatternProp, Pattern,
};
use tsn_core::OpCode;

impl Compiler {
    pub(crate) fn compile_import(&mut self, decl: &ImportDecl) -> Result<(), String> {
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

    pub(crate) fn compile_export(&mut self, decl: &ExportDecl) -> Result<(), String> {
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
