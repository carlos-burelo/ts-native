use std::collections::HashSet;
use tsn_core::ast::*;

/// Collect all import paths from an AST.
pub fn collect_imports(program: &Program) -> HashSet<String> {
    let mut collector = ImportCollector::new();
    collector.visit_program(program);
    collector.imports
}

struct ImportCollector {
    imports: HashSet<String>,
}

impl ImportCollector {
    fn new() -> Self {
        Self {
            imports: HashSet::new(),
        }
    }

    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Decl(decl) => self.visit_decl(decl),
            Stmt::Expr { .. } => {}
            _ => {}
        }
    }

    fn visit_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Import(import) => {
                self.imports.insert(import.source.clone());
            }
            Decl::Export(export) => {
                match export {
                    ExportDecl::Named { source, .. } => {
                        if let Some(src) = source {
                            self.imports.insert(src.clone());
                        }
                    }
                    _ => {}
                }
            }
            Decl::Namespace(ns) => {
                for decl in &ns.body {
                    self.visit_decl(decl);
                }
            }
            _ => {}
        }
    }
}
