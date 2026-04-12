use crate::binder::{BindResult, Binder};
use crate::symbol::{Symbol, SymbolKind};
use crate::types::Type;
use std::collections::HashMap;
use std::fs::canonicalize;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tsn_core::ast::{Decl, ExportDecl, ExportDefaultDecl, Pattern, Stmt};

static MODULE_BIND_CACHE: Mutex<Option<HashMap<String, Arc<BindResult>>>> = Mutex::new(None);
static MODULE_EXPORT_CACHE: Mutex<Option<HashMap<String, Arc<ExportMap>>>> = Mutex::new(None);

fn export_cache_get_ref(path: &str) -> Option<Arc<ExportMap>> {
    let guard = MODULE_EXPORT_CACHE.lock().ok()?;
    let cache = guard.as_ref()?;
    cache.get(path).map(Arc::clone)
}

fn export_cache_insert_ref(path: String, exports: Arc<ExportMap>) {
    if let Ok(mut guard) = MODULE_EXPORT_CACHE.lock() {
        let cache = guard.get_or_insert_with(HashMap::new);
        cache.insert(path, exports);
    }
}

fn cache_get_ref(abs_path: &str) -> Option<Arc<BindResult>> {
    let guard = MODULE_BIND_CACHE.lock().ok()?;
    let cache = guard.as_ref()?;
    if let Some(cached) = cache.get(abs_path) {
        return Some(Arc::clone(cached));
    }

    let canonical_abs = canonical_or_original(Path::new(abs_path));
    if canonical_abs != abs_path {
        return cache.get(&canonical_abs).map(Arc::clone);
    }

    None
}

fn cache_insert_ref(abs_path: &str, bind: Arc<BindResult>) {
    let canonical_abs = canonical_or_original(Path::new(abs_path));
    if let Ok(mut guard) = MODULE_BIND_CACHE.lock() {
        let cache = guard.get_or_insert_with(HashMap::new);
        cache.entry(canonical_abs).or_insert(bind);
    }
}

fn cache_get_or_insert_ref(abs_path: &str) -> Option<Arc<BindResult>> {
    if let Some(cached) = cache_get_ref(abs_path) {
        return Some(cached);
    }

    let canonical_abs = canonical_or_original(Path::new(abs_path));
    let source = read_to_string(&canonical_abs).ok()?;
    let tokens = tsn_lexer::scan(&source, &canonical_abs);
    let program = tsn_parser::parse(tokens, &canonical_abs).ok()?;
    let result = Arc::new(Binder::bind(&program));
    cache_insert_ref(&canonical_abs, Arc::clone(&result));
    Some(result)
}

pub fn invalidate_module_cache() {
    if let Ok(mut guard) = MODULE_BIND_CACHE.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = MODULE_EXPORT_CACHE.lock() {
        *guard = None;
    }
}

pub type ExportMap = HashMap<String, Symbol>;

pub fn stdlib_path_for(specifier: &str) -> Option<PathBuf> {
    tsn_modules::ModuleLoader::from_env().tsn_source_path(specifier)
}

pub fn resolve_stdlib_module_exports(specifier: &str) -> ExportMap {
    resolve_stdlib_module_exports_ref(specifier)
        .as_ref()
        .clone()
}

pub fn resolve_stdlib_module_exports_ref(specifier: &str) -> Arc<ExportMap> {
    let abs = match stdlib_path_for(specifier) {
        Some(p) => p.to_string_lossy().into_owned(),
        None => return Arc::new(HashMap::new()),
    };
    let mut visiting = vec![];
    resolve_module_exports_ref(&abs, &mut visiting)
}

pub fn resolve_stdlib_module_bind_ref(specifier: &str) -> Option<Arc<BindResult>> {
    let abs = stdlib_path_for(specifier)?;
    resolve_module_bind_ref(&abs.to_string_lossy())
}

pub fn resolve_stdlib_module_bind(specifier: &str) -> Option<BindResult> {
    resolve_stdlib_module_bind_ref(specifier).map(|bind| (*bind).clone())
}

