use std::sync::OnceLock;
use tsn_compiler::FunctionProto;

use crate::args::{DebugFlags, RunOpts};
use crate::error::CliError;

type PipelineResult<T> = Result<T, CliError>;

mod debug;
use debug::{
    debug_binds, debug_consts, debug_expr, debug_import_graph, debug_lsp, debug_modules,
    debug_scope, debug_symbols, debug_types,
};

const R: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const C_TOKENS: &str = "\x1b[36m";
const C_AST: &str = "\x1b[32m";
const C_BYTECODE: &str = "\x1b[33m";
const C_SYMBOLS: &str = "\x1b[35m";
const C_BINDS: &str = "\x1b[34m";
const C_MODULES: &str = "\x1b[37m";
const C_TYPES: &str = "\x1b[96m";
const C_ERRORS: &str = "\x1b[91m";
const C_CONSTS: &str = "\x1b[93m";
const C_SCOPE: &str = "\x1b[92m";
static BUILTIN_PROTOS_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/builtins.bin"));
static BUILTIN_PROTOS: OnceLock<Result<Vec<FunctionProto>, String>> = OnceLock::new();
const COMPILE_CACHE_VERSION: u32 = 1;
const CACHE_HEADER_LEN: usize = 4 + 8 + 8;

fn header(color: &str, phase: &str, file: &str) {
    let label = format!(" {phase} ");
    let right = format!(" {file} ");
    let fill_len = 60usize.saturating_sub(label.len() + right.len());
    let fill = "-".repeat(fill_len);
    eprintln!(
        "{}{}{}{}{} {}{}{}",
        BOLD, color, label, fill, right, R, DIM, R
    );
}

fn footer(color: &str, msg: &str) {
    eprintln!("{}{}  -- {} {}", color, DIM, msg, R);
}

pub fn run(opts: &RunOpts) -> PipelineResult<()> {
    let source = if let Some(ref s) = opts.eval {
        s.clone()
    } else {
        read_source(&opts.file_path)?
    };
    let (proto, precompiled) = if opts.eval.is_none() && !opts.debug.any() {
        compile_source_cached(&source, &opts.file_path, opts.verbose)?
    } else {
        compile_source(&source, &opts.file_path, opts.verbose, &opts.debug)?
    };
    if opts.no_run {
        return Ok(());
    }
    execute(proto, precompiled, &source, &opts.file_path, &opts.debug)
}

pub fn compile_file(
    path: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<FunctionProto> {
    let source = read_source(path)?;
    let (proto, _) = compile_source(&source, path, verbose, debug)?;
    Ok(proto)
}

fn compile_source(
    source: &str,
    path: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<(FunctionProto, std::collections::HashMap<String, std::sync::Arc<FunctionProto>>)> {
    let tokens = lex(source, path, verbose, debug);
    let program = parse(tokens, source, path, verbose, debug)?;
    compile(&program, source, verbose, debug)
}

fn compile_source_cached(source: &str, path: &str, verbose: bool) -> PipelineResult<(FunctionProto, std::collections::HashMap<String, std::sync::Arc<FunctionProto>>)> {
    let source_hash = source_cache_hash(source);
    let cache_path = compile_cache_path(path);

    match load_cached_proto(&cache_path, source_hash) {
        Ok(Some(proto)) => {
            if verbose {
                eprintln!("[tsn] compile cache hit");
            }
            let tokens = tsn_lexer::scan(source, path);
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
            let precompiled = crate::module_precompile::precompile_direct_imports(&program, path);
            return Ok((proto, precompiled));
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

    let (proto, precompiled) = compile_source(source, path, verbose, &DebugFlags::default())?;
    if let Err(e) = store_cached_proto(&cache_path, source_hash, &proto) {
        if verbose {
            eprintln!("[tsn] compile cache write skipped: {}", e);
        }
    }
    Ok((proto, precompiled))
}

fn read_source(path: &str) -> PipelineResult<String> {
    std::fs::read_to_string(path).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[io]{}: cannot read '{}': {}",
            BOLD, C_ERRORS, R, path, e
        ))
    })
}

