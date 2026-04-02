use super::class::member_key_name;
use crate::expressions::parse_expr;
use crate::stream::TokenStream;
use crate::types::{parse_type, parse_type_params};
use tsn_core::ast::{
    Decl, EnumDecl, EnumMember, InterfaceDecl, InterfaceMember, NamespaceDecl, Stmt, StructDecl,
    StructField, SumField, SumTypeDecl, SumVariant, TypeAliasDecl,
};
use tsn_core::TokenKind;

pub fn parse_interface_decl(s: &mut TokenStream) -> Result<InterfaceDecl, String> {
    let range = s.range();
    s.expect(TokenKind::Interface)?;
    let id = s.expect_lexeme(TokenKind::Identifier)?;
    let type_params = if s.check(TokenKind::LAngle) {
        parse_type_params(s)?
    } else {
        vec![]
    };
    let extends = if s.eat(TokenKind::Extends) {
        let mut e = vec![parse_type(s)?];
        while s.eat(TokenKind::Comma) {
            e.push(parse_type(s)?);
        }
        e
    } else {
        vec![]
    };

    s.expect(TokenKind::LBrace)?;
    let mut body = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        s.eat(TokenKind::Semicolon);
        if s.check(TokenKind::RBrace) {
            break;
        }
        body.push(parse_interface_member(s)?);
    }
    s.expect(TokenKind::RBrace)?;

    Ok(InterfaceDecl {
        id,
        type_params,
        extends,
        body,
        doc: None,
        range,
    })
}

pub fn parse_interface_member(s: &mut TokenStream) -> Result<InterfaceMember, String> {
    let mem_range = s.range();
    let readonly = s.eat(TokenKind::Readonly);

    if s.check(TokenKind::LBracket) {
        s.advance();
        let param = super::super::patterns::parse_single_param(s)?;
        s.expect(TokenKind::RBracket)?;
        s.expect(TokenKind::Colon)?;
        let return_type = parse_type(s)?;
        s.eat(TokenKind::Semicolon);
        return Ok(InterfaceMember::Index {
            param,
            return_type,
            range: mem_range,
        });
    }
    if s.check(TokenKind::LParen) {
        let params = super::super::patterns::parse_params(s)?;
        s.expect(TokenKind::Colon)?;
        let return_type = parse_type(s)?;
        s.eat(TokenKind::Semicolon);
        return Ok(InterfaceMember::Callable {
            params,
            return_type,
            range: mem_range,
        });
    }

    let key = member_key_name(s)?;
    let optional = s.eat(TokenKind::Question);

    if s.check(TokenKind::LParen) || s.check(TokenKind::LAngle) {
        let type_params = if s.check(TokenKind::LAngle) {
            parse_type_params(s)?
        } else {
            vec![]
        };
        let params = super::super::patterns::parse_params(s)?;
        let return_type = if s.eat(TokenKind::Colon) {
            Some(parse_type(s)?)
        } else {
            None
        };
        s.eat(TokenKind::Semicolon);
        Ok(InterfaceMember::Method {
            key,
            type_params,
            params,
            return_type,
            optional,
            range: mem_range,
        })
    } else {
        s.expect(TokenKind::Colon)?;
        let type_ann = parse_type(s)?;
        s.eat(TokenKind::Semicolon);
        Ok(InterfaceMember::Property {
            key,
            type_ann,
            optional,
            readonly,
            range: mem_range,
        })
    }
}

/// Parses `type Name<T> = | Variant1(f: T) | Variant2` and returns `Decl::SumType`.
pub fn parse_sum_type_or_alias(s: &mut TokenStream) -> Result<Decl, String> {
    let range = s.range();
    s.expect(TokenKind::Type)?;
    let id = s.expect_lexeme(TokenKind::Identifier)?;
    let type_params = if s.check(TokenKind::LAngle) {
        parse_type_params(s)?
    } else {
        vec![]
    };
    s.expect(TokenKind::Eq)?;

    // Leading `|` → sum type
    if s.check(TokenKind::Pipe) {
        let decl = parse_sum_type_body(id, type_params, range, s)?;
        s.eat_semicolon();
        return Ok(Decl::SumType(decl));
    }

    // Regular type alias
    let alias = parse_type(s)?;
    s.eat_semicolon();
    Ok(Decl::TypeAlias(TypeAliasDecl {
        id,
        type_params,
        alias,
        doc: None,
        range,
    }))
}

