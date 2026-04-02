use tsn_compiler::{FunctionProto, PoolEntry};

pub fn debug_binds(proto: &FunctionProto, filename: &str) {
    use super::super::{footer, header, C_BINDS};
    header(C_BINDS, "binds", filename);
    let mut count = 0usize;
    print_fn_binds(proto, 1, &mut count);
    footer(C_BINDS, &format!("{} function(s)", count));
}

fn print_fn_binds(proto: &FunctionProto, depth: usize, count: &mut usize) {
    use super::super::{BOLD, C_BINDS, R};
    let pad = "  ".repeat(depth);
    let name = proto.name.as_deref().unwrap_or("<anon>");
    let mut flags = String::new();
    if proto.is_async {
        flags.push_str(" async");
    }
    if proto.is_generator {
        flags.push_str(" gen");
    }
    eprintln!(
        "{}{}fn {}{:<24}{}  arity:{:<3}  upvalues:{}{}",
        pad, C_BINDS, BOLD, name, R, proto.arity, proto.upvalue_count, flags
    );
    *count += 1;
    for entry in &proto.chunk.constants {
        if let PoolEntry::Function(nested) = entry {
            print_fn_binds(nested, depth + 1, count);
        }
    }
}
