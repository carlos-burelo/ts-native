mod match_expr;
mod object;
mod template;

use super::helpers::{parse_int_radix, split_regex, unescape_string};
use super::{parse_call_args, parse_seq_expr, try_parse_arrow};
use crate::stream::TokenStream;
use crate::types::parse_type_args;
use tsn_core::ast::{ArrayEl, Expr};
use tsn_core::TokenKind;

use self::match_expr::parse_match_expr;
use self::object::parse_object_expr;
use self::template::parse_template;

pub fn parse_primary_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let range = s.range();

    match s.kind() {
        TokenKind::IntegerLiteral
        | TokenKind::BinaryLiteral
        | TokenKind::OctalLiteral
        | TokenKind::HexLiteral => {
            let raw = s.consume_lexeme();
            let value: i64 = parse_int_radix(&raw).unwrap_or(0);
            Ok(Expr::IntLiteral { value, raw, range })
        }
        TokenKind::FloatLiteral => {
            let raw = s.consume_lexeme();
            let value: f64 = raw.replace('_', "").parse().unwrap_or(0.0);
            Ok(Expr::FloatLiteral { value, raw, range })
        }
        TokenKind::BigIntLiteral => Ok(Expr::BigIntLiteral {
            raw: s.consume_lexeme(),
            range,
        }),
        TokenKind::DecimalLiteral => Ok(Expr::DecimalLiteral {
            raw: s.consume_lexeme(),
            range,
        }),
        TokenKind::Str => {
            let value = unescape_string(s.lexeme());
            s.advance();
            Ok(Expr::StrLiteral { value, range })
        }
        TokenKind::Char => {
            let ch = unescape_string(s.lexeme()).chars().next().unwrap_or('\0');
            s.advance();
            Ok(Expr::CharLiteral { value: ch, range })
        }
        TokenKind::True => {
            s.advance();
            Ok(Expr::BoolLiteral { value: true, range })
        }
        TokenKind::False => {
            s.advance();
            Ok(Expr::BoolLiteral {
                value: false,
                range,
            })
        }
        TokenKind::Null => {
            s.advance();
            Ok(Expr::NullLiteral { range })
        }
        TokenKind::RegularExpression => {
            let raw = s.consume_lexeme();
            let (pattern, flags) = split_regex(&raw);
            Ok(Expr::RegexLiteral {
                pattern,
                flags,
                range,
            })
        }

        TokenKind::Template | TokenKind::TemplateHead => parse_template(s),

        TokenKind::Identifier => Ok(Expr::Identifier {
            name: s.consume_lexeme(),
            range,
        }),
        TokenKind::Placeholder => {
            s.advance();
            Ok(Expr::Identifier {
                name: "_".to_owned(),
                range,
            })
        }

        TokenKind::This => {
            s.advance();
            Ok(Expr::This { range })
        }
        TokenKind::Super => {
            s.advance();
            Ok(Expr::Super { range })
        }

        TokenKind::LBracket => parse_array_expr(s),
        TokenKind::LBrace => parse_object_expr(s),

        TokenKind::LParen => {
            s.advance();
            if s.check(TokenKind::RParen) {
                s.restore(s.save() - 1);
                return Err("unit paren — should be handled by arrow parser".to_owned());
            }
            let expr = parse_seq_expr(s)?;
            s.expect(TokenKind::RParen)?;
            Ok(Expr::Paren {
                expression: Box::new(expr),
                range,
            })
        }

        TokenKind::New => parse_new_expr(s, range),
        TokenKind::Function => parse_function_expr(s),

        TokenKind::Async => {
            let save = s.save();
            if let Ok(Some(arrow)) = try_parse_arrow(s) {
                return Ok(arrow);
            }
            s.restore(save);
            s.advance();
            parse_function_expr_inner(s, true)
        }

        TokenKind::Class => parse_class_expr(s),
        TokenKind::Match => parse_match_expr(s),

        _ => {
            let name = s.lexeme().to_owned();
            let kind = s.kind();
            if kind.can_be_identifier() {
                s.advance();
                Ok(Expr::Identifier { name, range })
            } else {
                Err(format!(
                    "Unexpected token {:?} ({:?}) in expression at {}:{}",
                    s.kind(),
                    s.lexeme(),
                    s.line(),
                    s.column()
                ))
            }
        }
    }
}

fn parse_array_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let range = s.range();
    s.advance();
    let mut elements = vec![];

    while !s.check(TokenKind::RBracket) && !s.is_eof() {
        if s.check(TokenKind::Comma) {
            elements.push(ArrayEl::Hole);
            s.advance();
            continue;
        }
        if s.check(TokenKind::DotDotDot) {
            s.advance();
            elements.push(ArrayEl::Spread(super::parse_assign_expr(s)?));
        } else {
            elements.push(ArrayEl::Expr(super::parse_assign_expr(s)?));
        }
        s.eat(TokenKind::Comma);
    }

    s.expect(TokenKind::RBracket)?;
    Ok(Expr::Array { elements, range })
}

fn parse_new_expr(s: &mut TokenStream, range: tsn_core::SourceRange) -> Result<Expr, String> {
    s.advance();
    let callee = super::parse_new_callee_expr(s)?;
    let mut type_args = vec![];
    if s.check(TokenKind::LAngle) {
        let save = s.save();
        match parse_type_args(s) {
            Ok(ta) if s.check(TokenKind::LParen) => {
                type_args = ta;
            }
            _ => {
                s.restore(save);
            }
        }
    }
    let args = if s.check(TokenKind::LParen) {
        let (_, a, _) = parse_call_args(s)?;
        a
    } else {
        vec![]
    };
    Ok(Expr::New {
        callee: Box::new(callee),
        type_args,
        args,
        range,
    })
}

fn parse_function_expr(s: &mut TokenStream) -> Result<Expr, String> {
    s.advance();
    parse_function_expr_inner(s, false)
}

fn parse_function_expr_inner(s: &mut TokenStream, is_async: bool) -> Result<Expr, String> {
    let range = s.range();
    let is_generator = s.eat(TokenKind::Star);
    let id = if s.check(TokenKind::Identifier) {
        Some(s.consume_lexeme())
    } else {
        None
    };
    let params = crate::parser::parse_params(s)?;
    let return_type = if s.eat(TokenKind::Colon) {
        Some(crate::types::parse_type(s)?)
    } else {
        None
    };
    let body = crate::parser::parse_block(s)?;
    Ok(Expr::Function {
        id,
        params,
        return_type,
        body: Box::new(body),
        is_async,
        is_generator,
        range,
    })
}

fn parse_class_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let range = s.range();
    let decl = crate::parser::parse_class_decl(s, vec![], false)?;
    Ok(Expr::ClassExpr {
        declaration: Box::new(decl),
        range,
    })
}
