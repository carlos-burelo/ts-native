use crate::binder::BindResult;
use crate::module_resolver::{resolve_stdlib_module_bind, resolve_stdlib_module_exports};
use crate::symbol::Symbol;
use crate::types::{ClassMemberInfo, Type};
use std::collections::HashMap;
use std::sync::OnceLock;

const BUILTIN_SPECS: &[&str] = &["builtin:global", "builtin:primitives", "builtin:classes"];
static BUILTIN_EXPORTS: OnceLock<HashMap<String, Symbol>> = OnceLock::new();
static BUILTIN_MEMBERS: OnceLock<BuiltinMembers> = OnceLock::new();

#[derive(Clone, Default)]
struct BuiltinMembers {
    class_methods: HashMap<String, HashMap<String, Type>>,
    class_members: HashMap<String, Vec<ClassMemberInfo>>,
    interface_members: HashMap<String, Vec<ClassMemberInfo>>,
    enum_members: HashMap<String, Vec<ClassMemberInfo>>,
    namespace_members: HashMap<String, Vec<ClassMemberInfo>>,
    flattened_members: HashMap<String, Vec<ClassMemberInfo>>,
    class_parents: HashMap<String, String>,
}

pub fn is_builtin_file(filename: &str) -> bool {
    filename.contains("tsn-stdlib/builtins")
        || filename.contains(r"tsn-stdlibuiltins")
        || filename.starts_with("builtin:")
}

pub fn load_global_exports() -> HashMap<String, Symbol> {
    BUILTIN_EXPORTS.get_or_init(build_builtin_exports).clone()
}

pub fn merge_builtin_members(bind: &mut BindResult) {
    let members = BUILTIN_MEMBERS.get_or_init(build_builtin_members);
    bind.class_members.extend(members.class_members.clone());
    bind.interface_members
        .extend(members.interface_members.clone());
    bind.enum_members.extend(members.enum_members.clone());
    bind.namespace_members
        .extend(members.namespace_members.clone());
    bind.class_methods.extend(members.class_methods.clone());
    bind.flattened_members
        .extend(members.flattened_members.clone());
    bind.class_parents.extend(members.class_parents.clone());
}

fn build_builtin_exports() -> HashMap<String, Symbol> {
    let mut globals = HashMap::new();
    for spec in BUILTIN_SPECS {
        globals.extend(resolve_stdlib_module_exports(spec));
    }
    globals
}

fn build_builtin_members() -> BuiltinMembers {
    let mut members = BuiltinMembers::default();
    for spec in BUILTIN_SPECS {
        if let Some(rb) = resolve_stdlib_module_bind(spec) {
            members.class_members.extend(rb.class_members);
            members.interface_members.extend(rb.interface_members);
            members.enum_members.extend(rb.enum_members);
            members.namespace_members.extend(rb.namespace_members);
            members.class_methods.extend(rb.class_methods);
            members.flattened_members.extend(rb.flattened_members);
            members.class_parents.extend(rb.class_parents);
        }
    }
    members
}
