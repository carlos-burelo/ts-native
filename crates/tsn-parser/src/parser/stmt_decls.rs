use crate::stream::TokenStream;
use tsn_core::ast::{Decl, Stmt};
use tsn_core::TokenKind;

pub(super) fn try_parse_decl_stmt(
    s: &mut TokenStream,
    kind: TokenKind,
    next_kind: TokenKind,
    decorators: Vec<tsn_core::ast::Decorator>,
) -> Option<Result<Stmt, String>> {
    if kind == TokenKind::Declare {
        s.advance();
        return try_parse_decl_stmt_mode(s, s.kind(), s.peek_kind(1), decorators, true);
    }

    try_parse_decl_stmt_mode(s, kind, next_kind, decorators, false)
}

pub(super) fn try_parse_decl_stmt_mode(
    s: &mut TokenStream,
    kind: TokenKind,
    next_kind: TokenKind,
    decorators: Vec<tsn_core::ast::Decorator>,
    is_declare: bool,
) -> Option<Result<Stmt, String>> {
    let result = match kind {
        TokenKind::Function => {
            let mut decl = match super::decls::parse_function_decl(s, decorators, false, is_declare)
            {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Function(decl))))
        }
        TokenKind::Async if next_kind == TokenKind::Function => {
            s.advance();
            let mut decl = match super::decls::parse_function_decl(s, decorators, true, is_declare)
            {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Function(decl))))
        }
        TokenKind::Class | TokenKind::Abstract => {
            let mut decl = match super::decls::parse_class_decl(s, decorators, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Class(decl))))
        }
        TokenKind::Interface => {
            let mut decl = match super::decls::parse_interface_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Interface(decl))))
        }
        TokenKind::Type => {
            let mut decl = match super::decls::parse_sum_type_or_alias(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            match &mut decl {
                Decl::TypeAlias(d) => d.doc = s.take_pending_doc(),
                Decl::SumType(d) => d.doc = s.take_pending_doc(),
                _ => {}
            }
            Ok(Stmt::Decl(Box::new(decl)))
        }
        TokenKind::Enum => {
            let mut decl = match super::decls::parse_enum_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Enum(decl))))
        }
        TokenKind::Namespace | TokenKind::Module => {
            let mut decl = match super::decls::parse_namespace_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Namespace(decl))))
        }
        TokenKind::Struct => {
            let mut decl = match super::decls::parse_struct_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            Ok(Stmt::Decl(Box::new(Decl::Struct(decl))))
        }
        TokenKind::Extension => {
            let decl = match super::decls::parse_extension_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            Ok(Stmt::Decl(Box::new(Decl::Extension(decl))))
        }
        TokenKind::Let | TokenKind::Const | TokenKind::Var => {
            let mut decl = match super::decls::parse_var_decl_with_declare(s, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.doc = s.take_pending_doc();
            s.eat_semicolon();
            Ok(Stmt::Decl(Box::new(Decl::Variable(decl))))
        }
        TokenKind::Import => {
            let decl = match super::decls::parse_import_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let _ = s.take_pending_doc();
            s.eat_semicolon();
            Ok(Stmt::Decl(Box::new(Decl::Import(decl))))
        }
        TokenKind::Export => {
            let decl = match super::decls::parse_export_decl(s, decorators) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let _ = s.take_pending_doc();
            s.eat_semicolon();
            Ok(Stmt::Decl(Box::new(Decl::Export(decl))))
        }
        _ => return None,
    };

    Some(result)
}
