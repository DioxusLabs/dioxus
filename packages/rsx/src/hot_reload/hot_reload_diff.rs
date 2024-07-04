use syn::visit_mut::VisitMut;
use syn::{File, Macro};

#[derive(Debug)]
pub struct ChangedRsx {
    /// The macro that was changed
    pub old: Macro,

    /// The new tokens for the macro
    pub new: Macro,
}

/// Find any rsx calls in the given file and return a list of all the rsx calls that have changed.
///
/// Takes in the two files, clones them, removes the rsx! contents and prunes any doc comments.
/// Then it compares the two files to see if they are different - if they are, the code changed.
/// Otherwise, the code is the same and we can move on to handling the changed rsx
pub fn diff_rsx(new: &File, old: &File) -> Option<Vec<ChangedRsx>> {
    let mut old = old.clone();
    let mut new = new.clone();

    let old_macros = collect_from_file(&mut old);
    let new_macros = collect_from_file(&mut new);

    if old_macros.len() != new_macros.len() {
        return None;
    }

    if old != new {
        return None;
    }

    let rsx_calls = old_macros
        .into_iter()
        .zip(new_macros.into_iter())
        .map(|(old, new)| ChangedRsx { old, new })
        .collect();

    Some(rsx_calls)
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
    let old = include_str!("../../tests/invalid/changedexpr.old.rsx");
    let new = include_str!("../../tests/invalid/changedexpr.new.rsx");
    let same = include_str!("../../tests/invalid/changedexpr.same.rsx");

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
