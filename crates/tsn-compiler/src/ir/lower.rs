use super::{
    BytecodeIrModule, IrBlock, IrFunction, IrInstr, IrOperand, IrTerminator, IrUpvalueDesc,
};
use crate::FunctionProto;
use std::collections::{BTreeSet, HashMap};
use tsn_core::OpCode;

pub fn lower_module(proto: &FunctionProto) -> Result<BytecodeIrModule, String> {
    Ok(BytecodeIrModule {
        entry: lower_function(proto)?,
    })
}

pub fn lower_function(proto: &FunctionProto) -> Result<IrFunction, String> {
    let decoded = decode_function(proto)?;
    let mut leaders = BTreeSet::new();
    leaders.insert(0usize);

    for ins in &decoded {
        match ins.term {
            DecodedTerm::Jump(target) | DecodedTerm::Loop(target) => {
                leaders.insert(target);
            }
            DecodedTerm::Branch {
                opcode: _,
                then_target,
                else_target,
            } => {
                leaders.insert(then_target);
                leaders.insert(else_target);
            }
            DecodedTerm::Return | DecodedTerm::Throw | DecodedTerm::None => {}
        }
    }

    let code_len = proto.chunk.code.len();
    let mut leader_list: Vec<usize> = leaders.into_iter().filter(|ip| *ip < code_len).collect();
    if leader_list.is_empty() {
        leader_list.push(0);
    }
    leader_list.sort_unstable();

    let mut blocks = Vec::new();
    let mut ip_to_block = HashMap::new();
    for (i, start) in leader_list.iter().enumerate() {
        let end = leader_list.get(i + 1).copied().unwrap_or(code_len);
        if start >= &end {
            continue;
        }
        let block_id = blocks.len();
        for ip in *start..end {
            ip_to_block.insert(ip, block_id);
        }

        let block_instrs: Vec<_> = decoded
            .iter()
            .filter(|ins| ins.instr.ip >= *start && ins.instr.ip < end)
            .map(|ins| ins.instr.clone())
            .collect();
        let term = block_instrs
            .last()
            .and_then(|last| decoded.iter().find(|d| d.instr.ip == last.ip))
            .map_or(IrTerminator::Fallthrough { next_ip: None }, |ins| {
                to_terminator(&ins.term, ins.next_ip)
            });

        blocks.push(IrBlock {
            id: block_id,
            start_ip: *start,
            end_ip: end,
            instrs: block_instrs,
            terminator: term,
            successors: Vec::new(),
            predecessors: Vec::new(),
        });
    }

    for i in 0..blocks.len() {
        let succs = terminator_successors(&blocks[i].terminator, &ip_to_block);
        blocks[i].successors = succs;
    }

    for i in 0..blocks.len() {
        let succs = blocks[i].successors.clone();
        for succ in succs {
            if let Some(target) = blocks.get_mut(succ) {
                target.predecessors.push(i);
            }
        }
    }

    let mut children = Vec::new();
    for d in decoded {
        if let IrOperand::Closure { function_const, .. } = d.instr.operand {
            if let Some(tsn_types::chunk::PoolEntry::Function(child)) =
                proto.chunk.constants.get(function_const as usize)
            {
                children.push(lower_function(child)?);
            }
        }
    }

    Ok(IrFunction {
        name: proto.name.clone(),
        arity: proto.arity,
        is_async: proto.is_async,
        is_generator: proto.is_generator,
        upvalue_count: proto.upvalue_count,
        cache_count: proto.cache_count,
        source_file: proto.chunk.source_file.clone(),
        constants_len: proto.chunk.constants.len(),
        blocks,
        children,
    })
}

#[derive(Clone)]
struct DecodedInstr {
    instr: IrInstr,
    next_ip: usize,
    term: DecodedTerm,
}

#[derive(Clone, Copy)]
enum DecodedTerm {
    None,
    Return,
    Throw,
    Jump(usize),
    Branch {
        opcode: OpCode,
        then_target: usize,
        else_target: usize,
    },
    Loop(usize),
}

