use super::primary::parse_primary_expr;
use super::{parse_assign_expr, parse_expr};
use crate::stream::TokenStream;
use crate::types::parse_type_args;
use tsn_core::ast::{Arg, Expr, UnaryOp, UpdateOp};
use tsn_core::SourceRange;
use tsn_core::TokenKind;

pub fn parse_unary_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let range = s.range();

    macro_rules! prefix_unary {
        ($op:expr) => {{
            s.advance();
            let o = parse_unary_expr(s)?;
            Ok(Expr::Unary {
                op: $op,
                prefix: true,
                operand: Box::new(o),
                range,
            })
        }};
    }

    match s.kind() {
        TokenKind::Bang => prefix_unary!(UnaryOp::Not),
        TokenKind::Tilde => prefix_unary!(UnaryOp::BitNot),
        TokenKind::Minus => prefix_unary!(UnaryOp::Minus),
        TokenKind::Plus => prefix_unary!(UnaryOp::Plus),
        TokenKind::Typeof => prefix_unary!(UnaryOp::Typeof),
        TokenKind::Void => {
            return Err("`void` is not supported; use `_` to discard values".to_owned())
        }
        TokenKind::Delete => return Err("`delete` is not supported".to_owned()),
        TokenKind::Await => {
            s.advance();
            let argument = parse_unary_expr(s)?;
            Ok(Expr::Await {
                argument: Box::new(argument),
                range,
            })
        }
        TokenKind::PlusPlus => {
            s.advance();
            let o = parse_unary_expr(s)?;
            Ok(Expr::Update {
                op: UpdateOp::Increment,
                prefix: true,
                operand: Box::new(o),
                range,
            })
        }
        TokenKind::MinusMinus => {
            s.advance();
            let o = parse_unary_expr(s)?;
            Ok(Expr::Update {
                op: UpdateOp::Decrement,
                prefix: true,
                operand: Box::new(o),
                range,
            })
        }
        _ => parse_postfix_expr(s),
    }
}

fn parse_postfix_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let mut expr = parse_call_expr(s)?;

    loop {
        let range = s.range();
        match s.kind() {
            TokenKind::PlusPlus => {
                s.advance();
                expr = Expr::Update {
                    op: UpdateOp::Increment,
                    prefix: false,
                    operand: Box::new(expr),
                    range,
                };
            }
            TokenKind::MinusMinus => {
                s.advance();
                expr = Expr::Update {
                    op: UpdateOp::Decrement,
                    prefix: false,
                    operand: Box::new(expr),
                    range,
                };
            }
            _ => break,
        }
    }

    Ok(expr)
}

pub fn parse_new_callee_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let mut expr = parse_primary_expr(s)?;
    loop {
        match s.kind() {
            TokenKind::Dot => {
                s.advance();
                let tok = s.consume();
                let prop_range = tok.range.clone();
                let start_range = *expr.range();
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: Box::new(Expr::Identifier {
                        name: tok.lexeme.to_string(),
                        range: prop_range.clone(),
                    }),
                    computed: false,
                    optional: false,
                    range: start_range.to(prop_range),
                };
            }
            TokenKind::LBracket => {
                s.advance();
                let idx = parse_expr(s)?;
                let bracket_tok = s.expect_token(TokenKind::RBracket)?;
                let start_range = *expr.range();
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: Box::new(idx),
                    computed: true,
                    optional: false,
                    range: start_range.to(bracket_tok.range),
                };
            }
            _ => break,
        }
    }
    Ok(expr)
}

