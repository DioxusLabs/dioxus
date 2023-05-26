use syn::{braced, parenthesized, parse::Parse, Expr, Ident, LitStr, Path, Token};

#[test]
fn parses() {
    use quote::quote;

    let tokens = quote! {
        // The name of the enum
        Route,
        // All nests that have dynamic segments must have a name used to generate the enum
        route(User, "user" / user_id: usize) {
            route(Product, "product" / product_id: usize / dynamic: usize ) {
                // Render creates a new route (that will be included in the enum) and is rendered with the given component
                // The component uses the struct of the parent route as a prop (in this case, Product)
                render(Other)
            }

            // You can nest routes inside a layout to wrap them in a component that accepts the struct of the parent route as a prop (in this case, User)
            layout(UserFrame) {
                route(Route1Props, "hello_world" / dynamic: usize ) {
                    // (Accepts Route1Props as a prop)
                    render(Route1)
                }

                // You can opt out of the layout by using !layout
                !layout(UserFrame) {
                    route(Route2Props, "hello_world" / dynamic: usize ) {
                        // (Accepts Route2Props as a prop)
                        render(Route2)
                    }
                }
            }
        }

        route(Route3Props, "hello_world" / dynamic: usize ) {
            // (Accepts Route3Props as a prop)
            render(Route3)
        }

        route(RedirectData, dynamic: usize / extra: String) {
            // Redirects accept a function that receives the struct of the parent route and returns the new route
            redirect(|data: RedirectData| todo!() )
        }
    };

    let _ = syn::parse2::<RouteTree>(tokens).unwrap();
}

struct RouteTree {
    name: Ident,
    roots: Vec<RouteSegment>,
}

impl Parse for RouteTree {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let _ = input.parse::<Token![,]>();

        let mut roots = Vec::new();
        while !input.is_empty() {
            roots.push(input.parse()?);
        }
        Ok(Self { name, roots })
    }
}

enum RouteSegment {
    Route(Route),
    Layout(Layout),
    Render(Render),
    Redirect(Redirect),
}

impl Parse for RouteSegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![!]) {
            input.parse::<Token![!]>()?;
            let ident: Ident = input.parse()?;
            if ident == "layout" {
                let mut layout: Layout = input.parse()?;
                layout.opt_out = true;
                Ok(RouteSegment::Layout(layout))
            } else {
                Err(lookahead.error())
            }
        } else if lookahead.peek(syn::Ident) {
            let ident: Ident = input.parse()?;
            if ident == "route" {
                let route = input.parse()?;
                Ok(RouteSegment::Route(route))
            } else if ident == "layout" {
                let layout = input.parse()?;
                Ok(RouteSegment::Layout(layout))
            } else if ident == "render" {
                let render = input.parse()?;
                Ok(RouteSegment::Render(render))
            } else if ident == "redirect" {
                let redirect = input.parse()?;
                Ok(RouteSegment::Redirect(redirect))
            } else {
                Err(lookahead.error())
            }
        } else {
            Err(lookahead.error())
        }
    }
}

struct Render {
    component: Path,
}

impl Parse for Render {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let component = inner.parse()?;

        Ok(Self { component })
    }
}

struct Layout {
    opt_out: bool,
    component: Path,
    children: Vec<RouteSegment>,
}

impl Parse for Layout {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let component = inner.parse()?;

        let content;
        braced!(content in input);
        let mut children = Vec::new();
        while !content.is_empty() {
            children.push(content.parse()?);
        }

        Ok(Self {
            opt_out: false,
            component,
            children,
        })
    }
}

struct Route {
    name: Ident,
    path: RoutePath,
    children: Vec<RouteSegment>,
}

impl Parse for Route {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let name = inner.parse()?;
        inner.parse::<Token![,]>()?;
        let path = inner.parse()?;

        let content;
        braced!(content in input);
        let mut children = Vec::new();
        while !content.is_empty() {
            children.push(content.parse()?);
        }

        Ok(Self {
            name,
            path,
            children,
        })
    }
}

struct RoutePath {
    segments: Vec<RoutePathSegment>,
    query: Option<QuerySegment>,
}

impl Parse for RoutePath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // parse all segments first
        let mut segments = Vec::new();
        // remove any leading slash
        if input.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
        }

        while !input.is_empty() {
            let peak = input.lookahead1();
            // check if the next segment is a query
            if peak.peek(syn::Token![?]) {
                break;
            } else if peak.peek(syn::Token![/]) {
                input.parse::<syn::Token![/]>()?;
            } else if peak.peek(syn::Ident) || peak.peek(syn::Token![...]) || peak.peek(syn::LitStr)
            {
                // parse the segment
                segments.push(input.parse()?);
            } else {
                return Err(peak.error());
            }
        }
        // then parse the query
        let query = if input.peek(syn::Token![?]) {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { segments, query })
    }
}

enum RoutePathSegment {
    Static(String),
    Dynamic(Ident, Path),
    CatchAll(Ident, Path),
}

impl Parse for RoutePathSegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![...]) {
            input.parse::<Token![...]>()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let type_: Path = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RoutePathSegment::CatchAll(name, type_))
        } else if lookahead.peek(LitStr) {
            let lit: LitStr = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RoutePathSegment::Static(lit.value()))
        } else if lookahead.peek(Ident) {
            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let type_: Path = input.parse()?;

            // parse the /
            let _ = input.parse::<Token![/]>();

            Ok(RoutePathSegment::Dynamic(ident, type_))
        } else {
            Err(lookahead.error())
        }
    }
}

struct QuerySegment {
    name: Ident,
    type_: Path,
}

impl Parse for QuerySegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![?]>()?;
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let type_ = input.parse()?;
        Ok(Self { name, type_ })
    }
}

struct Redirect {
    function: Expr,
}

impl Parse for Redirect {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        parenthesized!(inner in input);
        let function = inner.parse()?;
        Ok(Self { function })
    }
}
