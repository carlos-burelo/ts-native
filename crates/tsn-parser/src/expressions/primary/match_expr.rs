use crate::stream::TokenStream;
use tsn_core::ast::{Expr, MatchBody, MatchCase, MatchPattern};
use tsn_core::TokenKind;

pub(super) fn parse_match_expr(s: &mut TokenStream) -> Result<Expr, String> {
    let range = s.range();
    s.advance();

    let subject = if s.eat(TokenKind::LParen) {
        let e = super::super::parse_expr(s)?;
        s.expect(TokenKind::RParen)?;
        e
    } else {
        super::super::parse_binary_expr(s, super::super::Prec::None)?
    };

    s.expect(TokenKind::LBrace)?;
    let mut cases = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        // parse_match_case can expand OR-patterns into multiple cases
        cases.extend(parse_match_case(s)?);
    }
    s.expect(TokenKind::RBrace)?;
    Ok(Expr::Match {
        subject: Box::new(subject),
        cases,
        range,
    })
}

/// Returns one or more cases (OR-patterns expand to multiple cases sharing body/guard).
fn parse_match_case(s: &mut TokenStream) -> Result<Vec<MatchCase>, String> {
    let range = s.range();

    // Collect all OR'd patterns: `P1 | P2 | P3`
    let mut patterns = vec![parse_match_pattern(s)?];
    while s.eat(TokenKind::Pipe) {
        patterns.push(parse_match_pattern(s)?);
    }

    let guard = if s.eat(TokenKind::If) {
        Some(super::super::parse_expr(s)?)
    } else {
        None
    };
    s.expect(TokenKind::FatArrow)?;
    let body = if s.check(TokenKind::LBrace) {
        MatchBody::Block(crate::parser::parse_block(s)?)
    } else {
        MatchBody::Expr(super::super::parse_expr(s)?)
    };
    s.eat(TokenKind::Comma);

    // Expand: one case per OR'd pattern, each sharing the same body and guard.
    let cases = patterns
        .into_iter()
        .map(|pattern| MatchCase {
            pattern,
            guard: guard.clone(),
            body: body.clone(),
            range,
        })
        .collect();
    Ok(cases)
}

fn parse_match_pattern(s: &mut TokenStream) -> Result<MatchPattern, String> {
    match s.kind() {
        TokenKind::Placeholder => {
            s.advance();
            Ok(MatchPattern::Wildcard)
        }
        TokenKind::Identifier => parse_identifier_match_pattern(s),
        _ => {
            let expr = super::parse_primary_expr(s)?;
            Ok(MatchPattern::Literal(expr))
        }
    }
}

fn parse_identifier_match_pattern(s: &mut TokenStream) -> Result<MatchPattern, String> {
    let id_range = s.range(); // capture range before consuming
    let name = s.consume_lexeme();
    if s.check(TokenKind::LParen) {
        return parse_variant_tuple_pattern(s, name);
    }
    if s.check(TokenKind::LBrace) {
        return parse_variant_record_pattern(s, name);
    }
    // If followed by '.' parse as a member expression (e.g. Direction.North).
    if s.check(TokenKind::Dot) {
        let id_expr = Expr::Identifier {
            name: name.clone(),
            range: id_range,
        };
        s.advance(); // consume '.'
        let prop_tok = s.consume();
        let prop_range = prop_tok.range.clone();
        let expr = Expr::Member {
            object: Box::new(id_expr),
            property: Box::new(Expr::Identifier {
                name: prop_tok.lexeme.to_string(),
                range: prop_range.clone(),
            }),
            computed: false,
            optional: false,
            range: id_range.to(prop_range),
        };
        return Ok(MatchPattern::Literal(expr));
    }
    Ok(MatchPattern::Identifier(name))
}

fn parse_variant_tuple_pattern(s: &mut TokenStream, name: String) -> Result<MatchPattern, String> {
    s.advance();
    let mut fields: Vec<(String, Option<MatchPattern>)> = Vec::new();
    while !s.check(TokenKind::RParen) && !s.is_eof() {
        if s.check(TokenKind::Identifier) {
            let field_name = s.consume_lexeme();
            let sub_pattern = if s.eat(TokenKind::Colon) {
                Some(parse_match_pattern(s)?)
            } else {
                None
            };
            fields.push((field_name, sub_pattern));
        }
        if s.check(TokenKind::Comma) {
            s.advance();
        }
    }
    s.expect(TokenKind::RParen)?;
    let mut all_fields = vec![(
        "__variant__".to_owned(),
        Some(MatchPattern::Identifier(name)),
    )];
    all_fields.extend(fields);
    Ok(MatchPattern::Record {
        fields: all_fields,
        rest: false,
    })
}

fn parse_variant_record_pattern(s: &mut TokenStream, name: String) -> Result<MatchPattern, String> {
    s.advance();
    let mut fields: Vec<(String, Option<MatchPattern>)> = Vec::new();
    let mut rest = false;
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        if s.check(TokenKind::DotDotDot) {
            s.advance();
            rest = true;
            break;
        }
        if s.check(TokenKind::Identifier) {
            let field_name = s.consume_lexeme();
            let sub_pattern = if s.eat(TokenKind::Colon) {
                Some(parse_match_pattern(s)?)
            } else {
                None
            };
            fields.push((field_name, sub_pattern));
        }
        if s.check(TokenKind::Comma) {
            s.advance();
        }
    }
    s.expect(TokenKind::RBrace)?;
    let mut all_fields = vec![(
        "__variant__".to_owned(),
        Some(MatchPattern::Identifier(name)),
    )];
    all_fields.extend(fields);
    Ok(MatchPattern::Record {
        fields: all_fields,
        rest,
    })
}
