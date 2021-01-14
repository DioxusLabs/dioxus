use crate::parser::{HtmlParser, NodesToPush};
use quote::quote;
use syn::{Stmt, Expr, ExprIf};

impl HtmlParser {
    /// Parse an incoming syn::Stmt node inside a block
    pub(crate) fn parse_statement(
        &mut self,
        stmt: &Stmt,
    ) {
        // Here we handle a block being a descendant within some html! call.
        //
        // The descendant should implement Into<IterableNodes>
        //
        // html { <div> { some_node } </div> }
        match stmt {
            Stmt::Expr(expr) => {
                self.parse_expr(stmt, expr);
            },
            _ => {
                self.push_iterable_nodes(NodesToPush::Stmt(stmt));
            }
        };
    }

    /// Parse an incoming syn::Expr node inside a block
    pub(crate) fn parse_expr(
        &mut self,
        stmt: &Stmt,
        expr: &Expr
    ) {
        match expr {
            Expr::If(expr_if) => {
                self.expand_if(stmt, expr_if);
            },
            _ => {
                self.push_iterable_nodes(NodesToPush::Stmt(stmt));
            }
        }
    }

    /// Expand an incoming Expr::If block
    /// This enables us to use JSX-style conditions inside of blocks such as
    /// the following example.
    /// 
    /// # Examples
    /// 
    /// ```rust,ignore
    /// html! {
    ///     <div>
    ///         {if condition_is_true {
    ///	            html! { <span>Hello World</span> }
    ///         }}
    ///     </div>
    /// }
    /// ```
    /// 
    /// Traditionally this would be possible as an if statement in rust is an
    /// expression, so the then, and the else block have to return matching types.
    /// Here we identify whether the block is missing the else and fill it in with
    /// a blank VirtualNode::text("")
    pub(crate) fn expand_if(
        &mut self,
        stmt: &Stmt,
        expr_if: &ExprIf
    ) {
        // Has else branch, we can parse the expression as normal.
        if let Some(_else_branch) = &expr_if.else_branch {
            self.push_iterable_nodes(NodesToPush::Stmt(stmt));
        } else {
            let condition = &expr_if.cond;
            let block = &expr_if.then_branch;
            let tokens = quote! {
                if #condition {
                    #block.into()
                } else {
                    VirtualNode::text("")
                }
            };

            self.push_iterable_nodes(NodesToPush::TokenStream(stmt, tokens));
        }
    }
}
