use super::{BUILTIN_CLASSES, BUILTIN_GLOBAL, BUILTIN_PRIMITIVES};
use super::spec::{ModuleKind, ModuleSpec};

/// The single source of truth for all module identity in the TSN system.
/// Every module that exists MUST be registered here.
pub static MODULE_REGISTRY: &[ModuleSpec] = &[
    // ── Builtin modules (eager, injected into global scope) ─────────
    ModuleSpec::new(BUILTIN_GLOBAL,     ModuleKind::Builtin, "builtins/global.tsn"),
    ModuleSpec::new(BUILTIN_PRIMITIVES, ModuleKind::Builtin, "builtins/primitives.tsn"),
    ModuleSpec::new(BUILTIN_CLASSES,    ModuleKind::Builtin, "builtins/classes.tsn"),

    // ── Stdlib modules (lazy, loaded on first import) ────────────────
    ModuleSpec::new("std:async",          ModuleKind::Stdlib, "std/async/mod.tsn"),
    ModuleSpec::new("std:collections",    ModuleKind::Stdlib, "std/collections/mod.tsn"),
    ModuleSpec::new("std:console",        ModuleKind::Stdlib, "std/console/mod.tsn"),
    ModuleSpec::new("std:crypto",         ModuleKind::Stdlib, "std/crypto/mod.tsn"),
    ModuleSpec::new("std:dispose",        ModuleKind::Stdlib, "std/dispose/mod.tsn"),
    ModuleSpec::new("std:fs",             ModuleKind::Stdlib, "std/fs/mod.tsn"),
    ModuleSpec::new("std:http",           ModuleKind::Stdlib, "std/http/mod.tsn"),
    ModuleSpec::new("std:io",             ModuleKind::Stdlib, "std/io/mod.tsn"),
    ModuleSpec::new("std:json",           ModuleKind::Stdlib, "std/json/mod.tsn"),
    ModuleSpec::new("std:math",           ModuleKind::Stdlib, "std/math/mod.tsn"),
    ModuleSpec::new("std:net",            ModuleKind::Stdlib, "std/net/mod.tsn"),
    ModuleSpec::new("std:path",           ModuleKind::Stdlib, "std/path/mod.tsn"),
    ModuleSpec::new("std:reflect",        ModuleKind::Stdlib, "std/reflect/mod.tsn"),
    ModuleSpec::new("std:result",         ModuleKind::Stdlib, "std/result/mod.tsn"),
    ModuleSpec::new("std:sys",            ModuleKind::Stdlib, "std/sys/mod.tsn"),
    ModuleSpec::new("std:test",           ModuleKind::Stdlib, "std/test/mod.tsn"),
    ModuleSpec::new("std:time",           ModuleKind::Stdlib, "std/time/mod.tsn"),
    ModuleSpec::new("std:types",          ModuleKind::Stdlib, "std/types/mod.tsn"),
];

/// Returns true if the given specifier is a known module in the registry.
pub fn is_known(specifier: &str) -> bool {
    MODULE_REGISTRY.iter().any(|m| m.id == specifier)
}

/// Returns the spec for a known module, or None.
pub fn spec_for(specifier: &str) -> Option<&'static ModuleSpec> {
    MODULE_REGISTRY.iter().find(|m| m.id == specifier)
}
