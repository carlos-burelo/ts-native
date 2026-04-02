use crate::error::CliError;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
Usage:
  tsn <file.tsn> [--verbose] [--noRun] [--debug=<phases>]  Execute a TSN source file
  tsn -e \"<source>\" [--verbose] [--noRun] [--debug=<phases>] Execute source code string
  tsn disasm <file.tsn>                                     Disassemble bytecode (no execution)
  tsn bench  <file.tsn> [--runs=N] [--withOutput]          Benchmark each pipeline phase
  tsn doctor                                                Print TSN runtime/install diagnostics
  tsn version                                               Print version information

Flags:
  --verbose     Print stage-level progress messages
  --noRun       Compile and show debug output, but skip execution
  --runs=N      Number of timed iterations for bench (default: 10)
  --withOutput  Keep program stdout enabled during bench execution

Debug phases (comma-separated list):
  --debug=tokens,ast,bytecode,symbols,binds,modules,types,expr
  --debug=errors,trace,calls,consts,scope,graph,lsp
  Line range filter: types:N  types:N-M  types:N-  expr:N  expr:N-M  expr:N-";

#[derive(Clone, Default)]
pub struct DebugFlags {
    pub tokens: bool,
    pub ast: bool,
    pub bytecode: bool,
    pub symbols: bool,
    pub binds: bool,
    pub modules: bool,
    pub types: bool,
    pub types_range: Option<(u32, u32)>,
    pub expr: bool,
    pub expr_range: Option<(u32, u32)>,
    pub errors: bool,
    pub trace: bool,
    pub calls: bool,
    pub consts: bool,
    pub scope: bool,
    pub graph: bool,
    pub lsp: bool,
}

fn parse_line_range(s: &str) -> Result<(u32, u32), crate::error::CliError> {
    if let Some((lo_str, hi_str)) = s.split_once('-') {
        let lo = lo_str.parse::<u32>().map_err(|_| {
            crate::error::CliError::usage(format!(
                "invalid line range: '{s}' (expected N, N-M, or N-)"
            ))
        })?;
        let hi = if hi_str.is_empty() {
            u32::MAX
        } else {
            hi_str.parse::<u32>().map_err(|_| {
                crate::error::CliError::usage(format!(
                    "invalid line range: '{s}' (expected N, N-M, or N-)"
                ))
            })?
        };
        Ok((lo, hi))
    } else {
        let line = s
            .parse::<u32>()
            .map_err(|_| crate::error::CliError::usage(format!("invalid line number: '{s}'")))?;
        Ok((line, line))
    }
}

impl DebugFlags {
    pub fn parse(spec: &str) -> Result<Self, CliError> {
        let mut flags = DebugFlags::default();
        for part in spec.split(',') {
            let phase = part.trim();
            if phase.is_empty() {
                continue;
            }
            if let Some(range_str) = phase.strip_prefix("types:") {
                flags.types = true;
                flags.types_range = Some(parse_line_range(range_str)?);
            } else if let Some(range_str) = phase.strip_prefix("expr:") {
                flags.expr = true;
                flags.expr_range = Some(parse_line_range(range_str)?);
            } else {
                match phase {
                    "tokens" => flags.tokens = true,
                    "ast" => flags.ast = true,
                    "bytecode" => flags.bytecode = true,
                    "symbols" => flags.symbols = true,
                    "binds" => flags.binds = true,
                    "modules" => flags.modules = true,
                    "types" => flags.types = true,
                    "expr" => flags.expr = true,
                    "errors" => flags.errors = true,
                    "trace" => flags.trace = true,
                    "calls" => flags.calls = true,
                    "consts" => flags.consts = true,
                    "scope" => flags.scope = true,
                    "graph" => flags.graph = true,
                    "lsp" => flags.lsp = true,
                    unknown => {
                        return Err(CliError::usage(format!(
                            "unknown debug phase: '{unknown}'\n\
                             Valid phases: tokens, ast, bytecode, symbols, binds, modules, types, expr, errors, trace, calls, consts, scope, graph, lsp\n\
                             Line range filter: types:N  types:N-M  types:N-  expr:N  expr:N-M  expr:N-"
                        )));
                    }
                }
            }
        }
        Ok(flags)
    }

    pub fn any(&self) -> bool {
        self.tokens
            || self.ast
            || self.bytecode
            || self.symbols
            || self.binds
            || self.modules
            || self.types
            || self.expr
            || self.errors
            || self.trace
            || self.calls
            || self.consts
            || self.scope
            || self.graph
            || self.lsp
    }
}

