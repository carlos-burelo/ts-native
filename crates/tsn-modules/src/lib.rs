mod loader;
mod registry;
mod spec;

pub const BUILTIN_GLOBAL: &str = "builtin:global";
pub const BUILTIN_PRIMITIVES: &str = "builtin:primitives";
pub const BUILTIN_CLASSES: &str = "builtin:classes";

pub const BUILTIN_MODULES: &[&str] = &[BUILTIN_GLOBAL, BUILTIN_PRIMITIVES, BUILTIN_CLASSES];

pub use loader::ModuleLoader;
pub use registry::{is_known, spec_for, MODULE_REGISTRY};
pub use spec::{ModuleKind, ModuleSpec};
