mod arith;
mod calls;
mod class;
mod collections;
mod compare;
mod exceptions;
mod modules;
mod props;
mod strings;
mod vars;

use self::modules::short_val;
use tsn_core::OpCode;
use tsn_types::future::FutureState;
use tsn_types::value::Value;

impl super::Vm {
    pub fn run_until(&mut self, stop_at: usize) -> Result<Value, String> {
        while self.frames.len() > stop_at {
            let last_idx = self.frames.len() - 1;
            let op = {
                let frame = &mut self.frames[last_idx];
                let code = &frame.closure.proto.chunk.code;
                if frame.ip >= code.len() {
                    if self.frames.len() > stop_at + 1 {
                        self.frames.pop();
                        continue;
                    } else {
                        return Ok(Value::Null);
                    }
                }
                let op_byte = code[frame.ip];
                frame.ip += 1;
                match OpCode::from_u16(op_byte) {
                    Some(op) => op,
                    None => return Err(format!("unknown opcode: {}", op_byte)),
                }
            };

            if self.trace {
                let frame = &self.frames[last_idx];
                let ip = frame.ip.saturating_sub(1);
                let top = self
                    .stack
                    .last()
                    .map(|v| format!("  {}", short_val(v)))
                    .unwrap_or_default();
                eprintln!(
                    "  @{:5} {:<28}{}",
                    ip,
                    format!("{:?}", op).trim_start_matches("Op"),
                    top
                );
            }

            if let Some(profile) = &self.opcode_profile {
                profile.record(op);
            }

            match op {
                OpCode::OpPop => {
                    self.pop()?;
                }
                OpCode::OpDup => {
                    let v = self.stack.last().ok_or("stack underflow")?.clone();
                    self.push(v);
                }
                OpCode::OpDup2 => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.push(b.clone());
                    self.push(a);
                    self.push(b);
                }
                OpCode::OpSwap => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(a);
                }
                OpCode::OpRot3 => {
                    let c = self.pop()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(c);
                    self.push(a);
                }
                OpCode::OpRot => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(a);
                }
                OpCode::OpPushNull => self.push(Value::Null),
                OpCode::OpPushTrue => self.push(Value::Bool(true)),
                OpCode::OpPushFalse => self.push(Value::Bool(false)),
                OpCode::OpPushConst => {
                    let idx = self.read_u16();
                    let v = self.read_const(idx);
                    self.push(v);
                }

                op @ (OpCode::OpAdd
                | OpCode::OpAddI32
                | OpCode::OpAddF64
                | OpCode::OpSub
                | OpCode::OpSubI32
                | OpCode::OpSubF64
                | OpCode::OpMul
                | OpCode::OpMulI32
                | OpCode::OpMulF64
                | OpCode::OpDiv
                | OpCode::OpDivI32
                | OpCode::OpDivF64
                | OpCode::OpMod
                | OpCode::OpPow
                | OpCode::OpNegate
                | OpCode::OpBitAnd
                | OpCode::OpBitOr
                | OpCode::OpBitXor
                | OpCode::OpShl
                | OpCode::OpShr
                | OpCode::OpUshr) => self.exec_arith_op(op)?,

                op @ (OpCode::OpEq
                | OpCode::OpNeq
                | OpCode::OpLt
                | OpCode::OpLte
                | OpCode::OpGt
                | OpCode::OpGte
                | OpCode::OpNot
                | OpCode::OpIsNull
                | OpCode::OpAssertNotNull
                | OpCode::OpTypeof
                | OpCode::OpInstanceof
                | OpCode::OpIn) => self.exec_compare_op(op)?,

                op @ (OpCode::OpStrConcat
                | OpCode::OpToString
                | OpCode::OpStrLength
                | OpCode::OpStrSlice) => self.exec_string_op(op)?,

                op @ (OpCode::OpGetGlobal
                | OpCode::OpSetGlobal
                | OpCode::OpDefineGlobal
                | OpCode::OpGetLocal
                | OpCode::OpSetLocal
                | OpCode::OpSetLocalDrop
                | OpCode::OpGetUpvalue
                | OpCode::OpSetUpvalue
                | OpCode::OpCloseUpvalue) => self.exec_var_op(op)?,

                OpCode::OpJump => {
                    let offset = self.read_u16() as usize;
                    self.frame_mut().ip += offset;
                }
                OpCode::OpJumpIfFalse => {
                    let offset = self.read_u16() as usize;
                    if !self.stack.last().ok_or("stack underflow")?.is_truthy()? {
                        self.frame_mut().ip += offset;
                    }
                }
                OpCode::OpJumpIfTrue => {
                    let offset = self.read_u16() as usize;
                    if self.stack.last().ok_or("stack underflow")?.is_truthy()? {
                        self.frame_mut().ip += offset;
                    }
                }
                OpCode::OpLoop => {
                    let target = self.read_u16() as usize;
                    self.frame_mut().ip -= target;
                }

