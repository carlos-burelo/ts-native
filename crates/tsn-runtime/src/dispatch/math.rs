use super::entry::{op, DispatchEntry};
use crate::modules::math;
use tsn_core::intrinsic::IntrinsicId;

pub(crate) static OPS: &[DispatchEntry] = &[
    op(IntrinsicId::MathAbs, "math_abs", math::math_abs),
    op(IntrinsicId::MathAcos, "math_acos", math::math_acos),
    op(IntrinsicId::MathAsin, "math_asin", math::math_asin),
    op(IntrinsicId::MathAtan, "math_atan", math::math_atan),
    op(IntrinsicId::MathAtan2, "math_atan2", math::math_atan2),
    op(IntrinsicId::MathCeil, "math_ceil", math::math_ceil),
    op(IntrinsicId::MathCos, "math_cos", math::math_cos),
    op(IntrinsicId::MathExp, "math_exp", math::math_exp),
    op(IntrinsicId::MathFloor, "math_floor", math::math_floor),
    op(IntrinsicId::MathLog, "math_log", math::math_log),
    op(IntrinsicId::MathMax, "math_max", math::math_max),
    op(IntrinsicId::MathMin, "math_min", math::math_min),
    op(IntrinsicId::MathPow, "math_pow", math::math_pow),
    op(IntrinsicId::MathRandom, "math_random", math::math_random),
    op(IntrinsicId::MathRound, "math_round", math::math_round),
    op(IntrinsicId::MathSin, "math_sin", math::math_sin),
    op(IntrinsicId::MathSqrt, "math_sqrt", math::math_sqrt),
    op(IntrinsicId::MathTan, "math_tan", math::math_tan),
    op(IntrinsicId::MathTrunc, "math_trunc", math::math_trunc),
    op(IntrinsicId::MathSign, "math_sign", math::math_sign),
];
