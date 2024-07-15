//! Compare two files and find any rsx calls that have changed
//!
//! This is used to determine if a hotreload is needed.
//! We use a special syn visitor to find all the rsx! calls in the file and then compare them to see
//! if they are the same. This visitor will actually remove the rsx! calls and replace them with a
//! dummy rsx! call. The final file type is thus mutated in place, leaving the original file idents
//! in place. We then compare the two files to see if they are the same. We're able to differentiate
//! between rust code changes and rsx code changes with much less code than the previous manual diff
//! approach.

use syn::visit_mut::VisitMut;
use syn::{File, Macro};

#[derive(Debug)]
pub struct ChangedRsx {
    /// The old macro - the original RSX from the original file
    pub old: Macro,

    /// The new macro
    pub new: Macro,
}

/// Find any rsx calls in the given file and return a list of all the rsx calls that have changed.
///
/// Takes in the two files, clones them, removes the rsx! contents and prunes any doc comments.
/// Then it compares the two files to see if they are different - if they are, the code changed.
/// Otherwise, the code is the same and we can move on to handling the changed rsx
///
/// Returns `None` if the files are the same and `Some` if they are different
/// If there are no rsx! calls in the files, the vec will be empty.
pub fn diff_rsx(new: &File, old: &File) -> Option<Vec<ChangedRsx>> {
    // Make a clone of these files in place so we don't have to worry about mutating the original
    let mut old = old.clone();
    let mut new = new.clone();

    // Collect all the rsx! macros from the old file - modifying the files in place
    let old_macros = collect_from_file(&mut old);
    let new_macros = collect_from_file(&mut new);

    // If the number of rsx! macros is different, then it's not hotreloadable
    if old_macros.len() != new_macros.len() {
        return None;
    }

    // If the files are not the same, then it's not hotreloadable
    if old != new {
        return None;
    }

    Some(
        old_macros
            .into_iter()
            .zip(new_macros)
            .map(|(old, new)| ChangedRsx { old, new })
            .collect(),
    )
}

pub fn collect_from_file(file: &mut File) -> Vec<Macro> {
    struct MacroCollector(Vec<Macro>);
    impl VisitMut for MacroCollector {
        /// Take out the rsx! macros, leaving a default in their place
        fn visit_macro_mut(&mut self, dest: &mut syn::Macro) {
            let name = &dest.path.segments.last().map(|i| i.ident.to_string());
            if let Some("rsx" | "render") = name.as_deref() {
                let mut default: syn::Macro = syn::parse_quote! { rsx! {} };
                std::mem::swap(dest, &mut default);
                self.0.push(default)
            }
        }

        /// Ignore doc comments by swapping them out with a default
        fn visit_attribute_mut(&mut self, i: &mut syn::Attribute) {
            if i.path().is_ident("doc") {
                *i = syn::parse_quote! { #[doc = ""] };
            }
        }
    }

    let mut macros = MacroCollector(vec![]);
    macros.visit_file_mut(file);
    macros.0
}

#[test]
fn changing_files() {
    let old = r#"
use dioxus::prelude::*;

/// some comment
pub fn CoolChild() -> Element {
    let a = 123;

    rsx! {
        div {
            {some_expr()}
        }
    }
}"#;

    let new = r#"
use dioxus::prelude::*;

/// some comment
pub fn CoolChild() -> Element {
    rsx! {
        div {
            {some_expr()}
        }
    }
}"#;

    let same = r#"
use dioxus::prelude::*;

/// some comment!!!!!
pub fn CoolChild() -> Element {
    let a = 123;

    rsx! {
        div {
            {some_expr()}
        }
    }
}"#;

    let old = syn::parse_file(old).unwrap();
    let new = syn::parse_file(new).unwrap();
    let same = syn::parse_file(same).unwrap();

    assert!(
        diff_rsx(&old, &new).is_none(),
        "Files with different expressions should not be hotreloadable"
    );

    assert!(
        diff_rsx(&new, &new).is_some(),
        "The same file should be reloadable with itself"
    );

    assert!(
        diff_rsx(&old, &same).is_some(),
        "Files with changed comments should be hotreloadable"
    );
}
