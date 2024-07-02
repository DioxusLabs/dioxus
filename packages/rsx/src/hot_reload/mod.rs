#[cfg(feature = "hot_reload")]
mod hot_reload_diff;
#[cfg(feature = "hot_reload")]
pub use hot_reload_diff::*;

#[cfg(feature = "hot_reload_traits")]
mod hot_reloading_context;
#[cfg(feature = "hot_reload_traits")]
pub use hot_reloading_context::*;

#[cfg(feature = "hot_reload")]
mod hot_reloading_file_map;
#[cfg(feature = "hot_reload")]
pub use hot_reloading_file_map::*;

// fn blah() {
//     AttrLiteral(HotLiteral {
//         value: Fmted(IfmtInput {
//             source: LitStr { token: "" },
//             segments: [
//                 Literal("hello world"),
//                 Literal(" "),
//                 Formatted(FormattedSegment {
//                     format_args: "",
//                     segment: Expr(Expr::If {
//                         attrs: [],
//                         if_token: If,
//                         cond: Expr::Path {
//                             attrs: [],
//                             qself: None,
//                             path: Path {
//                                 leading_colon: None,
//                                 segments: [PathSegment {
//                                     ident: Ident { sym: some_expr },
//                                     arguments: PathArguments::None,
//                                 }],
//                             },
//                         },
//                         then_branch: Block {
//                             brace_token: Brace,
//                             stmts: [Stmt::Expr(
//                                 Expr::Macro {
//                                     attrs: [],
//                                     mac: Macro {
//                                         path: Path {
//                                             leading_colon: Some(PathSep),
//                                             segments: [
//                                                 PathSegment {
//                                                     ident: Ident { sym: std },
//                                                     arguments: PathArguments::None,
//                                                 },
//                                                 PathSep,
//                                                 PathSegment {
//                                                     ident: Ident { sym: format_args },
//                                                     arguments: PathArguments::None,
//                                                 },
//                                             ],
//                                         },
//                                         bang_token: Not,
//                                         delimiter: MacroDelimiter::Paren(Paren),
//                                         tokens: TokenStream[Literal {
//                                             lit: "abc",
//                                             span: bytes(15..20),
//                                         }],
//                                     },
//                                 },
//                                 None,
//                             )],
//                         },
//                         else_branch: Some((
//                             Else,
//                             Expr::Block {
//                                 attrs: [],
//                                 label: None,
//                                 block: Block {
//                                     brace_token: Brace,
//                                     stmts: [Stmt::Expr(
//                                         Expr::Lit {
//                                             attrs: [],
//                                             lit: Lit::Str { token: "" },
//                                         },
//                                         None,
//                                     )],
//                                 },
//                             },
//                         )),
//                     }),
//                 }),
//             ],
//         }),
//         hr_idx: DynIdx {
//             idx: Cell { value: None },
//         },
//     });
// }
// fn blah() {
//     AttrLiteral(HotLiteral {
//         value: Fmted(IfmtInput {
//             source: LitStr { token: "" },
//             segments: [
//                 Literal("hello world"),
//                 Literal(" "),
//                 Formatted(FormattedSegment {
//                     format_args: "",
//                     segment: Expr(Expr::If {
//                         attrs: [],
//                         if_token: If,
//                         cond: Expr::Path {
//                             attrs: [],
//                             qself: None,
//                             path: Path {
//                                 leading_colon: None,
//                                 segments: [PathSegment {
//                                     ident: Ident { sym: some_expr },
//                                     arguments: PathArguments::None,
//                                 }],
//                             },
//                         },
//                         then_branch: Block {
//                             brace_token: Brace,
//                             stmts: [Stmt::Expr(
//                                 Expr::Lit {
//                                     attrs: [],
//                                     lit: Lit::Str { token: "abc" },
//                                 },
//                                 None,
//                             )],
//                         },
//                         else_branch: Some((
//                             Else,
//                             Expr::Block {
//                                 attrs: [],
//                                 label: None,
//                                 block: Block {
//                                     brace_token: Brace,
//                                     stmts: [Stmt::Expr(
//                                         Expr::Lit {
//                                             attrs: [],
//                                             lit: Lit::Str { token: "" },
//                                         },
//                                         None,
//                                     )],
//                                 },
//                             },
//                         )),
//                     }),
//                 }),
//             ],
//         }),
//         hr_idx: DynIdx {
//             idx: Cell { value: None },
//         },
//     });
// }
