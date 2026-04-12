// Props module - re-exports property access implementations
mod getter_setter;
mod index;
mod iterator;
mod property;
mod symbol;

// Re-export as Vm extension methods
pub use index::{get_index, set_index};
pub use property::{get_property_cached, set_property_cached};
pub use symbol::get_symbol_property;

// Re-export primitive property modules
pub(crate) mod array;
mod bool;
mod decimal;
mod float;
pub(super) mod future;
pub(super) mod generator;
mod int;
mod range;
mod str;