pub fn resolve_module_bind_ref(abs_path: &str) -> Option<Arc<BindResult>> {
    cache_get_or_insert_ref(abs_path)
}

pub fn resolve_module_bind(abs_path: &str) -> Option<BindResult> {
    resolve_module_bind_ref(abs_path).map(|bind| (*bind).clone())
}

pub fn find_module_bind_for_type(type_name: &str, origin_modules: &[String]) -> Option<BindResult> {
    find_module_bind_for_type_ref(type_name, origin_modules).map(|bind| (*bind).clone())
}

pub fn find_module_bind_for_type_ref(
    type_name: &str,
    origin_modules: &[String],
) -> Option<Arc<BindResult>> {
    for path in origin_modules {
        if let Some(bind) = resolve_module_bind_ref(path) {
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
    tsn_modules::is_known(specifier)
}

pub fn resolve_specifier_path(base_dir: &Path, specifier: &str) -> Option<String> {
    let joined = base_dir.join(specifier);
    let candidates = if joined.extension().is_some() {
        vec![joined]
    } else {
        vec![joined.with_extension("tsn"), joined]
    };

    for candidate in candidates {
        if candidate.exists() {
            if let Ok(canonical) = canonicalize(&candidate) {
                return Some(canonical.to_string_lossy().into_owned());
            }
            return Some(candidate.to_string_lossy().into_owned());
        }
    }

    None
}

pub fn resolve_module_exports(abs_path: &str, visiting: &mut Vec<String>) -> ExportMap {
    resolve_module_exports_ref(abs_path, visiting)
        .as_ref()
        .clone()
}

pub fn resolve_module_exports_ref(abs_path: &str, visiting: &mut Vec<String>) -> Arc<ExportMap> {
    if let Some(cached) = export_cache_get_ref(abs_path) {
        return cached;
    }

    let canonical_abs = canonical_or_original(Path::new(abs_path));

    if canonical_abs != abs_path {
        if let Some(cached) = export_cache_get_ref(&canonical_abs) {
            return cached;
        }
    }

    if visiting.iter().any(|v| v == &canonical_abs) {
        return Arc::new(HashMap::new());
    }

    visiting.push(canonical_abs.clone());
    let result = Arc::new(resolve_inner(&canonical_abs, visiting));
    visiting.pop();

    export_cache_insert_ref(canonical_abs, Arc::clone(&result));

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

    let bind = if let Some(cached) = cache_get_ref(abs_path) {
        cached
    } else {
        let computed = Arc::new(Binder::bind(&program));
        cache_insert_ref(abs_path, Arc::clone(&computed));
        computed
    };
    let mut exports = ExportMap::new();
    let base_dir = Path::new(abs_path).parent().unwrap_or(Path::new("."));
    collect_exports(
        &program.body,
        bind.as_ref(),
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
                    resolve_stdlib_module_exports_ref(src)
                } else {
                    let src_abs = resolve_relative(base_dir, src);
                    resolve_module_exports_ref(&src_abs, visiting)
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
                    resolve_stdlib_module_exports_ref(source)
                } else {
                    let src_abs = resolve_relative(base_dir, source);
                    resolve_module_exports_ref(&src_abs, visiting)
                };
                for (name, sym) in src_exports.iter() {
                    out.entry(name.clone()).or_insert_with(|| sym.clone());
                }
            }
            ExportDecl::All {
                source,
                alias: Some(ns),
                ..
            } => {
                let src_exports = if is_known_stdlib(source) {
                    resolve_stdlib_module_exports_ref(source)
                } else {
                    let src_abs = resolve_relative(base_dir, source);
                    resolve_module_exports_ref(&src_abs, visiting)
                };
                let mut ns_sym = Symbol::new(SymbolKind::Namespace, ns.clone(), 0);
                ns_sym.ty = Some(Type::named(ns.clone()));
                ns_sym.origin_module = Some(abs_path.to_owned());
                for (sub_name, sub_sym) in src_exports.iter() {
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

fn canonical_or_original(path: &Path) -> String {
    if let Ok(canonical) = canonicalize(path) {
        return canonical.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}
