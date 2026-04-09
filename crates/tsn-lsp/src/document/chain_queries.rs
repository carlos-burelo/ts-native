use tsn_checker::{types::FunctionType, types::ObjectTypeMember, SymbolKind, Type};
use tsn_core::{TokenKind, TypeKind};

use super::{ChainResult, DocumentState, MemberKind, MemberRecord};

impl DocumentState {
    pub fn resolve_chain_at(&self, line: u32, col: u32) -> Option<ChainResult> {
        let ident_idx = self.tokens.iter().position(|t| {
            t.line == line
                && (t.kind == TokenKind::Identifier || t.kind.can_be_identifier())
                && t.col <= col
                && col < t.col + t.length
        })?;

        let mut chain: Vec<usize> = vec![ident_idx];
        let mut curr = ident_idx;
        // Type derived from a `new Expr()` or call expression before a `.` (rparen-anchored).
        let mut rparen_base_type: Option<Type> = None;

        while curr >= 2 && self.tokens[curr - 1].kind == TokenKind::Dot {
            let prev = &self.tokens[curr - 2];
            if prev.kind == TokenKind::Identifier || prev.kind.can_be_identifier() {
                curr -= 2;
                chain.insert(0, curr);
            } else if prev.kind == TokenKind::RParen {
                // e.g., `new Builder().method` or `foo().method`
                // The type of the expression ending at `)` is in expr_types.
                rparen_base_type = self.expr_types.get(&prev.offset).map(|i| i.ty.clone());
                break;
            } else {
                break;
            }
        }

        let base_ident_idx = chain[0];
        let base_ident = &self.tokens[base_ident_idx];
        let mut current_type = if let Some(ty) = rparen_base_type {
            ty
        } else if let Some(sym) = self.symbol_at_pos(base_ident.line, base_ident.col) {
            if base_ident_idx == ident_idx && chain.len() == 1 {
                return Some(ChainResult::Symbol(sym));
            }

            if sym.ty.is_dynamic()
                && matches!(
                    sym.kind,
                    SymbolKind::Class
                        | SymbolKind::Interface
                        | SymbolKind::Namespace
                        | SymbolKind::Enum
                )
            {
                Type(TypeKind::Named(sym.name.clone(), None))
            } else {
                sym.ty.clone()
            }
        } else if base_ident.kind == TokenKind::This {
            self.symbols
                .iter()
                .filter(|s| {
                    !s.is_from_stdlib
                        && matches!(s.kind, SymbolKind::Class | SymbolKind::Interface)
                        && s.line <= base_ident.line
                })
                .max_by_key(|s| s.line)
                .map(|s| Type(TypeKind::Named(s.name.clone(), None)))
                .unwrap_or(Type::Dynamic)
        } else if let Some(prim) = literal_primitive_type(base_ident.kind) {
            Type(TypeKind::Named(prim.to_owned(), None))
        } else {
            Type::Dynamic
        };

        let mut parent_name = match &current_type.0 {
            TypeKind::Named(n, _) if base_ident.kind != TokenKind::This => n.clone(),
            TypeKind::Generic(n, _, _) if base_ident.kind != TokenKind::This => n.clone(),
            _ if base_ident.kind == TokenKind::This => self
                .symbols
                .iter()
                .filter(|s| {
                    !s.is_from_stdlib
                        && matches!(s.kind, SymbolKind::Class | SymbolKind::Interface)
                        && s.line <= base_ident.line
                })
                .max_by_key(|s| s.line)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "this".to_owned()),
            _ => literal_primitive_type(base_ident.kind)
                .map(str::to_owned)
                .unwrap_or_else(|| base_ident.lexeme.clone()),
        };

