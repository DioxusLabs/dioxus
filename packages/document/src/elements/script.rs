use super::*;
use crate::document;
use dioxus_html as dioxus_elements;

#[non_exhaustive]
#[derive(Clone, Props, PartialEq)]
pub struct ScriptProps {
    /// The contents of the script tag. If present, the children must be a single text node.
    pub children: Element,
    /// Scripts are deduplicated by their src attribute
    pub src: Option<String>,
    pub defer: Option<bool>,
    pub crossorigin: Option<String>,
    pub fetchpriority: Option<String>,
    pub integrity: Option<String>,
    pub nomodule: Option<bool>,
    pub nonce: Option<String>,
    pub referrerpolicy: Option<String>,
    pub r#type: Option<String>,
    #[props(extends = script, extends = GlobalAttributes)]
    pub additional_attributes: Vec<Attribute>,
}

impl ScriptProps {
    /// Get all the attributes for the script tag
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(defer) = &self.defer {
            attributes.push(("defer", defer.to_string()));
        }
        if let Some(crossorigin) = &self.crossorigin {
            attributes.push(("crossorigin", crossorigin.clone()));
        }
        if let Some(fetchpriority) = &self.fetchpriority {
            attributes.push(("fetchpriority", fetchpriority.clone()));
        }
        if let Some(integrity) = &self.integrity {
            attributes.push(("integrity", integrity.clone()));
        }
        if let Some(nomodule) = &self.nomodule {
            attributes.push(("nomodule", nomodule.to_string()));
        }
        if let Some(nonce) = &self.nonce {
            attributes.push(("nonce", nonce.clone()));
        }
        if let Some(referrerpolicy) = &self.referrerpolicy {
            attributes.push(("referrerpolicy", referrerpolicy.clone()));
        }
        if let Some(r#type) = &self.r#type {
            attributes.push(("type", r#type.clone()));
        }
        if let Some(src) = &self.src {
            attributes.push(("src", src.clone()));
        }
        attributes
    }

    pub fn script_contents(&self) -> Result<String, ExtractSingleTextNodeError<'_>> {
        extract_single_text_node(&self.children)
    }
}

/// Render a [`script`](crate::elements::script) tag into the head of the page.
///
///
/// If present, the children of the script component must be a single static or formatted string. If there are more children or the children contain components, conditionals, loops, or fragments, the script will not be added.
///
///
/// Any scripts you add will be deduplicated by their `src` attribute (if present).
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn LoadScript() -> Element {
///     rsx! {
///         // You can use the Script component to render a script tag into the head of the page
///         document::Script {
///             src: asset!("/assets/script.js"),
///         }
///     }
/// }
/// ```
///
/// <div class="warning">
///
/// Any updates to the props after the first render will not be reflected in the head.
///
/// </div>
#[component]
pub fn Script(props: ScriptProps) -> Element {
    use_update_warning(&props, "Script {}");

    use_hook(|| {
        let document = document();
        let mut insert_script = document.create_head_component();
        if let Some(src) = &props.src {
            if !should_insert_script(src) {
                insert_script = false;
            }
        }

        if !insert_script {
            return;
        }

        // Make sure the props are in a valid form - they must either have a source or children
        if let (None, Err(err)) = (&props.src, props.script_contents()) {
            // If the script has neither contents nor src, log an error
            err.log("Script")
        }

        document.create_script(props);
    });

    VNode::empty()
}

#[derive(Default, Clone)]
struct ScriptContext(DeduplicationContext);

fn should_insert_script(src: &str) -> bool {
    get_or_insert_root_context::<ScriptContext>()
        .0
        .should_insert(src)
}
