use tsn_compiler::{FunctionProto, Literal, PoolEntry};
use tsn_core::{well_known, OpCode};

pub fn print(proto: &FunctionProto) {
    disasm_proto(proto, "");
}

fn disasm_proto(proto: &FunctionProto, indent: &str) {
    let name = proto.name.as_deref().unwrap_or("<anonymous>");
    eprintln!(
        "{}╔══ fn {} (arity={}, async={}, gen={})",
        indent, name, proto.arity, proto.is_async, proto.is_generator
    );
    eprintln!("{}║  constants ({}):", indent, proto.chunk.constants.len());
    for (i, entry) in proto.chunk.constants.iter().enumerate() {
        let desc = pool_entry_desc(entry);
        eprintln!("{}║  [{:03}] {}", indent, i, desc);
    }
    eprintln!("{}║  bytecode ({} words):", indent, proto.chunk.code.len());
    let mut ip = 0usize;
    while ip < proto.chunk.code.len() {
        disasm_instr(
            &proto.chunk.code,
            &proto.chunk.lines,
            &proto.chunk.constants,
            &mut ip,
            indent,
        );
    }
    eprintln!("{}╚══", indent);

    let nested_indent = format!("{}  ", indent);
    for entry in &proto.chunk.constants {
        if let PoolEntry::Function(nested) = entry {
            disasm_proto(nested, &nested_indent);
        }
    }
}

fn disasm_instr(code: &[u16], lines: &[u32], pool: &[PoolEntry], ip: &mut usize, indent: &str) {
    let offset = *ip;
    let line = lines.get(offset).copied().unwrap_or(0);
    let raw = code[offset];
    *ip += 1;

    let Some(op) = OpCode::from_u16(raw) else {
        eprintln!(
            "{}║    {:04x}  ({:3})  <unknown opcode: {}>",
            indent, offset, line, raw
        );
        return;
    };

    if has_two_operands(op) {
        let operand1 = code[*ip];
        *ip += 1;
        let operand2 = if *ip < code.len() { code[*ip] } else { 0 };
        *ip += 1;

        let hint = match op {
            OpCode::OpGetProperty | OpCode::OpSetProperty => match pool.get(operand1 as usize) {
                Some(PoolEntry::Literal(Literal::Str(s))) => {
                    format!("  ; \"{}\" [ic:{}]", s, operand2)
                }
                _ => format!("  ; [ic:{}]", operand2),
            },
            _ => pool_hint(op, operand1, pool),
        };
        eprintln!(
            "{}║    {:04x}  ({:3})  {:<28} {} {}{}",
            indent,
            offset,
            line,
            op_name(op),
            operand1,
            operand2,
            hint
        );
    } else if has_operand(op) {
        if *ip >= code.len() {
            eprintln!(
                "{}║    {:04x}  ({:3})  {:<28} <missing operand>",
                indent,
                offset,
                line,
                op_name(op)
            );
            return;
        }
        let operand = code[*ip];
        *ip += 1;
        let hint = pool_hint(op, operand, pool);
        eprintln!(
            "{}║    {:04x}  ({:3})  {:<28} {}{}",
            indent,
            offset,
            line,
            op_name(op),
            operand,
            hint
        );
    } else {
        eprintln!(
            "{}║    {:04x}  ({:3})  {}",
            indent,
            offset,
            line,
            op_name(op)
        );
    }
}

fn has_operand(op: OpCode) -> bool {
    matches!(
        op,
        OpCode::OpPushConst
            | OpCode::OpGetGlobal
            | OpCode::OpSetGlobal
            | OpCode::OpDefineGlobal
            | OpCode::OpGetLocal
            | OpCode::OpSetLocal
            | OpCode::OpSetLocalDrop
            | OpCode::OpJump
            | OpCode::OpJumpIfFalse
            | OpCode::OpJumpIfTrue
            | OpCode::OpLoop
            | OpCode::OpTry
            | OpCode::OpCall
            | OpCode::OpCallSpread
            | OpCode::OpBuildArray
            | OpCode::OpBuildObject
            | OpCode::OpGetSuper
            | OpCode::OpClass
            | OpCode::OpMethod
            | OpCode::OpDefineStatic
            | OpCode::OpDefineGetter
            | OpCode::OpDefineSetter
            | OpCode::OpDefineStaticGetter
            | OpCode::OpDefineStaticSetter
            | OpCode::OpClosure
            | OpCode::OpGetUpvalue
            | OpCode::OpSetUpvalue
            | OpCode::OpCloseUpvalue
            | OpCode::OpImport
            | OpCode::OpReexport
            | OpCode::OpMergeExports
            | OpCode::OpGetFixedField
            | OpCode::OpSetFixedField
            | OpCode::OpInvokeVirtual
            | OpCode::OpGetSymbol
            | OpCode::OpDeclareField
            | OpCode::OpCallIntrinsicSpread
    )
}

