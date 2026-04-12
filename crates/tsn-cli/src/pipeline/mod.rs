/// Compilation pipeline: source → lex → parse → check → compile → execute
mod builtins;
mod cache;
mod check;
pub mod colors;
mod compile;
pub mod debug;
mod execute;
mod hash;
mod lex;
mod parse;

use crate::args::{DebugFlags, RunOpts};
use crate::error::CliError;
use tsn_compiler::FunctionProto;

type PipelineResult<T> = Result<T, CliError>;

// Re-export types needed by main.rs and bench.rs
pub use builtins::builtin_protos_owned;
pub use compile::CompileOutput;
pub use execute::execute;

const COMPILE_CACHE_VERSION: u32 = 2;

pub fn run(opts: &RunOpts) -> PipelineResult<()> {
    let source = if let Some(ref s) = opts.eval {
        s.clone()
    } else {
        read_source(&opts.file_path)?
    };
    let compiled = if opts.eval.is_none() && !opts.debug.any() {
        compile_source_cached(&source, &opts.file_path, opts.verbose)?
    } else {
        compile_source(&source, &opts.file_path, opts.verbose, &opts.debug)?
    };
    if opts.no_run {
        return Ok(());
    }
    execute(
        compiled.entry_proto,
        compiled.precompiled,
        &source,
        &opts.file_path,
        &opts.debug,
    )
}

pub fn compile_file(
    path: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<FunctionProto> {
    let source = read_source(path)?;
    let compiled = compile_source(&source, path, verbose, debug)?;
    Ok(compiled.entry_proto)
}

fn compile_source(
    source: &str,
    path: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<CompileOutput> {
    let tokens = lex::lex(source, path, verbose, debug);
    let program = parse::parse(tokens, source, path, verbose, debug)?;
    let check_result = check::check(&program, source)?;
    let binary_fp = binary_epoch_fingerprint();
    compile::compile(&program, source, check_result, verbose, debug, binary_fp)
}

fn compile_source_cached(source: &str, path: &str, verbose: bool) -> PipelineResult<CompileOutput> {
    let cache_path = cache::compile_cache_path(path);
    let binary_fp = binary_epoch_fingerprint();

    match cache::load_cached_graph(&cache_path, binary_fp, COMPILE_CACHE_VERSION) {
        Ok(Some(graph_artifact)) => {
            if verbose {
                eprintln!("[tsn] compile cache hit");
            }
            return cache::compile_output_from_graph(graph_artifact);
        }
        Ok(None) => {}
        Err(e) => {
            if verbose {
                eprintln!("[tsn] compile cache read skipped: {}", e);
            }
        }
    }

    if verbose {
        eprintln!("[tsn] compile cache miss");
    }

    let compiled = compile_source(source, path, verbose, &DebugFlags::default())?;
    if let Err(e) = cache::store_cached_graph(&cache_path, &compiled.graph_artifact) {
        if verbose {
            eprintln!("[tsn] compile cache write skipped: {}", e);
        }
    }
    Ok(compiled)
}

fn read_source(path: &str) -> PipelineResult<String> {
    std::fs::read_to_string(path).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[io]{}: cannot read '{}': {}",
            colors::BOLD,
            colors::C_ERRORS,
            colors::R,
            path,
            e
        ))
    })
}

fn binary_epoch_fingerprint() -> u64 {
    let Ok(exe) = std::env::current_exe() else {
        return 0;
    };
    let Ok(meta) = std::fs::metadata(exe) else {
        return 0;
    };
    let Ok(modified) = meta.modified() else {
        return 0;
    };
    let Ok(since_epoch) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return 0;
    };
    since_epoch.as_secs() ^ since_epoch.subsec_nanos() as u64
}

fn graph_hash_from_sources(
    source_hashes: &std::collections::HashMap<String, u64>,
    binary_fingerprint: u64,
) -> u64 {
    let mut hash = hash::fnv1a64_u64(0xcbf29ce484222325u64, COMPILE_CACHE_VERSION as u64);
    hash = hash::fnv1a64_u64(hash, binary_fingerprint);

    let mut items: Vec<(&str, u64)> = source_hashes
        .iter()
        .map(|(path, source_hash)| (path.as_str(), *source_hash))
        .collect();
    items.sort_by(|a, b| a.0.cmp(b.0));

    for (path, source_hash) in items {
        hash = hash::fnv1a64_extend(hash, path.as_bytes());
        hash = hash::fnv1a64_u64(hash, source_hash);
    }

    hash
}
