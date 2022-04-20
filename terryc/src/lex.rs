use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use crate::{FileId, Input};
use crate::sym::Symbol;

#[derive(Clone, Copy)]
pub struct Ident {
    pub span: Span,
    pub symbol: Symbol,
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.symbol.eq(&other.symbol)
    }
}

impl Eq for Ident {}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.symbol.fmt(f)
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.symbol.fmt(f)
    }
}

impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.symbol.hash(state);
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
    UnclosedComment,
    InvalidFloat,
    InvalidInt,
}

#[derive(Debug)]
pub struct Error {
    line: u32,
    kind: ErrorKind,
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    RArrow,
    Comma,
    Colon,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Star,
    Not,
    NotEq,
    Eq,
    EqEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Slash,
    String(Symbol),
    Integer(u128),
//    Decimal(f64),
    Keyword(Ident),
    Ident(Ident),
    Eof,
}

#[salsa::query_group(LexStorage)]
pub trait Lex: Input {
    fn lex(&self, file: FileId) -> Result<Vec<Token>, ErrorReported>;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    lo: usize,
    hi: usize,
}

impl Span {
    pub fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }

    pub fn lo(&self) -> usize {
        self.lo
    }

    pub fn hi(&self) -> usize {
        self.hi
    }

    pub fn to(self, other: Span) -> Span {
        Span::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.lo..self.hi).fmt(f)
    }
}

impl ariadne::Span for Span {
    type SourceId = ();

    fn source(&self) -> &Self::SourceId {
        &()
    }

    fn start(&self) -> usize {
        self.lo
    }

    fn end(&self) -> usize {
        self.hi
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn dummy() -> Self {
        Token {
            kind: TokenKind::Dot,
            span: Span { lo: 0, hi: 0 },
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

pub struct Lexer<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: u32,
    has_errors: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ErrorReported;

impl Error {
    fn emit(self) {
        println!("{self:?}");
    }
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            has_errors: false,
        }
    }

    fn error(&mut self, kind: ErrorKind) {
        self.has_errors = true;
        Error {
            line: self.line,
            kind,
        }
        .emit()
    }

    fn is_end(&self) -> bool {
        self.current >= self.src.len()
    }

    fn char_at(&self, idx: usize) -> Option<char> {
        self.src.split_at(idx).1.chars().next()
    }

    fn peek(&self) -> Option<char> {
        self.char_at(self.current)
    }

    fn peek2(&self) -> Option<char> {
        self.char_at(self.current + 1)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        self.current += c.map_or(0, char::len_utf8);
        c
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() != Some(c) {
            return false;
        }
        self.current += c.len_utf8();
        true
    }

    fn string(&mut self) -> Option<TokenKind> {
        while let Some(c) = self.peek() {
            if c == '"' {
                break;
            }
            if c == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_end() {
            self.error(ErrorKind::UnterminatedString);
            return None;
        }

        self.advance();

        let s = &self.src[self.start + 1..self.current - 1];
        Some(TokenKind::String(s)) // TODO unescape
    }

    fn number(&mut self) -> Option<TokenKind> {
        while let Some(c) = self.peek() && c.is_ascii_digit() {
            self.advance();
        }

        let kind = /*if Some('.') == self.peek()
            && self.peek2().map(|c| c.is_ascii_digit()).unwrap_or_default()
        {
            self.advance();
            while let Some(c) = self.peek() && c.is_ascii_digit() {
                self.advance();
            }

            let s = &self.src[self.start..self.current];
            let Ok(num) = f64::from_str(s).map_err(|_| self.error(ErrorKind::InvalidFloat)) else { return None };
            TokenKind::Decimal(num)
        } else */{
            let s = &self.src[self.start..self.current];
            let Ok(num) = u128::from_str(s).map_err(|_| self.error(ErrorKind::InvalidInt)) else { return None };
            TokenKind::Integer(num)
        };

        Some(kind)
    }

    fn identifier(&mut self) -> TokenKind {
        while let Some(c) = self.peek() && c.is_ascii_alphanumeric() {
            self.advance();
        }

        let s = &self.src[self.start..self.current];
        let symbol = Symbol::new(s);
        let span = Span {
            lo: self.start,
            hi: self.current,
        };

        if symbol.is_keyword() {
            TokenKind::Keyword(Ident {
                symbol,
                span,
            })
        } else {
            TokenKind::Ident(Ident {
                symbol,
                span,
            })
        }
    }

    fn scan_token(&mut self) -> Option<TokenKind> {
        use TokenKind::*;

        let c = match self.advance() {
            Some(c) => c,
            None => return None,
        };

        let kind = match c {
            '(' => LeftParen,
            ')' => RightParen,
            '{' => LeftBrace,
            '}' => RightBrace,
            ',' => Comma,
            '.' => Dot,
            '-' if self.eat('>') => RArrow,
            '-' => Minus,
            '+' => Plus,
            ';' => Semicolon,
            '*' => Star,
            ':' => Colon,
            '!' if self.eat('=') => NotEq,
            '!' => Not,
            '=' if self.eat('=') => EqEq,
            '=' => Eq,
            '<' if self.eat('=') => LessEq,
            '<' => Less,
            '>' if self.eat('=') => GreaterEq,
            '>' => Greater,

            '/' if self.eat('/') => {
                while let Some(c) = self.peek() && c != '\n' {
                    self.advance();
                }
                return None;
            }

            '/' if self.eat('*') => {
                let mut nest = 1;

                while nest > 0 {
                    if self.is_end() {
                        self.error(ErrorKind::UnclosedComment);
                        return None;
                    }
                    while let Some(c) = self.peek() {
                        if c == '/' && self.eat('*') {
                            nest += 1;
                        } else if c == '*' && self.eat('/') {
                            nest -= 1;
                            break;
                        }

                        self.advance();
                    }
                }

                return None;
            }

            '/' => Slash,

            // ignore whitespace.
            ' ' | '\r' | '\t' | '\n' => return None,

            '"' => return self.string(),

            c if c.is_ascii_digit() => return self.number(),
            c if c.is_ascii_alphabetic() || c == '_' => self.identifier(),

            c => {
                self.error(ErrorKind::UnexpectedCharacter(c));
                return None;
            }
        };

        Some(kind)
    }

    pub fn scan_tokens(mut self) -> Result<Vec<Token>, ErrorReported> {
        while !self.is_end() {
            self.start = self.current;
            let Some(kind) = self.scan_token() else { continue };
            let span = Span {
                lo: self.start,
                hi: self.current,
            };
            self.tokens.push(Token { kind, span })
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                lo: self.current,
                hi: self.current,
            },
        });

        if self.has_errors {
            Err(ErrorReported)
        } else {
            Ok(self.tokens)
        }
    }
}

fn lex(gcx: &dyn Lex, file: FileId) -> Result<Vec<Token>, ErrorReported> {
    let Some(src) = gcx.get_file(file) else { return Err(ErrorReported); };
    let mut lexer = Lexer::new(&src);
    lexer.scan_tokens()
}
