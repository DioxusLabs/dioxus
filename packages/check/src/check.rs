use std::path::PathBuf;

use syn::{spanned::Spanned, visit::Visit};

use crate::{
    issues::{Issue, IssueReport},
    metadata::{
        AnyLoopInfo, ClosureInfo, ComponentInfo, ConditionalInfo, FnInfo, ForInfo, HookInfo,
        IfInfo, LoopInfo, MatchInfo, Span, WhileInfo,
    },
};

struct VisitHooks {
    issues: Vec<Issue>,
    context: Vec<Node>,
}

impl VisitHooks {
    const fn new() -> Self {
        Self {
            issues: vec![],
            context: vec![],
        }
    }
}

/// Checks a Dioxus file for issues.
pub fn check_file(path: PathBuf, file_content: &str) -> IssueReport {
    let file = syn::parse_file(file_content).unwrap();
    let mut visit_hooks = VisitHooks::new();
    visit_hooks.visit_file(&file);
    IssueReport {
        path,
        file_content: file_content.to_string(),
        issues: visit_hooks.issues,
    }
}

#[derive(Debug, Clone)]
enum Node {
    If(IfInfo),
    Match(MatchInfo),
    For(ForInfo),
    While(WhileInfo),
    Loop(LoopInfo),
    Closure(ClosureInfo),
    ComponentFn(ComponentInfo),
    HookFn(HookInfo),
    OtherFn(FnInfo),
}

fn returns_element(ty: &syn::ReturnType) -> bool {
    match ty {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ref ty) => {
            if let syn::Type::Path(ref path) = **ty {
                if let Some(segment) = path.path.segments.last() {
                    if segment.ident == "Element" {
                        return true;
                    }
                }
            }
            false
        }
    }
}

fn is_hook_ident(ident: &syn::Ident) -> bool {
    ident.to_string().starts_with("use_")
}

fn is_component_fn(item_fn: &syn::ItemFn) -> bool {
    returns_element(&item_fn.sig.output)
}

fn fn_name_and_name_span(item_fn: &syn::ItemFn) -> (String, Span) {
    let name = item_fn.sig.ident.to_string();
    let name_span = item_fn.sig.ident.span().into();
    (name, name_span)
}

impl<'ast> syn::visit::Visit<'ast> for VisitHooks {
    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        if let syn::Expr::Path(ref path) = *i.func {
            if let Some(segment) = path.path.segments.last() {
                if is_hook_ident(&segment.ident) {
                    let hook_info = HookInfo::new(
                        i.span().into(),
                        segment.ident.span().into(),
                        segment.ident.to_string(),
                    );
                    let mut container_fn: Option<Node> = None;
                    for node in self.context.iter().rev() {
                        match node {
                            Node::If(if_info) => {
                                let issue = Issue::HookInsideConditional(
                                    hook_info.clone(),
                                    ConditionalInfo::If(if_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Match(match_info) => {
                                let issue = Issue::HookInsideConditional(
                                    hook_info.clone(),
                                    ConditionalInfo::Match(match_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::For(for_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::For(for_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::While(while_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::While(while_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Loop(loop_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::Loop(loop_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Closure(closure_info) => {
                                let issue = Issue::HookInsideClosure(
                                    hook_info.clone(),
                                    closure_info.clone(),
                                );
                                self.issues.push(issue);
                            }
                            Node::ComponentFn(_) | Node::HookFn(_) | Node::OtherFn(_) => {
                                container_fn = Some(node.clone());
                                break;
                            }
                        }
                    }

                    if let Some(Node::OtherFn(_)) = container_fn {
                        let issue = Issue::HookOutsideComponent(hook_info);
                        self.issues.push(issue);
                    }
                }
            }
        }
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let (name, name_span) = fn_name_and_name_span(i);
        if is_component_fn(i) {
            self.context.push(Node::ComponentFn(ComponentInfo::new(
                i.span().into(),
                name,
                name_span,
            )));
        } else if is_hook_ident(&i.sig.ident) {
            self.context.push(Node::HookFn(HookInfo::new(
                i.span().into(),
                i.sig.ident.span().into(),
                name,
            )));
        } else {
            self.context
                .push(Node::OtherFn(FnInfo::new(i.span().into(), name, name_span)));
        }
        syn::visit::visit_item_fn(self, i);
        self.context.pop();
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        self.context.push(Node::If(IfInfo::new(
            i.span().into(),
            i.if_token.span().into(),
        )));
        syn::visit::visit_expr_if(self, i);
        self.context.pop();
    }

    fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
        self.context.push(Node::Match(MatchInfo::new(
            i.span().into(),
            i.match_token.span().into(),
        )));
        syn::visit::visit_expr_match(self, i);
        self.context.pop();
    }

    fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
        self.context.push(Node::For(ForInfo::new(
            i.span().into(),
            i.for_token.span().into(),
        )));
        syn::visit::visit_expr_for_loop(self, i);
        self.context.pop();
    }

    fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
        self.context.push(Node::While(WhileInfo::new(
            i.span().into(),
            i.while_token.span().into(),
        )));
        syn::visit::visit_expr_while(self, i);
        self.context.pop();
    }

    fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
        self.context.push(Node::Loop(LoopInfo::new(
            i.span().into(),
            i.loop_token.span().into(),
        )));
        syn::visit::visit_expr_loop(self, i);
        self.context.pop();
    }

    fn visit_expr_closure(&mut self, i: &'ast syn::ExprClosure) {
        self.context
            .push(Node::Closure(ClosureInfo::new(i.span().into())));
        syn::visit::visit_expr_closure(self, i);
        self.context.pop();
    }
}
