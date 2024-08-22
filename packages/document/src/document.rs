use super::*;

/// A provider for document-related functionality. By default most methods are driven through [`eval`].
pub trait Document {
    /// Get a reference to the document as `dyn Any`
    fn as_any(&self) -> &dyn std::any::Any;

    /// Run `eval` against this document, returning an [`Eval`] that can be used to await the result.
    fn eval(&self, js: String) -> Eval;

    /// Set the title of the document
    fn set_title(&self, title: String);

    /// Create a new element in the head
    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    );

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        self.create_head_element("meta", props.attributes(), None);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        self.create_head_element("script", props.attributes(), props.script_contents());
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        self.create_head_element("style", props.attributes(), props.style_contents());
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        self.create_head_element("link", props.attributes(), None);
    }
}
