/// VM execution and runtime error formatting
use std::collections::HashMap;
use std::sync::Arc;
use tsn_compiler::FunctionProto;
use tsn_vm::{RuntimeError, Vm};

use crate::args::DebugFlags;
use crate::error::CliError;

use super::builtins;
use super::colors::{BOLD, C_ERRORS, DIM, R};

type PipelineResult<T> = Result<T, CliError>;

pub fn execute(
    proto: FunctionProto,
    precompiled: HashMap<String, Arc<FunctionProto>>,
    source: &str,
    path: &str,
    debug: &DebugFlags,
) -> PipelineResult<()> {
    let mut machine = Vm::new();
    machine.trace = debug.trace;
    machine.calls = debug.calls;

    // Set precompiled modules in the VM to avoid runtime compilation.
    if !precompiled.is_empty() {
        machine.set_precompiled_protos(precompiled);
    }

    for builtin_proto in builtins::builtin_protos_owned()? {
        machine
            .run_proto(builtin_proto)
            .map_err(|e| CliError::fatal(format!("failed to run builtin: {}", e.message)))?;
    }

    let result = machine.run_proto(proto);
    result
        .map(|_| ())
        .map_err(|e| CliError::fatal(format_runtime_error(source, path, &e)))
}

pub fn format_runtime_error(source: &str, path: &str, err: &RuntimeError) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "{BOLD}{C_ERRORS}error[runtime]{R}: {BOLD}{C_ERRORS}{}{R}\n",
        err.message,
        BOLD = BOLD,
        C_ERRORS = C_ERRORS,
        R = R
    ));

    if let Some(top) = err.stack.first() {
        if top.line > 0 {
            let src_line = source
                .lines()
                .nth((top.line as usize).saturating_sub(1))
                .unwrap_or("");

            out.push_str(&format!(
                "  {DIM}┌─{R} {path}:{line}\n  {DIM}│{R}\n{DIM} {line}│{R}{src_line}\n  {DIM}│{R} {C_ERRORS}{BOLD}^\n  {DIM}└─{R} {C_ERRORS}{BOLD}{msg}{R}\n",
                DIM = DIM,
                R = R,
                path = path,
                line = top.line,
                src_line = src_line,
                C_ERRORS = C_ERRORS,
                BOLD = BOLD,
                msg = err.message
            ));
        }
    }

    if !err.stack.is_empty() {
        out.push_str(&format!("\n{BOLD}stacktrace:{R}\n", BOLD = BOLD, R = R));
        for (i, frame) in err.stack.iter().enumerate() {
            let loc = if frame.line > 0 {
                format!("{}:{}", path, frame.line)
            } else {
                path.to_owned()
            };
            let prefix = if i == 0 {
                "┌─"
            } else if i == err.stack.len() - 1 {
                "└─"
            } else {
                "├─"
            };
            out.push_str(&format!(
                "  {DIM}{prefix}{R} {BOLD}{C_ERRORS}{}{R} ({})\n",
                frame.fn_name,
                loc,
                DIM = DIM,
                prefix = prefix,
                R = R,
                C_ERRORS = C_ERRORS,
                BOLD = BOLD
            ));
        }
    }

    out
}
