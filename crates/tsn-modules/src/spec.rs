/// Module kind: Builtin (always loaded) or Stdlib (loaded on first import).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModuleKind {
    /// Injected into global scope of every file. No import needed.
    Builtin,
    /// Loaded lazily on first `import { X } from "std:foo"`.
    Stdlib,
}

/// Canonical descriptor of a known module.
pub struct ModuleSpec {
    /// e.g. "builtin:global", "std:math"
    pub id: &'static str,
    pub kind: ModuleKind,
    /// Relative path to the TSN source file inside the stdlib root.
    /// e.g. "std/math/mod.tsn" or "builtins/global.tsn"
    pub tsn_source: &'static str,
}

impl ModuleSpec {
    pub const fn new(id: &'static str, kind: ModuleKind, tsn_source: &'static str) -> Self {
        Self {
            id,
            kind,
            tsn_source,
        }
    }
}
