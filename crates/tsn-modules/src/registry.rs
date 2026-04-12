use super::spec::{ModuleKind, ModuleSpec};
use super::{
    BUILTIN_CLASSES, BUILTIN_GLOBAL, BUILTIN_PRIMITIVES, STD_ASYNC, STD_COLLECTIONS, STD_CONSOLE,
    STD_CRYPTO, STD_DISPOSE, STD_FS, STD_HTTP, STD_IO, STD_JSON, STD_MATH, STD_NET, STD_PATH,
    STD_REFLECT, STD_RESULT, STD_SYS, STD_TEST, STD_TIME, STD_TYPES,
};

/// The single source of truth for all module identity in the TSN system.
/// Every module that exists MUST be registered here.
pub static MODULE_REGISTRY: &[ModuleSpec] = &[
    // Builtin modules (eager, injected into global scope).
    ModuleSpec::new(BUILTIN_GLOBAL, ModuleKind::Builtin, "builtins/global.tsn"),
    ModuleSpec::new(
        BUILTIN_PRIMITIVES,
        ModuleKind::Builtin,
        "builtins/primitives.tsn",
    ),
    ModuleSpec::new(BUILTIN_CLASSES, ModuleKind::Builtin, "builtins/classes.tsn"),
    // Stdlib modules (lazy, loaded on first import).
    ModuleSpec::new(STD_ASYNC, ModuleKind::Stdlib, "std/async/mod.tsn"),
    ModuleSpec::new(
        STD_COLLECTIONS,
        ModuleKind::Stdlib,
        "std/collections/mod.tsn",
    ),
    ModuleSpec::new(STD_CONSOLE, ModuleKind::Stdlib, "std/console/mod.tsn"),
    ModuleSpec::new(STD_CRYPTO, ModuleKind::Stdlib, "std/crypto/mod.tsn"),
    ModuleSpec::new(STD_DISPOSE, ModuleKind::Stdlib, "std/dispose/mod.tsn"),
    ModuleSpec::new(STD_FS, ModuleKind::Stdlib, "std/fs/mod.tsn"),
    ModuleSpec::new(STD_HTTP, ModuleKind::Stdlib, "std/http/mod.tsn"),
    ModuleSpec::new(STD_IO, ModuleKind::Stdlib, "std/io/mod.tsn"),
    ModuleSpec::new(STD_JSON, ModuleKind::Stdlib, "std/json/mod.tsn"),
    ModuleSpec::new(STD_MATH, ModuleKind::Stdlib, "std/math/mod.tsn"),
    ModuleSpec::new(STD_NET, ModuleKind::Stdlib, "std/net/mod.tsn"),
    ModuleSpec::new(STD_PATH, ModuleKind::Stdlib, "std/path/mod.tsn"),
    ModuleSpec::new(STD_REFLECT, ModuleKind::Stdlib, "std/reflect/mod.tsn"),
    ModuleSpec::new(STD_RESULT, ModuleKind::Stdlib, "std/result/mod.tsn"),
    ModuleSpec::new(STD_SYS, ModuleKind::Stdlib, "std/sys/mod.tsn"),
    ModuleSpec::new(STD_TEST, ModuleKind::Stdlib, "std/test/mod.tsn"),
    ModuleSpec::new(STD_TIME, ModuleKind::Stdlib, "std/time/mod.tsn"),
    ModuleSpec::new(STD_TYPES, ModuleKind::Stdlib, "std/types/mod.tsn"),
];

/// Returns true if the given specifier is a known module in the registry.
pub fn is_known(specifier: &str) -> bool {
    MODULE_REGISTRY.iter().any(|m| m.id == specifier)
}

/// Returns the spec for a known module, or None.
pub fn spec_for(specifier: &str) -> Option<&'static ModuleSpec> {
    MODULE_REGISTRY.iter().find(|m| m.id == specifier)
}