fn has_two_operands(op: OpCode) -> bool {
    matches!(
        op,
        OpCode::OpInvokeRuntimeStatic
            | OpCode::OpGetProperty
            | OpCode::OpSetProperty
            | OpCode::OpGetPropertyMaybe
            | OpCode::OpCallIntrinsic
            | OpCode::OpCallIntrinsicSpread
    )
}

fn op_name(op: OpCode) -> &'static str {
    match op {
        OpCode::OpPop => "POP",
        OpCode::OpPushConst => "PUSH_CONST",
        OpCode::OpPushNull => "PUSH_NULL",
        OpCode::OpPushTrue => "PUSH_TRUE",
        OpCode::OpPushFalse => "PUSH_FALSE",
        OpCode::OpDup => "DUP",
        OpCode::OpDup2 => "DUP2",
        OpCode::OpRot => "ROT",
        OpCode::OpRot3 => "ROT3",
        OpCode::OpSwap => "SWAP",
        OpCode::OpAdd => "ADD",
        OpCode::OpSub => "SUB",
        OpCode::OpMul => "MUL",
        OpCode::OpDiv => "DIV",
        OpCode::OpMod => "MOD",
        OpCode::OpPow => "POW",
        OpCode::OpNegate => "NEGATE",
        OpCode::OpNot => "NOT",
        OpCode::OpToString => "TO_STRING",
        OpCode::OpEq => "EQ",
        OpCode::OpNeq => "NEQ",
        OpCode::OpLt => "LT",
        OpCode::OpLte => "LTE",
        OpCode::OpGt => "GT",
        OpCode::OpGte => "GTE",
        OpCode::OpBitAnd => "BIT_AND",
        OpCode::OpBitOr => "BIT_OR",
        OpCode::OpBitXor => "BIT_XOR",
        OpCode::OpShl => "SHL",
        OpCode::OpShr => "SHR",
        OpCode::OpUshr => "USHR",
        OpCode::OpGetGlobal => "GET_GLOBAL",
        OpCode::OpSetGlobal => "SET_GLOBAL",
        OpCode::OpDefineGlobal => "DEFINE_GLOBAL",
        OpCode::OpGetLocal => "GET_LOCAL",
        OpCode::OpSetLocal => "SET_LOCAL",
        OpCode::OpSetLocalDrop => "SET_LOCAL_DROP",
        OpCode::OpJump => "JUMP",
        OpCode::OpJumpIfFalse => "JUMP_IF_FALSE",
        OpCode::OpJumpIfTrue => "JUMP_IF_TRUE",
        OpCode::OpLoop => "LOOP",
        OpCode::OpCall => "CALL",
        OpCode::OpReturn => "RETURN",
        OpCode::OpBuildArray => "BUILD_ARRAY",
        OpCode::OpBuildObject => "BUILD_OBJECT",
        OpCode::OpGetIndex => "GET_INDEX",
        OpCode::OpSetIndex => "SET_INDEX",
        OpCode::OpGetProperty => "GET_PROPERTY",
        OpCode::OpSetProperty => "SET_PROPERTY",
        OpCode::OpObjectRest => "OBJECT_REST",
        OpCode::OpClass => "CLASS",
        OpCode::OpMethod => "METHOD",
        OpCode::OpTry => "TRY",
        OpCode::OpThrow => "THROW",
        OpCode::OpPopTry => "POP_TRY",
        OpCode::OpImport => "IMPORT",
        OpCode::OpReexport => "REEXPORT",
        OpCode::OpMergeExports => "MERGE_EXPORTS",
        OpCode::OpInherit => "INHERIT",
        OpCode::OpGetSuper => "GET_SUPER",
        OpCode::OpDefineStatic => "DEFINE_STATIC",
        OpCode::OpDefineGetter => "DEFINE_GETTER",
        OpCode::OpDefineSetter => "DEFINE_SETTER",
        OpCode::OpDefineStaticGetter => "DEFINE_STATIC_GETTER",
        OpCode::OpDefineStaticSetter => "DEFINE_STATIC_SETTER",
        OpCode::OpClosure => "CLOSURE",
        OpCode::OpGetUpvalue => "GET_UPVALUE",
        OpCode::OpSetUpvalue => "SET_UPVALUE",
        OpCode::OpCloseUpvalue => "CLOSE_UPVALUE",
        OpCode::OpAwait => "AWAIT",
        OpCode::OpYield => "YIELD",
        OpCode::OpStrLength => "STR_LENGTH",
        OpCode::OpStrConcat => "STR_CONCAT",
        OpCode::OpStrSlice => "STR_SLICE",
        OpCode::OpArrayLength => "ARRAY_LENGTH",
        OpCode::OpArrayPush => "ARRAY_PUSH",
        OpCode::OpArrayPop => "ARRAY_POP",
        OpCode::OpArrayExtend => "ARRAY_EXTEND",
        OpCode::OpIsNull => "IS_NULL",
        OpCode::OpIsArray => "IS_ARRAY",
        OpCode::OpTypeof => "TYPEOF",
        OpCode::OpInstanceof => "INSTANCEOF",
        OpCode::OpIn => "IN",
        OpCode::OpInvokeRuntimeStatic => "INVOKE_RUNTIME_STATIC",
        OpCode::OpAddI32 => "ADD_I32",
        OpCode::OpAddF64 => "ADD_F64",
        OpCode::OpSubI32 => "SUB_I32",
        OpCode::OpSubF64 => "SUB_F64",
        OpCode::OpMulI32 => "MUL_I32",
        OpCode::OpMulF64 => "MUL_F64",
        OpCode::OpDivI32 => "DIV_I32",
        OpCode::OpDivF64 => "DIV_F64",
        OpCode::OpGetFixedField => "GET_FIXED_FIELD",
        OpCode::OpSetFixedField => "SET_FIXED_FIELD",
        OpCode::OpInvokeVirtual => "INVOKE_VIRTUAL",
        OpCode::OpObjectKeys => "OBJECT_KEYS",
        OpCode::OpGetPropertyMaybe => "GET_PROPERTY_MAYBE",
        OpCode::OpGetSymbol => "GET_SYMBOL",
        OpCode::OpDeclareField => "DECLARE_FIELD",
        OpCode::OpCallIntrinsic => "CALL_INTRINSIC",
        OpCode::OpCallIntrinsicSpread => "CALL_INTRINSIC_SPREAD",
        OpCode::OpWrapSpread => "WRAP_SPREAD",
        OpCode::OpCallSpread => "CALL_SPREAD",
        OpCode::OpBindMethod => "BIND_METHOD",
        OpCode::OpAssertNotNull => "ASSERT_NOT_NULL",
    }
}

