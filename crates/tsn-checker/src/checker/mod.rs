pub(crate) mod compat;
mod decls;
mod stmts;

use crate::binder::{BindResult, Binder};
use crate::scope::ScopeId;
use crate::types::Type;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tsn_core::ast::Program;
use tsn_core::Diagnostic;

pub(crate) use crate::checker_annotations::collect_type_annotations;
pub(crate) use crate::checker_enrichment::enrich_call_returns;

#[derive(Clone, Debug)]
pub struct ExprInfo {
    pub ty: Type,
    pub symbol_id: Option<crate::symbol::SymbolId>,
}

impl std::fmt::Display for ExprInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty)
    }
}

pub struct CheckResult {
    pub bind: BindResult,
    pub diagnostics: Vec<Diagnostic>,
    pub expr_types: FxHashMap<u32, ExprInfo>,
    pub flattened_members: HashMap<String, Vec<crate::types::ClassMemberInfo>>,
    pub type_annotations: tsn_core::TypeAnnotations,
    pub profile: CheckProfile,

    pub extension_calls: FxHashMap<u32, String>,

    pub extension_members: FxHashMap<u32, String>,
    pub extension_set_members: FxHashMap<u32, String>,
}

#[derive(Clone, Debug, Default)]
pub struct CheckProfile {
    pub load_globals: Duration,
    pub bind: Duration,
    pub merge_builtin_members: Duration,
    pub enrich_call_returns: Duration,
    pub check_stmts: Duration,
    pub collect_annotations: Duration,
    pub finalize: Duration,
}

pub struct Checker {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) current_scope: crate::scope::ScopeId,
    pub(crate) expected_return_type: Option<Type>,
    pub(crate) narrowed_types: FxHashMap<crate::symbol::SymbolId, Vec<Type>>,
    pub(crate) narrowings_cache:
        FxHashMap<(usize, bool, crate::scope::ScopeId), Vec<(crate::symbol::SymbolId, Type)>>,
    pub(crate) child_indices: FxHashMap<ScopeId, usize>,
    pub(crate) expr_types: FxHashMap<u32, ExprInfo>,
    pub(crate) infer_cache: FxHashMap<(usize, ScopeId, u32), Type>,
    pub(crate) infer_env_rev: u32,
    pub(crate) compat_cache: FxHashMap<(usize, usize, usize), bool>,
    pub(crate) type_node_cache: FxHashMap<(usize, usize), Type>,
    pub(crate) symbol_type_params_cache: FxHashMap<(String, u8), Vec<String>>,
    pub(crate) var_types: FxHashMap<crate::symbol::SymbolId, Type>,

    pub(crate) current_class: Option<String>,

    pub(crate) abstract_classes: FxHashSet<String>,

    pub(crate) is_assignment_target: bool,
    pub(crate) in_pipeline_rhs: bool,
    pub(crate) extension_calls: FxHashMap<u32, String>,
    pub(crate) extension_members: FxHashMap<u32, String>,
    pub(crate) extension_set_members: FxHashMap<u32, String>,
    pub(crate) member_exists_cache: FxHashMap<(String, String), bool>,

    /// Emitir hint cuando `dynamic` es inferido por gap (no escrito explícitamente).
    /// Default: false. Activable via opciones del compilador.
    pub warn_implicit_dynamic: bool,

    /// Tipo esperado para la expresión actual, propagado desde el contexto externo.
    /// None = sin expectativa (inferencia libre).
    /// Usado para contextual typing de lambdas, literales de objeto/array.
    pub(crate) expected_type: Option<Type>,
}

impl Checker {
    pub fn check(program: &Program) -> CheckResult {
        Self::check_with_profile(program)
    }

    pub fn check_with_profile(program: &Program) -> CheckResult {
        let mut profile = CheckProfile::default();

        let is_builtin = crate::builtins::is_builtin_file(&program.filename);
        let globals_ref = if !is_builtin {
            let started = Instant::now();
            let globals = crate::builtins::global_exports_ref();
            profile.load_globals = started.elapsed();
            Some(globals)
        } else {
            None
        };

        let started = Instant::now();
        let mut bind = match globals_ref {
            Some(globals) => Binder::bind_with_global_refs(program, globals),
            None => Binder::bind(program),
        };
        profile.bind = started.elapsed();

        if !is_builtin {
            let started = Instant::now();
            crate::builtins::merge_builtin_members(&mut bind);
            profile.merge_builtin_members = started.elapsed();
        }

        let started = Instant::now();
        enrich_call_returns(&mut bind, program);
        profile.enrich_call_returns = started.elapsed();

        let mut checker = Checker {
            current_scope: bind.global_scope,
            diagnostics: Vec::new(),
            expected_return_type: None,
            narrowed_types: FxHashMap::default(),
            narrowings_cache: FxHashMap::default(),
            child_indices: FxHashMap::with_capacity_and_hasher(64, Default::default()),
            expr_types: FxHashMap::with_capacity_and_hasher(2048, Default::default()),
            infer_cache: FxHashMap::with_capacity_and_hasher(4096, Default::default()),
            infer_env_rev: 0,
            compat_cache: FxHashMap::with_capacity_and_hasher(4096, Default::default()),
            type_node_cache: FxHashMap::with_capacity_and_hasher(1024, Default::default()),
            symbol_type_params_cache: FxHashMap::with_capacity_and_hasher(256, Default::default()),
            var_types: FxHashMap::default(),
            current_class: None,
            abstract_classes: FxHashSet::default(),
            is_assignment_target: false,
            in_pipeline_rhs: false,
            extension_calls: FxHashMap::default(),
            extension_members: FxHashMap::default(),
            extension_set_members: FxHashMap::default(),
            member_exists_cache: FxHashMap::with_capacity_and_hasher(256, Default::default()),
            warn_implicit_dynamic: false,
            expected_type: None,
        };

        let started = Instant::now();
        checker.check_stmts(&program.body, &bind);
        profile.check_stmts = started.elapsed();

        let mut final_diagnostics = std::mem::take(&mut bind.diagnostics);
        final_diagnostics.extend(checker.diagnostics);

        let started = Instant::now();
        let annotations = collect_type_annotations(program, &bind);
        profile.collect_annotations = started.elapsed();
        let flattened = std::mem::take(&mut bind.flattened_members);

        let started = Instant::now();
        for (sid, ty) in &checker.var_types {
            let sym = bind.arena.get_mut(*sid);
            if sym.ty.is_none() || sym.ty.as_ref().unwrap().is_dynamic() {
                sym.ty = Some(ty.clone());
            }
        }
        profile.finalize = started.elapsed();

        CheckResult {
            bind,
            diagnostics: final_diagnostics,
            expr_types: checker.expr_types,
            flattened_members: flattened,
            type_annotations: annotations,
            profile,
            extension_calls: checker.extension_calls,
            extension_members: checker.extension_members,
            extension_set_members: checker.extension_set_members,
        }
    }

