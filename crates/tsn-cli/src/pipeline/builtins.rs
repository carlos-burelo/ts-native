/// Builtin prototypes loading from embedded binary
use std::sync::OnceLock;
use tsn_compiler::FunctionProto;

use crate::error::CliError;

type PipelineResult<T> = Result<T, CliError>;

static BUILTIN_PROTOS_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/builtins.bin"));
static BUILTIN_PROTOS: OnceLock<Result<Vec<FunctionProto>, String>> = OnceLock::new();

pub fn builtin_protos_owned() -> PipelineResult<Vec<FunctionProto>> {
    let result = BUILTIN_PROTOS.get_or_init(|| {
        bincode::deserialize(BUILTIN_PROTOS_BYTES)
            .map_err(|e| format!("failed to deserialize embedded builtin protos: {e}"))
    });
    result
        .as_ref()
        .cloned()
        .map_err(|e| CliError::fatal(e.clone()))
}
