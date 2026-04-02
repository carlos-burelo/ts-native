use tsn_core::ast::{Decl, ExportDecl, Program, Stmt};

pub fn debug_import_graph(program: &Program) {
    use super::super::{footer, header, BOLD, C_MODULES, DIM, R};
    use super::modules::format_import_specifiers;

    struct Edge {
        source: String,
        line: u32,
        specifiers: String,
        is_reexport: bool,
    }

    let mut edges: Vec<Edge> = Vec::new();

    for stmt in &program.body {
        let Stmt::Decl(decl) = stmt else { continue };
        match decl.as_ref() {
            Decl::Import(imp) => {
                edges.push(Edge {
                    source: imp.source.clone(),
                    line: imp.range.start.line,
                    specifiers: format_import_specifiers(&imp.specifiers),
                    is_reexport: false,
                });
            }
            Decl::Export(exp) => match exp {
                ExportDecl::All {
                    source,
                    alias,
                    range,
                } => {
                    let specs = alias
                        .as_ref()
                        .map(|a| format!("* as {}", a))
                        .unwrap_or_else(|| "*".to_owned());
                    edges.push(Edge {
                        source: source.clone(),
                        line: range.start.line,
                        specifiers: specs,
                        is_reexport: true,
                    });
                }
                ExportDecl::Named {
                    specifiers,
                    source: Some(src),
                    range,
                } => {
                    let names: Vec<String> = specifiers
                        .iter()
                        .map(|s| {
                            if s.local == s.exported {
                                s.local.clone()
                            } else {
                                format!("{} as {}", s.local, s.exported)
                            }
                        })
                        .collect();
                    edges.push(Edge {
                        source: src.clone(),
                        line: range.start.line,
                        specifiers: format!("{{ {} }}", names.join(", ")),
                        is_reexport: true,
                    });
                }
                _ => {}
            },
            _ => {}
        }
    }

    header(C_MODULES, "import-graph", &program.filename);

    if edges.is_empty() {
        eprintln!("  {}(no imports or re-exports){}", DIM, R);
        footer(C_MODULES, "0 edge(s)");
        return;
    }

    let src_w = edges
        .iter()
        .map(|e| e.source.len() + 2)
        .max()
        .unwrap_or(14)
        .max(14);

    eprintln!("  {}*{} {}{}{}", DIM, R, BOLD, program.filename, R);

    let total = edges.len();
    let reexport_count = edges.iter().filter(|e| e.is_reexport).count();

    for (i, edge) in edges.iter().enumerate() {
        let is_last = i + 1 == total;
        let branch = if is_last { "\\--" } else { "+--" };
        let src_q = format!("\"{}\"", edge.source);
        let reexport = if edge.is_reexport {
            format!("  {}[re-export]{}", DIM, R)
        } else {
            String::new()
        };

        eprintln!(
            "  {}{}{} {}{:<src_w$}{}  {}ln:{:>3}{}  {}{}{}{}",
            DIM, branch, R, BOLD, src_q, R, DIM, edge.line, R, BOLD, edge.specifiers, R, reexport
        );
    }

    let note = if reexport_count > 0 {
        format!("{} edge(s), {} re-export(s)", total, reexport_count)
    } else {
        format!("{} edge(s)", total)
    };
    footer(C_MODULES, &note);
}
