use tsn_core::ast::{Decl, ExportDecl, ExportDefaultDecl, ImportSpecifier, Program, Stmt, VarKind};

pub fn debug_modules(program: &Program) {
    use super::super::colors::{footer, header, BOLD, C_MODULES, R};
    header(C_MODULES, "modules", &program.filename);
    let mut imports = 0usize;
    let mut exports = 0usize;

    for stmt in &program.body {
        let Stmt::Decl(decl) = stmt else { continue };
        match decl.as_ref() {
            Decl::Import(imp) => {
                let spec_str = format_import_specifiers(&imp.specifiers);
                eprintln!(
                    "  {}import{}  {}{:<32}{}  {}",
                    C_MODULES,
                    R,
                    BOLD,
                    format!("\"{}\"", imp.source),
                    R,
                    spec_str
                );
                imports += 1;
            }
            Decl::Export(exp) => {
                let desc = format_export_decl(exp);
                eprintln!("  {}export{}  {}", C_MODULES, R, desc);
                exports += 1;
            }
            _ => {}
        }
    }

    footer(
        C_MODULES,
        &format!("{} import(s), {} export(s)", imports, exports),
    );
}

pub fn format_import_specifiers(specs: &[ImportSpecifier]) -> String {
    if specs.is_empty() {
        return "(side-effect)".to_owned();
    }
    let mut named: Vec<&str> = Vec::new();
    let mut default_name: Option<&str> = None;
    let mut ns_name: Option<&str> = None;

    for s in specs {
        match s {
            ImportSpecifier::Named { local, .. } => named.push(local),
            ImportSpecifier::Default { local, .. } => default_name = Some(local),
            ImportSpecifier::Namespace { local, .. } => ns_name = Some(local),
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if let Some(n) = default_name {
        parts.push(n.to_owned());
    }
    if let Some(ns) = ns_name {
        parts.push(format!("* as {}", ns));
    }
    if !named.is_empty() {
        parts.push(format!("{{ {} }}", named.join(", ")));
    }
    parts.join(", ")
}

fn format_export_decl(exp: &ExportDecl) -> String {
    match exp {
        ExportDecl::Named {
            specifiers, source, ..
        } => {
            let names: Vec<&str> = specifiers.iter().map(|s| s.exported.as_str()).collect();
            let from = source
                .as_ref()
                .map(|s| format!(" from \"{}\"", s))
                .unwrap_or_default();
            format!("{{ {}{} }}", names.join(", "), from)
        }
        ExportDecl::Default { declaration, .. } => {
            let what = match declaration.as_ref() {
                ExportDefaultDecl::Function(f) => format!("fn {}", f.id),
                ExportDefaultDecl::Class(c) => {
                    format!("class {}", c.id.as_deref().unwrap_or("<anon>"))
                }
                ExportDefaultDecl::Expr(_) => "<expr>".to_owned(),
            };
            format!("default  {}", what)
        }
        ExportDecl::All { source, alias, .. } => {
            let alias_str = alias
                .as_ref()
                .map(|a| format!(" as {}", a))
                .unwrap_or_default();
            format!("* from \"{}\"{}", source, alias_str)
        }
        ExportDecl::Decl { declaration, .. } => {
            let name = match declaration.as_ref() {
                Decl::Function(f) => format!("fn {}", f.id),
                Decl::Class(c) => format!("class {}", c.id.as_deref().unwrap_or("<anon>")),
                Decl::Variable(v) => {
                    let kw = match v.kind {
                        VarKind::Const => "const",
                        VarKind::Let => "let",
                    };
                    format!("{} ...", kw)
                }
                Decl::Interface(i) => format!("interface {}", i.id),
                Decl::TypeAlias(t) => format!("type {}", t.id),
                Decl::Enum(e) => format!("enum {}", e.id),
                Decl::Namespace(n) => format!("namespace {}", n.id),
                Decl::Struct(s) => format!("struct {}", s.id),
                _ => "<decl>".to_owned(),
            };
            format!("decl  {}", name)
        }
    }
}