pub struct RunOpts {
    pub file_path: String,
    pub eval: Option<String>,
    pub verbose: bool,

    pub no_run: bool,
    pub debug: DebugFlags,
}

pub struct DisasmOpts {
    pub file_path: String,
    pub debug: DebugFlags,
}

pub struct BenchOpts {
    pub file_path: String,
    pub runs: usize,
    pub debug: DebugFlags,
    pub no_run: bool,
    pub with_output: bool,
}

pub enum Command {
    Run(RunOpts),
    Disasm(DisasmOpts),
    Bench(BenchOpts),
    Doctor,
    Version,
}

impl Command {
    pub fn parse() -> Result<Self, CliError> {
        let args: Vec<String> = std::env::args().skip(1).collect();

        if args.is_empty() {
            return Err(CliError::usage(USAGE));
        }

        match args[0].as_str() {
            "version" | "--version" | "-V" => Ok(Command::Version),
            "doctor" => Ok(Command::Doctor),

            "disasm" => {
                let mut file_path: Option<String> = None;
                let mut debug = DebugFlags::default();
                for arg in args.iter().skip(1) {
                    if let Some(spec) = arg.strip_prefix("--debug=") {
                        debug = DebugFlags::parse(spec)?;
                    } else if !arg.starts_with('-') {
                        file_path = Some(arg.clone());
                    } else {
                        return Err(CliError::usage(format!("unknown flag: {arg}\n\n{USAGE}")));
                    }
                }
                let file_path = file_path.ok_or_else(|| {
                    CliError::usage("disasm requires a file argument\n\n  tsn disasm <file.tsn>")
                })?;
                Ok(Command::Disasm(DisasmOpts { file_path, debug }))
            }

            "bench" => {
                let mut file_path: Option<String> = None;
                let mut runs: usize = 10;
                let mut debug = DebugFlags::default();
                let mut no_run = false;
                let mut with_output = false;
                for arg in args.iter().skip(1) {
                    if let Some(spec) = arg.strip_prefix("--debug=") {
                        debug = DebugFlags::parse(spec)?;
                    } else if let Some(n) = arg.strip_prefix("--runs=") {
                        runs = n.parse::<usize>().map_err(|_| {
                            CliError::usage(format!("--runs expects a positive integer, got: {n}"))
                        })?;
                        if runs == 0 {
                            return Err(CliError::usage("--runs must be at least 1"));
                        }
                    } else if arg == "--noRun" || arg == "--no-run" {
                        no_run = true;
                    } else if arg == "--withOutput" || arg == "--with-output" {
                        with_output = true;
                    } else if !arg.starts_with('-') {
                        file_path = Some(arg.clone());
                    } else {
                        return Err(CliError::usage(format!("unknown flag: {arg}\n\n{USAGE}")));
                    }
                }
                let file_path = file_path.ok_or_else(|| {
                    CliError::usage(
                        "bench requires a file argument\n\n  tsn bench <file.tsn> [--runs=N]",
                    )
                })?;
                Ok(Command::Bench(BenchOpts {
                    file_path,
                    runs,
                    debug,
                    no_run,
                    with_output,
                }))
            }

            "help" | "--help" | "-h" => Err(CliError::usage(USAGE)),

            _ => {
                let mut file_path: Option<String> = None;
                let mut eval: Option<String> = None;
                let mut verbose = false;
                let mut no_run = false;
                let mut debug = DebugFlags::default();

                let mut i = 0;
                while i < args.len() {
                    let arg = &args[i];
                    if let Some(spec) = arg.strip_prefix("--debug=") {
                        debug = DebugFlags::parse(spec)?;
                    } else if arg == "-e" {
                        if i + 1 >= args.len() {
                            return Err(CliError::usage("-e requires a source string argument"));
                        }
                        eval = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        match arg.as_str() {
                            "--verbose" | "-v" => verbose = true,
                            "--noRun" | "--no-run" => no_run = true,
                            other if !other.starts_with('-') => {
                                file_path = Some(other.to_owned());
                            }
                            other => {
                                return Err(CliError::usage(format!(
                                    "unknown flag: {other}\n\n{USAGE}"
                                )));
                            }
                        }
                    }
                    i += 1;
                }

                let final_path = if let Some(p) = file_path {
                    p
                } else if eval.is_some() {
                    "(eval)".to_owned()
                } else {
                    return Err(CliError::usage(USAGE));
                };

                Ok(Command::Run(RunOpts {
                    file_path: final_path,
                    eval,
                    verbose,
                    no_run,
                    debug,
                }))
            }
        }
    }

    pub fn version_string() -> String {
        format!("tsn {VERSION}")
    }
}
