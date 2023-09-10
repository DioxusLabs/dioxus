use dioxus_core::{Element, Scope};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::*;
use crate::component_body::{DeserializerArgs, DeserializerOutput, is_type_eq, TypeHelper};

/// General struct for parsing a component body.
/// However, because it's ambiguous, it does not implement [`ToTokens`](quote::to_tokens::ToTokens).
///
/// Refer to the [module documentation](crate::component_body) for more.
pub struct ComponentBody {
    /// The component function definition. You can parse this back into a [`ComponentBody`].
    /// For example, you might modify it, parse it into a [`ComponentBody`], and deserialize that
    /// using some deserializer. This is how deserializers use other deserializers.
    ///
    /// **`item_fn.sig.inputs` includes the context argument!**
    /// Keep this in mind when creating deserializers, because you often might want to ignore it.
    /// That might be annoying, but it would be bad design for this kind of struct to not be parsable from itself.
    pub item_fn: ItemFn,
    /// Parsing tries to ensure that this argument will be a [`Scope`].
    /// **However, macros have limitations that prevent this from always working,
    /// so don't take this for granted!**
    pub cx_arg: FnArg,
    /// The pattern (name) and type of the context argument.
    pub cx_pat_type: PatType,
    /// If the function has any arguments other than the context.
    pub has_extra_args: bool,
}

impl ComponentBody {
    // There's a lot of Results out there... let's make sure that this is a syn::Result.
    // Let's also make sure there's not a warning.
    #[allow(unused_qualifications)]
    pub fn deserialize<TOutput, TArgs>(&self, args: TArgs) -> syn::Result<TOutput>
        where
            TOutput: DeserializerOutput,
            TArgs: DeserializerArgs<TOutput>,
    {
        args.to_output(self)
    }
}

impl Parse for ComponentBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let item_fn: ItemFn = input.parse()?;
        let scope_type_path = Scope::get_path_string();

        let (cx_arg, cx_pat_type) = if let Some(first_arg) = item_fn.sig.inputs.first() {
            let incorrect_first_arg_err = Err(Error::new(
                first_arg.span(),
                format!("First argument must be a <{}>", scope_type_path),
            ));

            match first_arg.to_owned() {
                FnArg::Receiver(_) => {
                    return incorrect_first_arg_err;
                }
                FnArg::Typed(f) => {
                    if is_type_eq::<Scope>(&f.ty) {
                        (first_arg.to_owned(), f)
                    } else {
                        return incorrect_first_arg_err;
                    }
                }
            }
        } else {
            return Err(Error::new(
                item_fn.sig.ident.span(), // Or maybe just item_f.sig.span()?
                format!(
                    "Must have at least one argument that's a <{}>",
                    scope_type_path
                ),
            ));
        };

        let element_type_path = Element::get_path_string();

        match &item_fn.sig.output {
            ReturnType::Default => {
                return Err(Error::new(
                    item_fn.sig.output.span(),
                    format!("Must return a <{}>", element_type_path),
                ))
            }
            ReturnType::Type(_, return_type) => {
                if !is_type_eq::<Element>(&return_type) {
                    return Err(Error::new(
                        item_fn.sig.output.span(),
                        format!("Must return a <{}>", element_type_path),
                    ));
                }
            }
        }

        let has_extra_args = item_fn.sig.inputs.len() > 1;

        Ok(Self {
            item_fn,
            cx_arg,
            cx_pat_type,
            has_extra_args,
        })
    }
}
