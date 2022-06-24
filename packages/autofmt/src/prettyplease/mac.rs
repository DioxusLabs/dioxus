use super::algorithm::Printer;
use super::token::Token;
use super::INDENT;
use proc_macro2::{Delimiter, Spacing, TokenStream};
use syn::{Ident, Macro, MacroDelimiter, PathArguments};

impl Printer {
    pub fn mac(&mut self, mac: &Macro, ident: Option<&Ident>) {
        let is_macro_rules = mac.path.leading_colon.is_none()
            && mac.path.segments.len() == 1
            && matches!(mac.path.segments[0].arguments, PathArguments::None)
            && mac.path.segments[0].ident == "macro_rules";
        if is_macro_rules {
            if let Some(ident) = ident {
                self.macro_rules(ident, &mac.tokens);
                return;
            }
        }
        self.path(&mac.path);
        self.word("!");
        if let Some(ident) = ident {
            self.nbsp();
            self.ident(ident);
        }
        let (open, close, delimiter_break) = match mac.delimiter {
            MacroDelimiter::Paren(_) => ("(", ")", Self::zerobreak as fn(&mut Self)),
            MacroDelimiter::Brace(_) => (" {", "}", Self::hardbreak as fn(&mut Self)),
            MacroDelimiter::Bracket(_) => ("[", "]", Self::zerobreak as fn(&mut Self)),
        };
        self.word(open);
        self.cbox(INDENT);
        delimiter_break(self);
        self.ibox(0);
        self.macro_rules_tokens(mac.tokens.clone(), false);
        self.end();
        delimiter_break(self);
        self.offset(-INDENT);
        self.end();
        self.word(close);
    }

    pub fn mac_semi_if_needed(&mut self, delimiter: &MacroDelimiter) {
        match delimiter {
            MacroDelimiter::Paren(_) | MacroDelimiter::Bracket(_) => self.word(";"),
            MacroDelimiter::Brace(_) => {}
        }
    }

    fn macro_rules(&mut self, name: &Ident, rules: &TokenStream) {
        enum State {
            Start,
            Matcher,
            Equal,
            Greater,
            Expander,
        }

        use State::*;

        self.word("macro_rules! ");
        self.ident(name);
        self.word(" {");
        self.cbox(INDENT);
        self.hardbreak_if_nonempty();
        let mut state = State::Start;
        for tt in rules.clone() {
            let token = Token::from(tt);
            match (state, token) {
                (Start, Token::Group(delimiter, stream)) => {
                    self.delimiter_open(delimiter);
                    if !stream.is_empty() {
                        self.cbox(INDENT);
                        self.zerobreak();
                        self.ibox(0);
                        self.macro_rules_tokens(stream, true);
                        self.end();
                        self.zerobreak();
                        self.offset(-INDENT);
                        self.end();
                    }
                    self.delimiter_close(delimiter);
                    state = Matcher;
                }
                (Matcher, Token::Punct('=', Spacing::Joint)) => {
                    self.word(" =");
                    state = Equal;
                }
                (Equal, Token::Punct('>', Spacing::Alone)) => {
                    self.word(">");
                    state = Greater;
                }
                (Greater, Token::Group(_delimiter, stream)) => {
                    self.word(" {");
                    self.neverbreak();
                    if !stream.is_empty() {
                        self.cbox(INDENT);
                        self.hardbreak();
                        self.ibox(0);
                        self.macro_rules_tokens(stream, false);
                        self.end();
                        self.hardbreak();
                        self.offset(-INDENT);
                        self.end();
                    }
                    self.word("}");
                    state = Expander;
                }
                (Expander, Token::Punct(';', Spacing::Alone)) => {
                    self.word(";");
                    self.hardbreak();
                    state = Start;
                }
                _ => unimplemented!("bad macro_rules syntax"),
            }
        }
        match state {
            Start => {}
            Expander => {
                self.word(";");
                self.hardbreak();
            }
            _ => self.hardbreak(),
        }
        self.offset(-INDENT);
        self.end();
        self.word("}");
    }