        if !current_type.is_dynamic() {
            for i in 1..chain.len() {
                let idx = chain[i];
                let member_name = &self.tokens[idx].lexeme;
                let found_member = match &current_type.0 {
                    TypeKind::Object(members) => members
                        .iter()
                        .find(|m| match m {
                            ObjectTypeMember::Property { name, .. } => name == member_name,
                            ObjectTypeMember::Method { name, .. } => name == member_name,
                            _ => false,
                        })
                        .map(|m| match m {
                            ObjectTypeMember::Property { ty, .. } => ty.clone(),
                            ObjectTypeMember::Method { return_type, .. } => (**return_type).clone(),
                            _ => Type::Dynamic,
                        }),
                    TypeKind::Named(name, _origin) => self
                        .find_named_or_extension_member(name, member_name)
                        .map(|m| m.ty.clone()),
                    _ => None,
                };

                if let Some(ty) = found_member {
                    if idx == ident_idx {
                        if let Some(res) = self.find_chain_result(&current_type, member_name) {
                            match res {
                                ChainResult::Member { member, .. } => {
                                    return Some(ChainResult::Member {
                                        member,
                                        parent_name: parent_name.clone(),
                                    });
                                }
                                _ => return Some(res),
                            }
                        }
                    }
                    parent_name = member_name.clone();
                    current_type = ty;
                } else {
                    break;
                }
            }
        }

        let target_ident = &self.tokens[ident_idx];
        if let Some(info) = self.expr_types.get(&target_ident.offset) {
            let ty = &info.ty;
            return Some(ChainResult::DynamicMember {
                member: MemberRecord {
                    name: target_ident.lexeme.clone(),
                    type_str: match &ty.0 {
                        TypeKind::Fn(FunctionType {
                            return_type,
                            is_arrow: false,
                            ..
                        }) => return_type.to_string(),
                        _ => ty.to_string(),
                    },
                    params_str: format_type_params_str(ty),
                    is_static: false,
                    is_optional: false,
                    kind: if matches!(&ty.0, TypeKind::Fn(_)) {
                        MemberKind::Method
                    } else {
                        MemberKind::Property
                    },
                    is_arrow: if let TypeKind::Fn(FunctionType { is_arrow, .. }) = &ty.0 {
                        *is_arrow
                    } else {
                        false
                    },
                    line: target_ident.line,
                    col: target_ident.col,
                    init_value: String::new(),
                    ty: ty.clone(),
                    members: Vec::new(),
                },
                parent_name: parent_name.clone(),
            });
        }

