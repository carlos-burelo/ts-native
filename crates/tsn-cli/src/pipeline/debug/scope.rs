use tsn_compiler::{FunctionProto, Literal, PoolEntry};

pub fn debug_scope(proto: &FunctionProto, filename: &str) {
    use super::super::{footer, header, C_SCOPE};
    header(C_SCOPE, "scope", filename);
    let mut count = 0usize;
    print_fn_scope(proto, 1, &mut count);
    footer(C_SCOPE, &format!("{} scope(s)", count));
}

fn print_fn_scope(proto: &FunctionProto, depth: usize, count: &mut usize) {
    use super::super::{BOLD, C_SCOPE, DIM, R};
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
        pad, C_SCOPE, BOLD, name, R, proto.arity, proto.upvalue_count, flags
    );
    *count += 1;

    let strs: Vec<&str> = proto
        .chunk
        .constants
        .iter()
        .filter_map(|e| {
            if let PoolEntry::Literal(Literal::Str(s)) = e {
                Some(s.as_ref())
            } else {
                None
            }
        })
        .collect();
    if !strs.is_empty() {
        let preview: Vec<String> = strs.iter().take(12).map(|s| format!("{:?}", s)).collect();
        let suffix = if strs.len() > 12 {
            format!(" +{}", strs.len() - 12)
        } else {
            String::new()
        };
        eprintln!("{}{}  str: {}{}{}", pad, DIM, preview.join(" "), suffix, R);
    }

    for entry in &proto.chunk.constants {
        if let PoolEntry::Function(nested) = entry {
            print_fn_scope(nested, depth + 1, count);
        }
    }
}