fn lex(source: &str, path: &str, verbose: bool, debug: &DebugFlags) -> Vec<tsn_core::Token> {
    let tokens = tsn_lexer::scan(source, path);

    if verbose {
        eprintln!("[tsn] lexed {} tokens", tokens.len());
    }

    if debug.tokens {
        header(C_TOKENS, "tokens", path);
        eprintln!(
            "{}  {:>4}  {:>8}  {:>6}  {:<22}  lexeme{}",
            DIM, "idx", "line:col", "off", "kind", R
        );
        for (i, tok) in tokens.iter().enumerate() {
            eprintln!(
                "  {:>4}  {:>4}:{:<4}  {:6}  {:<22}  {}{:?}{}",
                i,
                tok.range.start.line,
                tok.range.start.column,
                tok.range.start.offset,
                format!("{:?}", tok.kind),
                DIM,
                tok.lexeme,
                R
            );
        }
        footer(C_TOKENS, &format!("{} tokens", tokens.len()));
    }

    tokens
}

fn parse(
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
        header(C_AST, "ast", path);
        eprintln!("{:#?}", program);
        footer(
            C_AST,
            &format!("{} top-level statements", program.body.len()),
        );
    }

    if debug.symbols {
        debug_symbols(&program);
    }
    if debug.modules {
        debug_modules(&program);
    }
    if debug.types {
        debug_types(&program, debug.types_range);
    }
    if debug.expr {
        debug_expr(&program, debug.expr_range);
    }
    if debug.graph {
        debug_import_graph(&program);
    }
    if debug.lsp {
        debug_lsp(path, source);
    }

    Ok(program)
}

fn compile(
    program: &tsn_core::ast::Program,
    source: &str,
    verbose: bool,
    debug: &DebugFlags,
) -> PipelineResult<(FunctionProto, std::collections::HashMap<String, std::sync::Arc<FunctionProto>>)> {
    let check_result = tsn_checker::Checker::check(program);
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

    let proto = tsn_compiler::compile_with_check_result(
        program,
        &check_result.type_annotations,
        &check_result.extension_calls,
        &check_result.extension_members,
        &check_result.extension_set_members,
    )
    .map_err(|e| CliError::fatal(format!("{}{}error[compiler]{}: {}", BOLD, C_ERRORS, R, e)))?;

    if verbose {
        eprintln!("[tsn] compiled {} bytecode words", proto.chunk.code.len());
    }

    let precompiled = crate::module_precompile::precompile_direct_imports(program, &program.filename);
    if verbose && !precompiled.is_empty() {
        eprintln!("[tsn] precompiled {} direct dependency modules", precompiled.len());
    }

    if debug.bytecode {
        header(C_BYTECODE, "bytecode", &program.filename);
        crate::disasm::print(&proto);
        footer(
            C_BYTECODE,
            &format!("{} bytecode words", proto.chunk.code.len()),
        );
    }

    if debug.binds {
        debug_binds(&proto, &program.filename);
    }

    if debug.consts {
        debug_consts(&proto, &program.filename);
    }
    if debug.scope {
        debug_scope(&proto, &program.filename);
    }

    Ok((proto, precompiled))
}

pub fn execute(
    proto: FunctionProto,
    precompiled: std::collections::HashMap<String, std::sync::Arc<FunctionProto>>,
    source: &str,
    path: &str,
    debug: &DebugFlags,
) -> PipelineResult<()> {
    let mut machine = tsn_vm::Vm::new();
    machine.trace = debug.trace;
    machine.calls = debug.calls;

    // Set precompiled modules in the VM to avoid runtime compilation.
    if !precompiled.is_empty() {
        machine.set_precompiled_protos(precompiled);
    }

    for builtin_proto in builtin_protos_owned()? {
        machine
            .run_proto(builtin_proto)
            .map_err(|e| CliError::fatal(format!("failed to run builtin: {}", e.message)))?;
    }

    let result = machine.run_proto(proto);
    result
        .map(|_| ())
        .map_err(|e| CliError::fatal(format_runtime_error(source, path, &e)))
}

