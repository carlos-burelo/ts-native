use tower_lsp::lsp_types::HoverContents;

pub fn debug_lsp(path: &str, source: &str) {
    use super::super::colors::{footer, header, BOLD, C_TYPES, R};

    header(C_TYPES, "lsp analysis", path);

    let uri = if cfg!(windows) {
        format!("file:///{}", path.replace('\\', "/"))
    } else {
        format!("file://{}", path)
    };

    let analysis = tsn_lsp::pipeline::run_pipeline(source.to_string(), uri);

    eprintln!("  {}Symbols:{}", BOLD, R);
    for sym in &analysis.symbols {
        if sym.line == u32::MAX {
            continue;
        }
        let kind_str = format!("{:?}", sym.kind);
        eprintln!(
            "    {:<12} {}{:<20}{} : {}{}{} (ln:{})",
            kind_str,
            BOLD,
            sym.name,
            R,
            C_TYPES,
            sym.type_str,
            R,
            sym.line + 1
        );
        for m in &sym.members {
            eprintln!(
                "      {:.>10} {}{:<18}{} : {}{}{}",
                format!("{:?}", m.kind),
                BOLD,
                m.name,
                R,
                C_TYPES,
                m.type_str,
                R
            );
        }
    }

    eprintln!("\n  {}Simulated Hovers:{}", BOLD, R);
    for tok in &analysis.tokens {
        if tok.kind == tsn_core::TokenKind::Identifier || tok.kind.can_be_identifier() {
            if let Some(hover) = tsn_lsp::features::hover::build_hover(&analysis, tok.line, tok.col)
            {
                let content = match hover.contents {
                    HoverContents::Scalar(c) => format_marked_string(c),
                    HoverContents::Array(arr) => arr
                        .into_iter()
                        .map(format_marked_string)
                        .collect::<Vec<_>>()
                        .join(" | "),
                    HoverContents::Markup(m) => m.value,
                };
                eprintln!(
                    "    (ln:{:>2}, col:{:>2}) {:<15} -> {}",
                    tok.line + 1,
                    tok.col + 1,
                    tok.lexeme,
                    content.replace('\n', " ")
                );
            }
        }
    }

    eprintln!("\n  {}Simulated Completions (.):{}", BOLD, R);
    for (_i, tok) in analysis.tokens.iter().enumerate() {
        if tok.kind == tsn_core::TokenKind::Dot {
            let (resp, _) = tsn_lsp::features::completion::build_completion_response(
                &analysis,
                tok.line,
                tok.col + 1,
                Some("."),
                "Invoked".to_string(),
                None,
            );
            if let Some(tower_lsp::lsp_types::CompletionResponse::Array(items)) = resp {
                let labels: Vec<_> = items.iter().map(|it| it.label.clone()).collect();
                eprintln!(
                    "    (ln:{:>2}, col:{:>2}) . -> [{}]",
                    tok.line + 1,
                    tok.col + 1,
                    labels.join(", ")
                );
            }
        }
    }

    if !analysis.expr_types.is_empty() {
        eprintln!("\n  {}Expression Types:{}", BOLD, R);
        let mut sorted: Vec<_> = analysis.expr_types.iter().collect();
        sorted.sort_by_key(|(off, _)| *off);
        for (off, ty) in sorted {
            eprintln!("    offset {:>4} : {}", off, ty);
        }
    }

    eprintln!("\n  {}Semantic Tokens:{}", BOLD, R);
    let sem_tokens = tsn_lsp::features::semantic_tokens::build_semantic_tokens(&analysis);
    let legend = &tsn_lsp::features::semantic_tokens::LEGEND;
    let mut curr_line = 0;
    let mut curr_col = 0;
    for chunk in sem_tokens.chunks_exact(5) {
        let delta_line = chunk[0];
        let delta_start = chunk[1];
        let length = chunk[2];
        let token_type_idx = chunk[3];

        curr_line += delta_line;
        if delta_line == 0 {
            curr_col += delta_start;
        } else {
            curr_col = delta_start;
        }

        let type_name = legend
            .token_types
            .get(token_type_idx as usize)
            .map(|t| t.as_str())
            .unwrap_or(tsn_core::well_known::DYNAMIC);

        // Find exact-start token, or extract sub-range from a containing token.
        let lexeme: String = analysis
            .tokens
            .iter()
            .find(|t| t.line == curr_line && t.col == curr_col)
            .map(|t| t.lexeme[..(length as usize).min(t.lexeme.len())].to_string())
            .or_else(|| {
                analysis
                    .tokens
                    .iter()
                    .find(|t| {
                        t.line == curr_line && t.col <= curr_col && curr_col < t.col + t.length
                    })
                    .map(|t| {
                        let start = (curr_col - t.col) as usize;
                        let end = (start + length as usize).min(t.lexeme.len());
                        t.lexeme[start..end].to_string()
                    })
            })
            .unwrap_or_else(|| "???".to_string());

        eprintln!(
            "    (ln:{:>2}, col:{:>2}) {:<15} -> {}",
            curr_line + 1,
            curr_col + 1,
            lexeme,
            type_name
        );
    }

    if !analysis.diagnostics.is_empty() {
        eprintln!("\n  {}LSP Diagnostics:{}", BOLD, R);
        for diag in &analysis.diagnostics {
            eprintln!("    [line {}] {}", diag.line + 1, diag.message);
        }
    }

    footer(
        C_TYPES,
        &format!(
            "{} symbols, {} expressions, {} semantic tokens",
            analysis.symbols.len(),
            analysis.expr_types.len(),
            sem_tokens.len() / 5
        ),
    );
}

fn format_marked_string(ms: tower_lsp::lsp_types::MarkedString) -> String {
    match ms {
        tower_lsp::lsp_types::MarkedString::String(s) => s,
        tower_lsp::lsp_types::MarkedString::LanguageString(ls) => ls.value,
    }
}