fn decode_function(proto: &FunctionProto) -> Result<Vec<DecodedInstr>, String> {
    let code = &proto.chunk.code;
    let lines = &proto.chunk.lines;
    let mut out = Vec::new();
    let mut ip = 0usize;

    while ip < code.len() {
        let start = ip;
        let raw = code[ip];
        let Some(opcode) = OpCode::from_u16(raw) else {
            return Err(format!("invalid opcode {} at ip {}", raw, start));
        };
        ip += 1;

        let (operand, term) = if opcode == OpCode::OpClosure {
            let fn_idx = read_word(code, &mut ip, start, "closure function idx")?;
            let upvalue_count = match proto.chunk.constants.get(fn_idx as usize) {
                Some(tsn_types::chunk::PoolEntry::Function(f)) => f.upvalue_count,
                _ => 0,
            };
            let mut upvalues = Vec::with_capacity(upvalue_count);
            for _ in 0..upvalue_count {
                let is_local = read_word(code, &mut ip, start, "closure upvalue local flag")? != 0;
                let index = read_word(code, &mut ip, start, "closure upvalue index")?;
                upvalues.push(IrUpvalueDesc { is_local, index });
            }
            (
                IrOperand::Closure {
                    function_const: fn_idx,
                    upvalues,
                },
                DecodedTerm::None,
            )
        } else if has_two_operands(opcode) {
            let a = read_word(code, &mut ip, start, "operand a")?;
            let b = read_word(code, &mut ip, start, "operand b")?;
            (IrOperand::Two(a, b), DecodedTerm::None)
        } else if has_operand(opcode) {
            let a = read_word(code, &mut ip, start, "operand")?;
            let term = match opcode {
                OpCode::OpJump => {
                    let next = ip;
                    DecodedTerm::Jump(next + a as usize)
                }
                OpCode::OpJumpIfFalse | OpCode::OpJumpIfTrue => {
                    let next = ip;
                    DecodedTerm::Branch {
                        opcode,
                        then_target: next + a as usize,
                        else_target: next,
                    }
                }
                OpCode::OpLoop => {
                    let next = ip;
                    DecodedTerm::Loop(next.saturating_sub(a as usize))
                }
                _ => DecodedTerm::None,
            };
            (IrOperand::One(a), term)
        } else {
            let term = match opcode {
                OpCode::OpReturn => DecodedTerm::Return,
                OpCode::OpThrow => DecodedTerm::Throw,
                _ => DecodedTerm::None,
            };
            (IrOperand::None, term)
        };

        let line = lines.get(start).copied().unwrap_or(0);
        out.push(DecodedInstr {
            instr: IrInstr {
                ip: start,
                line,
                opcode,
                operand,
            },
            next_ip: ip,
            term,
        });
    }

    Ok(out)
}

fn read_word(code: &[u16], ip: &mut usize, start: usize, what: &str) -> Result<u16, String> {
    if *ip >= code.len() {
        return Err(format!(
            "truncated instruction at ip {} while reading {}",
            start, what
        ));
    }
    let value = code[*ip];
    *ip += 1;
    Ok(value)
}

fn to_terminator(term: &DecodedTerm, next_ip: usize) -> IrTerminator {
    match *term {
        DecodedTerm::Return => IrTerminator::Return,
        DecodedTerm::Throw => IrTerminator::Throw,
        DecodedTerm::Jump(target) => IrTerminator::Jump { target_ip: target },
        DecodedTerm::Loop(target) => IrTerminator::Loop { target_ip: target },
        DecodedTerm::Branch {
            opcode,
            then_target,
            else_target,
        } => IrTerminator::Branch {
            opcode,
            then_ip: then_target,
            else_ip: else_target,
        },
        DecodedTerm::None => IrTerminator::Fallthrough {
            next_ip: Some(next_ip),
        },
    }
}

fn terminator_successors(term: &IrTerminator, ip_to_block: &HashMap<usize, usize>) -> Vec<usize> {
    let mut out = Vec::new();
    match term {
        IrTerminator::Jump { target_ip } | IrTerminator::Loop { target_ip } => {
            if let Some(block) = ip_to_block.get(target_ip) {
                out.push(*block);
            }
        }
        IrTerminator::Branch {
            then_ip, else_ip, ..
        } => {
            if let Some(block) = ip_to_block.get(then_ip) {
                out.push(*block);
            }
            if let Some(block) = ip_to_block.get(else_ip) {
                if !out.contains(block) {
                    out.push(*block);
                }
            }
        }
        IrTerminator::Fallthrough { next_ip } => {
            if let Some(ip) = next_ip {
                if let Some(block) = ip_to_block.get(ip) {
                    out.push(*block);
                }
            }
        }
        IrTerminator::Return | IrTerminator::Throw => {}
    }
    out
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
