use crate::scope::{Scope, ScopeArena, ScopeId, ScopeKind};
use crate::symbol::{Symbol, SymbolArena, SymbolKind};
use crate::types::Type;
use std::collections::HashMap;
use tsn_core::ast::{ForInit, Program, Stmt, VarDeclarator};
use tsn_core::Diagnostic;
mod decl_classes;
mod decl_values;
mod decls;
mod imports;
mod type_inference;
mod type_resolution;
use crate::module_resolver::resolve_module_bind_ref;
pub use crate::types::{ClassMemberInfo, ClassMemberKind, TypeContext};
use tsn_core::ast::{Expr, Pattern, TypeNode, VarKind};
pub use type_inference::{infer_expr_type, pattern_lead_name, widen_literal};
pub use type_resolution::{resolve_primitive, resolve_type_node};

#[derive(Clone)]
pub struct BindResult {
    pub arena: SymbolArena,
    pub scopes: ScopeArena,
    pub global_scope: ScopeId,
    pub diagnostics: Vec<Diagnostic>,
    pub class_methods: HashMap<String, HashMap<String, Type>>,
    pub class_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub interface_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub object_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub enum_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub namespace_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub flattened_members: HashMap<String, Vec<ClassMemberInfo>>,
    pub class_parents: HashMap<String, String>,
    pub override_errors: Vec<(String, String, String, u32, u32)>,
    pub source_file: String,
    pub sum_type_variants: HashMap<String, Vec<String>>,
    pub sum_variant_parent: HashMap<String, String>,
    pub sum_variant_fields: HashMap<String, Vec<(String, Type)>>,
    pub extension_methods: HashMap<String, HashMap<String, String>>,
    pub extension_getters: HashMap<String, HashMap<String, String>>,
    pub extension_setters: HashMap<String, HashMap<String, String>>,
}

impl TypeContext for BindResult {
    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.interface_members.get(name).cloned();
                }
            }
        }
        self.interface_members.get(name).cloned()
    }

    fn get_class_members(&self, name: &str, origin: Option<&str>) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.class_members.get(name).cloned();
                }
            }
        }
        self.class_members.get(name).cloned()
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.namespace_members.get(name).cloned();
                }
            }
        }
        self.namespace_members.get(name).cloned()
    }

    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        let scope = self.scopes.get(self.global_scope);
        let id = scope.resolve(name, &self.scopes)?;
        self.arena.get(id).ty.clone()
    }

    fn source_file(&self) -> Option<&str> {
        Some(&self.source_file)
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        let scope = self.scopes.get(self.global_scope);
        let id = scope.resolve(name, &self.scopes)?;
        let sym = self.arena.get(id);
        let node = sym.alias_node.as_ref()?;
        Some((sym.type_params.clone(), *node.clone()))
    }
}

impl BindResult {
    pub fn global_symbols(&self) -> impl Iterator<Item = &Symbol> {
        let scope = self.scopes.get(self.global_scope);
        scope.ordered.iter().map(|&id| self.arena.get(id))
    }
}

pub struct Binder {
    arena: SymbolArena,
    scopes: ScopeArena,
    current: ScopeId,
    class_methods: HashMap<String, HashMap<String, Type>>,
    class_members: HashMap<String, Vec<ClassMemberInfo>>,
    interface_members: HashMap<String, Vec<ClassMemberInfo>>,
    object_members: HashMap<String, Vec<ClassMemberInfo>>,
    enum_members: HashMap<String, Vec<ClassMemberInfo>>,
    namespace_members: HashMap<String, Vec<ClassMemberInfo>>,
    flattened_members: HashMap<String, Vec<ClassMemberInfo>>,
    class_parents: HashMap<String, String>,
    override_errors: Vec<(String, String, String, u32, u32)>,
    diagnostics: Vec<Diagnostic>,
    pub(crate) source_file: String,
    sum_type_variants: HashMap<String, Vec<String>>,
    sum_variant_parent: HashMap<String, String>,
    sum_variant_fields: HashMap<String, Vec<(String, Type)>>,
    pub(crate) extension_methods: HashMap<String, HashMap<String, String>>,
    pub(crate) extension_getters: HashMap<String, HashMap<String, String>>,
    pub(crate) extension_setters: HashMap<String, HashMap<String, String>>,
}

