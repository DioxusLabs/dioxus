use ::quote::{quote, ToTokens};
use ::std::ops::Not;
use ::syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *,
};
use proc_macro2::TokenStream;

pub fn format_args_f_impl(input: IfmtInput) -> Result<TokenStream> {
    let IfmtInput {
        format_literal,
        positional_args,
        named_args,
    } = input;

    let named_args = named_args.into_iter().map(|(ident, expr)| {
        quote! {
            #ident = #expr
        }
    });

    Ok(quote! {
        format_args!(
            #format_literal
            #(, #positional_args)*
            #(, #named_args)*
        )
    })
}

#[allow(dead_code)] // dumb compiler does not see the struct being used...
#[derive(Debug)]
pub struct IfmtInput {
    pub format_literal: LitStr,
    pub positional_args: Vec<Expr>,
    pub named_args: Vec<(Ident, Expr)>,
}

impl IfmtInput {
    fn parse_segments(self) -> Result<Self> {
        let IfmtInput {
            mut format_literal,
            mut positional_args,
            mut named_args,
        } = self;

        let s = format_literal.value();
        let out_format_literal = &mut String::with_capacity(s.len());

        let mut iterator = s.char_indices().peekable();
        while let Some((i, c)) = iterator.next() {
            out_format_literal.push(c);
            if c != '{' {
                continue;
            }
            // encountered `{`, let's see if it was `{{`
            if let Some(&(_, '{')) = iterator.peek() {
                let _ = iterator.next();
                out_format_literal.push('{');
                continue;
            }
            let (end, colon_or_closing_brace) = iterator
                .find(|&(_, c)| c == '}' || c == ':')
                .expect(concat!(
                    "Invalid format string literal\n",
                    "note: if you intended to print `{`, ",
                    "you can escape it using `{{`",
                ));
            // We use defer to ensure all the `continue`s append the closing char.
            let mut out_format_literal = defer(&mut *out_format_literal, |it| {
                it.push(colon_or_closing_brace)
            });
            let out_format_literal: &mut String = *out_format_literal;
            let mut arg = s[i + 1..end].trim();
            if let Some("=") = arg.get(arg.len().saturating_sub(1)..) {
                assert_eq!(
                    out_format_literal.pop(), // Remove the opening brace
                    Some('{'),
                );
                arg = &arg[..arg.len() - 1];
                out_format_literal.push_str(arg);
                out_format_literal.push_str(" = {");
            }
            if arg.is_empty() {
                continue;
            }

            #[derive(Debug)]
            enum Segment {
                Ident(Ident),
                LitInt(LitInt),
            }
            let segments: Vec<Segment> = {
                impl Parse for Segment {
                    fn parse(input: ParseStream<'_>) -> Result<Self> {
                        let lookahead = input.lookahead1();
                        if lookahead.peek(Ident) {
                            input.parse().map(Segment::Ident)
                        } else if lookahead.peek(LitInt) {
                            input.parse().map(Segment::LitInt)
                        } else {
                            Err(lookahead.error())
                        }
                    }
                }
                match ::syn::parse::Parser::parse_str(
                    Punctuated::<Segment, Token![.]>::parse_separated_nonempty,
                    arg,
                ) {
                    Ok(segments) => segments.into_iter().collect(),
                    Err(err) => return Err(err),
                }
            };
            match segments.len() {
                0 => unreachable!("`parse_separated_nonempty` returned empty"),
                1 => {
                    out_format_literal.push_str(arg);
                    match { segments }.pop().unwrap() {
                        Segment::LitInt(_) => {
                            // found something like `{0}`, let `format_args!`
                            // handle it.
                            continue;
                        }
                        Segment::Ident(ident) => {
                            // if `ident = ...` is not yet among the extra args
                            if named_args.iter().all(|(it, _)| *it != ident) {
                                named_args.push((
                                    ident.clone(),
                                    parse_quote!(#ident), // Expr
                                ));
                            }
                        }
                    }
                }
                _ => {
                    ::std::fmt::Write::write_fmt(
                        out_format_literal,
                        format_args!("{}", positional_args.len()),
                    )
                    .expect("`usize` or `char` Display impl cannot panic");
                    let segments: Punctuated<TokenStream, Token![.]> = segments
                        .into_iter()
                        .map(|it| match it {
                            Segment::Ident(ident) => ident.into_token_stream(),
                            Segment::LitInt(literal) => literal.into_token_stream(),
                        })
                        .collect();
                    positional_args.push(parse_quote! {
                        #segments
                    })
                }
            }
        }
        format_literal = LitStr::new(out_format_literal, format_literal.span());

        Ok(Self {
            format_literal,
            positional_args,
            named_args,
        })
    }

    fn parse_positional_args(input: ParseStream) -> Result<Self> {
        let format_literal = input.parse()?;
        let mut positional_args = vec![];
        loop {
            if input.parse::<Option<Token![,]>>()?.is_none() {
                return Ok(Self {
                    format_literal,
                    positional_args,
                    named_args: vec![],
                });
            }
            if input.peek(Ident) && input.peek2(Token![=]) && input.peek3(Token![=]).not() {
                // Found a positional parameter
                break;
            }
            positional_args.push(input.parse()?);
        }
        let named_args = Punctuated::<_, Token![,]>::parse_terminated_with(input, |input| {
            Ok({
                let name: Ident = input.parse()?;
                let _: Token![=] = input.parse()?;
                let expr: Expr = input.parse()?;
                (name, expr)
            })
        })?
        .into_iter()
        .collect();

        Ok(Self {
            format_literal,
            positional_args,
            named_args,
        })
    }
}

impl Parse for IfmtInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_positional_args(input).and_then(|new| new.parse_segments())
    }
}

pub fn defer<'a, T: 'a, Drop: 'a>(x: T, drop: Drop) -> impl ::core::ops::DerefMut<Target = T> + 'a
where
    Drop: FnOnce(T),
{
    use ::core::mem::ManuallyDrop;
    struct Ret<T, Drop>(ManuallyDrop<T>, ManuallyDrop<Drop>)
    where
        Drop: FnOnce(T);
    impl<T, Drop> ::core::ops::Drop for Ret<T, Drop>
    where
        Drop: FnOnce(T),
    {
        fn drop(&'_ mut self) {
            use ::core::ptr;
            unsafe {
                // # Safety
                //
                //   - This is the canonical example of using `ManuallyDrop`.
                let value = ManuallyDrop::into_inner(ptr::read(&self.0));
                let drop = ManuallyDrop::into_inner(ptr::read(&self.1));
                drop(value);
            }
        }
    }
    impl<T, Drop> ::core::ops::Deref for Ret<T, Drop>
    where
        Drop: FnOnce(T),
    {
        type Target = T;
        #[inline]
        fn deref(&'_ self) -> &'_ Self::Target {
            &self.0
        }
    }
    impl<T, Drop> ::core::ops::DerefMut for Ret<T, Drop>
    where
        Drop: FnOnce(T),
    {
        #[inline]
        fn deref_mut(&'_ mut self) -> &'_ mut Self::Target {
            &mut self.0
        }
    }
    Ret(ManuallyDrop::new(x), ManuallyDrop::new(drop))
}