                op @ (OpCode::OpBuildArray
                | OpCode::OpArrayLength
                | OpCode::OpArrayPush
                | OpCode::OpArrayPop
                | OpCode::OpArrayExtend
                | OpCode::OpIsArray
                | OpCode::OpBuildObject
                | OpCode::OpObjectRest
                | OpCode::OpInvokeRuntimeStatic) => self.exec_collection_op(op)?,

                op @ (OpCode::OpGetProperty
                | OpCode::OpGetPropertyMaybe
                | OpCode::OpSetProperty
                | OpCode::OpGetIndex
                | OpCode::OpSetIndex
                | OpCode::OpGetFixedField
                | OpCode::OpSetFixedField
                | OpCode::OpGetSymbol) => self.exec_prop_op(op)?,

                OpCode::OpClosure => {
                    self.op_closure()?;
                }
                OpCode::OpWrapSpread => {
                    self.op_wrap_spread()?;
                }
                OpCode::OpBindMethod => {
                    self.op_bind_method()?;
                }
                OpCode::OpCall => {
                    let arg_count = self.read_u16() as usize;
                    let callee = {
                        let idx = self.stack.len() - 1 - arg_count;
                        self.stack.get(idx).cloned().ok_or("no callee on stack")?
                    };
                    self.call_value(callee, arg_count)?;
                }
                OpCode::OpCallSpread => {
                    let arg_count = self.read_u16() as usize;
                    let callee = {
                        let idx = self.stack.len() - 1 - arg_count;
                        self.stack.get(idx).cloned().ok_or("no callee on stack")?
                    };
                    self.call_value_with_spread(callee, arg_count)?;
                }
                OpCode::OpReturn => {
                    if let Some(v) = self.op_return(stop_at)? {
                        return Ok(v);
                    }
                }

                op @ (OpCode::OpClass
                | OpCode::OpMethod
                | OpCode::OpDefineStatic
                | OpCode::OpDeclareField
                | OpCode::OpDefineGetter
                | OpCode::OpDefineSetter
                | OpCode::OpDefineStaticGetter
                | OpCode::OpDefineStaticSetter
                | OpCode::OpInvokeVirtual) => self.exec_class_op(op)?,

                OpCode::OpInherit => {
                    self.op_inherit()?;
                }
                OpCode::OpGetSuper => {
                    self.op_get_super()?;
                }
                OpCode::OpObjectKeys => {
                    self.op_object_keys()?;
                }

                op @ (OpCode::OpTry | OpCode::OpPopTry | OpCode::OpThrow) => {
                    self.exec_exception_op(op)?;
                }

                OpCode::OpAwait => {
                    let top = self.pop()?;
                    match top {
                        Value::Future(ref fut) => match fut.peek_state() {
                            FutureState::Resolved(v) => self.push(v),
                            FutureState::Rejected(v) => self.dispatch_value(v)?,
                            FutureState::Pending => {
                                self.frame_mut().ip -= 1;
                                self.vm_suspend = Some(super::VmSuspend::Future(fut.clone()));
                                return Ok(Value::Null);
                            }
                        },
                        other => self.push(other),
                    }
                }
                OpCode::OpYield => {
                    let val = self.pop()?;
                    self.vm_suspend = Some(super::VmSuspend::Yield(val));
                    return Ok(Value::Null);
                }

                op @ (OpCode::OpImport | OpCode::OpReexport | OpCode::OpMergeExports) => {
                    self.exec_import_op(op)?;
                }

                OpCode::OpCallIntrinsic => {
                    let intrinsic_id = self.read_u16();
                    let argc = self.read_u16() as usize;
                    let start = self.stack.len().saturating_sub(argc);
                    let args: Vec<Value> = self.stack.drain(start..).collect();
                    let result = tsn_runtime::dispatch_intrinsic(intrinsic_id, self, &args)?;
                    self.push(result);
                }
                OpCode::OpCallIntrinsicSpread => {
                    let intrinsic_id = self.read_u16();
                    let argc = self.read_u16() as usize;
                    let start = self.stack.len().saturating_sub(argc);
                    let raw_args: Vec<Value> = self.stack.drain(start..).collect();
                    let args = self.flatten_spread_args(raw_args)?;
                    let result = tsn_runtime::dispatch_intrinsic(intrinsic_id, self, &args)?;
                    self.push(result);
                }
            }
        }
        Ok(Value::Null)
    }
}
