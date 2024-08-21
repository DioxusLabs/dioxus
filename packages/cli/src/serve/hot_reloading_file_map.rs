use dioxus_core::internal::{HotReloadTemplateWithLocation, HotReloadedTemplate};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::{
    hot_reload::{diff_rsx, ChangedRsx},
    CallBody,
};
use krates::cm::MetadataCommand;
use krates::Cmd;
pub use std::collections::HashMap;
use std::{ffi::OsStr, path::PathBuf};
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
use syn::spanned::Spanned;

pub struct FileMap {
    pub map: HashMap<PathBuf, CachedSynFile>,

    /// Any errors that occurred while building the FileMap that were not fatal
    pub errors: Vec<io::Error>,

    pub in_workspace: HashMap<PathBuf, Option<PathBuf>>,
}

/// A cached file that has been parsed
///
/// We store the templates found in this file
pub struct CachedSynFile {
    pub raw: String,
    pub templates: HashMap<String, HotReloadedTemplate>,
}

impl FileMap {
    /// Create a new FileMap from a crate directory
    ///
    /// TODO: this should be created with a gitignore filter
    pub fn create<Ctx: HotReloadingContext>(path: PathBuf) -> io::Result<FileMap> {
        Self::create_with_filter::<Ctx>(path, |p| {
            // skip some stuff we know is large by default
            p.file_name() == Some(OsStr::new("target"))
                || p.file_name() == Some(OsStr::new("node_modules"))
        })
    }

    /// Create a new FileMap from a crate directory
    ///
    /// Takes a filter that when returns true, the file will be filtered out (ie not tracked)
    /// Note that this is inverted from a typical .filter() method.
    pub fn create_with_filter<Ctx: HotReloadingContext>(
        crate_dir: PathBuf,
        mut filter: impl FnMut(&Path) -> bool,
    ) -> io::Result<FileMap> {
        let FileMapSearchResult { map, errors } = find_rs_files(crate_dir.clone(), &mut filter);

        let mut map = Self {
            map,
            errors,
            in_workspace: HashMap::new(),
        };

        map.load_assets::<Ctx>(crate_dir.as_path());

        Ok(map)
    }

    /// Start watching assets for changes
    ///
    /// This just diffs every file against itself and populates the tracked assets as it goes
    pub fn load_assets<Ctx: HotReloadingContext>(&mut self, crate_dir: &Path) {
        let keys = self.map.keys().cloned().collect::<Vec<_>>();
        for file in keys {
            _ = self.update_rsx::<Ctx>(file.as_path(), crate_dir);
        }
    }

    /// Insert a file into the map and force a full rebuild
    fn full_rebuild(&mut self, file_path: PathBuf, src: String) -> HotreloadError {
        let cached_file = CachedSynFile {
            raw: src.clone(),
            templates: HashMap::new(),
        };

        self.map.insert(file_path, cached_file);
        HotreloadError::Notreloadable
    }

    /// Try to update the rsx in a file
    pub fn update_rsx<Ctx: HotReloadingContext>(
        &mut self,
        file_path: &Path,
        crate_dir: &Path,
    ) -> Result<Vec<HotReloadTemplateWithLocation>, HotreloadError> {
        let src = std::fs::read_to_string(file_path)?;

        // If we can't parse the contents we want to pass it off to the build system to tell the user that there's a syntax error
        let syntax = syn::parse_file(&src).map_err(|_err| HotreloadError::Parse)?;

        let in_workspace = self.child_in_workspace(crate_dir)?;

        // Get the cached file if it exists, otherwise try to create it
        let Some(old_cached) = self.map.get_mut(file_path) else {
            // if this is a new file, rebuild the project
            let mut map = FileMap::create::<Ctx>(crate_dir.to_path_buf())?;

            if let Some(err) = map.errors.pop() {
                return Err(HotreloadError::Failure(err));
            }

            // merge the new map into the old map
            self.map.extend(map.map);

            return Err(HotreloadError::Notreloadable);
        };

        // If the cached file is not a valid rsx file, rebuild the project, forcing errors
        // TODO: in theory the error is simply in the RsxCallbody. We could attempt to parse it using partial expansion
        // And collect out its errors instead of giving up to a full rebuild
        let old = syn::parse_file(&old_cached.raw).map_err(|_e| HotreloadError::Parse)?;

        let instances = match diff_rsx(&syntax, &old) {
            // If the changes were just some rsx, we can just update the template
            //
            // However... if the changes involved code in the rsx itself, this should actually be a CodeChanged
            Some(rsx_calls) => rsx_calls,

            // If the changes were some code, we should insert the file into the map and rebuild
            // todo: not sure we even need to put the cached file into the map, but whatever
            None => {
                return Err(self.full_rebuild(file_path.to_path_buf(), src));
            }
        };

        let mut out_templates = vec![];

        for calls in instances.into_iter() {
            let ChangedRsx { old, new } = calls;

            let old_start = old.span().start();

            let old_parsed = syn::parse2::<CallBody>(old.tokens);
            let new_parsed = syn::parse2::<CallBody>(new.tokens);
            let (Ok(old_call_body), Ok(new_call_body)) = (old_parsed, new_parsed) else {
                continue;
            };

            // if the file!() macro is invoked in a workspace, the path is relative to the workspace root, otherwise it's relative to the crate root
            // we need to check if the file is in a workspace or not and strip the prefix accordingly
            let prefix = match in_workspace {
                Some(ref workspace) => workspace,
                _ => crate_dir,
            };

            let Ok(file) = file_path.strip_prefix(prefix) else {
                continue;
            };

            let template_location = template_location(old_start, file);

            // Returns a list of templates that are hotreloadable
            let hotreload_result = dioxus_rsx::hot_reload::HotReloadResult::new::<Ctx>(
                &old_call_body.body,
                &new_call_body.body,
                template_location.clone(),
            );

            // if the template is not hotreloadable, we need to do a full rebuild
            let Some(mut results) = hotreload_result else {
                return Err(self.full_rebuild(file_path.to_path_buf(), src));
            };

            // Be careful to not send the bad templates
            results.templates.retain(|idx, template| {
                // dioxus cannot handle empty templates...
                if template.roots.is_empty() {
                    return false;
                }
                let template_location = format_template_name(&template_location, *idx);

                // if the template is the same, don't send its
                if old_cached.templates.get(&template_location) == Some(&*template) {
                    return false;
                };

                // Update the most recent idea of the template
                // This lets us know if the template has changed so we don't need to send it
                old_cached
                    .templates
                    .insert(template_location, template.clone());

                true
            });

            out_templates.extend(results.templates.into_iter().map(|(idx, template)| {
                HotReloadTemplateWithLocation {
                    location: format_template_name(&template_location, idx),
                    template,
                }
            }));
        }

        Ok(out_templates)
    }

