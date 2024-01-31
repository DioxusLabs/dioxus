use crate::{CallBody, HotReloadingContext};
use dioxus_core::Template;
use krates::cm::MetadataCommand;
use krates::Cmd;
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
    UpdatedRsx(Vec<Template>),
    NeedsRebuild,
}

/// The result of building a FileMap
pub struct FileMapBuildResult<Ctx: HotReloadingContext> {
    /// The FileMap that was built
    pub map: FileMap<Ctx>,
    /// Any errors that occurred while building the FileMap that were not fatal
    pub errors: Vec<io::Error>,
}

pub struct FileMap<Ctx: HotReloadingContext> {
    pub map: HashMap<PathBuf, (String, Option<Template>)>,
    in_workspace: HashMap<PathBuf, Option<PathBuf>>,
    phantom: std::marker::PhantomData<Ctx>,
}

impl<Ctx: HotReloadingContext> FileMap<Ctx> {
    /// Create a new FileMap from a crate directory
    pub fn create(path: PathBuf) -> io::Result<FileMapBuildResult<Ctx>> {
        Self::create_with_filter(path, |_| false)
    }

    /// Create a new FileMap from a crate directory
    pub fn create_with_filter(
        path: PathBuf,
        mut filter: impl FnMut(&Path) -> bool,
    ) -> io::Result<FileMapBuildResult<Ctx>> {
        struct FileMapSearchResult {
            map: HashMap<PathBuf, (String, Option<Template>)>,
            errors: Vec<io::Error>,
        }
        fn find_rs_files(
            root: PathBuf,
            filter: &mut impl FnMut(&Path) -> bool,
        ) -> FileMapSearchResult {
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
                            files.insert(root, (src, None));
                        }
                        Err(err) => {
                            errors.push(err);
                        }
                    }
                }
            }
            FileMapSearchResult { map: files, errors }
        }

        let FileMapSearchResult { map, errors } = find_rs_files(path, &mut filter);
        let result = Self {
            map,
            in_workspace: HashMap::new(),
            phantom: std::marker::PhantomData,
        };
        Ok(FileMapBuildResult {
            map: result,
            errors,
        })
    }

    /// Try to update the rsx in a file
    pub fn update_rsx(&mut self, file_path: &Path, crate_dir: &Path) -> io::Result<UpdateResult> {
        let mut file = File::open(file_path)?;
        let mut src = String::new();
        file.read_to_string(&mut src)?;
        if let Ok(syntax) = syn::parse_file(&src) {
            let in_workspace = self.child_in_workspace(crate_dir)?;
            if let Some((old_src, template_slot)) = self.map.get_mut(file_path) {
                if let Ok(old) = syn::parse_file(old_src) {
                    match find_rsx(&syntax, &old) {
                        DiffResult::CodeChanged => {
                            self.map.insert(file_path.to_path_buf(), (src, None));
                        }
                        DiffResult::RsxChanged(changed) => {
                            let mut messages: Vec<Template> = Vec::new();
                            for (old, new) in changed.into_iter() {
                                let old_start = old.span().start();

                                if let (Ok(old_call_body), Ok(new_call_body)) = (
                                    syn::parse2::<CallBody>(old.tokens),
                                    syn::parse2::<CallBody>(new),
                                ) {
                                    // if the file!() macro is invoked in a workspace, the path is relative to the workspace root, otherwise it's relative to the crate root
                                    // we need to check if the file is in a workspace or not and strip the prefix accordingly
                                    let prefix = if let Some(workspace) = &in_workspace {
                                        workspace
                                    } else {
                                        crate_dir
                                    };
                                    if let Ok(file) = file_path.strip_prefix(prefix) {
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
                                                return Ok(UpdateResult::NeedsRebuild);
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
                                            return Ok(UpdateResult::NeedsRebuild);
                                        }
                                    }
                                }
                            }
                            return Ok(UpdateResult::UpdatedRsx(messages));
                        }
                    }
                }
            } else {
                // if this is a new file, rebuild the project
                let FileMapBuildResult { map, mut errors } =
                    FileMap::create(crate_dir.to_path_buf())?;
                if let Some(err) = errors.pop() {
                    return Err(err);
                }
                *self = map;
            }
        }
        Ok(UpdateResult::NeedsRebuild)
    }

    fn child_in_workspace(&mut self, crate_dir: &Path) -> io::Result<Option<PathBuf>> {
        if let Some(in_workspace) = self.in_workspace.get(crate_dir) {
            Ok(in_workspace.clone())
        } else {
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
}