    /// Run `f` with a temporary expected type, then restore the previous one.
    pub(crate) fn with_expected<R>(
        &mut self,
        ty: Option<Type>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let prev = self.expected_type.take();
        self.expected_type = ty;
        let result = f(self);
        self.expected_type = prev;
        result
    }

    pub(crate) fn next_child_scope(&mut self, bind: &BindResult) -> Option<ScopeId> {
        let children = &bind.scopes.get(self.current_scope).children;
        let idx = self.child_indices.entry(self.current_scope).or_insert(0);
        if *idx < children.len() {
            let child_id = children[*idx];
            *idx += 1;
            Some(child_id)
        } else {
            None
        }
    }

    pub(crate) fn types_compatible_cached(
        &mut self,
        declared: &Type,
        inferred: &Type,
        bind: Option<&BindResult>,
    ) -> bool {
        compat::types_compatible_with_cache(declared, inferred, bind, &mut self.compat_cache)
    }

    pub(crate) fn mark_infer_env_dirty(&mut self) {
        self.infer_env_rev = self.infer_env_rev.wrapping_add(1);
        if self.infer_cache.len() > 16_384 {
            self.infer_cache.clear();
        }
    }

    pub(crate) fn resolve_type_node_cached(
        &mut self,
        node: &tsn_core::ast::TypeNode,
        bind: &BindResult,
    ) -> Type {
        let key = (
            node as *const tsn_core::ast::TypeNode as usize,
            bind as *const BindResult as usize,
        );
        if let Some(cached) = self.type_node_cache.get(&key) {
            return cached.clone();
        }
        let resolved = crate::binder::resolve_type_node(node, Some(bind));
        self.type_node_cache.insert(key, resolved.clone());
        resolved
    }

    pub(crate) fn symbol_type_params(
        &mut self,
        name: &str,
        kind: crate::symbol::SymbolKind,
        bind: &BindResult,
    ) -> Vec<String> {
        let key = (name.to_owned(), symbol_kind_cache_key(kind));
        if let Some(cached) = self.symbol_type_params_cache.get(&key) {
            return cached.clone();
        }

        let resolved = if let Some(sid) = bind
            .scopes
            .get(bind.global_scope)
            .resolve(name, &bind.scopes)
        {
            let sym = bind.arena.get(sid);
            if sym.kind == kind {
                sym.type_params.clone()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        self.symbol_type_params_cache.insert(key, resolved.clone());
        resolved
    }

    pub(crate) fn symbol_type_params_any(&mut self, name: &str, bind: &BindResult) -> Vec<String> {
        let key = (name.to_owned(), 255);
        if let Some(cached) = self.symbol_type_params_cache.get(&key) {
            return cached.clone();
        }

        let resolved = if let Some(sid) = bind
            .scopes
            .get(bind.global_scope)
            .resolve(name, &bind.scopes)
        {
            bind.arena.get(sid).type_params.clone()
        } else {
            Vec::new()
        };

        self.symbol_type_params_cache.insert(key, resolved.clone());
        resolved
    }
}

fn symbol_kind_cache_key(kind: crate::symbol::SymbolKind) -> u8 {
    match kind {
        crate::symbol::SymbolKind::Var => 0,
        crate::symbol::SymbolKind::Let => 1,
        crate::symbol::SymbolKind::Const => 2,
        crate::symbol::SymbolKind::Function => 3,
        crate::symbol::SymbolKind::Class => 4,
        crate::symbol::SymbolKind::Interface => 5,
        crate::symbol::SymbolKind::TypeAlias => 6,
        crate::symbol::SymbolKind::Enum => 7,
        crate::symbol::SymbolKind::Parameter => 8,
        crate::symbol::SymbolKind::Property => 9,
        crate::symbol::SymbolKind::Method => 10,
        crate::symbol::SymbolKind::TypeParameter => 11,
        crate::symbol::SymbolKind::Namespace => 12,
        crate::symbol::SymbolKind::Struct => 13,
        crate::symbol::SymbolKind::Extension => 14,
    }
}
