//! Collect macros from a file
//!
//! Returns all macros that match a pattern. You can use this information to autoformat them later

use syn::{visit::Visit, File, Macro};

/// Collect all the rsx macros from a file
pub fn collect_rsx_macros_from_file(file: &File) -> Vec<&Macro> {
    struct MacroCollector<'a, 'b>(&'a mut Vec<&'b Macro>);
    impl<'a, 'b> Visit<'b> for MacroCollector<'a, 'b> {
        // todo: maybe visit the rsx fields too for their macros
        fn visit_macro(&mut self, i: &'b Macro) {
            let name = i.path.segments.last().map(|i| i.ident.to_string());
            if let Some("rsx" | "render") = name.as_deref() {
                self.0.push(i)
            }
        }
    }

    let mut macros = vec![];

    MacroCollector::visit_file(&mut MacroCollector(&mut macros), file);

    macros
}
