use std::time::Duration;

use tsn_core::OpCode;

const R: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const BLU: &str = "\x1b[34m";
const RED: &str = "\x1b[31m";
const GRN: &str = "\x1b[32m";

pub fn print_parse_breakdown(profile: &tsn_parser::ParseProfile) {
    let rows = [
        ("program_loop", profile.program_loop),
        ("stmt_or_decl", profile.stmt_or_decl),
        ("block", profile.block),
        ("recover", profile.recover),
    ];
    print_duration_rows("Parser Breakdown", GRN, &rows);
}

pub fn print_check_breakdown(profile: &tsn_checker::CheckProfile) {
    let rows = [
        ("load_globals", profile.load_globals),
        ("bind", profile.bind),
        ("merge_builtins", profile.merge_builtin_members),
        ("enrich_calls", profile.enrich_call_returns),
        ("check_stmts", profile.check_stmts),
        ("annotations", profile.collect_annotations),
        ("finalize", profile.finalize),
    ];
    print_duration_rows("Checker Breakdown", RED, &rows);
}

pub fn print_opcode_hotspots(counts: &[u64]) {
    let mut rows: Vec<(OpCode, u64)> = counts
        .iter()
        .enumerate()
        .filter_map(|(idx, count)| {
            if *count == 0 {
                return None;
            }
            OpCode::from_u16(idx as u16).map(|op| (op, *count))
        })
        .collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let total: u64 = rows.iter().map(|(_, count)| *count).sum();
    if total == 0 {
        return;
    }

    eprintln!();
    eprintln!("  {}{}VM Opcode Hotspots{}{}", BOLD, BLU, R, DIM);
    for (op, count) in rows.into_iter().take(12) {
        let share = count as f64 / total as f64;
        eprintln!(
            "  {:<20} {:>10}  {:>4.0}%",
            format!("{:?}", op).trim_start_matches("Op"),
            fmt_num_u64(count),
            share * 100.0
        );
    }
    eprintln!("  {:<20} {:>10}", "total", fmt_num_u64(total));
    eprintln!("{}", R);
}

fn print_duration_rows(title: &str, color: &str, rows: &[(&str, Duration)]) {
    let total: Duration = rows.iter().map(|(_, d)| *d).sum();
    if total.is_zero() {
        return;
    }

    eprintln!();
    eprintln!("  {}{}{}{}{}", BOLD, color, title, R, DIM);
    for (name, dur) in rows {
        let share = dur.as_nanos() as f64 / total.as_nanos() as f64;
        eprintln!(
            "  {:<14} {:>10}  {:>4.0}%",
            name,
            fmt_dur(*dur),
            share * 100.0
        );
    }
    eprintln!("  {:<14} {:>10}", "total", fmt_dur(total));
    eprintln!("{}", R);
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

fn fmt_num_u64(n: u64) -> String {
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
