use tsn_core::{IntrinsicId, TypeKind};

use crate::types::Type;

pub fn intrinsic_return_type(id: IntrinsicId) -> Type {
    use IntrinsicId::*;
    match id {
        ArrayIsArray | MapHas | SetHas | FsExists | NetIsIp | NetIsIpv4 | NetIsIpv6
        | StrIncludes | StrStartsWith | StrEndsWith | StrIsEmpty | StrIsBlank | StrIsDigit
        | StrIsLetter | StrIsWhitespace | IntIsInteger | FloatIsNaN | FloatIsFinite
        | FloatIsInteger | ReflectHasMeta => Type(TypeKind::Bool),

        TimeNow | MapSize | SetSize | StrLength | ArrayLength | StrIndexOf | StrLastIndexOf
        | StrCodePointAt | StrCharCodeAt | ArrayIndexOf | ArrayFindIndex | IntAbs | IntSign
        | IntNegate | IntBitwiseNot | IntMin | IntMax | IntClamp | IntPow | IntParse | MathCeil
        | MathFloor | MathRound | MathTrunc | MathSign | ReflectDefineMeta | TimerSet
        | SymbolIterator | SymbolAsyncIterator => Type(TypeKind::Int),

        FloatAbs | FloatSign | FloatNegate | FloatMin | FloatMax | FloatPow | FloatParse
        | IntToFloat | MathAbs | MathAcos | MathAsin | MathAtan | MathAtan2 | MathCos | MathExp
        | MathLog | MathMax | MathMin | MathPow | MathRandom | MathSin | MathSqrt | MathTan
        | MathNan | MathInfinity => Type(TypeKind::Float),

        JsonStringify | PathNormalize | FsRead | FsReadText | FsKind | FsReadLink | FsTempDir
        | SysPlatform | SysCwd | CryptoSha256 | CryptoSha512 | CryptoRandomHex
        | CryptoBase64Enc | CryptoBase64Dec | CryptoHmac | CryptoUuid | NetJoinHostPort
        | NetBuildQuery | NetAppendQuery | NetEncUriComponent | NetDecUriComponent
        | NetBasicAuth | IntToStr | IntToFixed | IntToHex | IntToBinary | IntToOctal
        | FloatToStr | FloatToFixed | CharToStr | StrFromCharCode | StrAt | StrCharAt
        | StrToLower | StrToUpper | StrTrim | StrTrimStart | StrTrimEnd | StrSlice
        | StrSubstring | StrSubstr | StrReplace | StrReplaceAll | StrRepeat | StrPadStart
        | StrPadEnd | StrConcat | StrCapitalize | StrReverse | TimeToIso | StrFromValue => {
            Type(TypeKind::Str)
        }

        ConsoleLog | IoWrite | IoWriteln | IoFlush | MapSet | MapDelete | MapClear | SetAdd
        | SetDelete | SetClear | FsWrite | FsWriteBytes | FsWriteText | FsAppendText | FsMkdir
        | FsMkdirAll | FsRemove | FsRemoveAll | FsRename | FsCopy | FsSymlink | FsWatch
        | SysExit | SysEnvSet | TimerClear | AssertTest | AssertSummary => Type(TypeKind::Void),

        _ => Type(TypeKind::Dynamic),
    }
}
