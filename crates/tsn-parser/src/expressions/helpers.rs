pub(super) fn parse_int_radix(raw: &str) -> Option<i64> {
    let cleaned = raw.replace('_', "");
    if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        i64::from_str_radix(bin, 2).ok()
    } else if let Some(oct) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        i64::from_str_radix(oct, 8).ok()
    } else {
        cleaned.parse().ok()
    }
}

pub(super) fn unescape_string(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('\'') => result.push('\''),
                Some('"') => result.push('"'),
                Some('0') => result.push('\0'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => {}
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub(super) fn split_regex(raw: &str) -> (String, String) {
    if let Some(stripped) = raw.strip_prefix('/') {
        if let Some(end) = stripped.rfind('/') {
            return (stripped[..end].to_owned(), stripped[end + 1..].to_owned());
        }
    }
    (raw.to_owned(), String::new())
}
