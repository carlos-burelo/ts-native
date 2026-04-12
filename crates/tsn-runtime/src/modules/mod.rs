pub mod array;
pub mod async_;
pub mod collections;
pub mod console;
pub mod crypto;
pub mod fs;
pub mod globals;
pub mod http;
pub mod io;
pub mod json;
pub mod map;
pub mod math;
pub mod net;
pub mod path;
pub mod primitives;
pub mod reflect;
pub mod set;
pub mod symbols;
pub mod sys;
pub mod testing;
pub mod time;

use tsn_types::Value;

use tsn_modules::{
    STD_ASYNC, STD_COLLECTIONS, STD_CONSOLE, STD_CRYPTO, STD_FS, STD_HTTP, STD_IO, STD_JSON,
    STD_MATH, STD_NET, STD_PATH, STD_REFLECT, STD_SYS, STD_TEST, STD_TIME,
};

type ModuleBuilder = fn() -> Value;

const STD_MODULE_BUILDERS: &[(&str, ModuleBuilder)] = &[
    (STD_ASYNC, async_::build),
    (STD_COLLECTIONS, collections::build),
    (STD_CONSOLE, console::build),
    (STD_CRYPTO, crypto::build),
    (STD_FS, fs::build),
    (STD_HTTP, http::build),
    (STD_IO, io::build),
    (STD_JSON, json::build),
    (STD_MATH, math::build),
    (STD_NET, net::build),
    (STD_PATH, path::build),
    (STD_REFLECT, reflect::build),
    (STD_SYS, sys::build),
    (STD_TEST, testing::build),
    (STD_TIME, time::build),
];

pub fn build_module_by_id(id: &str) -> Option<Value> {
    STD_MODULE_BUILDERS
        .iter()
        .find(|(module_id, _)| *module_id == id)
        .map(|(_, build)| build())
}
