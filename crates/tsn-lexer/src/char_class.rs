#[inline]
pub(crate) fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

#[inline]
pub(crate) fn is_hex_digit(c: u8) -> bool {
    is_digit(c) || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

#[inline]
pub(crate) fn is_binary_digit(c: u8) -> bool {
    c == b'0' || c == b'1'
}

#[inline]
pub(crate) fn is_octal_digit(c: u8) -> bool {
    c >= b'0' && c <= b'7'
}

#[inline]
pub(crate) fn is_identifier_start(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_' || c == b'$' || c > 127
}

#[inline]
pub(crate) fn is_identifier_part(c: u8) -> bool {
    is_identifier_start(c) || is_digit(c)
}
