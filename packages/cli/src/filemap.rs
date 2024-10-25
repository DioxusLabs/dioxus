use dioxus_core::internal::{
    HotReloadTemplateWithLocation, HotReloadedTemplate, TemplateGlobalKey,
};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{ChangedRsx, HotReloadResult};
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};
use syn::spanned::Spanned;

/// A struct that stores state of rsx! files and their parsed bodies.
///
/// This keeps track of changes to rsx files and helps determine if a file can be hotreloaded or if
/// the project needs to be rebuilt.
pub(crate) struct HotreloadFilemap {
    /// Map of rust files to their contents
    ///
    /// Once this is created, we won't change the contents, to preserve the ability to hotreload
    /// from the original source mapping, unless the file change results in a full rebuild.
    map: HashMap<PathBuf, CachedFile>,
}

struct CachedFile {
    contents: String,
    most_recent: Option<String>,
    templates: HashMap<TemplateGlobalKey, HotReloadedTemplate>,
}

pub enum HotreloadResult {
    Rsx(Vec<HotReloadTemplateWithLocation>),
    Notreloadable,
    NotParseable,
}

impl HotreloadFilemap {
    /// Create a new empty filemap.
    ///
    /// Make sure to fill the filemap, either automatically with `fill_from_filesystem` or manually with `add_file`;
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Add a file to the filemap.
    pub(crate) fn add_file(&mut self, path: PathBuf, contents: String) {
        self.map.insert(
            path,
            CachedFile {
                contents,
                most_recent: None,
                templates: Default::default(),
            },
        );
    }

    /// Commit the changes to the filemap, overwriting the contents of the files
    ///
    /// Removes any cached templates and replaces the contents of the files with the most recent
    ///
    /// todo: we should-reparse the contents so we never send a new version, ever
    pub fn force_rebuild(&mut self) {
        for cached_file in self.map.values_mut() {
            if let Some(most_recent) = cached_file.most_recent.take() {
                cached_file.contents = most_recent;
            }
            cached_file.templates.clear();
        }
    }

    /// Try to update the rsx in a file, returning the templates that were hotreloaded
    ///
    /// If the templates could not be hotreloaded, this will return an error. This error isn't fatal, per se,
    /// but it does mean that we could not successfully hotreload the file in-place.
    ///
    /// It's expected that the file path you pass in is relative the crate root. We have no way of
    /// knowing if it's *not*, so we'll assume it is.
    ///
    /// This does not do any caching on what intermediate state, like previous hotreloads, so you need
    /// to do that yourself.
    pub(crate) fn update_rsx<Ctx: HotReloadingContext>(
        &mut self,
        path: &Path,
        new_contents: String,
    ) -> HotreloadResult {
        // Get the cached file if it exists
        let Some(cached_file) = self.map.get_mut(path) else {
            return HotreloadResult::NotParseable;
        };

        // We assume we can parse the old file and the new file
        // We should just ignore hotreloading files that we can't parse
        // todo(jon): we could probably keep the old `File` around instead of re-parsing on every hotreload
        let (Ok(old_file), Ok(new_file)) = (
            syn::parse_file(&cached_file.contents),
            syn::parse_file(&new_contents),
        ) else {
            tracing::debug!("Diff rsx returned not parseable");
            return HotreloadResult::NotParseable;
        };

        // Update the most recent version of the file, so when we force a rebuild, we keep operating on the most recent version
        cached_file.most_recent = Some(new_contents);

        // todo(jon): allow server-fn hotreloading
        // also whyyyyyyyyy is this (new, old) instead of (old, new)? smh smh smh
        let Some(changed_rsx) = dioxus_rsx_hotreload::diff_rsx(&new_file, &old_file) else {
            tracing::debug!("Diff rsx returned notreladable");
            return HotreloadResult::Notreloadable;
        };

        let mut out_templates = vec![];
        for ChangedRsx { old, new } in changed_rsx {
            let old_start = old.span().start();

            let old_parsed = syn::parse2::<CallBody>(old.tokens);
            let new_parsed = syn::parse2::<CallBody>(new.tokens);
            let (Ok(old_call_body), Ok(new_call_body)) = (old_parsed, new_parsed) else {
                continue;
            };

            // Format the template location, normalizing the path
            let file_name: String = path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");

            // Returns a list of templates that are hotreloadable
            let results = HotReloadResult::new::<Ctx>(
                &old_call_body.body,
                &new_call_body.body,
                file_name.clone(),
            );

            // If no result is returned, we can't hotreload this file and need to keep the old file
            let Some(results) = results else {
                return HotreloadResult::Notreloadable;
            };

            // Only send down templates that have roots, and ideally ones that have changed
            // todo(jon): maybe cache these and don't send them down if they're the same
            for (index, template) in results.templates {
                if template.roots.is_empty() {
                    continue;
                }

                // Create the key we're going to use to identify this template
                let key = TemplateGlobalKey {
                    file: file_name.clone(),
                    line: old_start.line,
                    column: old_start.column + 1,
                    index,
                };

                // if the template is the same, don't send its
                if cached_file.templates.get(&key) == Some(&template) {
                    continue;
                };

                cached_file.templates.insert(key.clone(), template.clone());
                out_templates.push(HotReloadTemplateWithLocation { template, key });
            }
        }

        HotreloadResult::Rsx(out_templates)
    }
}
