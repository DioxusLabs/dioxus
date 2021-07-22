//! Parse the various css types from strings directly (avoid pulling in syn if working at runtime)
//!
//! Differences to spec:
//!  - Exponential floats are not supported for now.
use std::{char, fmt, iter};

const REPLACEMENT_CHAR: char = '�';

#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive] // Don't allow user to create
pub struct Span {
    /// Inclusive
    start: usize,
    /// Exclusive
    end: usize,
}

impl Span {
    fn new(start: usize, end: usize) -> Self {
        assert!(end > start, "end must be greater than start");
        Span { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

#[derive(Debug)]
pub struct InvalidChar {
    ch: char,
    pos: usize,
}

impl fmt::Display for InvalidChar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "invalid character `{}` found at position {}",
            self.ch.escape_debug(),
            self.pos
        )
    }
}

#[derive(Debug)]
pub struct Lexer<'src> {
    src: &'src str,
    cursor: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Result<Lexer<'src>, InvalidChar> {
        // Check that the user has already replaced characters as specified at
        // https://www.w3.org/TR/css-syntax-3/#input-preprocessing
        for (pos, ch) in src.char_indices() {
            if ch == '\r' || ch == '\u{d}' || ch == '\0' {
                return Err(InvalidChar { ch, pos });
            }
        }
        Ok(Lexer { src, cursor: 0 })
    }

    fn len(&self) -> usize {
        self.src.len()
    }

    fn remaining(&self) -> usize {
        self.src.len() - self.cursor
    }

    pub fn next_token(&mut self) -> Option<Token> {
        match self.peek() {
            Some(token) => {
                self.consume(&token);
                Some(token)
            }
            None => None,
        }
    }

    pub fn peek(&self) -> Option<Token> {
        // https://www.w3.org/TR/css-syntax-3/#tokenizer-definitions
        if let Some(comment) = self.comment() {
            return Some(comment);
        }
        if let Some(tok) = self.whitespace() {
            return Some(tok);
        }
        if let Some(tok) = self.string() {
            return Some(tok);
        }
        match self.chars().next() {
            Some(other) => Some(Token::new(
                TokenKind::Error,
                Span::new(self.cursor, self.cursor + other.len_utf8()),
            )),
            None => None,
        }
    }

    pub fn peek_n(&self, n: usize) -> Option<Token> {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!() //self.peek().is_none()
    }

    pub fn resolve_span(&self, span: Span) -> &'src str {
        if span.end > self.len() {
            panic!("End of requested span is past the end of the source");
        }
        &self.src[span.start..span.end]
    }

    /// Create another independent lexer at the given start point
    fn fork(&self) -> Lexer {
        Lexer {
            src: self.src,
            cursor: self.cursor,
        }
    }

    pub fn consume(&mut self, tok: &Token) {
        assert!(
            tok.len() <= self.remaining(),
            "trying to consume a token that would be bigger \
            than all remaining text"
        );
        self.cursor += tok.len();
    }

    /// Resolve a position from cursor to position from start of src
    fn resolve_pos(&self, pos: usize) -> usize {
        self.cursor + pos
    }

    /// Create a span from the current position with the given length
    fn span(&self, len: usize) -> Span {
        debug_assert!(self.cursor + len <= self.len());
        Span::new(self.cursor, self.cursor + len)
    }

    /// Create a span from the current position to the end
    fn span_to_end(&self) -> Span {
        Span::new(self.cursor, self.len())
    }

    /// Iterate over the remaining chars of the input
    fn chars(&self) -> std::str::Chars {
        self.src[self.cursor..].chars()
    }

    /// Iterate over the remaining chars of the input
    fn char_indices(&self) -> std::str::CharIndices {
        self.src[self.cursor..].char_indices()
    }

    /// Parse a comment
    fn comment(&self) -> Option<Token> {
        let mut ch_iter = self.char_indices().peekable();
        if let Some((_, '/')) = ch_iter.next() {
            if let Some((_, '*')) = ch_iter.next() {
                loop {
                    match ch_iter.next() {
                        Some((_, '*')) => {
                            if let Some((idx, '/')) = ch_iter.peek() {
                                return Some(Token {
                                    kind: TokenKind::Comment,
                                    span: self.span(*idx + '/'.len_utf8()),
                                });
                            }
                        }
                        None => {
                            return Some(Token::new(
                                TokenKind::UnclosedComment,
                                self.span_to_end(),
                            ));
                        }
                        _ => (),
                    }
                }
            }
        }
        None
    }

    /// Parse whitespace
    fn whitespace(&self) -> Option<Token> {
        let mut ch_iter = self.chars();
        let mut len = match ch_iter.next() {
            Some(ch) if ch.is_ascii_whitespace() => ch.len_utf8(),
            _ => return None,
        };
        loop {
            match ch_iter.next() {
                Some(ch) if ch.is_ascii_whitespace() => len += ch.len_utf8(),
                _ => break,
            }
        }
        Some(Token {
            kind: TokenKind::Whitespace,
            span: self.span(len),
        })
    }

    /// Parse either a single or double quoted string
    fn string(&self) -> Option<Token> {
        let mut ch_iter = self.char_indices().fuse().peekable();
        let delim = match ch_iter.next() {
            Some((_, '"')) => '"',
            Some((_, '\'')) => '\'',
            _ => return None,
        };
        let mut decoded_string = String::new();
        loop {
            match ch_iter.next() {
                Some((end, ch)) if ch == delim => {
                    return Some(Token {
                        kind: TokenKind::String(decoded_string),
                        span: self.span(end + 1), // '"'.len_utf8() == 1
                    });
                }
                Some((end, '\n')) => {
                    return Some(Token {
                        kind: TokenKind::BadString(decoded_string),
                        span: self.span(end + 1), // '\n'.len_utf8() == 1
                    });
                }
                Some((_, '\\')) => match ch_iter.peek() {
                    Some((_, ch)) => {
                        if *ch == '\n' {
                            // do nothing - skip the backslash and newline.
                            ch_iter.next().unwrap();
                        } else if let Some(decoded_ch) = unescape(&mut ch_iter) {
                            decoded_string.push(decoded_ch);
                        } else {
                            decoded_string.push(ch_iter.next().unwrap().1);
                        }
                    }
                    None => {
                        // The spec says not to add the last '\'.
                        // a bad string will be returned on next pass
                        ch_iter.next().unwrap();
                    }
                },
                Some((_, ch)) => decoded_string.push(ch),
                None => {
                    return Some(Token {
                        kind: TokenKind::BadString(decoded_string),
                        span: self.span_to_end(),
                    })
                }
            }
        }
    }

    /*
    fn hash(&self) -> Option<Token> {
        let mut iter = self.char_indices();
        match iter.next() {
            Some((_, '#')) => (),
            None => return None,
        };
        match iter.next() {
            Some((_, '\\')) => {}
            _ => Some(Token {
                kind: TokenKind::Delim('#'),
                span: self.span(1),
            }),
        }
    }
    */
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    pub fn len(&self) -> usize {
        self.span.len()
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    Ident,
    Function,
    At,
    Hash,
    String(String),
    BadString(String),
    Url,
    BadUrl,
    Delim(char),
    Number,
    Percentage,
    Dimension,
    Whitespace,
    /// <!--
    CDO,
    /// -->
    CDC,
    /// :
    Colon,
    /// ;
    Semicolon,
    /// ,
    Comma,
    /// [
    LBracket,
    /// ]
    RBracket,
    /// (
    LParen,
    /// )
    RParen,
    /// {
    LBrace,
    /// }
    RBrace,
    Comment,
    UnclosedComment,
    /// Could not parse the next token
    Error,
}