fn parse_sum_type_body(
    id: String,
    type_params: Vec<tsn_core::ast::TypeParam>,
    range: tsn_core::source::SourceRange,
    s: &mut TokenStream,
) -> Result<SumTypeDecl, String> {
    let mut variants = Vec::new();

    while s.check(TokenKind::Pipe) {
        s.advance(); // eat |
        let vrange = s.range();
        let vname = s.expect_lexeme(TokenKind::Identifier)?;

        let mut fields = Vec::new();
        if s.check(TokenKind::LParen) {
            s.advance(); // eat (
            while !s.check(TokenKind::RParen) && !s.is_eof() {
                let fname = s.expect_lexeme(TokenKind::Identifier)?;
                s.expect(TokenKind::Colon)?;
                let fty = parse_type(s)?;
                fields.push(SumField {
                    name: fname,
                    ty: fty,
                });
                if s.check(TokenKind::Comma) {
                    s.advance();
                }
            }
            s.expect(TokenKind::RParen)?;
        }

        variants.push(SumVariant {
            name: vname,
            fields,
            range: vrange,
        });
    }

    Ok(SumTypeDecl {
        id,
        type_params,
        variants,
        doc: None,
        range,
    })
}

pub fn parse_enum_decl(s: &mut TokenStream) -> Result<EnumDecl, String> {
    let range = s.range();
    s.expect(TokenKind::Enum)?;
    let id = s.expect_lexeme(TokenKind::Identifier)?;
    s.expect(TokenKind::LBrace)?;
    let mut members = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        let mem_range = s.range();
        let name = s.expect_lexeme(TokenKind::Identifier)?;
        let init = if s.eat(TokenKind::Eq) {
            Some(parse_expr(s)?)
        } else {
            None
        };
        members.push(EnumMember {
            id: name,
            init,
            range: mem_range,
        });
        if !s.eat(TokenKind::Comma) {
            break;
        }
    }
    s.expect(TokenKind::RBrace)?;
    Ok(EnumDecl {
        id,
        members,
        doc: None,
        range,
    })
}

pub fn parse_namespace_decl(s: &mut TokenStream) -> Result<NamespaceDecl, String> {
    let range = s.range();
    s.advance();
    let id = s.expect_lexeme(TokenKind::Identifier)?;
    s.expect(TokenKind::LBrace)?;
    let mut body = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        while s.eat(TokenKind::Semicolon) {}
        if s.check(TokenKind::RBrace) {
            break;
        }
        let stmt = super::super::stmts::parse_stmt_or_decl_inner(s)?;
        if let Stmt::Decl(d) = stmt {
            body.push(*d);
        }
    }
    s.expect(TokenKind::RBrace)?;
    Ok(NamespaceDecl {
        id,
        body,
        doc: None,
        range,
    })
}

pub fn parse_struct_decl(s: &mut TokenStream) -> Result<StructDecl, String> {
    let range = s.range();
    s.expect(TokenKind::Struct)?;
    let id = s.expect_lexeme(TokenKind::Identifier)?;
    s.expect(TokenKind::LBrace)?;
    let mut fields = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        while s.eat(TokenKind::Semicolon) {}
        if s.check(TokenKind::RBrace) {
            break;
        }
        let field_range = s.range();
        let name = s.expect_lexeme(TokenKind::Identifier)?;
        s.expect(TokenKind::Colon)?;
        let type_ann = parse_type(s)?;
        let default = if s.eat(TokenKind::Eq) {
            Some(parse_expr(s)?)
        } else {
            None
        };
        fields.push(StructField {
            name,
            type_ann,
            default,
            range: field_range,
        });
        s.eat(TokenKind::Comma);
    }
    s.expect(TokenKind::RBrace)?;
    Ok(StructDecl {
        id,
        fields,
        doc: None,
        range,
    })
}
