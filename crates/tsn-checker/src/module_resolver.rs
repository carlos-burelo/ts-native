use crate::binder::{BindResult, Binder};
use crate::symbol::{Symbol, SymbolKind};
use crate::types::Type;
use std::collections::HashMap;
use std::fs::{canonicalize, read_to_string};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tsn_core::ast::{Decl, ExportDecl, ExportDefaultDecl, Pattern, Stmt};

static MODULE_BIND_CACHE: Mutex<Option<HashMap<String, BindResult>>> = Mutex::new(None);

fn cache_get_or_insert(abs_path: &str) -> Option<BindResult> {
    let mut guard = MODULE_BIND_CACHE.lock().ok()?;
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(cached) = cache.get(abs_path) {
        return Some(cached.clone());
    }

    let source = read_to_string(abs_path).ok()?;
    let tokens = tsn_lexer::scan(&source, abs_path);
    let program = tsn_parser::parse(tokens, abs_path).ok()?;
    let result = Binder::bind(&program);
    cache.insert(abs_path.to_owned(), result.clone());
    Some(result)
}

pub fn invalidate_module_cache() {
    if let Ok(mut guard) = MODULE_BIND_CACHE.lock() {
        *guard = None;
    }
}

pub type ExportMap = HashMap<String, Symbol>;

pub fn stdlib_dir() -> Option<PathBuf> {
    for cand in tsn_core::paths::stdlib_candidates() {
        if cand.is_dir() {
            if let Ok(abs) = canonicalize(&cand) {
                return Some(abs);
            }
            return Some(cand);
        }
    }

    None
}

pub fn stdlib_path_for(specifier: &str) -> Option<PathBuf> {
    let root = stdlib_dir()?;
    let path = if let Some(rest) = specifier.strip_prefix("std:") {
        root.join("std").join(rest).join("mod.tsn")
    } else if let Some(rest) = specifier.strip_prefix("builtin:") {
        root.join("builtins").join(format!("{}.tsn", rest))
    } else {
        return None;
    };

    if path.is_file() {
        Some(path)
    } else {
        if specifier.starts_with("std:") {
            let alt = path
                .parent()?
                .join(format!("{}.tsn", specifier.strip_prefix("std:")?));
            if alt.is_file() {
                return Some(alt);
            }
        }
        None
    }
}

pub fn resolve_stdlib_module_exports(specifier: &str) -> ExportMap {
    let abs = match stdlib_path_for(specifier) {
        Some(p) => p.to_string_lossy().into_owned(),
        None => return HashMap::new(),
    };
    let mut visiting = vec![];
    resolve_module_exports(&abs, &mut visiting)
}

pub fn resolve_stdlib_module_bind(specifier: &str) -> Option<BindResult> {
    let abs = stdlib_path_for(specifier)?;
    resolve_module_bind(&abs.to_string_lossy())
}

pub fn resolve_module_bind(abs_path: &str) -> Option<BindResult> {
    cache_get_or_insert(abs_path)
}

pub fn find_module_bind_for_type(type_name: &str, origin_modules: &[String]) -> Option<BindResult> {
    for path in origin_modules {
        if let Some(bind) = resolve_module_bind(path) {
            if bind.class_members.contains_key(type_name)
                || bind.namespace_members.contains_key(type_name)
                || bind.interface_members.contains_key(type_name)
            {
                return Some(bind);
            }
        }
    }
    None
}

pub fn is_known_stdlib(specifier: &str) -> bool {
    stdlib_path_for(specifier).is_some()
}

pub fn resolve_specifier_path(base_dir: &Path, specifier: &str) -> Option<String> {
    let joined = base_dir.join(specifier);
    let candidates = if joined.extension().is_some() {
        vec![joined]
    } else {
        vec![joined.with_extension("tsn"), joined]
    };

    for candidate in candidates {
        if let Ok(abs) = canonicalize(&candidate) {
            return Some(abs.to_string_lossy().into_owned());
        }
    }

    None
}

pub fn resolve_module_exports(abs_path: &str, visiting: &mut Vec<String>) -> ExportMap {
    if visiting.iter().any(|v| v == abs_path) {
        return HashMap::new();
    }
    visiting.push(abs_path.to_owned());
    let result = resolve_inner(abs_path, visiting);
    visiting.pop();
    result
}

fn resolve_inner(abs_path: &str, visiting: &mut Vec<String>) -> ExportMap {
    let source = match read_to_string(abs_path) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };

    let tokens = tsn_lexer::scan(&source, abs_path);
    let program = match tsn_parser::parse(tokens, abs_path) {
        Ok(p) => p,
        Err(_) => return HashMap::new(),
    };

    let bind = Binder::bind(&program);
    let mut exports = ExportMap::new();
    let base_dir = Path::new(abs_path).parent().unwrap_or(Path::new("."));
    collect_exports(
        &program.body,
        &bind,
        abs_path,
        base_dir,
        visiting,
        &mut exports,
    );
    exports
}

