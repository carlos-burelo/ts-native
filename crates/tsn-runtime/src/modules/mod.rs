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

pub fn build_module_by_id(id: &str) -> Option<Value> {
    match id {
        "std:async"       => Some(async_::build()),
        "std:collections" => Some(collections::build()),
        "std:console"     => Some(console::build()),
        "std:crypto"      => Some(crypto::build()),
        "std:fs"          => Some(fs::build()),
        "std:http"        => Some(http::build()),
        "std:io"          => Some(io::build()),
        "std:json"        => Some(json::build()),
        "std:math"        => Some(math::build()),
        "std:net"         => Some(net::build()),
        "std:path"        => Some(path::build()),
        "std:reflect"     => Some(reflect::build()),
        "std:sys"         => Some(sys::build()),
        "std:test"        => Some(testing::build()),
        "std:time"        => Some(time::build()),
        _ => None,
    }
}