pub(crate) fn builtin_protos_owned() -> PipelineResult<Vec<FunctionProto>> {
    let result = BUILTIN_PROTOS.get_or_init(|| {
        bincode::deserialize(BUILTIN_PROTOS_BYTES)
            .map_err(|e| format!("failed to deserialize embedded builtin protos: {e}"))
    });
    result
        .as_ref()
        .cloned()
        .map_err(|e| CliError::fatal(e.clone()))
}

fn source_cache_hash(source: &str) -> u64 {
    let mut h = fnv1a64(source.as_bytes());
    h = fnv1a64_u64(h, binary_epoch_fingerprint());
    h = fnv1a64_u64(h, COMPILE_CACHE_VERSION as u64);
    h
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

fn compile_cache_path(source_path: &str) -> std::path::PathBuf {
    let abs = std::fs::canonicalize(source_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(source_path));
    let key = fnv1a64(abs.to_string_lossy().as_bytes());
    tsn_core::paths::tsn_cache_dir()
        .join("compile")
        .join(format!("{key:016x}.bin"))
}

fn load_cached_proto(
    cache_path: &std::path::Path,
    expected_source_hash: u64,
) -> PipelineResult<Option<FunctionProto>> {
    let bytes = match std::fs::read(cache_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(CliError::fatal(format!(
                "{}{}error[cache]{}: cannot read '{}': {}",
                BOLD,
                C_ERRORS,
                R,
                cache_path.display(),
                e
            )));
        }
    };

    if bytes.len() < CACHE_HEADER_LEN {
        return Ok(None);
    }

    let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let hash = u64::from_le_bytes([
        bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
    ]);
    if version != COMPILE_CACHE_VERSION || hash != expected_source_hash {
        return Ok(None);
    }

    let payload = &bytes[CACHE_HEADER_LEN..];
    let proto: FunctionProto = match bincode::deserialize(payload) {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    Ok(Some(proto))
}

fn store_cached_proto(
    cache_path: &std::path::Path,
    source_hash: u64,
    proto: &FunctionProto,
) -> PipelineResult<()> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::fatal(format!(
                "{}{}error[cache]{}: cannot create '{}': {}",
                BOLD,
                C_ERRORS,
                R,
                parent.display(),
                e
            ))
        })?;
    }

    let payload = bincode::serialize(proto).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[cache]{}: serialize failed: {}",
            BOLD, C_ERRORS, R, e
        ))
    })?;

    let mut bytes = Vec::with_capacity(CACHE_HEADER_LEN + payload.len());
    bytes.extend_from_slice(&COMPILE_CACHE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&source_hash.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&payload);

    std::fs::write(cache_path, bytes).map_err(|e| {
        CliError::fatal(format!(
            "{}{}error[cache]{}: cannot write '{}': {}",
            BOLD,
            C_ERRORS,
            R,
            cache_path.display(),
            e
        ))
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fnv1a64_u64(seed: u64, value: u64) -> u64 {
    let mut hash = seed;
    for b in value.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn format_runtime_error(source: &str, path: &str, err: &tsn_vm::RuntimeError) -> String {
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

fn format_error_with_context(
    source: &str,
    path: &str,
    line: u32,
    col: u32,
    kind: &str,
    msg: &str,
) -> String {
    let src_line = source
        .lines()
        .nth((line as usize).saturating_sub(1))
        .unwrap_or("");
    let col_idx = (col as usize).saturating_sub(1);

    let caret_pad = " ".repeat(col_idx);
    let _line_str = line.to_string();

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