fn collect_exports(
    stmts: &[Stmt],
    bind: &BindResult,
    abs_path: &str,
    base_dir: &Path,
    visiting: &mut Vec<String>,
    out: &mut ExportMap,
) {
    for stmt in stmts {
        let Stmt::Decl(decl) = stmt else { continue };
        let Decl::Export(e) = decl.as_ref() else {
            continue;
        };

        match e {
            ExportDecl::Decl { declaration, .. } => {
                if let Some(name) = decl_primary_name(declaration) {
                    if let Some(sym) = lookup_global(bind, &name) {
                        let mut s = sym.clone();
                        s.origin_module = Some(abs_path.to_owned());
                        out.insert(name, s);
                    }
                }
            }
            ExportDecl::Named {
                specifiers,
                source: None,
                ..
            } => {
                for spec in specifiers {
                    if let Some(sym) = lookup_global(bind, &spec.local) {
                        let mut s = sym.clone();
                        s.name = spec.exported.clone();
                        s.origin_module = Some(abs_path.to_owned());
                        out.insert(spec.exported.clone(), s);
                    }
                }
            }
            ExportDecl::Named {
                specifiers,
                source: Some(src),
                ..
            } => {
                let src_exports = if is_known_stdlib(src) {
                    resolve_stdlib_module_exports(src)
                } else {
                    let src_abs = resolve_relative(base_dir, src);
                    resolve_module_exports(&src_abs, visiting)
                };
                for spec in specifiers {
                    if let Some(sym) = src_exports.get(&spec.local) {
                        let mut s = sym.clone();
                        s.name = spec.exported.clone();
                        out.insert(spec.exported.clone(), s);
                    }
                }
            }
            ExportDecl::All {
                source,
                alias: None,
                ..
            } => {
                let src_exports = if is_known_stdlib(source) {
                    resolve_stdlib_module_exports(source)
                } else {
                    let src_abs = resolve_relative(base_dir, source);
                    resolve_module_exports(&src_abs, visiting)
                };
                for (name, sym) in src_exports {
                    out.entry(name).or_insert(sym);
                }
            }
            ExportDecl::All {
                source,
                alias: Some(ns),
                ..
            } => {
                let src_exports = if is_known_stdlib(source) {
                    resolve_stdlib_module_exports(source)
                } else {
                    let src_abs = resolve_relative(base_dir, source);
                    resolve_module_exports(&src_abs, visiting)
                };
                let mut ns_sym = Symbol::new(SymbolKind::Namespace, ns.clone(), 0);
                ns_sym.ty = Some(Type::named(ns.clone()));
                ns_sym.origin_module = Some(abs_path.to_owned());
                for (sub_name, sub_sym) in &src_exports {
                    out.insert(format!("{}.{}", ns, sub_name), sub_sym.clone());
                }
                out.insert(ns.clone(), ns_sym);
            }
            ExportDecl::Default { declaration, .. } => match declaration.as_ref() {
                ExportDefaultDecl::Function(f) => {
                    if let Some(sym) = lookup_global(bind, &f.id) {
                        let mut s = sym.clone();
                        s.name = "default".to_owned();
                        out.insert("default".to_owned(), s);
                    }
                }
                ExportDefaultDecl::Class(c) => {
                    let id = c.id.as_deref().unwrap_or("<anon>");
                    if let Some(sym) = lookup_global(bind, id) {
                        let mut s = sym.clone();
                        s.name = "default".to_owned();
                        out.insert("default".to_owned(), s);
                    }
                }
                ExportDefaultDecl::Expr(_) => {}
            },
        }
    }
}

fn decl_primary_name(decl: &Decl) -> Option<String> {
    match decl {
        Decl::Variable(v) => v.declarators.first().and_then(|d| match &d.id {
            Pattern::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        }),
        Decl::Function(f) => Some(f.id.clone()),
        Decl::Class(c) => c.id.clone(),
        Decl::Enum(e) => Some(e.id.clone()),
        Decl::Interface(i) => Some(i.id.clone()),
        Decl::TypeAlias(t) => Some(t.id.clone()),
        Decl::Namespace(n) => Some(n.id.clone()),
        Decl::Struct(s) => Some(s.id.clone()),
        _ => None,
    }
}

fn lookup_global<'a>(bind: &'a BindResult, name: &str) -> Option<&'a Symbol> {
    let scope = bind.scopes.get(bind.global_scope);
    scope.bindings.get(name).map(|&id| bind.arena.get(id))
}

fn resolve_relative(base_dir: &Path, specifier: &str) -> String {
    resolve_specifier_path(base_dir, specifier)
        .unwrap_or_else(|| base_dir.join(specifier).to_string_lossy().into_owned())
}
