/// ANSI color constants and formatting helpers for pipeline output
pub const R: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const C_TOKENS: &str = "\x1b[36m";
pub const C_AST: &str = "\x1b[32m";
pub const C_BYTECODE: &str = "\x1b[33m";
pub const C_SYMBOLS: &str = "\x1b[35m";
pub const C_BINDS: &str = "\x1b[34m";
pub const C_MODULES: &str = "\x1b[37m";
pub const C_TYPES: &str = "\x1b[96m";
pub const C_ERRORS: &str = "\x1b[91m";
pub const C_CONSTS: &str = "\x1b[93m";
pub const C_SCOPE: &str = "\x1b[92m";

/// Print a formatted header with phase name and file
pub fn header(phase_color: &str, phase: &str, file: &str) {
    let label = format!(" {phase} ");
    let right = format!(" {file} ");
    let fill_len = 60usize.saturating_sub(label.len() + right.len());
    let fill = "-".repeat(fill_len);
    eprintln!(
        "{}{}{}{}{} {}{}{}",
        BOLD, phase_color, label, fill, right, R, DIM, R
    );
}

/// Print a formatted footer with message
pub fn footer(msg_color: &str, msg: &str) {
    eprintln!("{}{}  -- {} {}", msg_color, DIM, msg, R);
}