    fn child_in_workspace(&mut self, crate_dir: &Path) -> io::Result<Option<PathBuf>> {
        if let Some(in_workspace) = self.in_workspace.get(crate_dir) {
            return Ok(in_workspace.clone());
        }

        let mut cmd = Cmd::new();
        let manafest_path = crate_dir.join("Cargo.toml");
        cmd.manifest_path(&manafest_path);
        let cmd: MetadataCommand = cmd.into();
        let metadata = cmd
            .exec()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        let in_workspace = metadata.workspace_root != crate_dir;
        let workspace_path = in_workspace.then(|| metadata.workspace_root.into());
        self.in_workspace
            .insert(crate_dir.to_path_buf(), workspace_path.clone());
        Ok(workspace_path)
    }
}

pub fn template_location(old_start: proc_macro2::LineColumn, file: &Path) -> String {
    let line = old_start.line;
    let column = old_start.column + 1;

    // Always ensure the path components are separated by `/`.
    let path = file
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    path + ":" + line.to_string().as_str() + ":" + column.to_string().as_str()
}

pub fn format_template_name(name: &str, index: usize) -> String {
    format!("{}:{}", name, index)
}

struct FileMapSearchResult {
    map: HashMap<PathBuf, CachedSynFile>,
    errors: Vec<io::Error>,
}

// todo: we could just steal the mod logic from rustc itself
fn find_rs_files(root: PathBuf, filter: &mut impl FnMut(&Path) -> bool) -> FileMapSearchResult {
    let mut files = HashMap::new();
    let mut errors = Vec::new();

    if root.is_dir() {
        let read_dir = match fs::read_dir(root) {
            Ok(read_dir) => read_dir,
            Err(err) => {
                errors.push(err);
                return FileMapSearchResult { map: files, errors };
            }
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !filter(&path) {
                let FileMapSearchResult {
                    map,
                    errors: child_errors,
                } = find_rs_files(path, filter);
                errors.extend(child_errors);
                files.extend(map);
            }
        }
    } else if root.extension().and_then(|s| s.to_str()) == Some("rs") {
        if let Ok(mut file) = File::open(root.clone()) {
            let mut src = String::new();
            match file.read_to_string(&mut src) {
                Ok(_) => {
                    let cached_file = CachedSynFile {
                        raw: src.clone(),
                        templates: HashMap::new(),
                    };

                    // track assets while we're here
                    files.insert(root, cached_file);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
    }

    FileMapSearchResult { map: files, errors }
}

#[derive(Debug)]
pub enum HotreloadError {
    Failure(io::Error),
    Parse,
    Notreloadable,
}

impl std::fmt::Display for HotreloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failure(err) => write!(f, "Failed to parse file: {}", err),
            Self::Parse => write!(f, "Failed to parse file"),
            Self::Notreloadable => write!(f, "Template is not hotreloadable"),
        }
    }
}

impl From<io::Error> for HotreloadError {
    fn from(err: io::Error) -> Self {
        HotreloadError::Failure(err)
    }
}
