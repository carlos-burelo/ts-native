use super::Compiler;
use tsn_core::ast::Expr;
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_expr_member_call(&mut self, expr: &Expr) -> Result<bool, String> {
        match expr {
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
                        return Ok(true);
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
                                return Ok(true);
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
                    return Ok(true);
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
                        return Ok(true);
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
                                            return Ok(true);
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
                    return Ok(true);
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
                                return Ok(true);
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
                                        return Ok(true);
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
            _ => return Ok(false),
        }
        Ok(true)
    }
}
