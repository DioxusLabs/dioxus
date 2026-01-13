use crate::DesktopContext;
use dioxus_core::queue_effect;
use dioxus_document::{
    create_element_in_head, Document, Eval, LinkProps, MetaProps, ScriptProps, StyleProps,
};
use dioxus_web_eval::WebEvaluator;

/// Represents the desktop-target's provider of evaluators.
#[derive(Clone)]
pub struct DesktopDocument {
    pub(crate) desktop_ctx: DesktopContext,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: DesktopContext) -> Self {
        Self { desktop_ctx }
    }
}

impl Document for DesktopDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(WebEvaluator::create(js))
    }

    fn set_title(&self, title: String) {
        self.desktop_ctx.set_title(&title);
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("meta", &props.attributes(), None));
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "script",
                &props.attributes(),
                props.script_contents().ok(),
            ));
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "style",
                &props.attributes(),
                props.style_contents().ok(),
            ));
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("link", &props.attributes(), None));
        });
    }
}