        None
    }

    fn find_chain_result(&self, ty: &Type, name: &str) -> Option<ChainResult> {
        match &ty.0 {
            TypeKind::Named(type_name, _origin) => {
                if let Some(sym) = self.symbols.iter().find(|s| &s.name == type_name) {
                    if let Some(m) = self.find_member_recursive(&sym.members, name) {
                        return Some(ChainResult::Member {
                            member: m,
                            parent_name: sym.name.clone(),
                        });
                    }
                }
                if let Some(m) = self.find_extension_member(type_name, name) {
                    return Some(ChainResult::Member {
                        member: m,
                        parent_name: type_name.clone(),
                    });
                }
                if let Some(sym) = self.symbols.iter().find(|s| &s.name == name) {
                    return Some(ChainResult::Symbol(sym));
                }
                None
            }
            TypeKind::Object(_) => {
                for sym in &self.symbols {
                    if &sym.ty == ty {
                        if let Some(m) = self.find_member_recursive(&sym.members, name) {
                            return Some(ChainResult::Member {
                                member: m,
                                parent_name: sym.name.clone(),
                            });
                        }
                    }
                    if let Some(m) = self.find_member_with_type_recursive(&sym.members, ty, name) {
                        let parent_name = self
                            .find_direct_parent_name_recursive(&sym.members, m)
                            .unwrap_or(&sym.name)
                            .to_owned();
                        return Some(ChainResult::Member {
                            member: m,
                            parent_name,
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn find_direct_parent_name_recursive<'a>(
        &'a self,
        members: &'a [MemberRecord],
        target: &MemberRecord,
    ) -> Option<&'a str> {
        for m in members {
            if m.members.iter().any(|inner| std::ptr::eq(inner, target)) {
                return Some(&m.name);
            }
            if let Some(name) = self.find_direct_parent_name_recursive(&m.members, target) {
                return Some(name);
            }
        }
        None
    }

    pub(super) fn find_named_or_extension_member<'a>(
        &'a self,
        type_name: &str,
        member_name: &str,
    ) -> Option<&'a MemberRecord> {
        self.symbols
            .iter()
            .find(|s| s.name == type_name)
            .and_then(|sym| sym.members.iter().find(|m| m.name == member_name))
            .or_else(|| self.find_extension_member(type_name, member_name))
    }

    pub(super) fn find_extension_member<'a>(
        &'a self,
        type_name: &str,
        member_name: &str,
    ) -> Option<&'a MemberRecord> {
        self.extension_members
            .get(type_name)
            .and_then(|members| members.iter().find(|m| m.name == member_name))
    }

    fn find_member_recursive<'a>(
        &'a self,
        members: &'a [MemberRecord],
        name: &str,
    ) -> Option<&'a MemberRecord> {
        for m in members {
            if m.name == name {
                return Some(m);
            }
            if let Some(nested) = self.find_member_recursive(&m.members, name) {
                return Some(nested);
            }
        }
        None
    }

    fn find_member_with_type_recursive<'a>(
        &'a self,
        members: &'a [MemberRecord],
        parent_ty: &Type,
        name: &str,
    ) -> Option<&'a MemberRecord> {
        for m in members {
            if &m.ty == parent_ty {
                if let Some(found) = self.find_member_recursive(&m.members, name) {
                    return Some(found);
                }
            }
            if let Some(found) = self.find_member_with_type_recursive(&m.members, parent_ty, name) {
                return Some(found);
            }
        }
        None
    }

    pub fn member_at_pos(
        &self,
        line: u32,
        col: u32,
    ) -> Option<(String, SymbolKind, &MemberRecord)> {
        let tok = self.tokens.iter().find(|t| {
            t.line == line
                && (t.kind == TokenKind::Identifier || t.kind.can_be_identifier())
                && t.col <= col
                && col < t.col + t.length
        })?;

        for sym in &self.symbols {
            if let Some(member) =
                self.find_member_at_pos_recursive(&sym.members, line, tok.col, &tok.lexeme)
            {
                let parent_name = self
                    .find_direct_parent_name_recursive(&sym.members, member)
                    .unwrap_or(&sym.name)
                    .to_owned();
                return Some((parent_name, sym.kind, member));
            }
        }

        for (type_name, members) in &self.extension_members {
            if let Some(member) =
                self.find_member_at_pos_recursive(members, line, tok.col, &tok.lexeme)
            {
                return Some((type_name.clone(), SymbolKind::Extension, member));
            }
        }
        None
    }

    fn find_member_at_pos_recursive<'a>(
        &'a self,
        members: &'a [MemberRecord],
        line: u32,
        col: u32,
        name: &str,
    ) -> Option<&'a MemberRecord> {
        for m in members {
            if m.line == line && m.name == name && m.col <= col {
                return Some(m);
            }
            if let Some(found) = self.find_member_at_pos_recursive(&m.members, line, col, name) {
                return Some(found);
            }
        }
        None
    }
}

fn format_type_params_str(ty: &Type) -> String {
    if let TypeKind::Fn(FunctionType { params, .. }) = &ty.0 {
        let mut out = String::new();
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if let Some(name) = &p.name {
                out.push_str(name);
            } else {
                out.push_str(&format!("arg{i}"));
            }
            if p.optional {
                out.push('?');
            }
            out.push_str(": ");
            out.push_str(&format!("{}", p.ty));
        }
        out
    } else {
        String::new()
    }
}

fn literal_primitive_type(kind: TokenKind) -> Option<&'static str> {
    use tsn_core::well_known as wk;
    match kind {
        TokenKind::Str => Some(wk::STR),
        TokenKind::Char => Some(wk::CHAR),
        TokenKind::IntegerLiteral
        | TokenKind::HexLiteral
        | TokenKind::BinaryLiteral
        | TokenKind::OctalLiteral => Some(wk::INT),
        TokenKind::FloatLiteral => Some(wk::FLOAT),
        TokenKind::DecimalLiteral => Some(wk::DECIMAL),
        TokenKind::BigIntLiteral => Some(wk::BIGINT),
        TokenKind::True | TokenKind::False => Some(wk::BOOL),
        _ => None,
    }
}
