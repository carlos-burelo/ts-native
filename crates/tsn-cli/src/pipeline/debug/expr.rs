use tsn_core::ast::Program;

pub fn debug_expr(program: &Program, range: Option<(u32, u32)>) {
    use super::super::{footer, header, C_TYPES, DIM, R};

    let result = tsn_checker::Checker::check(program);

    if result.expr_types.is_empty() {
        return;
    }

    header(C_TYPES, "expression types", &program.filename);

    let src = std::fs::read_to_string(&program.filename).unwrap_or_default();
    let src_bytes = src.as_bytes();

    let mut line_starts: Vec<usize> = vec![0];
    for (i, &b) in src_bytes.iter().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let offset_to_linecol = |off: u32| -> (u32, u32) {
        let off = off as usize;
        let line_idx = line_starts.partition_point(|&s| s <= off).saturating_sub(1);
        let col = off - line_starts[line_idx];
        ((line_idx + 1) as u32, (col + 1) as u32)
    };

    let snippet = |off: u32| -> String {
        let start = off as usize;
        if start >= src_bytes.len() {
            return String::new();
        }
        let slice = &src_bytes[start..];
        let raw_end = slice
            .iter()
            .position(|&b| b == b'\n' || b == b'\r')
            .unwrap_or(slice.len())
            .min(52);
        let text = String::from_utf8_lossy(&slice[..raw_end]);
        let text = text.trim_end();
        if raw_end == 52 && slice.len() > 52 {
            format!("{}…", text)
        } else {
            text.to_string()
        }
    };

    let in_range = |line: u32| match range {
        Some((lo, hi)) => line >= lo && line <= hi,
        None => true,
    };

    let mut sorted_exprs: Vec<_> = result.expr_types.iter().collect();
    sorted_exprs.sort_by_key(|(off, _)| *off);
    let total_exprs = sorted_exprs.len();
    let sorted_exprs: Vec<_> = sorted_exprs
        .into_iter()
        .filter(|(off, _)| in_range(offset_to_linecol(**off).0))
        .collect();

    for (offset, ty) in &sorted_exprs {
        let (line, col) = offset_to_linecol(**offset);
        let snip = snippet(**offset);
        eprintln!(
            "  {}{}ln:{:<3} col:{:<3}{}  {}{:<54}{}  {}",
            DIM, C_TYPES, line, col, R, DIM, snip, R, ty
        );
    }

    let msg = if range.is_some() && sorted_exprs.len() < total_exprs {
        format!(
            "{} expression(s) shown ({} total)",
            sorted_exprs.len(),
            total_exprs
        )
    } else {
        format!("{} expression(s)", sorted_exprs.len())
    };
    footer(C_TYPES, &msg);
}
