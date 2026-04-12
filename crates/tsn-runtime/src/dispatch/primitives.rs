use super::entry::{op, DispatchEntry};
use crate::modules::{primitives, symbols};
use tsn_core::intrinsic::IntrinsicId;

pub(crate) static OPS: &[DispatchEntry] = &[
    op(
        IntrinsicId::StrFromCharCode,
        "str_from_char_code",
        primitives::str_from_char_code,
    ),
    op(IntrinsicId::StrLength, "str_length", primitives::str_length),
    op(
        IntrinsicId::StrToLower,
        "str_to_lower",
        primitives::str_to_lower,
    ),
    op(
        IntrinsicId::StrToUpper,
        "str_to_upper",
        primitives::str_to_upper,
    ),
    op(IntrinsicId::StrTrim, "str_trim", primitives::str_trim),
    op(
        IntrinsicId::StrIncludes,
        "str_includes",
        primitives::str_includes,
    ),
    op(
        IntrinsicId::StrStartsWith,
        "str_starts_with",
        primitives::str_starts_with,
    ),
    op(
        IntrinsicId::StrEndsWith,
        "str_ends_with",
        primitives::str_ends_with,
    ),
    op(IntrinsicId::StrSplit, "str_split", primitives::str_split),
    op(
        IntrinsicId::StrSubstring,
        "str_substring",
        primitives::str_substring,
    ),
    op(
        IntrinsicId::StrCharAt,
        "str_char_at",
        primitives::str_char_at,
    ),
    op(
        IntrinsicId::StrIndexOf,
        "str_index_of",
        primitives::str_index_of,
    ),
    op(IntrinsicId::IntParse, "int_parse", primitives::int_parse),
    op(IntrinsicId::IntToStr, "int_to_str", primitives::int_to_str),
    op(
        IntrinsicId::IntToFixed,
        "int_to_fixed",
        primitives::int_to_fixed,
    ),
    op(IntrinsicId::IntAbs, "int_abs", primitives::int_abs),
    op(
        IntrinsicId::FloatParse,
        "float_parse",
        primitives::float_parse,
    ),
    op(
        IntrinsicId::FloatToStr,
        "float_to_str",
        primitives::float_to_str,
    ),
    op(
        IntrinsicId::FloatToFixed,
        "float_to_fixed",
        primitives::float_to_fixed,
    ),
    op(IntrinsicId::FloatAbs, "float_abs", primitives::float_abs),
    op(
        IntrinsicId::CharToStr,
        "char_to_str",
        primitives::char_to_str,
    ),
    op(
        IntrinsicId::CharCodeAt,
        "char_code_at",
        primitives::char_code_at,
    ),
    op(
        IntrinsicId::SymbolIterator,
        "symbol_iterator",
        symbols::symbol_iterator,
    ),
    op(
        IntrinsicId::SymbolAsyncIterator,
        "symbol_async_iterator",
        symbols::symbol_async_iterator,
    ),
    op(
        IntrinsicId::StrTrimStart,
        "str_trim_start",
        primitives::str_trim_start,
    ),
    op(
        IntrinsicId::StrTrimEnd,
        "str_trim_end",
        primitives::str_trim_end,
    ),
    op(
        IntrinsicId::StrLastIndexOf,
        "str_last_index_of",
        primitives::str_last_index_of,
    ),
    op(IntrinsicId::StrSlice, "str_slice", primitives::str_slice),
    op(
        IntrinsicId::StrReplace,
        "str_replace",
        primitives::str_replace,
    ),
    op(
        IntrinsicId::StrReplaceAll,
        "str_replace_all",
        primitives::str_replace_all,
    ),
    op(
        IntrinsicId::StrCharCodeAt,
        "str_char_code_at",
        primitives::str_char_code_at,
    ),
    op(IntrinsicId::StrRepeat, "str_repeat", primitives::str_repeat),
    op(
        IntrinsicId::StrPadStart,
        "str_pad_start",
        primitives::str_pad_start,
    ),
    op(
        IntrinsicId::StrPadEnd,
        "str_pad_end",
        primitives::str_pad_end,
    ),
    op(IntrinsicId::StrConcat, "str_concat", primitives::str_concat),
    op(IntrinsicId::StrSubstr, "str_substr", primitives::str_substr),
    op(
        IntrinsicId::StrIsEmpty,
        "str_is_empty",
        primitives::str_is_empty,
    ),
    op(
        IntrinsicId::StrIsBlank,
        "str_is_blank",
        primitives::str_is_blank,
    ),
    op(
        IntrinsicId::StrIsDigit,
        "str_is_digit",
        primitives::str_is_digit,
    ),
    op(
        IntrinsicId::StrIsLetter,
        "str_is_letter",
        primitives::str_is_letter,
    ),
    op(
        IntrinsicId::StrIsWhitespace,
        "str_is_whitespace",
        primitives::str_is_whitespace,
    ),
    op(
        IntrinsicId::StrReverse,
        "str_reverse",
        primitives::str_reverse,
    ),
    op(
        IntrinsicId::StrCapitalize,
        "str_capitalize",
        primitives::str_capitalize,
    ),
    op(IntrinsicId::StrLines, "str_lines", primitives::str_lines),
    op(IntrinsicId::StrWords, "str_words", primitives::str_words),
    op(IntrinsicId::StrToInt, "str_to_int", primitives::str_to_int),
    op(
        IntrinsicId::StrToFloat,
        "str_to_float",
        primitives::str_to_float,
    ),
    op(
        IntrinsicId::StrFromValue,
        "str_from_value",
        primitives::str_from_value,
    ),
    op(IntrinsicId::IntSign, "int_sign", primitives::int_sign),
    op(IntrinsicId::IntNegate, "int_negate", primitives::int_negate),
    op(
        IntrinsicId::IntBitwiseNot,
        "int_bitwise_not",
        primitives::int_bitwise_not,
    ),
    op(IntrinsicId::IntMin, "int_min", primitives::int_min),
    op(IntrinsicId::IntMax, "int_max", primitives::int_max),
    op(IntrinsicId::IntClamp, "int_clamp", primitives::int_clamp),
    op(IntrinsicId::IntToHex, "int_to_hex", primitives::int_to_hex),
    op(
        IntrinsicId::IntToBinary,
        "int_to_binary",
        primitives::int_to_binary,
    ),
    op(
        IntrinsicId::IntToOctal,
        "int_to_octal",
        primitives::int_to_octal,
    ),
    op(
        IntrinsicId::IntToFloat,
        "int_to_float",
        primitives::int_to_float,
    ),
    op(IntrinsicId::IntPow, "int_pow", primitives::int_pow),
    op(IntrinsicId::FloatSign, "float_sign", primitives::float_sign),
    op(
        IntrinsicId::FloatNegate,
        "float_negate",
        primitives::float_negate,
    ),
    op(IntrinsicId::FloatMin, "float_min", primitives::float_min),
    op(IntrinsicId::FloatMax, "float_max", primitives::float_max),
    op(IntrinsicId::FloatPow, "float_pow", primitives::float_pow),
    op(IntrinsicId::StrAt, "str_at", primitives::str_at),
    op(
        IntrinsicId::StrCodePointAt,
        "str_code_point_at",
        primitives::str_code_point_at,
    ),
    op(
        IntrinsicId::IntIsInteger,
        "int_is_integer",
        primitives::int_is_integer,
    ),
    op(
        IntrinsicId::FloatIsNaN,
        "float_is_nan",
        primitives::float_is_nan,
    ),
    op(
        IntrinsicId::FloatIsFinite,
        "float_is_finite",
        primitives::float_is_finite,
    ),
    op(
        IntrinsicId::FloatIsInteger,
        "float_is_integer",
        primitives::float_is_integer,
    ),
];