impl TypeContext for Binder {
    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.interface_members.get(name).cloned();
                }
            }
        }
        self.interface_members.get(name).cloned()
    }

    fn get_class_members(&self, name: &str, origin: Option<&str>) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.class_members.get(name).cloned();
                }
            }
        }
        self.class_members.get(name).cloned()
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin {
            if origin != self.source_file {
                if let Some(rb) = resolve_module_bind_ref(origin) {
                    return rb.namespace_members.get(name).cloned();
                }
            }
        }
        self.namespace_members.get(name).cloned()
    }

    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        let scope = self.scopes.get(self.current);
        let id = scope.resolve(name, &self.scopes)?;
        self.arena.get(id).ty.clone()
    }

    fn source_file(&self) -> Option<&str> {
        Some(&self.source_file)
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        let scope = self.scopes.get(self.current);
        let id = scope.resolve(name, &self.scopes)?;
        let sym = self.arena.get(id);
        let node = sym.alias_node.as_ref()?;
        Some((sym.type_params.clone(), *node.clone()))
    }
}

impl Binder {
    pub fn bind(program: &Program) -> BindResult {
        Self::bind_with_globals_iter(program, HashMap::new())
    }

    pub fn bind_with_globals(program: &Program, globals: HashMap<String, Symbol>) -> BindResult {
        Self::bind_with_globals_iter(program, globals)
    }

    pub fn bind_with_global_refs(
        program: &Program,
        globals: &HashMap<String, Symbol>,
    ) -> BindResult {
        Self::bind_with_globals_iter(
            program,
            globals
                .iter()
                .map(|(name, sym)| (name.clone(), sym.clone())),
        )
    }

    fn bind_with_globals_iter<I>(program: &Program, globals: I) -> BindResult
    where
        I: IntoIterator<Item = (String, Symbol)>,
    {
        let mut b = Binder {
            arena: SymbolArena::default(),
            scopes: ScopeArena::default(),
            current: 0,
            class_methods: HashMap::new(),
            class_members: HashMap::new(),
            interface_members: HashMap::new(),
            object_members: HashMap::new(),
            enum_members: HashMap::new(),
            namespace_members: HashMap::new(),
            flattened_members: HashMap::new(),
            class_parents: HashMap::new(),
            override_errors: Vec::new(),
            diagnostics: Vec::new(),
            source_file: program.filename.clone(),
            sum_type_variants: HashMap::new(),
            sum_variant_parent: HashMap::new(),
            sum_variant_fields: HashMap::new(),
            extension_methods: HashMap::new(),
            extension_getters: HashMap::new(),
            extension_setters: HashMap::new(),
        };

        let global = b.scopes.push(Scope::new(ScopeKind::Global, None));
        b.current = global;

        for (name, sym) in globals {
            let id = b.arena.push(sym);
            b.scopes.get_mut(global).define(name, id);
        }

        b.bind_stmts(&program.body);

        BindResult {
            arena: b.arena,
            scopes: b.scopes,
            global_scope: global,
            diagnostics: b.diagnostics,
            class_methods: b.class_methods,
            class_members: b.class_members,
            interface_members: b.interface_members,
            object_members: b.object_members,
            enum_members: b.enum_members,
            namespace_members: b.namespace_members,
            flattened_members: b.flattened_members,
            class_parents: b.class_parents,
            override_errors: b.override_errors,
            source_file: b.source_file,
            sum_type_variants: b.sum_type_variants,
            sum_variant_parent: b.sum_variant_parent,
            sum_variant_fields: b.sum_variant_fields,
            extension_methods: b.extension_methods,
            extension_getters: b.extension_getters,
            extension_setters: b.extension_setters,
        }
    }

