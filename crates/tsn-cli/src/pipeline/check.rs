/// Type checking and compile-time error formatting
use tsn_checker::Checker;
use tsn_core::ast::Program;

use super::colors::{BOLD, C_ERRORS, R};
use crate::error::CliError;

type PipelineResult<T> = Result<T, CliError>;

pub struct CheckResult {
    pub checker_result: tsn_checker::CheckResult,
}

pub fn check(program: &Program, source: &str) -> PipelineResult<CheckResult> {
    let check_result = Checker::check(program);
    if !check_result.diagnostics.is_empty() {
        let mut msgs = Vec::new();
        let error_count = check_result
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .count();

        for d in &check_result.diagnostics {
            msgs.push(format_error_with_context(
                source,
                &program.filename,
                d.range.start.line,
                d.range.start.column,
                if d.is_error() { "type" } else { "warning" },
                &d.message,
            ));
        }

        if error_count > 0 {
            let footer = format!(
                "\n{}{}error{}: could not compile `{}` due to {} previous error{}",
                BOLD,
                C_ERRORS,
                R,
                program.filename,
                error_count,
                if error_count > 1 { "s" } else { "" }
            );
            return Err(CliError::fatal(format!("{}\n{}", msgs.join("\n"), footer)));
        } else {
            for m in msgs {
                eprintln!("{}", m);
            }
        }
    }

    Ok(CheckResult {
        checker_result: check_result,
    })
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