fn pool_hint(op: OpCode, operand: u16, pool: &[PoolEntry]) -> String {
    match op {
        OpCode::OpPushConst => match pool.get(operand as usize) {
            Some(PoolEntry::Literal(lit)) => format!("  ; {}", format_literal(lit)),
            Some(PoolEntry::Function(f)) => {
                let name = f.name.as_deref().unwrap_or("?");
                format!("  ; <fn {}>", name)
            }
            None => "  ; <out-of-range>".to_owned(),
        },
        OpCode::OpGetGlobal
        | OpCode::OpSetGlobal
        | OpCode::OpDefineGlobal
        | OpCode::OpGetSuper
        | OpCode::OpClass
        | OpCode::OpMethod
        | OpCode::OpDefineStatic
        | OpCode::OpDefineGetter
        | OpCode::OpDefineSetter
        | OpCode::OpDefineStaticGetter
        | OpCode::OpDefineStaticSetter
        | OpCode::OpImport
        | OpCode::OpReexport
        | OpCode::OpMergeExports
        | OpCode::OpInvokeRuntimeStatic
        | OpCode::OpInvokeVirtual
        | OpCode::OpDeclareField => match pool.get(operand as usize) {
            Some(PoolEntry::Literal(Literal::Str(s))) => format!("  ; \"{}\"", s),
            _ => String::new(),
        },
        OpCode::OpGetFixedField | OpCode::OpSetFixedField => format!("  ; slot[{}]", operand),
        OpCode::OpClosure => match pool.get(operand as usize) {
            Some(PoolEntry::Function(f)) => {
                let name = f.name.as_deref().unwrap_or("?");
                format!("  ; <fn {}>", name)
            }
            _ => String::new(),
        },
        OpCode::OpJump | OpCode::OpJumpIfFalse | OpCode::OpJumpIfTrue | OpCode::OpLoop => {
            format!("  ; → {:04x}", operand)
        }
        OpCode::OpCall | OpCode::OpCallSpread => format!("  ; {} arg(s)", operand),
        OpCode::OpBuildArray => format!("  ; {} element(s)", operand),
        OpCode::OpBuildObject => format!("  ; {} pair(s)", operand),
        OpCode::OpTry => format!("  ; catch → {:04x}", operand),
        OpCode::OpGetUpvalue | OpCode::OpSetUpvalue | OpCode::OpCloseUpvalue => {
            format!("  ; upvalue[{}]", operand)
        }
        OpCode::OpGetLocal | OpCode::OpSetLocal | OpCode::OpSetLocalDrop => {
            format!("  ; slot[{}]", operand)
        }
        _ => String::new(),
    }
}

fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Null => well_known::NULL.to_owned(),
        Literal::Bool(b) => b.to_string(),
        Literal::Int(n) => n.to_string(),
        Literal::Float(f) => format!("{:?}", f),
        Literal::Str(s) => format!("\"{}\"", s),
        Literal::BigInt(n) => format!("{}n", n),
        Literal::Decimal(d) => format!("{}d", d),
        Literal::Symbol(s) => format!("Symbol({:?})", s),
    }
}

fn pool_entry_desc(entry: &PoolEntry) -> String {
    match entry {
        PoolEntry::Literal(lit) => format_literal(lit),
        PoolEntry::Function(f) => {
            let name = f.name.as_deref().unwrap_or("?");
            format!("<fn {} (arity={})>", name, f.arity)
        }
    }
}
