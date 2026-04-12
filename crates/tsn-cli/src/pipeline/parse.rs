/// Parser execution and AST debugging
use crate::args::DebugFlags;
use crate::error::CliError;

use super::colors::C_AST;
use super::debug;

type PipelineResult<T> = Result<T, CliError>;

pub fn parse(
    tokens: Vec<tsn_core::Token>,
    source: &str,
    path: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<tsn_core::ast::Program> {
    let program = tsn_parser::parse(tokens, path).map_err(|errs| {
        let msgs: Vec<String> = errs
            .iter()
            .map(|e| {
                format_error_with_context(
                    source,
                    path,
                    e.range.start.line,
                    e.range.start.column,
                    "parse",
                    &e.message,
                )
            })
            .collect();
        CliError::fatal(msgs.join("\n"))
    })?;

    if verbose {
        eprintln!("[tsn] parsed {} top-level statements", program.body.len());
    }

    if debug.ast {
        super::colors::header(C_AST, "ast", path);
        eprintln!("{:#?}", program);
        super::colors::footer(
            C_AST,
            &format!("{} top-level statements", program.body.len()),
        );
    }

    if debug.symbols {
        debug::debug_symbols(&program);
    }
    if debug.modules {
        debug::debug_modules(&program);
    }
    if debug.types {
        debug::debug_types(&program, debug.types_range);
    }
    if debug.expr {
        debug::debug_expr(&program, debug.expr_range);
    }
    if debug.graph {
        debug::debug_import_graph(&program);
    }
    if debug.lsp {
        debug::debug_lsp(path, source);
    }

    Ok(program)
}

fn format_error_with_context(
    source: &str,
    path: &str,
    line: u32,
    col: u32,
    kind: &str,
    msg: &str,
) -> String {
    use super::colors::*;

    let src_line = source
        .lines()
        .nth((line as usize).saturating_sub(1))
        .unwrap_or("");
    let col_idx = (col as usize).saturating_sub(1);

    let caret_pad = " ".repeat(col_idx);

    let color = if kind == "warning" {
        C_CONSTS
    } else {
        C_ERRORS
    };

    format!(
        "{BOLD}{color}error[{kind}]{R}: {BOLD}{color}{msg}{R}\n  {DIM}┌─{R} {path}:{line}:{col}\n  {DIM}│{R}\n{DIM} {line} │{R}  {src_line}\n  {DIM}│{R} {color}{BOLD}{caret_pad}^\n  {DIM}└─{R} {color}{BOLD}{msg}{R}",
        BOLD = BOLD,
        color = color,
        kind = kind,
        R = R,
        msg = msg,
        DIM = DIM,
        path = path,
        line = line,
        col = col,
        src_line = src_line,
        caret_pad = caret_pad,
    )
}
