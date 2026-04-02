use crate::ParseProfile;
use tsn_core::{Diagnostic, SourceRange, Token, TokenKind};

pub struct TokenStream {
    tokens: Vec<Token>,
    pos: usize,
    pub filename: String,

    pending_doc: Option<String>,

    pub errors: Vec<Diagnostic>,
    pub profile: ParseProfile,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>, filename: String) -> Self {
        TokenStream {
            tokens,
            pos: 0,
            filename,
            pending_doc: None,
            errors: Vec::new(),
            profile: ParseProfile::default(),
        }
    }

    pub fn push_error(&mut self, message: String, range: SourceRange) {
        self.errors.push(Diagnostic::error(message, range));
    }

    #[inline]
    pub fn kind(&self) -> TokenKind {
        self.token().kind
    }

    #[inline]
    pub fn peek_kind(&self, offset: usize) -> TokenKind {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            self.tokens[idx].kind
        } else {
            TokenKind::EOF
        }
    }

    #[inline]
    pub fn token(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            self.tokens.last().expect("empty token stream")
        }
    }

    #[inline]
    pub fn range(&self) -> SourceRange {
        self.token().range.clone()
    }

    #[inline]
    pub fn lexeme(&self) -> &str {
        &self.token().lexeme
    }

    #[inline]
    pub fn check(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.kind() == TokenKind::EOF
    }

    pub fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    pub fn consume(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            t
        } else {
            self.tokens.last().cloned().expect("empty token stream")
        }
    }

    pub fn consume_lexeme(&mut self) -> String {
        let lexeme = self.lexeme().to_owned();
        self.advance();
        lexeme
    }

    pub fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            let tok = self.token();
            Err(format!(
                "Expected {:?}, got {:?} ({:?}) at {}:{}",
                kind, tok.kind, tok.lexeme, tok.range.start.line, tok.range.start.column
            ))
        }
    }

    pub fn expect_lexeme(&mut self, kind: TokenKind) -> Result<String, String> {
        if self.check(kind) {
            Ok(self.consume_lexeme())
        } else {
            let tok = self.token();
            Err(format!(
                "Expected {:?}, got {:?} ({:?}) at {}:{}",
                kind, tok.kind, tok.lexeme, tok.range.start.line, tok.range.start.column
            ))
        }
    }

    pub fn expect_token(&mut self, kind: TokenKind) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.consume())
        } else {
            let tok = self.token();
            Err(format!(
                "Expected {:?}, got {:?} ({:?}) at {}:{}",
                kind, tok.kind, tok.lexeme, tok.range.start.line, tok.range.start.column
            ))
        }
    }

    pub fn expect_id(&mut self) -> Result<String, String> {
        if self.kind().can_be_identifier() {
            Ok(self.consume_lexeme())
        } else {
            let tok = self.token();
            Err(format!(
                "Expected Identifier, got {:?} ({:?}) at {}:{}",
                tok.kind, tok.lexeme, tok.range.start.line, tok.range.start.column
            ))
        }
    }

    pub fn peek_expect(&self, kind: TokenKind) -> Result<(), String> {
        if self.check(kind) {
            Ok(())
        } else {
            let tok = self.token();
            Err(format!(
                "Expected {:?}, got {:?} at {}:{}",
                kind, tok.kind, tok.range.start.line, tok.range.start.column
            ))
        }
    }

    pub fn line(&self) -> u32 {
        self.token().range.start.line
    }

    pub fn column(&self) -> u32 {
        self.token().range.start.column
    }

    pub fn eat_semicolon(&mut self) {
        self.eat(TokenKind::Semicolon);
    }

    /// Start-line of the last consumed token. Returns 0 if nothing consumed yet.
    pub fn prev_line(&self) -> u32 {
        if self.pos > 0 {
            self.tokens[self.pos - 1].range.start.line
        } else {
            0
        }
    }

    pub fn is_arrow_ahead(&self) -> bool {
        self.check(TokenKind::FatArrow)
    }

    pub fn save(&self) -> usize {
        self.pos
    }

    pub fn restore(&mut self, p: usize) {
        self.pos = p;
    }

    pub fn store_pending_doc(&mut self, doc: String) {
        self.pending_doc = Some(doc);
    }

    pub fn take_pending_doc(&mut self) -> Option<String> {
        self.pending_doc.take()
    }
}