// Helpers

/// Hex to char (up to 6 characters, e.g. "ffffff").
///
/// For example `"5c" => '\'`. Returns None if first char is not hex.  Consumes the hex values.
fn unescape(input: &mut iter::Peekable<impl Iterator<Item = (usize, char)>>) -> Option<char> {
    fn hex_acc(acc: &mut u32, next: char) {
        debug_assert!(*acc & 0xf0000000 == 0); // make sure we don't overflow
        (*acc) = (*acc << 4) + next.to_digit(16).unwrap()
    }

    let (_, ch) = match input.peek() {
        Some((idx, ch)) if ch.is_ascii_hexdigit() => input.next().unwrap(),
        _ => return None,
    };

    let mut acc = 0;
    let mut count = 0;
    hex_acc(&mut acc, ch);

    // Here we use that the length of all valid hexdigits in utf8 is 1.
    while count < 5
        && input
            .peek()
            .map(|(_, ch)| ch.is_ascii_hexdigit())
            .unwrap_or(false)
    {
        let ch = input.next().unwrap().1;
        hex_acc(&mut acc, ch);
        count += 1;
    }

    // consume a whitespace char if it's there
    if input
        .peek()
        .map(|(_, ch)| ch.is_ascii_whitespace())
        .unwrap_or(false)
    {
        input.next().unwrap();
    }

    // maybe we could just directly use `char::from_u32(acc).unwrap_or(REPLACEMENT_CHAR)`
    // null, surrogate, or too big
    Some(
        if acc == 0 || (acc >= 0xd800 && acc < 0xe000) || acc >= 0x110000 {
            REPLACEMENT_CHAR
        } else {
            char::from_u32(acc).unwrap() // there should be no other invalid chars.
        },
    )
}