    fn macro_rules_tokens(&mut self, stream: TokenStream, matcher: bool) {
        #[derive(PartialEq)]
        enum State {
            Start,
            Dollar,
            DollarIdent,
            DollarIdentColon,
            DollarParen,
            DollarParenSep,
            Pound,
            PoundBang,
            Dot,
            Colon,
            Colon2,
            Ident,
            IdentBang,
            Delim,
            Other,
        }

        use State::*;

        let mut state = Start;
        let mut previous_is_joint = true;
        for tt in stream {
            let token = Token::from(tt);
            let (needs_space, next_state) = match (&state, &token) {
                (Dollar, Token::Ident(_)) => (false, if matcher { DollarIdent } else { Other }),
                (DollarIdent, Token::Punct(':', Spacing::Alone)) => (false, DollarIdentColon),
                (DollarIdentColon, Token::Ident(_)) => (false, Other),
                (DollarParen, Token::Punct('+' | '*' | '?', Spacing::Alone)) => (false, Other),
                (DollarParen, Token::Ident(_) | Token::Literal(_)) => (false, DollarParenSep),
                (DollarParen, Token::Punct(_, Spacing::Joint)) => (false, DollarParen),
                (DollarParen, Token::Punct(_, Spacing::Alone)) => (false, DollarParenSep),
                (DollarParenSep, Token::Punct('+' | '*', _)) => (false, Other),
                (Pound, Token::Punct('!', _)) => (false, PoundBang),
                (Dollar, Token::Group(Delimiter::Parenthesis, _)) => (false, DollarParen),
                (Pound | PoundBang, Token::Group(Delimiter::Bracket, _)) => (false, Other),
                (Ident, Token::Group(Delimiter::Parenthesis | Delimiter::Bracket, _)) => {
                    (false, Delim)
                }
                (Ident, Token::Punct('!', Spacing::Alone)) => (false, IdentBang),
                (IdentBang, Token::Group(Delimiter::Parenthesis | Delimiter::Bracket, _)) => {
                    (false, Other)
                }
                (Colon, Token::Punct(':', _)) => (false, Colon2),
                (_, Token::Group(Delimiter::Parenthesis | Delimiter::Bracket, _)) => (true, Delim),
                (_, Token::Group(Delimiter::Brace | Delimiter::None, _)) => (true, Other),
                (_, Token::Ident(ident)) if !is_keyword(ident) => {
                    (state != Dot && state != Colon2, Ident)
                }
                (_, Token::Literal(_)) => (state != Dot, Ident),
                (_, Token::Punct(',' | ';', _)) => (false, Other),
                (_, Token::Punct('.', _)) if !matcher => (state != Ident && state != Delim, Dot),
                (_, Token::Punct(':', Spacing::Joint)) => (state != Ident, Colon),
                (_, Token::Punct('$', _)) => (true, Dollar),
                (_, Token::Punct('#', _)) => (true, Pound),
                (_, _) => (true, Other),
            };
            if !previous_is_joint {
                if needs_space {
                    self.space();
                } else if let Token::Punct('.', _) = token {
                    self.zerobreak();
                }
            }
            previous_is_joint = match token {
                Token::Punct(_, Spacing::Joint) | Token::Punct('$', _) => true,
                _ => false,
            };
            self.single_token(
                token,
                if matcher {
                    |printer, stream| printer.macro_rules_tokens(stream, true)
                } else {
                    |printer, stream| printer.macro_rules_tokens(stream, false)
                },
            );
            state = next_state;
        }
    }
}

fn is_keyword(ident: &Ident) -> bool {
    match ident.to_string().as_str() {
        "as" | "box" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod"
        | "move" | "mut" | "pub" | "ref" | "return" | "static" | "struct" | "trait" | "type"
        | "unsafe" | "use" | "where" | "while" | "yield" => true,
        _ => false,
    }
}