    fn bind_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.bind_stmt(stmt);
        }
    }

    fn bind_var_declarators(
        &mut self,
        declarators: &[VarDeclarator],
        kind: VarKind,
        doc: Option<&String>,
    ) {
        let sym_kind = match kind {
            VarKind::Const => SymbolKind::Const,
            VarKind::Let => SymbolKind::Let,
        };

        for declarator in declarators {
            let line = declarator.range.start.line;
            let ty = declarator
                .type_ann
                .as_ref()
                .or_else(|| match &declarator.id {
                    Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|ann| resolve_type_node(ann, Some(self)))
                .or_else(|| {
                    declarator
                        .init
                        .as_ref()
                        .map(|expr| infer_expr_type(expr, Some(self)))
                        .filter(|ty| !ty.is_dynamic())
                });

            self.bind_pattern(&declarator.id, sym_kind, line, doc.cloned(), ty);

            if let Pattern::Identifier { name, .. } = &declarator.id {
                if let Some(Expr::Object { properties, .. }) = &declarator.init {
                    let fields = self.collect_object_members(properties);
                    if !fields.is_empty() {
                        self.object_members.insert(name.clone(), fields);
                    }
                }
            }

            if let Some(init_expr) = &declarator.init {
                self.bind_expr(init_expr);
            }
        }
    }

    fn bind_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Decl(decl) => self.bind_decl(decl),
            Stmt::Block { stmts, .. } => {
                let child = self.scopes.child(ScopeKind::Block, self.current);
                let saved = self.current;
                self.current = child;
                self.bind_stmts(stmts);
                self.current = saved;
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.bind_expr(test);
                self.bind_stmt(consequent);
                if let Some(alt) = alternate {
                    self.bind_stmt(alt);
                }
            }
            Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
                self.bind_expr(test);
                self.bind_stmt(body);
            }
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                if let Some(init) = init {
                    match init.as_ref() {
                        ForInit::Var { kind, declarators } => {
                            self.bind_var_declarators(declarators, *kind, None);
                        }
                        ForInit::Expr(e) => {
                            self.bind_expr(e);
                        }
                    }
                }
                if let Some(t) = test {
                    self.bind_expr(t);
                }
                if let Some(u) = update {
                    self.bind_expr(u);
                }
                self.bind_stmt(body);
            }
            Stmt::ForIn {
                kind: _,
                left,
                right,
                body,
                ..
            }
            | Stmt::ForOf {
                kind: _,
                left,
                right,
                body,
                ..
            } => {
                self.bind_pattern(left, SymbolKind::Let, right.range().start.line, None, None);
                self.bind_expr(right);
                self.bind_stmt(body);
            }
            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.bind_expr(discriminant);
                for case in cases {
                    if let Some(t) = &case.test {
                        self.bind_expr(t);
                    }
                    self.bind_stmts(&case.body);
                }
            }
            Stmt::Try {
                block,
                catch,
                finally,
                ..
            } => {
                self.bind_stmt(block);
                if let Some(clause) = catch {
                    if let Some(p) = &clause.param {
                        self.bind_pattern(p, SymbolKind::Let, block.range().start.line, None, None);
                    }
                    self.bind_stmt(&clause.body);
                }
                if let Some(fin) = finally {
                    self.bind_stmt(fin);
                }
            }
            Stmt::Labeled { body, .. } => {
                self.bind_stmt(body);
            }
            Stmt::Expr { expression, .. } => {
                self.bind_expr(expression);
            }
            Stmt::Return { argument, .. } => {
                if let Some(arg) = argument {
                    self.bind_expr(arg);
                }
            }
            Stmt::Throw { argument, .. } => {
                self.bind_expr(argument);
            }
            _ => {}
        }
    }
}
