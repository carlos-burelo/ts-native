mod assign;
use super::Compiler;
use crate::chunk::{Literal, PoolEntry};
use assign::compound_op_to_arith;
use tsn_core::ast::{
    ArrayEl, ArrowBody, AssignOp, BinaryOp, Expr, LogicalOp, Modifiers, ObjectProp, Param, Pattern,
    PropKey, Stmt, TemplatePart, UnaryOp, UpdateOp,
};
use tsn_core::SourceRange;
use tsn_core::{NumericKind, OpCode};

impl Compiler {
    fn emit_call_opcode(&mut self, arg_count: u16, has_spread: bool) {
        self.emit1(
            if has_spread {
                OpCode::OpCallSpread
            } else {
                OpCode::OpCall
            },
            arg_count,
        );
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        self.line = expr.range().start.line;
        match expr {
            Expr::NullLiteral { .. } => self.emit(OpCode::OpPushNull),
            Expr::BoolLiteral { value, .. } => {
                if *value {
                    self.emit(OpCode::OpPushTrue);
                } else {
                    self.emit(OpCode::OpPushFalse);
                }
            }
            Expr::IntLiteral { value, .. } => {
                let idx = self.add_const(PoolEntry::Literal(Literal::Int(*value)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::FloatLiteral { value, .. } => {
                let idx = self.add_const(PoolEntry::Literal(Literal::Float(*value)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::BigIntLiteral { raw, .. } => {
                let s = raw.trim_end_matches('n').replace('_', "");
                let num: i128 = if let Some(rest) =
                    s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
                {
                    i128::from_str_radix(rest, 16).unwrap_or(0)
                } else if let Some(rest) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
                    i128::from_str_radix(rest, 8).unwrap_or(0)
                } else if let Some(rest) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
                    i128::from_str_radix(rest, 2).unwrap_or(0)
                } else {
                    s.parse().unwrap_or(0)
                };
                let idx = self.add_const(PoolEntry::Literal(Literal::BigInt(num)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::DecimalLiteral { raw, .. } => {
                use std::str::FromStr;
                let d = rust_decimal::Decimal::from_str(raw.trim_end_matches('d'))
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let idx = self.add_const(PoolEntry::Literal(Literal::Decimal(d)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::StrLiteral { value, .. } => {
                let idx = self.add_str(value);
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::CharLiteral { value, .. } => {
                let idx = self.add_str(value.to_string());
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::RegexLiteral { pattern, flags, .. } => {
                let raw = format!("/{}/{}", pattern, flags);
                let idx = self.add_str(&raw);
                self.emit1(OpCode::OpPushConst, idx);
            }

            Expr::Identifier { name, .. } => {
                self.emit_get_var(name);
            }

            Expr::This { .. } => {
                self.emit_get_var("this");
            }
            Expr::Super { .. } => {
                let idx = self.add_str("super");
                self.emit1(OpCode::OpGetSuper, idx);
            }

            Expr::Unary { op, operand, .. } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Minus => self.emit(OpCode::OpNegate),
                    UnaryOp::Plus => {}
                    UnaryOp::Not => self.emit(OpCode::OpNot),
                    UnaryOp::BitNot => {
                        let neg1 = self.add_const(PoolEntry::Literal(Literal::Int(-1)));
                        self.emit1(OpCode::OpPushConst, neg1);
                        self.emit(OpCode::OpBitXor);
                    }
                    UnaryOp::Typeof => self.emit(OpCode::OpTypeof),
                }
            }

            Expr::Update {
                op,
                prefix,
                operand,
                ..
            } => match operand.as_ref() {
                Expr::Identifier { name, .. } => {
                    self.emit_get_var(name);
                    if !prefix {
                        self.emit(OpCode::OpDup);
                    }
                    let one = self.add_const(PoolEntry::Literal(Literal::Int(1)));
                    self.emit1(OpCode::OpPushConst, one);
                    match op {
                        UpdateOp::Increment => self.emit(OpCode::OpAdd),
                        UpdateOp::Decrement => self.emit(OpCode::OpSub),
                    }
                    self.emit_set_var(name);
                    if !prefix {
                        self.emit(OpCode::OpPop);
                    }
                }
                _ => {
                    self.compile_expr(operand)?;
                }
            },

            Expr::Binary {
                op,
                left,
                right,
                range,
            } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinaryOp::Add => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpAddI32,
                            Some(NumericKind::Float) => OpCode::OpAddF64,
                            None => OpCode::OpAdd,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Sub => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpSubI32,
                            Some(NumericKind::Float) => OpCode::OpSubF64,
                            None => OpCode::OpSub,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Mul => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpMulI32,
                            Some(NumericKind::Float) => OpCode::OpMulF64,
                            None => OpCode::OpMul,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Div => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Float) | Some(NumericKind::Int) => OpCode::OpDivF64,
                            None => OpCode::OpDiv,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Mod => self.emit(OpCode::OpMod),
                    BinaryOp::Pow => self.emit(OpCode::OpPow),
                    BinaryOp::Eq => self.emit(OpCode::OpEq),
                    BinaryOp::NotEq => self.emit(OpCode::OpNeq),
                    BinaryOp::Lt => self.emit(OpCode::OpLt),
                    BinaryOp::LtEq => self.emit(OpCode::OpLte),
                    BinaryOp::Gt => self.emit(OpCode::OpGt),
                    BinaryOp::GtEq => self.emit(OpCode::OpGte),
                    BinaryOp::BitAnd => self.emit(OpCode::OpBitAnd),
                    BinaryOp::BitOr => self.emit(OpCode::OpBitOr),
                    BinaryOp::BitXor => self.emit(OpCode::OpBitXor),
                    BinaryOp::Shl => self.emit(OpCode::OpShl),
                    BinaryOp::Shr => self.emit(OpCode::OpShr),
                    BinaryOp::UShr => self.emit(OpCode::OpUshr),
                    BinaryOp::Instanceof => self.emit(OpCode::OpInstanceof),
                    BinaryOp::In => self.emit(OpCode::OpIn),
                }
            }

            Expr::Logical {
                op, left, right, ..
            } => {
                self.compile_expr(left)?;
                match op {
                    LogicalOp::And => {
                        let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(skip);
                    }
                    LogicalOp::Or => {
                        let skip = self.emit_jump(OpCode::OpJumpIfTrue);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(skip);
                    }
                    LogicalOp::Nullish => {
                        self.emit(OpCode::OpDup);
                        self.emit(OpCode::OpIsNull);
                        let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                        self.emit(OpCode::OpPop);
                        let end_jump = self.emit_jump(OpCode::OpJump);
                        self.patch_jump(null_jump);
                        self.emit(OpCode::OpPop);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(end_jump);
                    }
                }
            }

            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.compile_expr(test)?;
                let else_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.compile_expr(consequent)?;
                let end_jump = self.emit_jump(OpCode::OpJump);
                self.patch_jump(else_jump);
                self.emit(OpCode::OpPop);
                self.compile_expr(alternate)?;
                self.patch_jump(end_jump);
            }

            Expr::Assign {
                op, target, value, ..
            } => match op {
                AssignOp::Assign => {
                    self.compile_expr(value)?;
                    self.emit_assign_target(target)?;
                }

                AssignOp::AndAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(skip);
                }
                AssignOp::OrAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    let skip = self.emit_jump(OpCode::OpJumpIfTrue);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(skip);
                }
                AssignOp::NullishAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    self.emit(OpCode::OpIsNull);
                    let is_null = self.emit_jump(OpCode::OpJumpIfFalse);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(is_null);
                }

                _ => {
                    let arith_op =
                        compound_op_to_arith(*op).expect("unhandled compound assignment operator");
                    self.compile_expr(target)?;
                    self.compile_expr(value)?;
                    self.emit(arith_op);
                    self.emit_assign_target(target)?;
                }
            },

            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if !*computed {
                    if let Some(mangled) = self
                        .lookup_extension_member(expr.range().start.offset)
                        .map(|s| s.to_owned())
                    {
                        self.compile_expr(object)?;
                        if *optional {
                            self.emit(OpCode::OpDup);
                            self.emit(OpCode::OpIsNull);
                            let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                            self.emit(OpCode::OpPop);
                            self.emit_get_var(&mangled);
                            if mangled.starts_with("__extget_") {
                                self.emit(OpCode::OpSwap);
                                self.emit1(OpCode::OpCall, 1);
                            } else {
                                self.emit(OpCode::OpBindMethod);
                            }
                            let end_jump = self.emit_jump(OpCode::OpJump);
                            self.patch_jump(null_jump);
                            self.emit(OpCode::OpPop);
                            self.emit(OpCode::OpPop);
                            self.emit(OpCode::OpPushNull);
                            self.patch_jump(end_jump);
                        } else {
                            self.emit_get_var(&mangled);
                            if mangled.starts_with("__extget_") {
                                self.emit(OpCode::OpSwap);
                                self.emit1(OpCode::OpCall, 1);
                            } else {
                                self.emit(OpCode::OpBindMethod);
                            }
                        }
                        return Ok(());
                    }
                }

                if !*computed && !*optional && self.class_field_layout_valid {
                    if matches!(object.as_ref(), Expr::This { .. }) {
                        if let Expr::Identifier { name: prop, .. } = property.as_ref() {
                            if let Some(slot) = self
                                .class_field_layout
                                .as_ref()
                                .and_then(|fl| fl.get(prop.as_str()))
                                .copied()
                            {
                                self.compile_expr(object)?;
                                self.emit1(OpCode::OpGetFixedField, slot as u16);
                                return Ok(());
                            }
                        }
                    }
                }
                self.compile_expr(object)?;
                if *optional {
                    self.emit(OpCode::OpDup);
                    self.emit(OpCode::OpIsNull);
                    let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                    self.emit(OpCode::OpPop);
                    if *computed {
                        self.compile_expr(property)?;
                        self.emit(OpCode::OpGetIndex);
                    } else {
                        let name = match property.as_ref() {
                            Expr::Identifier { name, .. } => name.clone(),
                            _ => {
                                return Err("non-identifier property in optional member expression"
                                    .to_owned())
                            }
                        };
                        let idx = self.add_str(&name);
                        let cs = self.alloc_cache_slot();
                        self.emit2(OpCode::OpGetPropertyMaybe, idx, cs);
                    }
                    let end_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(null_jump);
                    self.emit(OpCode::OpPop);
                    self.patch_jump(end_jump);
                } else if *computed {
                    self.compile_expr(property)?;
                    self.emit(OpCode::OpGetIndex);
                } else {
                    let name = match property.as_ref() {
                        Expr::Identifier { name, .. } => name.clone(),
                        _ => return Err("non-identifier property in member expression".to_owned()),
                    };
                    let idx = self.add_str(&name);
                    let cs = self.alloc_cache_slot();
                    self.emit2(OpCode::OpGetProperty, idx, cs);
                }
            }

            Expr::Call {
                callee,
                args,
                optional,
                range: call_range,
                ..
            } => {
                if self.try_lower_intrinsic(callee, args)? {
                    return Ok(());
                }

                if let Some(mangled) = self
                    .lookup_extension_call(call_range.start.offset)
                    .map(|s| s.to_owned())
                {
                    if let Expr::Member {
                        object,
                        optional: member_optional,
                        ..
                    } = callee.as_ref()
                    {
                        if *optional || *member_optional {
                            self.compile_expr(object)?;
                            self.emit(OpCode::OpDup);
                            self.emit(OpCode::OpIsNull);
                            let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                            self.emit(OpCode::OpPop);

                            self.emit_get_var(&mangled);
                            self.emit(OpCode::OpSwap);
                            let (arg_count, has_spread) = self.compile_args(args)?;
                            self.emit_call_opcode(arg_count + 1, has_spread);

                            let end_jump = self.emit_jump(OpCode::OpJump);
                            self.patch_jump(null_jump);
                            self.emit(OpCode::OpPop);
                            self.emit(OpCode::OpPop);
                            self.emit(OpCode::OpPushNull);
                            self.patch_jump(end_jump);
                        } else {
                            self.emit_get_var(&mangled);
                            self.compile_expr(object)?;
                            let (arg_count, has_spread) = self.compile_args(args)?;
                            self.emit_call_opcode(arg_count + 1, has_spread);
                        }
                        return Ok(());
                    }
                }

                let opt_base: Option<&Expr> = if *optional {
                    Some(callee.as_ref())
                } else if let Expr::Member {
                    object,
                    optional: true,
                    ..
                } = callee.as_ref()
                {
                    Some(object.as_ref())
                } else {
                    None
                };

                if let Some(base) = opt_base {
                    self.compile_expr(base)?;
                    self.emit(OpCode::OpDup);
                    self.emit(OpCode::OpIsNull);
                    let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                    self.emit(OpCode::OpPop);

                    if *optional {
                        let (arg_count, has_spread) = self.compile_args(args)?;
                        self.emit_call_opcode(arg_count, has_spread);
                    } else if let Expr::Member {
                        property, computed, ..
                    } = callee.as_ref()
                    {
                        if !computed {
                            if let Expr::Identifier { name, .. } = property.as_ref() {
                                if let Some(vtable) = &self.class_vtable_layout {
                                    if let Some(&idx) = vtable.get(name) {
                                        let (arg_count, has_spread) = self.compile_args(args)?;
                                        if !has_spread {
                                            self.emit2(
                                                OpCode::OpInvokeVirtual,
                                                idx as u16,
                                                arg_count,
                                            );
                                            let end_jump = self.emit_jump(OpCode::OpJump);
                                            self.patch_jump(null_jump);
                                            self.emit(OpCode::OpPop);
                                            self.emit(OpCode::OpPop);
                                            self.emit(OpCode::OpPushNull);
                                            self.patch_jump(end_jump);
                                            return Ok(());
                                        }
                                    }
                                }

                                let str_idx = self.add_str(name);
                                let cs = self.alloc_cache_slot();
                                self.emit2(OpCode::OpGetProperty, str_idx, cs);
                                let (arg_count, has_spread) = self.compile_args(args)?;
                                self.emit_call_opcode(arg_count, has_spread);
                            } else {
                                return Err(
                                    "non-identifier property in optional method call".to_owned()
                                );
                            }
                        } else {
                            self.compile_expr(property)?;
                            self.emit(OpCode::OpGetIndex);
                            let (arg_count, has_spread) = self.compile_args(args)?;
                            self.emit_call_opcode(arg_count, has_spread);
                        }
                    }

                    let end_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(null_jump);
                    self.emit(OpCode::OpPop);
                    self.emit(OpCode::OpPop);
                    self.emit(OpCode::OpPushNull);
                    self.patch_jump(end_jump);
                    return Ok(());
                }

                // super.method(args) — emit OpGetSuper "method" directly to bypass vtable
                if let Expr::Member {
                    object,
                    property,
                    computed,
                    ..
                } = callee.as_ref()
                {
                    if !computed {
                        if matches!(object.as_ref(), Expr::Super { .. }) {
                            if let Expr::Identifier { name, .. } = property.as_ref() {
                                let idx = self.add_str(name);
                                self.emit1(OpCode::OpGetSuper, idx);
                                let (arg_count, has_spread) = self.compile_args(args)?;
                                self.emit_call_opcode(arg_count, has_spread);
                                return Ok(());
                            }
                        }
                    }
                }

                if let Expr::Member {
                    object,
                    property,
                    computed,
                    ..
                } = callee.as_ref()
                {
                    if !computed {
                        if let Expr::Identifier { name, .. } = property.as_ref() {
                            if let Some(vtable) = &self.class_vtable_layout {
                                if let Some(&idx) = vtable.get(name) {
                                    self.compile_expr(object)?;
                                    let (arg_count, has_spread) = self.compile_args(args)?;
                                    if !has_spread {
                                        self.emit2(OpCode::OpInvokeVirtual, idx as u16, arg_count);
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }

                if let Expr::Super { .. } = callee.as_ref() {
                    let super_idx = self.add_str("super");
                    self.emit1(OpCode::OpGetSuper, super_idx);
                } else {
                    self.compile_expr(callee)?;
                }
                let (arg_count, has_spread) = self.compile_args(args)?;
                self.emit_call_opcode(arg_count, has_spread);
            }

            Expr::New { callee, args, .. } => {
                self.compile_expr(callee)?;
                let (arg_count, has_spread) = self.compile_args(args)?;
                self.emit_call_opcode(arg_count, has_spread);
            }

            Expr::Array { elements, .. } => {
                let has_spread = elements.iter().any(|e| matches!(e, ArrayEl::Spread(_)));
                if has_spread {
                    // incremental build: start with empty array, push/extend each element
                    self.emit1(OpCode::OpBuildArray, 0);
                    for el in elements {
                        match el {
                            ArrayEl::Hole => {
                                self.emit(OpCode::OpPushNull);
                                self.emit(OpCode::OpArrayPush);
                                self.emit(OpCode::OpPop); // discard length
                            }
                            ArrayEl::Expr(e) => {
                                self.compile_expr(e)?;
                                self.emit(OpCode::OpArrayPush);
                                self.emit(OpCode::OpPop); // discard length
                            }
                            ArrayEl::Spread(e) => {
                                self.compile_expr(e)?;
                                self.emit(OpCode::OpArrayExtend);
                            }
                        }
                    }
                } else {
                    let mut count = 0u16;
                    for el in elements {
                        match el {
                            ArrayEl::Hole => self.emit(OpCode::OpPushNull),
                            ArrayEl::Expr(e) => self.compile_expr(e)?,
                            ArrayEl::Spread(_) => unreachable!(),
                        }
                        count += 1;
                    }
                    self.emit1(OpCode::OpBuildArray, count);
                }
            }

            Expr::Object { properties, .. } => {
                let mut in_first_segment = true;
                let mut segment_count = 0u16;

                for prop in properties {
                    match prop {
                        ObjectProp::Property { key, value, .. } => {
                            match key {
                                PropKey::Identifier(s) | PropKey::Str(s) => {
                                    let idx = self.add_str(s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                }
                                PropKey::Int(n) => {
                                    let s = n.to_string();
                                    let idx = self.add_str(&s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                }
                                PropKey::Computed(e) => {
                                    self.compile_expr(e)?;
                                    self.emit(OpCode::OpToString);
                                }
                            }
                            self.compile_expr(value)?;
                            segment_count += 1;
                        }
                        ObjectProp::Method {
                            key,
                            params,
                            body,
                            is_async,
                            is_generator,
                            ..
                        } => {
                            let key_str = match key {
                                PropKey::Identifier(s) | PropKey::Str(s) => {
                                    let idx = self.add_str(s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                    s.clone()
                                }
                                PropKey::Int(n) => {
                                    let s = n.to_string();
                                    let idx = self.add_str(&s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                    s
                                }
                                PropKey::Computed(e) => {
                                    self.compile_expr(e)?;
                                    self.emit(OpCode::OpToString);
                                    "<computed>".to_owned()
                                }
                            };
                            let (proto, upvalues) = super::compile_function_with_parent(
                                &key_str,
                                params,
                                body,
                                *is_async,
                                *is_generator,
                                true,
                                self,
                            )?;
                            self.emit_closure(proto, upvalues);
                            segment_count += 1;
                        }
                        ObjectProp::Spread { argument, .. } => {
                            if in_first_segment {
                                self.emit1(OpCode::OpBuildObject, segment_count);
                                in_first_segment = false;
                            } else if segment_count > 0 {
                                self.emit1(OpCode::OpBuildObject, segment_count);
                                self.emit(OpCode::OpObjectRest);
                            }
                            segment_count = 0;

                            self.compile_expr(argument)?;
                            self.emit(OpCode::OpObjectRest);
                        }
                        _ => {}
                    }
                }

                if in_first_segment {
                    self.emit1(OpCode::OpBuildObject, segment_count);
                } else if segment_count > 0 {
                    self.emit1(OpCode::OpBuildObject, segment_count);
                    self.emit(OpCode::OpObjectRest);
                }
            }

            Expr::Function {
                id,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                let name = id.clone().unwrap_or_else(|| "<anonymous>".to_owned());
                let (proto, upvalues) = super::compile_function_with_parent(
                    &name,
                    params,
                    body,
                    *is_async,
                    *is_generator,
                    false,
                    self,
                )?;
                self.emit_closure(proto, upvalues);
            }

            Expr::Arrow {
                params,
                body,
                is_async,
                ..
            } => {
                let stmt_body = match body.as_ref() {
                    ArrowBody::Block(s) => s.clone(),
                    ArrowBody::Expr(e) => Stmt::Return {
                        argument: Some(Box::new(e.clone())),
                        range: e.range().clone(),
                    },
                };
                let (proto, upvalues) = super::compile_function_with_parent(
                    "<arrow>", params, &stmt_body, *is_async, false, false, self,
                )?;
                self.emit_closure(proto, upvalues);
            }

            Expr::Await { argument, .. } => {
                self.compile_expr(argument)?;
                self.emit(OpCode::OpAwait);
            }

            Expr::Yield {
                argument,
                delegate: _,
                ..
            } => {
                if let Some(val) = argument {
                    self.compile_expr(val)?;
                } else {
                    self.emit(OpCode::OpPushNull);
                }
                self.emit(OpCode::OpYield);
            }

            Expr::Sequence { expressions, .. } => {
                for (i, e) in expressions.iter().enumerate() {
                    self.compile_expr(e)?;
                    if i + 1 < expressions.len() {
                        self.emit(OpCode::OpPop);
                    }
                }
            }

            Expr::Paren { expression, .. } => self.compile_expr(expression)?,

            Expr::Template { parts, .. } => {
                let mut count = 0u16;
                for part in parts {
                    match part {
                        TemplatePart::Literal(s) => {
                            let idx = self.add_str(s);
                            self.emit1(OpCode::OpPushConst, idx);
                        }
                        TemplatePart::Interpolation(e) => {
                            self.compile_expr(e)?;
                            self.emit(OpCode::OpToString);
                        }
                    }
                    count += 1;
                }
                if count > 0 {
                    for _ in 1..count {
                        self.emit(OpCode::OpStrConcat);
                    }
                } else {
                    let idx = self.add_str("");
                    self.emit1(OpCode::OpPushConst, idx);
                }
            }

            Expr::As { expression, .. } => {
                self.compile_expr(expression)?;
            }
            Expr::Satisfies { expression, .. } => {
                self.compile_expr(expression)?;
            }

            Expr::NonNull { expression, .. } => {
                self.compile_expr(expression)?;
                self.emit(OpCode::OpAssertNotNull);
            }

            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                self.compile_expr(start)?;
                self.compile_expr(end)?;
                let flag = if *inclusive { 1u16 } else { 0u16 };
                let method = self.add_str("__range__");
                self.emit2(OpCode::OpInvokeRuntimeStatic, method, flag);
            }

            Expr::Pipeline { left, right, .. } => {
                if pipeline_has_placeholder(right) {
                    let range = SourceRange::default();
                    let param = Param {
                        pattern: Pattern::Identifier {
                            name: "_".to_owned(),
                            type_ann: None,
                            range: range.clone(),
                        },
                        type_ann: None,
                        default: None,
                        is_rest: false,
                        is_optional: false,
                        modifiers: Modifiers::default(),
                        range: range.clone(),
                    };
                    let body = Stmt::Return {
                        argument: Some(Box::new(right.as_ref().clone())),
                        range,
                    };
                    let (proto, upvalues) = super::compile_function_with_parent(
                        "<pipe>",
                        &[param],
                        &body,
                        false,
                        false,
                        false,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.compile_expr(left)?;
                    self.emit1(OpCode::OpCall, 1);
                } else {
                    self.compile_expr(right)?;
                    self.compile_expr(left)?;
                    self.emit1(OpCode::OpCall, 1);
                }
            }

            Expr::ClassExpr { declaration, .. } => {
                self.compile_class_decl(declaration)?;
            }

            Expr::Spread { argument, .. } => {
                self.compile_expr(argument)?;
            }

            Expr::Match { subject, cases, .. } => {
                self.compile_expr(subject)?;
                let mut end_jumps = vec![];
                for case in cases {
                    let locals_before = self.scope.local_count();

                    let skip = match &case.pattern {
                        tsn_core::ast::MatchPattern::Wildcard => {
                            self.scope.declare_local("__match_subj__");
                            None
                        }
                        tsn_core::ast::MatchPattern::Literal(lit) => {
                            self.emit(OpCode::OpDup);
                            self.compile_expr(lit)?;
                            self.emit(OpCode::OpEq);
                            let j = self.emit_jump(OpCode::OpJumpIfFalse);
                            self.emit(OpCode::OpPop);

                            self.scope.declare_local("__match_subj__");
                            Some(j)
                        }
                        tsn_core::ast::MatchPattern::Identifier(name) => {
                            self.scope.declare_local(name.as_str());
                            None
                        }
                        tsn_core::ast::MatchPattern::Record { fields, .. } => {
                            let variant_name = fields.first().and_then(|(key, sub)| {
                                if key == "__variant__" {
                                    if let Some(tsn_core::ast::MatchPattern::Identifier(n)) = sub {
                                        return Some(n.as_str());
                                    }
                                }
                                None
                            });

                            if let Some(vname) = variant_name {
                                self.emit(OpCode::OpDup);
                                let tag_key = self.add_str("__tag");
                                let cs = self.alloc_cache_slot();
                                self.emit2(OpCode::OpGetProperty, tag_key, cs);
                                let vname_idx = self.add_str(vname);
                                self.emit1(OpCode::OpPushConst, vname_idx);
                                self.emit(OpCode::OpEq);
                                let j = self.emit_jump(OpCode::OpJumpIfFalse);
                                self.emit(OpCode::OpPop);

                                let subj_slot = self.scope.declare_local("__match_subj__");

                                for (field_name, sub_pat) in fields.iter().skip(1) {
                                    self.emit1(OpCode::OpGetLocal, subj_slot);
                                    let fkey = self.add_str(field_name);
                                    let cs2 = self.alloc_cache_slot();
                                    self.emit2(OpCode::OpGetProperty, fkey, cs2);

                                    let binding = match sub_pat {
                                        Some(tsn_core::ast::MatchPattern::Identifier(n)) => {
                                            n.as_str()
                                        }
                                        _ => field_name.as_str(),
                                    };
                                    self.scope.declare_local(binding);
                                }

                                Some(j)
                            } else {
                                let subj_slot = self.scope.declare_local("__match_subj__");
                                for (field_name, sub_pat) in fields {
                                    self.emit1(OpCode::OpGetLocal, subj_slot);
                                    let fkey = self.add_str(field_name);
                                    let cs = self.alloc_cache_slot();
                                    self.emit2(OpCode::OpGetProperty, fkey, cs);
                                    let binding = match sub_pat {
                                        Some(tsn_core::ast::MatchPattern::Identifier(n)) => {
                                            n.as_str()
                                        }
                                        _ => field_name.as_str(),
                                    };
                                    self.scope.declare_local(binding);
                                }
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(guard) = &case.guard {
                        self.compile_expr(guard)?;
                        let gj = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        let s2 = self.emit_jump(OpCode::OpJump);
                        self.patch_jump(gj);
                        self.emit(OpCode::OpPop);
                        let _ = s2;
                    }

                    match &case.body {
                        tsn_core::ast::MatchBody::Block(s) => self.compile_stmt(s)?,
                        tsn_core::ast::MatchBody::Expr(e) => self.compile_expr(e)?,
                    }

                    let locals_added = self.scope.local_count() - locals_before;

                    let fields_to_pop = if locals_added > 1 {
                        locals_added - 1
                    } else {
                        0
                    };
                    for _ in 0..fields_to_pop {
                        self.emit(OpCode::OpSwap);
                        self.emit(OpCode::OpPop);
                    }

                    self.scope.locals.truncate(locals_before);

                    let end = self.emit_jump(OpCode::OpJump);
                    end_jumps.push(end);

                    if let Some(s) = skip {
                        self.patch_jump(s);
                        self.emit(OpCode::OpPop);
                    }
                }
                self.emit(OpCode::OpPushNull);
                for j in end_jumps {
                    self.patch_jump(j);
                }
                self.emit(OpCode::OpSwap);
                self.emit(OpCode::OpPop);
            }

            Expr::TaggedTemplate { tag, template, .. } => {
                self.compile_expr(tag)?;
                self.compile_expr(template)?;
                self.emit1(OpCode::OpCall, 1);
            }
        }
        Ok(())
    }
}

fn pipeline_has_placeholder(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier { name, .. } => name == "_",
        Expr::Call { callee, args, .. } => {
            use tsn_core::ast::Arg;
            pipeline_has_placeholder(callee)
                || args.iter().any(|a| match a {
                    Arg::Positional(e) | Arg::Spread(e) => pipeline_has_placeholder(e),
                    Arg::Named { value, .. } => pipeline_has_placeholder(value),
                })
        }
        Expr::Member { object, .. } => pipeline_has_placeholder(object),
        Expr::Paren { expression, .. } => pipeline_has_placeholder(expression),
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            pipeline_has_placeholder(left) || pipeline_has_placeholder(right)
        }
        Expr::Unary { operand, .. } => pipeline_has_placeholder(operand),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            pipeline_has_placeholder(test)
                || pipeline_has_placeholder(consequent)
                || pipeline_has_placeholder(alternate)
        }
        Expr::Array { elements, .. } => {
            use tsn_core::ast::ArrayEl;
            elements.iter().any(|el| match el {
                ArrayEl::Expr(e) | ArrayEl::Spread(e) => pipeline_has_placeholder(e),
                ArrayEl::Hole => false,
            })
        }
        _ => false,
    }
}
