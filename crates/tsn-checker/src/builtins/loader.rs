use crate::binder::BindResult;
use crate::module_resolver::{resolve_stdlib_module_bind, resolve_stdlib_module_exports};
use crate::symbol::Symbol;
use crate::types::{ClassMemberInfo, Type};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::OnceLock;

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

pub fn global_exports_ref() -> &'static HashMap<String, Symbol> {
    BUILTIN_EXPORTS.get_or_init(build_builtin_exports)
}

pub fn merge_builtin_members(bind: &mut BindResult) {
    let members = BUILTIN_MEMBERS.get_or_init(build_builtin_members);
    merge_cloned_map(&mut bind.class_members, &members.class_members);
    merge_cloned_map(&mut bind.interface_members, &members.interface_members);
    merge_cloned_map(&mut bind.enum_members, &members.enum_members);
    merge_cloned_map(&mut bind.namespace_members, &members.namespace_members);
    merge_cloned_map(&mut bind.class_methods, &members.class_methods);
    merge_cloned_map(&mut bind.flattened_members, &members.flattened_members);
    merge_cloned_map(&mut bind.class_parents, &members.class_parents);
}

fn merge_cloned_map<K, V>(target: &mut HashMap<K, V>, source: &HashMap<K, V>)
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    target.reserve(source.len());
    for (key, value) in source {
        target.insert(key.clone(), value.clone());
    }
}

fn build_builtin_exports() -> HashMap<String, Symbol> {
    let mut globals = HashMap::new();
    for spec in tsn_modules::BUILTIN_MODULES {
        globals.extend(resolve_stdlib_module_exports(spec));
    }
    globals
}

fn build_builtin_members() -> BuiltinMembers {
    let mut members = BuiltinMembers::default();
    for spec in tsn_modules::BUILTIN_MODULES {
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
