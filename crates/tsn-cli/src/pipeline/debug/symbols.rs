use tsn_core::ast::{Decl, Pattern, Program, Stmt, VarKind};

pub fn debug_symbols(program: &Program) {
    use super::super::colors::{footer, header, C_SYMBOLS};
    header(C_SYMBOLS, "symbols", &program.filename);
    let mut count = 0usize;
    collect_symbol_stmts(&program.body, 1, &mut count);
    footer(C_SYMBOLS, &format!("{} symbol(s)", count));
}

fn collect_symbol_stmts(stmts: &[Stmt], depth: usize, count: &mut usize) {
    let pad = "  ".repeat(depth);
    for stmt in stmts {
        let Stmt::Decl(decl) = stmt else { continue };
        collect_symbol_decl(decl, &pad, count);
    }
}

fn collect_symbol_decl(decl: &Decl, pad: &str, count: &mut usize) {
    use super::super::colors::{BOLD, C_SYMBOLS, R};
    match decl {
        Decl::Function(f) => {
            let mut flags = String::new();
            if f.modifiers.is_async {
                flags.push_str(" async");
            }
            if f.modifiers.is_generator {
                flags.push_str(" gen");
            }
            eprintln!(
                "{}{}fn   {}{:<24}{}  ln:{:<5}  (params:{}{})",
                pad,
                C_SYMBOLS,
                BOLD,
                f.id,
                R,
                f.range.start.line,
                f.params.len(),
                flags
            );
            *count += 1;
        }
        Decl::Class(c) => {
            let name = c.id.as_deref().unwrap_or("<anon>");
            let ext = if c.super_class.is_some() {
                " extends ..."
            } else {
                ""
            };
            eprintln!(
                "{}{}cls  {}{:<24}{}  ln:{:<5}{}",
                pad, C_SYMBOLS, BOLD, name, R, c.range.start.line, ext
            );
            *count += 1;
        }
        Decl::Variable(v) => {
            let kw = match v.kind {
                VarKind::Const => "const",
                VarKind::Let => "let  ",
            };
            for d in &v.declarators {
                let name = pattern_lead_name(&d.id);
                eprintln!(
                    "{}{}var  {}{} {:<20}{}  ln:{}",
                    pad, C_SYMBOLS, BOLD, kw, name, R, d.range.start.line
                );
                *count += 1;
            }
        }
        Decl::Interface(i) => {
            eprintln!(
                "{}{}iface{}{:<24}{}  ln:{}",
                pad, C_SYMBOLS, BOLD, i.id, R, i.range.start.line
            );
            *count += 1;
        }
        Decl::TypeAlias(t) => {
            eprintln!(
                "{}{}type {}{:<24}{}  ln:{}",
                pad, C_SYMBOLS, BOLD, t.id, R, t.range.start.line
            );
            *count += 1;
        }
        Decl::Enum(e) => {
            eprintln!(
                "{}{}enum {}{:<24}{}  ln:{:<5}  ({} members)",
                pad,
                C_SYMBOLS,
                BOLD,
                e.id,
                R,
                e.range.start.line,
                e.members.len()
            );
            *count += 1;
        }
        Decl::Namespace(n) => {
            eprintln!(
                "{}{}ns   {}{:<24}{}  ln:{}",
                pad, C_SYMBOLS, BOLD, n.id, R, n.range.start.line
            );
            *count += 1;
            let child_pad = format!("  {}", pad);
            for d in &n.body {
                collect_symbol_decl(d, &child_pad, count);
            }
        }
        Decl::Struct(s) => {
            eprintln!(
                "{}{}strct{}{:<24}{}  ln:{:<5}  ({} fields)",
                pad,
                C_SYMBOLS,
                BOLD,
                s.id,
                R,
                s.range.start.line,
                s.fields.len()
            );
            *count += 1;
        }
        Decl::Extension(e) => {
            eprintln!(
                "{}{}ext  {}<type>{}  ln:{}",
                pad, C_SYMBOLS, BOLD, R, e.range.start.line
            );
            *count += 1;
        }
        Decl::SumType(st) => {
            eprintln!(
                "{}{}sum  {}{:<24}{}  ln:{:<5}  ({} variants)",
                pad,
                C_SYMBOLS,
                BOLD,
                st.id,
                R,
                st.range.start.line,
                st.variants.len()
            );
            *count += 1;
        }
        Decl::Import(_) | Decl::Export(_) => {}
    }
}

fn pattern_lead_name(p: &Pattern) -> &str {
    match p {
        Pattern::Identifier { name, .. } => name,
        Pattern::Array { .. } => "<array>",
        Pattern::Object { .. } => "<object>",
        Pattern::Rest { .. } => "<rest>",
        Pattern::Assignment { left, .. } => pattern_lead_name(left),
    }
}
