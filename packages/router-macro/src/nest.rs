use quote::format_ident;
use syn::{parse::Parse, Ident, LitStr, Variant};

use crate::segment::RouteSegment;

pub enum Nest {
    Static(String),
    Layout(Layout),
}

impl Parse for Nest {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // First parse the route
        let route: LitStr = input.parse()?;

        if route.value().contains('(') {
            // Then parse the layout name
            let _ = input.parse::<syn::Token![,]>();
            let layout_name: Ident = input.parse()?;

            // Then parse the component name
            let _ = input.parse::<syn::Token![,]>();
            let comp: Variant = input.parse()?;

            // Then parse the props name
            let _ = input.parse::<syn::Token![,]>();
            let props_name: Ident = input
                .parse()
                .unwrap_or_else(|_| format_ident!("{}Props", comp.ident.to_string()));

            Ok(Self::Layout(Layout {
                route: route.value(),
                route_segments: Vec::new(),
                layout_name,
                comp,
                props_name,
            }))
        } else {
            Ok(Self::Static(route.value()))
        }
    }
}

struct Layout {
    pub route: String,
    pub route_segments: Vec<RouteSegment>,
    pub layout_name: Ident,
    pub comp: Variant,
    pub props_name: Ident,
}

// #[derive(Clone, Debug, PartialEq, Routable)]
// enum Route {
//     // Each Variant is a route with a linked component, dynamic segments are defined with the syntax: (name) and the type is inferred from the field type. The type must implement FromStr
//     #[route("/(dynamic)" Component1)]
//     Route1 { dynamic: usize },
//     // You can nest routes which makes all routes in the block relative to a parent route. Nested routes are flattened into the parent enum
//     // Nest accepts a optional layout component. The layout component that wraps all children and renders them where the Outlet component is found. It can accept parameters from the nested route, just like a normal route
//     #[nest("/(dynamic)" root_dynamic_segment Component { dynamic: String })]
//         // If the component is not specified, the component is assumed to be at the path of the route (in this case /pages/hello_world.rs or /pages/hello_world/index.rs)
//         #[route("/")]
//         // You can opt out of a parent Layout
//         #[layout(!root_dynamic_segment)]
//         Route2 {
//             // implicitly adds
//             // root_dynamic_segment: ComponentProps,
//         },
//     #[end_nest]
//     // Queries are defined with the syntax: ?(name) and the type is inferred from the field type. The type must implement From<&str> (not FromStr because the query parsing must be infallible). The query part of the url is not included in the route path for file based routing. (in this case /pages/takes_query.rs or /pages/takes_query/index.rs)
//     #[route("/takes_query?(dynamic)")]
//     Route3 { dynamic: u32 },
//     // Redirects are defined with the redirect attribute
//     #[redirect("/old_hello_world/(dynamic)")]
//     #[route("/hello_world/(dynamic)")]
//     Route4 { dynamic: u32 },
//     // members that can be parsed from all trailing segments are defined with the syntax: (...name) and the type is inferred from the field type. The type must implement FromSegments.
//     // Because this route is defined after Route3, it will only be matched if Route3 does not match and it will act as a fallback
//     #[route("/(...number2)")]
//     Route5 { number1: u32, number2: u32 },
// }
