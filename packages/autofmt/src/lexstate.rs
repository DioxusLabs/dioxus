//! A tiny line-oriented lexer that tracks whether a position in rust source is inside a
//! multiline string literal or block comment.
//!
//! The formatter applies several line-based indentation transforms to expression bodies.
//! Lines that begin inside a multiline string literal must be left completely untouched,
//! otherwise the contents of the string drift right on every format pass.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexState {
    #[default]
    Normal,
    /// Inside a normal `"..."` (or `b"..."`) string
    Str,
    /// Inside a raw string literal with the given number of `#`s
    RawStr(usize),
    /// Inside (possibly nested) `/* ... */` block comments
    BlockComment(usize),
}

impl LexState {
    pub fn is_in_string(&self) -> bool {
        matches!(self, LexState::Str | LexState::RawStr(_))
    }

    /// Advance the lex state through a single line of source (without its trailing newline)
    pub fn advance(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match *self {
                LexState::Normal => match bytes[i] {
                    b'"' => {
                        *self = LexState::Str;
                        i += 1;
                    }
                    b'r' | b'b' => {
                        // Don't treat the `r` in `for` or `b` in `web` as a literal prefix
                        let prev_is_ident = i > 0
                            && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
                        if prev_is_ident {
                            i += 1;
                            continue;
                        }

                        let mut j = i + 1;
                        if bytes[i] == b'b' && j < bytes.len() && bytes[j] == b'r' {
                            j += 1;
                        }
                        let is_raw = j > i + 1 || bytes[i] == b'r';
                        let mut hashes = 0;
                        while j < bytes.len() && bytes[j] == b'#' {
                            hashes += 1;
                            j += 1;
                        }

                        if j < bytes.len() && bytes[j] == b'"' {
                            if is_raw {
                                *self = LexState::RawStr(hashes);
                            } else {
                                // b"..." - escapes work like a normal string
                                *self = LexState::Str;
                            }
                            i = j + 1;
                        } else {
                            i += 1;
                        }
                    }
                    b'\'' => {
                        // char literal or lifetime
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            // escaped char literal - skip to the closing quote
                            let mut j = i + 2;
                            while j < bytes.len() && bytes[j] != b'\'' {
                                j += 1;
                            }
                            i = j + 1;
                        } else if i + 2 < bytes.len() && bytes[i + 2] == b'\'' {
                            i += 3;
                        } else {
                            // a lifetime - skip just the quote so `'a"` still finds the quote
                            i += 1;
                        }
                    }
                    b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => return,
                    b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                        *self = LexState::BlockComment(1);
                        i += 2;
                    }
                    _ => i += 1,
                },
                LexState::Str => match bytes[i] {
                    b'\\' => i += 2,
                    b'"' => {
                        *self = LexState::Normal;
                        i += 1;
                    }
                    _ => i += 1,
                },
                LexState::RawStr(hashes) => {
                    if bytes[i] == b'"' {
                        let mut j = i + 1;
                        let mut seen = 0;
                        while seen < hashes && j < bytes.len() && bytes[j] == b'#' {
                            seen += 1;
                            j += 1;
                        }
                        if seen == hashes {
                            *self = LexState::Normal;
                            i = j;
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                LexState::BlockComment(depth) => {
                    if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        *self = if depth == 1 {
                            LexState::Normal
                        } else {
                            LexState::BlockComment(depth - 1)
                        };
                        i += 2;
                    } else if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        *self = LexState::BlockComment(depth + 1);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(src: &str) -> Vec<bool> {
        // for each line, whether the line *starts* inside a string
        let mut state = LexState::default();
        src.lines()
            .map(|line| {
                let starts_in_string = state.is_in_string();
                state.advance(line);
                starts_in_string
            })
            .collect()
    }

    #[test]
    fn tracks_raw_strings() {
        let src = "let x = r#\"\nhello\nworld\n\"#;\nlet y = 1;";
        assert_eq!(states(src), vec![false, true, true, true, false]);
    }

    #[test]
    fn tracks_normal_strings() {
        let src = "let x = \"\nhello \\\" quoted\nworld\n\";\nlet y = 1;";
        assert_eq!(states(src), vec![false, true, true, true, false]);
    }

    #[test]
    fn ignores_quotes_in_line_comments() {
        let src = "// \"not a string\nlet x = 1;";
        assert_eq!(states(src), vec![false, false]);
    }

    #[test]
    fn ignores_quotes_in_block_comments() {
        let src = "/* \"\nstill comment \" */\nlet x = 1;";
        assert_eq!(states(src), vec![false, false, false]);
    }

    #[test]
    fn handles_char_literals_and_lifetimes() {
        let src = "let x: Vec<&'a str> = vec!['\"'];\nlet y = 1;";
        assert_eq!(states(src), vec![false, false]);
    }

    #[test]
    fn handles_ident_prefix_r() {
        let src = "for x in 0..10 {\nweb\"\nstill string\n\";";
        assert_eq!(states(src), vec![false, false, true, true]);
    }
}
