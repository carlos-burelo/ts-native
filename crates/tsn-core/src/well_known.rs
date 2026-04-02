/// Canonical string names for all built-in types — single source of truth.
/// Lives in tsn-core so every crate (checker, compiler, lsp) can use it
/// without duplicating string literals.

// Primitives
pub const INT: &str = "int";
pub const FLOAT: &str = "float";
pub const DECIMAL: &str = "decimal";
pub const BIGINT: &str = "bigint";
pub const STR: &str = "str";
pub const CHAR: &str = "char";
pub const BOOL: &str = "bool";
pub const SYMBOL: &str = "symbol";
pub const VOID: &str = "void";
pub const NULL: &str = "null";
pub const NEVER: &str = "never";
pub const DYNAMIC: &str = "dynamic";

// Generic builtins
pub const ARRAY: &str = "Array";
pub const FUTURE: &str = "Future";
pub const RECORD: &str = "Record";
pub const MAP: &str = "Map";
pub const SET: &str = "Set";

// Error hierarchy
pub const ERROR: &str = "Error";
pub const TYPE_ERROR: &str = "TypeError";
pub const RANGE_ERROR: &str = "RangeError";

// Special interfaces
pub const DISPOSABLE: &str = "Disposable";
pub const ASYNC_DISPOSABLE: &str = "AsyncDisposable";

// Stdlib types frequently referenced by name
pub const RESULT: &str = "Result";

// Utility types
pub const PARTIAL: &str = "Partial";
pub const REQUIRED: &str = "Required";
pub const READONLY: &str = "Readonly";
pub const NON_NULLABLE: &str = "NonNullable";
pub const MUTABLE: &str = "Mutable";
pub const EXCLUDE: &str = "Exclude";
pub const EXTRACT: &str = "Extract";
pub const RETURN_TYPE: &str = "ReturnType";
pub const AWAITED: &str = "Awaited";
pub const PICK: &str = "Pick";
pub const OMIT: &str = "Omit";
