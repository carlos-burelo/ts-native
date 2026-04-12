/// Bytecode compilation and debugging output
use std::collections::HashMap;
use std::sync::Arc;
use tsn_compiler::FunctionProto;
use tsn_core::ast::Program;
use tsn_types::ModuleGraphArtifact;

use crate::args::DebugFlags;
use crate::error::CliError;

use super::check::CheckResult;
use super::colors::C_BYTECODE;
use super::debug;

type PipelineResult<T> = Result<T, CliError>;

pub struct CompileOutput {
    pub entry_proto: FunctionProto,
    pub precompiled: HashMap<String, Arc<FunctionProto>>,
    pub graph_artifact: ModuleGraphArtifact,
}

pub fn compile(
    program: &Program,
    source: &str,
    check_result: CheckResult,
    verbose: bool,
    debug: &DebugFlags,
    binary_epoch_fingerprint: u64,
) -> PipelineResult<CompileOutput> {
    let proto = tsn_compiler::compile_with_check_result(
        program,
        &check_result.checker_result.type_annotations,
        &check_result.checker_result.extension_calls,
        &check_result.checker_result.extension_members,
        &check_result.checker_result.extension_set_members,
    )
    .map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[compiler]{}: {}",
            super::colors::BOLD,
            super::colors::C_ERRORS,
            super::colors::R,
            e
        ))
    })?;

    if verbose {
        eprintln!("[tsn] compiled {} bytecode words", proto.chunk.code.len());
    }

    let graph_build =
        crate::module_precompile::build_module_graph(program, source, &program.filename, &proto)
            .map_err(|e| {
                CliError::fatal(format!(
                    "{}{}error[module-graph]{}: {}",
                    super::colors::BOLD,
                    super::colors::C_ERRORS,
                    super::colors::R,
                    e
                ))
            })?;

    let graph_hash =
        super::graph_hash_from_sources(&graph_build.source_hashes, binary_epoch_fingerprint);
    let graph_artifact = crate::module_precompile::build_graph_artifact(
        super::COMPILE_CACHE_VERSION,
        graph_hash,
        graph_build,
    );
    let precompiled = precompiled_from_graph(&graph_artifact);

    if verbose && !precompiled.is_empty() {
        eprintln!("[tsn] precompiled {} dependency modules", precompiled.len());
    }

    if debug.bytecode {
        super::colors::header(C_BYTECODE, "bytecode", &program.filename);
        crate::disasm::print(&proto);
        super::colors::footer(
            C_BYTECODE,
            &format!("{} bytecode words", proto.chunk.code.len()),
        );
    }

    if debug.binds {
        debug::debug_binds(&proto, &program.filename);
    }

    if debug.consts {
        debug::debug_consts(&proto, &program.filename);
    }
    if debug.scope {
        debug::debug_scope(&proto, &program.filename);
    }

    Ok(CompileOutput {
        entry_proto: proto,
        precompiled,
        graph_artifact,
    })
}

pub fn precompiled_from_graph(
    graph_artifact: &ModuleGraphArtifact,
) -> HashMap<String, Arc<FunctionProto>> {
    graph_artifact
        .modules
        .iter()
        .filter(|(path, _)| *path != &graph_artifact.entry_path)
        .map(|(path, proto)| (path.clone(), Arc::new(proto.clone())))
        .collect()
}