fn parse_call_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let mut expr = parse_primary_expr(s)?;

    loop {
        match s.kind() {
            TokenKind::Dot => {
                s.advance();
                let prop_tok = s.consume();
                let prop_range = prop_tok.range.clone();
                let start_range = *expr.range();
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: Box::new(Expr::Identifier {
                        name: prop_tok.lexeme.to_string(),
                        range: prop_range.clone(),
                    }),
                    computed: false,
                    optional: false,
                    range: start_range.to(prop_range),
                };
            }
            TokenKind::QuestionDot => {
                s.advance();
                if s.check(TokenKind::LBracket) {
                    s.advance();
                    let idx = parse_expr(s)?;
                    let bracket_tok = s.expect_token(TokenKind::RBracket)?;
                    let start_range = *expr.range();
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property: Box::new(idx),
                        computed: true,
                        optional: true,
                        range: start_range.to(bracket_tok.range),
                    };
                } else if s.check(TokenKind::LParen) {
                    let (type_args, args, call_range) = parse_call_args(s)?;
                    let start_range = *expr.range();
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        type_args,
                        args,
                        optional: true,
                        range: start_range.to(call_range),
                    };
                } else {
                    let prop_tok = s.consume();
                    let prop_range = prop_tok.range.clone();
                    let start_range = *expr.range();
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property: Box::new(Expr::Identifier {
                            name: prop_tok.lexeme.to_string(),
                            range: prop_range.clone(),
                        }),
                        computed: false,
                        optional: true,
                        range: start_range.to(prop_range),
                    };
                }
            }
            TokenKind::LBracket => {
                // If '[' is on a new line, treat as a new statement (ASI).
                if s.line() > s.prev_line() {
                    break;
                }
                s.advance();
                let idx = parse_expr(s)?;
                let bracket_tok = s.expect_token(TokenKind::RBracket)?;
                let start_range = *expr.range();
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: Box::new(idx),
                    computed: true,
                    optional: false,
                    range: start_range.to(bracket_tok.range),
                };
            }
            TokenKind::QuestionLBracket => {
                s.advance();
                let idx = parse_expr(s)?;
                let bracket_tok = s.expect_token(TokenKind::RBracket)?;
                let start_range = *expr.range();
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: Box::new(idx),
                    computed: true,
                    optional: true,
                    range: start_range.to(bracket_tok.range),
                };
            }
            TokenKind::LParen => {
                let (type_args, args, call_range) = parse_call_args(s)?;
                let start_range = *expr.range();
                expr = Expr::Call {
                    callee: Box::new(expr),
                    type_args,
                    args,
                    optional: false,
                    range: start_range.to(call_range),
                };
            }
            TokenKind::LAngle => {
                if !looks_like_generic_call(s) {
                    break;
                }
                let save = s.save();
                let start_range = *expr.range();
                match try_parse_generic_call(s, expr.clone(), start_range) {
                    Ok(call) => {
                        expr = call;
                    }
                    Err(_) => {
                        s.restore(save);
                        break;
                    }
                }
            }
            // Non-null assertion: expr! — stays in the loop so expr!.foo, expr!?.foo, etc. chain.
            TokenKind::Bang => {
                let range = s.range();
                s.advance();
                expr = Expr::NonNull {
                    expression: Box::new(expr),
                    range,
                };
            }
            _ => break,
        }
    }

    Ok(expr)
}

fn looks_like_generic_call(s: &TokenStream) -> bool {
    let mut depth = 0i32;
    let mut off = 0usize;
    while off < 32 {
        match s.peek_kind(off) {
            TokenKind::LAngle => depth += 1,
            TokenKind::RAngle => {
                depth -= 1;
                if depth == 0 {
                    return s.peek_kind(off + 1) == TokenKind::LParen;
                }
            }
            TokenKind::EOF
            | TokenKind::Semicolon
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::Eq
            | TokenKind::FatArrow => return false,
            _ => {}
        }
        off += 1;
    }
    false
}

fn try_parse_generic_call(
    s: &mut TokenStream,
    callee: Expr,
    expr_range: SourceRange,
) -> Result<Expr, String> {
    let type_args = parse_type_args(s)?;
    if !s.check(TokenKind::LParen) {
        return Err("not a generic call".to_owned());
    }
    let (_, args, call_range) = parse_call_args(s)?;
    Ok(Expr::Call {
        callee: Box::new(callee),
        type_args,
        args,
        optional: false,
        range: expr_range.to(call_range),
    })
}

pub fn parse_call_args(
    s: &mut TokenStream,
) -> Result<(Vec<tsn_core::ast::TypeNode>, Vec<Arg>, SourceRange), String> {
    s.expect(TokenKind::LParen)?;
    let mut args = vec![];
    while !s.check(TokenKind::RParen) && !s.is_eof() {
        if s.check(TokenKind::DotDotDot) {
            s.advance();
            args.push(Arg::Spread(parse_assign_expr(s)?));
        } else if s.check(TokenKind::Identifier) && s.peek_kind(1) == TokenKind::Colon {
            let label = s.consume_lexeme();
            s.advance();
            args.push(Arg::Named {
                label,
                value: parse_assign_expr(s)?,
            });
        } else {
            args.push(Arg::Positional(parse_assign_expr(s)?));
        }
        if !s.eat(TokenKind::Comma) {
            break;
        }
    }
    let rparen = s.expect_token(TokenKind::RParen)?;
    Ok((vec![], args, rparen.range))
}

pub fn parse_call_args_pub(
    s: &mut TokenStream,
) -> Result<(Vec<tsn_core::ast::TypeNode>, Vec<Arg>, SourceRange), String> {
    parse_call_args(s)
}