#[cfg(test)]
mod test {
    use super::{Lexer, Span, Token, TokenKind};

    #[test]
    fn comment() {
        println!();
        let mut input = Lexer::new("/* a valid comment */").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::Comment,
                span,
            }) => {
                assert_eq!(
                    input.resolve_span(span),
                    "/* a valid comment */".to_string()
                );
                assert_eq!(span.len(), 21);
            }
            _ => panic!("not a comment"),
        };

        let mut input = Lexer::new("/* a comment").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::UnclosedComment,
                span,
            }) => {
                assert_eq!(input.resolve_span(span), "/* a comment".to_string());
                assert_eq!(span.len(), 12);
            }
            _ => panic!("not a comment"),
        };

        let mut input = Lexer::new("/!* not a comment").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::Error,
                span,
            }) => {}
            _ => panic!("not a comment"),
        };
    }

    #[test]
    fn string() {
        println!("h");
        let mut input = Lexer::new("\" a vali\\64\\e9 \\\n string \"").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::String(s),
                span,
            }) => {
                assert_eq!(s, " a validé string ".to_string());
                assert_eq!(span.len(), 26);
            }
            _ => panic!("not a string"),
        };

        let mut input = Lexer::new("' a valid string '").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::String(s),
                span,
            }) => {
                assert_eq!(s, " a valid string ".to_string());
                assert_eq!(span.len(), 18);
            }
            _ => panic!("not a string"),
        };

        let mut input = Lexer::new("\" a string").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::BadString(s),
                span,
            }) => {
                assert_eq!(s, " a string".to_string());
                assert_eq!(span.len(), 10);
            }
            _ => panic!("not a string"),
        };
    }

    #[test]
    fn whitespace() {
        println!();
        let mut input = Lexer::new("\n\t ").unwrap();
        match input.next_token() {
            Some(Token {
                kind: TokenKind::Whitespace,
                span,
            }) => {
                assert_eq!(input.resolve_span(span), "\n\t ".to_string());
                assert_eq!(span.len(), 3);
            }
            _ => panic!("not a string"),
        };
    }

    #[test]
    fn escape() {
        let mut iter = "e9".char_indices().peekable();
        assert_eq!(super::unescape(&mut iter), Some('é'));
    }
}
