use tsn_compiler::{FunctionProto, Literal, PoolEntry};
use tsn_core::well_known;

pub fn debug_consts(proto: &FunctionProto, filename: &str) {
    use super::super::colors::{footer, header, C_CONSTS};
    header(C_CONSTS, "consts", filename);
    let mut total = 0usize;
    print_fn_consts(proto, "", &mut total);
    footer(C_CONSTS, &format!("{} constant(s) total", total));
}

fn print_fn_consts(proto: &FunctionProto, indent: &str, total: &mut usize) {
    let name = proto.name.as_deref().unwrap_or("<anon>");
    eprintln!(
        "{}+-- fn {}  ({} constants)",
        indent,
        name,
        proto.chunk.constants.len()
    );
    for (i, entry) in proto.chunk.constants.iter().enumerate() {
        eprintln!("{}|  [{:03}] {}", indent, i, pool_const_desc(entry));
        *total += 1;
    }
    eprintln!("{}+--", indent);
    let nested = format!("{}  ", indent);
    for entry in &proto.chunk.constants {
        if let PoolEntry::Function(nested_proto) = entry {
            print_fn_consts(nested_proto, &nested, total);
        }
    }
}

fn pool_const_desc(entry: &PoolEntry) -> String {
    match entry {
        PoolEntry::Literal(Literal::Null) => well_known::NULL.to_owned(),
        PoolEntry::Literal(Literal::Bool(b)) => format!("bool   {}", b),
        PoolEntry::Literal(Literal::Int(n)) => format!("int    {}", n),
        PoolEntry::Literal(Literal::Float(f)) => format!("float  {}", f),
        PoolEntry::Literal(Literal::Str(s)) => format!("str    {:?}", s.as_ref()),
        PoolEntry::Literal(Literal::BigInt(n)) => format!("bigint  {}", n),
        PoolEntry::Literal(Literal::Decimal(d)) => format!("decimal {}", d),
        PoolEntry::Literal(Literal::Symbol(s)) => format!("symbol  {:?}", s),
        PoolEntry::Function(p) => {
            let name = p.name.as_deref().unwrap_or("<anon>");
            let flags = if p.is_async {
                " async"
            } else if p.is_generator {
                " gen"
            } else {
                ""
            };
            format!("fn     {} (arity={}{})", name, p.arity, flags)
        }
    }
}
