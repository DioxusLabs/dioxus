use crate::{CallBody, HotReloadingContext};
use dioxus_core::Template;
pub use proc_macro2::TokenStream;
pub use std::collections::HashMap;
use std::path::PathBuf;
pub use std::sync::Mutex;
pub use std::time::SystemTime;
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
pub use syn::__private::ToTokens;
use syn::spanned::Spanned;

use super::hot_reload_diff::{find_rsx, DiffResult};

pub enum UpdateResult {
    UpdatedRsx(Vec<Template<'static>>),
    NeedsRebuild,
}

pub struct FileMap<Ctx: HotReloadingContext> {
    pub map: HashMap<PathBuf, (String, Option<Template<'static>>)>,
    phantom: std::marker::PhantomData<Ctx>,
}

impl<Ctx: HotReloadingContext> FileMap<Ctx> {
    /// Create a new FileMap from a crate directory
    pub fn new(path: PathBuf) -> Self {
        fn find_rs_files(
            root: PathBuf,
        ) -> io::Result<HashMap<PathBuf, (String, Option<Template<'static>>)>> {
            let mut files = HashMap::new();
            if root.is_dir() {
                for entry in (fs::read_dir(root)?).flatten() {
                    let path = entry.path();
                    files.extend(find_rs_files(path)?);
                }
            } else if root.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(mut file) = File::open(root.clone()) {
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    files.insert(root, (src, None));
                }
            }
            Ok(files)
        }

        let result = Self {
            map: find_rs_files(path).unwrap(),
            phantom: std::marker::PhantomData,
        };
        result
    }

    /// Try to update the rsx in a file
    pub fn update_rsx(&mut self, file_path: &Path, crate_dir: &Path) -> UpdateResult {
        let mut file = File::open(file_path).unwrap();
        let mut src = String::new();
        file.read_to_string(&mut src).expect("Unable to read file");
        if let Ok(syntax) = syn::parse_file(&src) {
            if let Some((old_src, template_slot)) = self.map.get_mut(file_path) {
                if let Ok(old) = syn::parse_file(old_src) {
                    match find_rsx(&syntax, &old) {
                        DiffResult::CodeChanged => {
                            self.map.insert(file_path.to_path_buf(), (src, None));
                        }
                        DiffResult::RsxChanged(changed) => {
                            let mut messages: Vec<Template<'static>> = Vec::new();
                            for (old, new) in changed.into_iter() {
                                let old_start = old.span().start();

                                if let (Ok(old_call_body), Ok(new_call_body)) = (
                                    syn::parse2::<CallBody>(old.tokens),
                                    syn::parse2::<CallBody>(new),
                                ) {
                                    if let Ok(file) = file_path.strip_prefix(crate_dir) {
                                        let line = old_start.line;
                                        let column = old_start.column + 1;
                                        let location = file.display().to_string()
                                        + ":"
                                        + &line.to_string()
                                        + ":"
                                        + &column.to_string()
                                        // the byte index doesn't matter, but dioxus needs it
                                        + ":0";

                                        if let Some(template) = new_call_body
                                            .update_template::<Ctx>(
                                                Some(old_call_body),
                                                Box::leak(location.into_boxed_str()),
                                            )
                                        {
                                            // dioxus cannot handle empty templates
                                            if template.roots.is_empty() {
                                                return UpdateResult::NeedsRebuild;
                                            } else {
                                                // if the template is the same, don't send it
                                                if let Some(old_template) = template_slot {
                                                    if old_template == &template {
                                                        continue;
                                                    }
                                                }
                                                *template_slot = Some(template);
                                                messages.push(template);
                                            }
                                        } else {
                                            return UpdateResult::NeedsRebuild;
                                        }
                                    }
                                }
                            }
                            return UpdateResult::UpdatedRsx(messages);
                        }
                    }
                }
            } else {
                // if this is a new file, rebuild the project
                *self = FileMap::new(crate_dir.to_path_buf());
            }
        }
        UpdateResult::NeedsRebuild
    }
}
