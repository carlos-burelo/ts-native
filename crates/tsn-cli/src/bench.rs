use std::time::{Duration, Instant};

use crate::args::DebugFlags;
use crate::bench_output::{print_check_breakdown, print_opcode_hotspots, print_parse_breakdown};
use crate::error::CliError;

const R: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[96m";
const YEL: &str = "\x1b[33m";
const GRN: &str = "\x1b[32m";
const MAG: &str = "\x1b[35m";
const BLU: &str = "\x1b[34m";
const WHT: &str = "\x1b[37m";
const RED: &str = "\x1b[31m";

struct PhaseStats {
    name: &'static str,
    color: &'static str,
    min: Duration,
    p50: Duration,
    max: Duration,
    total: Duration,
    stddev: Duration,
    runs: usize,
}

impl PhaseStats {
    fn from_samples(name: &'static str, color: &'static str, samples: &[Duration]) -> Self {
        let min = *samples.iter().min().expect("samples must not be empty");
        let max = *samples.iter().max().expect("samples must not be empty");
        let total: Duration = samples.iter().sum();
        let mean_ns = total.as_nanos() as f64 / samples.len() as f64;

        let mut sorted = samples.to_vec();
        sorted.sort();
        let p50 = sorted[sorted.len() / 2];

        let variance = samples
            .iter()
            .map(|s| {
                let d = s.as_nanos() as f64 - mean_ns;
                d * d
            })
            .sum::<f64>()
            / samples.len() as f64;
        let stddev = Duration::from_nanos(variance.sqrt() as u64);

        PhaseStats {
            name,
            color,
            min,
            p50,
            max,
            total,
            stddev,
            runs: samples.len(),
        }
    }

    fn mean(&self) -> Duration {
        self.total / self.runs as u32
    }
}

fn fmt_dur(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns < 1_000 {
        format!("{} ns", ns)
    } else if ns < 1_000_000 {
        let us = ns as f64 / 1_000.0;
        if us < 10.0 {
            fmt_f(us, 2) + " µs"
        } else if us < 100.0 {
            fmt_f(us, 1) + " µs"
        } else {
            fmt_f(us, 0) + " µs"
        }
    } else if ns < 1_000_000_000 {
        let ms = ns as f64 / 1_000_000.0;
        if ms < 10.0 {
            fmt_f(ms, 3) + " ms"
        } else if ms < 100.0 {
            fmt_f(ms, 2) + " ms"
        } else {
            fmt_f(ms, 1) + " ms"
        }
    } else {
        let s = ns as f64 / 1_000_000_000.0;
        fmt_f(s, 2) + " s"
    }
}

fn fmt_f(f: f64, decimals: usize) -> String {
    if decimals == 0 {
        return format!("{:.0}", f);
    }
    let s = format!("{:.prec$}", f, prec = decimals);
    let s = s.trim_end_matches('0');
    s.trim_end_matches('.').to_owned()
}

fn fmt_bytes(n: usize) -> String {
    if n < 1_024 {
        format!("{} B", n)
    } else if n < 1_048_576 {
        format!("{:.1} KB", n as f64 / 1_024.0)
    } else {
        format!("{:.2} MB", n as f64 / 1_048_576.0)
    }
}

const W_NAME: usize = 10;
const W_TIME: usize = 9;
const W_SIG: usize = 8;
const SEP: &str = "─";

fn sep_line() {
    let name_col = SEP.repeat(W_NAME);
    let time_col = SEP.repeat(W_TIME);
    let sig_col = SEP.repeat(W_SIG);
    eprintln!(
        "  {}{}  {}  {}  {}  {}  {}  {}{}",
        DIM, name_col, time_col, time_col, time_col, time_col, sig_col, time_col, R
    );
}

fn header_line() {
    eprintln!(
        "  {}{:<W_NAME$}  {:>W_TIME$}  {:>W_TIME$}  {:>W_TIME$}  {:>W_TIME$}  {:>W_SIG$}  {:>W_TIME$}{}",
        DIM, "Phase", "min", "p50", "mean", "max", "σ", "total", R
    );
}

fn phase_line(stat: &PhaseStats, share: f64) {
    let bar = make_bar(share, 8);
    let pct = format!("{:.0}%", share * 100.0);

    eprintln!(
        "  {}{}{:<W_NAME$}{}  {:>W_TIME$}  {:>W_TIME$}  {}{:>W_TIME$}{}  {:>W_TIME$}  {}{:>W_SIG$}{}  {}{}{}  {}{} {}{}",
        BOLD, stat.color, stat.name, R,
        fmt_dur(stat.min),
        fmt_dur(stat.p50),
        CYAN, fmt_dur(stat.mean()), R,
        fmt_dur(stat.max),
        DIM, fmt_dur(stat.stddev), R,
        DIM, fmt_dur(stat.total), R,
        DIM, bar, pct, R,
    );
}

fn total_line(phases: &[PhaseStats]) {
    let min: Duration = phases.iter().map(|p| p.min).sum();
    let max: Duration = phases.iter().map(|p| p.max).sum();
    let total: Duration = phases.iter().map(|p| p.total).sum();
    let p50: Duration = phases.iter().map(|p| p.p50).sum();
    let runs = phases[0].runs;
    let mean = total / runs as u32;

    eprintln!(
        "  {}{}{:<W_NAME$}{}  {:>W_TIME$}  {:>W_TIME$}  {}{:>W_TIME$}{}  {:>W_TIME$}  {:>W_SIG$}  {}{}{}",
        BOLD, GRN, "total", R,
        fmt_dur(min),
        fmt_dur(p50),
        CYAN, fmt_dur(mean), R,
        fmt_dur(max),
        "",
        DIM, fmt_dur(total), R,
    );
}

fn make_bar(fraction: f64, width: usize) -> String {
    let filled = (fraction * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn time_n<F: Fn() -> Result<(), String>>(runs: usize, f: F) -> Result<Vec<Duration>, CliError> {
    f().map_err(|e| CliError::fatal(format!("bench warmup failed: {e}")))?;

    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        f().map_err(|e| CliError::fatal(format!("bench run failed: {e}")))?;
        samples.push(start.elapsed());
    }
    Ok(samples)
}

pub fn run_bench(
    path: &str,
    runs: usize,
    debug: &DebugFlags,
    no_run: bool,
    with_output: bool,
) -> Result<(), CliError> {
    if debug.any() || no_run {
        crate::pipeline::compile_file(path, false, debug)?;
        if no_run {
            return Ok(());
        }
    }

    let source = std::fs::read_to_string(path)
        .map_err(|e| CliError::fatal(format!("cannot read '{path}': {e}")))?;

    let read_samples = time_n(runs, || {
        std::fs::read_to_string(path)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })?;

    let lex_samples = time_n(runs, || {
        let _ = tsn_lexer::scan(&source, path);
        Ok(())
    })?;

    let tokens = tsn_lexer::scan(&source, path);
    let token_count = tokens.len();

    let tokens_ref = &tokens;
    let parse_samples = time_n(runs, || {
        tsn_parser::parse(tokens_ref.clone(), path)
            .map(|_| ())
            .map_err(|errs| {
                errs.iter()
                    .map(|e| {
                        format!(
                            "{}:{}:{}: {}",
                            path, e.range.start.line, e.range.start.column, e.message
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
    })?;

    let (program, parse_profile) =
        tsn_parser::parse_with_profile(tokens, path).map_err(|errs| {
            let msgs: Vec<String> = errs
                .iter()
                .map(|e| {
                    format!(
                        "{}:{}:{}: {}",
                        path, e.range.start.line, e.range.start.column, e.message
                    )
                })
                .collect();
            CliError::fatal(format!("parse errors:\n{}", msgs.join("\n")))
        })?;

    let program_ref = &program;
    let check_samples = time_n(runs, || {
        let _ = tsn_checker::Checker::check(program_ref);
        Ok(())
    })?;

    let check_result = tsn_checker::Checker::check_with_profile(&program);

    let compile_samples = time_n(runs, || {
        tsn_compiler::compile_with_annotations(program_ref, &check_result.type_annotations)
            .map(|_| ())
            .map_err(|e| e)
    })?;

    let proto = tsn_compiler::compile_with_annotations(&program, &check_result.type_annotations)
        .map_err(|e| CliError::fatal(format!("compile error: {e}")))?;

    let graph_build = crate::module_precompile::build_module_graph(&program, &source, path, &proto)
        .map_err(|e| CliError::fatal(format!("module graph build error: {e}")))?;
    let precompiled: std::collections::HashMap<
        String,
        std::sync::Arc<tsn_compiler::FunctionProto>,
    > = graph_build
        .modules
        .into_iter()
        .filter(|(module_path, _)| module_path != &graph_build.entry_path)
        .map(|(module_path, module_proto)| (module_path, std::sync::Arc::new(module_proto)))
        .collect();

    let builtin_protos: Vec<tsn_compiler::FunctionProto> = crate::pipeline::builtin_protos_owned()?;

    let globals_snapshot = tsn_vm::build_globals_snapshot(&builtin_protos)
        .map_err(|e| CliError::fatal(format!("builtin init failed: {}", e)))?;

    let proto_ref = &proto;
    let snapshot_ref = &globals_snapshot;
    let precompiled_ref = &precompiled;
    let exec_samples = time_n(runs, || {
        tsn_runtime::reset_testing_counters();
        tsn_runtime::set_console_silent(!with_output);
        tsn_runtime::set_testing_silent(!with_output);
        let mut machine = tsn_vm::Vm::from_globals_snapshot(snapshot_ref.clone());
        machine.set_precompiled_protos(precompiled_ref.clone());
        let result = machine
            .run_proto(proto_ref.clone())
            .map(|_| ())
            .map_err(|e| e.to_string());
        tsn_runtime::set_console_silent(false);
        tsn_runtime::set_testing_silent(false);
        result
    })?;

    tsn_runtime::reset_testing_counters();
    tsn_runtime::set_console_silent(true);
    tsn_runtime::set_testing_silent(true);
    let mut profile_vm = tsn_vm::Vm::from_globals_snapshot(globals_snapshot.clone());
    profile_vm.set_precompiled_protos(precompiled.clone());
    profile_vm.enable_opcode_profile();
    profile_vm
        .run_proto(proto.clone())
        .map_err(|e| CliError::fatal(format!("profile run failed: {}", e)))?;
    let opcode_counts = profile_vm.opcode_profile_snapshot().unwrap_or_default();
    tsn_runtime::set_console_silent(false);
    tsn_runtime::set_testing_silent(false);

    let stats = vec![
        PhaseStats::from_samples("read", WHT, &read_samples),
        PhaseStats::from_samples("lex", YEL, &lex_samples),
        PhaseStats::from_samples("parse", GRN, &parse_samples),
        PhaseStats::from_samples("check", RED, &check_samples),
        PhaseStats::from_samples("compile", MAG, &compile_samples),
        PhaseStats::from_samples("execute", BLU, &exec_samples),
    ];

    let total_mean: Duration = stats.iter().map(|s| s.mean()).sum();
    let throughput = if total_mean.as_nanos() > 0 {
        1_000_000_000.0 / total_mean.as_nanos() as f64
    } else {
        f64::INFINITY
    };

    let line_count = source.lines().count();
    let byte_count = source.len();

    eprintln!();
    eprintln!(
        "  {}{}Benchmark{} · {}{}{}  {}({} runs){}",
        BOLD, CYAN, R, BOLD, path, R, DIM, runs, R
    );
    eprintln!(
        "  {}Source  {} lines  {}  {} tokens{}",
        DIM,
        fmt_num(line_count),
        fmt_bytes(byte_count),
        fmt_num(token_count),
        R
    );
    eprintln!();
    header_line();
    sep_line();
    for s in &stats {
        let share = s.mean().as_nanos() as f64 / total_mean.as_nanos() as f64;
        phase_line(s, share);
    }
    sep_line();
    total_line(&stats);
    eprintln!();
    eprintln!(
        "  {}Throughput:{} {}{:.1} runs/s{}  {}(mean end-to-end: {}){}",
        DIM,
        R,
        RED,
        throughput,
        R,
        DIM,
        fmt_dur(total_mean),
        R
    );
    if !with_output {
        eprintln!(
            "  {}Execution measured with stdout muted (--withOutput to disable){}",
            DIM, R
        );
    }
    print_parse_breakdown(&parse_profile);
    print_check_breakdown(&check_result.profile);
    print_opcode_hotspots(&opcode_counts);
    eprintln!();

    Ok(())
}

fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}
