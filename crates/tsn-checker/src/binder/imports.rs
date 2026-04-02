use tsn_core::ast::{ExportDecl, ExportDefaultDecl, ImportDecl, ImportSpecifier};
use tsn_core::Diagnostic;
use tsn_core::TypeKind;

use crate::module_resolver;
use crate::symbol::{Symbol, SymbolKind};

impl super::Binder {
    pub(super) fn bind_import(&mut self, i: &ImportDecl) {
        let is_relative = i.source.starts_with('.') || i.source.starts_with('/');

        let relative_target = if is_relative && !self.source_file.is_empty() {
            let base = std::path::Path::new(&self.source_file)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            module_resolver::resolve_specifier_path(base, &i.source)
        } else {
            None
        };

        let is_stdlib = !is_relative && module_resolver::is_known_stdlib(&i.source);

        let relative_exports = if let Some(abs) = &relative_target {
            let mut visiting = vec![self.source_file.clone()];
            Some(module_resolver::resolve_module_exports(abs, &mut visiting))
        } else {
            None
        };

        let stdlib_exports = if is_stdlib {
            Some(module_resolver::resolve_stdlib_module_exports(&i.source))
        } else {
            None
        };

        if is_relative {
            if relative_target.is_none() {
                self.diagnostics.push(Diagnostic::error(
                    format!("cannot resolve module '{}'", i.source),
                    i.range.clone(),
                ));
            }
        } else if !is_stdlib {
            self.diagnostics.push(Diagnostic::error(
                format!("cannot resolve module '{}'", i.source),
                i.range.clone(),
            ));
        }

        for spec in &i.specifiers {
            let (local, imported, line, range) = match spec {
                ImportSpecifier::Named {
                    local,
                    imported,
                    range,
                } => (
                    local.clone(),
                    imported.as_str().to_owned(),
                    range.start.line,
                    range.clone(),
                ),
                ImportSpecifier::Default { local, range } => (
                    local.clone(),
                    "default".to_owned(),
                    range.start.line,
                    range.clone(),
                ),
                ImportSpecifier::Namespace { local, range } => (
                    local.clone(),
                    "*".to_owned(),
                    range.start.line,
                    range.clone(),
                ),
            };

            let module_path = relative_target.clone().or_else(|| {
                module_resolver::stdlib_path_for(&i.source)
                    .map(|p| p.to_string_lossy().into_owned())
            });
            let exports_ref: Option<&module_resolver::ExportMap> =
                relative_exports.as_ref().or(stdlib_exports.as_ref());

            let sym = if let Some(exports) = exports_ref {
                if imported == "*" {
                    let mut s = Symbol::new(SymbolKind::Namespace, local.clone(), line);
                    s.ty = Some(crate::types::Type(TypeKind::Named(
                        local.clone(),
                        module_path.clone(),
                    )));
                    s.origin_module = module_path;
                    s
                } else {
                    match exports.get(&imported) {
                        Some(resolved) => {
                            let mut s = resolved.clone();
                            s.name = local.clone();
                            s.line = line;
                            s.original_name = Some(imported.clone());
                            s.origin_module =
                                resolved.origin_module.clone().or(module_path.clone());
                            if let (Some(ref mut ty), Some(origin)) = (&mut s.ty, &s.origin_module)
                            {
                                *ty = ty.with_origin_recursive(origin);
                            }
                            s
                        }
                        None => {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "module '{}' has no exported member named '{}'",
                                    i.source, imported
                                ),
                                range,
                            ));
                            let mut s = Symbol::new(SymbolKind::Let, local.clone(), line);
                            s.original_name = Some(imported.clone());
                            s.origin_module = module_path;
                            s
                        }
                    }
                }
            } else {
                Symbol::new(SymbolKind::Let, local.clone(), line)
            };

            self.define(local, sym);
        }
    }

    pub(super) fn bind_export(&mut self, e: &ExportDecl) {
        match e {
            ExportDecl::Decl { declaration, .. } => {
                self.bind_decl(declaration);
            }
            ExportDecl::Default { declaration, .. } => match declaration.as_ref() {
                ExportDefaultDecl::Function(f) => self.bind_function(f),
                ExportDefaultDecl::Class(c) => self.bind_class(c),
                ExportDefaultDecl::Expr(_) => {}
            },
            ExportDecl::Named { .. } | ExportDecl::All { .. } => {}
        }
    }
}
