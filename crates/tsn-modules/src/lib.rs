mod loader;
mod registry;
mod spec;

pub const BUILTIN_GLOBAL: &str = "builtin:global";
pub const BUILTIN_PRIMITIVES: &str = "builtin:primitives";
pub const BUILTIN_CLASSES: &str = "builtin:classes";

pub const STD_ASYNC: &str = "std:async";
pub const STD_COLLECTIONS: &str = "std:collections";
pub const STD_CONSOLE: &str = "std:console";
pub const STD_CRYPTO: &str = "std:crypto";
pub const STD_DISPOSE: &str = "std:dispose";
pub const STD_FS: &str = "std:fs";
pub const STD_HTTP: &str = "std:http";
pub const STD_IO: &str = "std:io";
pub const STD_JSON: &str = "std:json";
pub const STD_MATH: &str = "std:math";
pub const STD_NET: &str = "std:net";
pub const STD_PATH: &str = "std:path";
pub const STD_REFLECT: &str = "std:reflect";
pub const STD_RESULT: &str = "std:result";
pub const STD_SYS: &str = "std:sys";
pub const STD_TEST: &str = "std:test";
pub const STD_TIME: &str = "std:time";
pub const STD_TYPES: &str = "std:types";

pub const BUILTIN_MODULES: &[&str] = &[BUILTIN_GLOBAL, BUILTIN_PRIMITIVES, BUILTIN_CLASSES];

pub const STD_MODULES: &[&str] = &[
    STD_ASYNC,
    STD_COLLECTIONS,
    STD_CONSOLE,
    STD_CRYPTO,
    STD_DISPOSE,
    STD_FS,
    STD_HTTP,
    STD_IO,
    STD_JSON,
    STD_MATH,
    STD_NET,
    STD_PATH,
    STD_REFLECT,
    STD_RESULT,
    STD_SYS,
    STD_TEST,
    STD_TIME,
    STD_TYPES,
];

pub use loader::ModuleLoader;
pub use registry::{is_known, spec_for, MODULE_REGISTRY};
pub use spec::{ModuleKind, ModuleSpec};
