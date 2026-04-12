mod class;
mod modules;
pub mod type_decls;

pub use class::parse_class_decl;
pub(super) use modules::{parse_export_decl, parse_import_decl};
pub(super) use type_decls::{
    parse_enum_decl, parse_interface_decl, parse_namespace_decl, parse_struct_decl,
    parse_sum_type_or_alias,
};

pub(super) use self::extension::parse_extension_decl;
mod extension;

use crate::expressions::parse_expr;
use crate::stream::TokenStream;
use crate::types::{parse_type, parse_type_params};
use tsn_core::ast::{
    Decorator, FunctionDecl, Modifiers, Pattern, VarDeclarator, VarKind, VariableDecl,
};
use tsn_core::TokenKind;

pub(super) fn parse_var_decl_with_declare(
    s: &mut TokenStream,
    is_declare: bool,
) -> Result<VariableDecl, String> {
    let range = s.range();
    let kind = match s.kind() {
        TokenKind::Let => {
            s.advance();
            VarKind::Let
        }
        TokenKind::Const => {
            s.advance();
            VarKind::Const
        }
        _ => return Err("`var` is not supported; use `let` or `const`".to_owned()),
    };

    let dec_range = s.range();
    let id = super::patterns::parse_pattern(s)?;
    parse_var_decl_after_head(s, range, kind, dec_range, id, is_declare)
}

pub(super) fn parse_var_decl_after_head(
    s: &mut TokenStream,
    range: tsn_core::SourceRange,
    kind: VarKind,
    first_range: tsn_core::SourceRange,
    first_id: Pattern,
    is_declare: bool,
) -> Result<VariableDecl, String> {
    let mut declarators = vec![parse_var_declarator_suffix(
        s,
        first_range,
        first_id,
        is_declare,
    )?];

    while s.eat(TokenKind::Comma) {
        let dec_range = s.range();
        let id = super::patterns::parse_pattern(s)?;
        declarators.push(parse_var_declarator_suffix(s, dec_range, id, is_declare)?);
    }

    Ok(VariableDecl {
        kind,
        declarators,
        is_declare,
        doc: None,
        range,
    })
}

fn parse_var_declarator_suffix(
    s: &mut TokenStream,
    range: tsn_core::SourceRange,
    id: Pattern,
    is_declare: bool,
) -> Result<VarDeclarator, String> {
    let type_ann = if s.eat(TokenKind::Colon) {
        Some(parse_type(s)?)
    } else {
        None
    };
    let init = if s.eat(TokenKind::Eq) {
        if is_declare {
            return Err("declare variables cannot have initializers".to_owned());
        }
        Some(parse_expr(s)?)
    } else {
        None
    };

    Ok(VarDeclarator {
        id,
        type_ann,
        init,
        range,
    })
}

pub(super) fn parse_function_decl(
    s: &mut TokenStream,
    decorators: Vec<Decorator>,
    is_async_pre: bool,
    is_declare: bool,
) -> Result<FunctionDecl, String> {
    let range = s.range();
    s.expect(TokenKind::Function)?;
    let is_generator = s.eat(TokenKind::Star);

    let id_offset = s.range().start.offset;
    let id = s.expect_id()?;
    let type_params = if s.check(TokenKind::LAngle) {
        parse_type_params(s)?
    } else {
        vec![]
    };
    let params = super::patterns::parse_params(s)?;
    let return_type = if s.eat(TokenKind::Colon) {
        Some(parse_type(s)?)
    } else {
        None
    };
    let body = if is_declare {
        if s.check(TokenKind::LBrace) {
            return Err("declare function cannot have a body".to_owned());
        }
        s.eat_semicolon();
        tsn_core::ast::Stmt::Empty { range: s.range() }
    } else {
        super::stmts::parse_block(s)?
    };

    Ok(FunctionDecl {
        id,
        id_offset,
        type_params,
        params,
        return_type,
        body,
        modifiers: Modifiers {
            is_async: is_async_pre,
            is_generator,
            is_declare,
            ..Default::default()
        },
        decorators,
        doc: None,
        range,
    })
}
