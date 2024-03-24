use super::{
    hot_reload_diff::{diff_rsx, DiffResult},
    ChangedRsx,
};
use crate::{CallBody, HotReloadingContext};
use dioxus_core::{
    prelude::{TemplateAttribute, TemplateNode},
    Template,
};
use krates::cm::MetadataCommand;
use krates::Cmd;
pub use proc_macro2::TokenStream;
pub use std::collections::HashMap;
pub use std::sync::Mutex;
pub use std::time::SystemTime;
use std::{collections::HashSet, ffi::OsStr, marker::PhantomData, path::PathBuf};
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
use syn::spanned::Spanned;

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
    pub map: HashMap<PathBuf, CachedSynFile>,

    in_workspace: HashMap<PathBuf, Option<PathBuf>>,

    phantom: PhantomData<Ctx>,
}

/// A cached file that has been parsed
///
/// We store the templates found in this file
pub struct CachedSynFile {
    pub raw: String,
    pub path: PathBuf,
    pub templates: HashMap<&'static str, Template>,
    pub tracked_assets: HashSet<PathBuf>,
}

impl<Ctx: HotReloadingContext> FileMap<Ctx> {
    /// Create a new FileMap from a crate directory
    ///
    /// TODO: this should be created with a gitignore filter
    pub fn create(path: PathBuf) -> io::Result<FileMapBuildResult<Ctx>> {
        Self::create_with_filter(path, |p| {
            // skip some stuff we know is large by default
            p.file_name() == Some(OsStr::new("target"))
                || p.file_name() == Some(OsStr::new("node_modules"))
        })
    }

    /// Create a new FileMap from a crate directory
    pub fn create_with_filter(
        crate_dir: PathBuf,
        mut filter: impl FnMut(&Path) -> bool,
    ) -> io::Result<FileMapBuildResult<Ctx>> {
        let FileMapSearchResult { map, errors } = find_rs_files(crate_dir.clone(), &mut filter);

        let mut map = Self {
            map,
            in_workspace: HashMap::new(),
            phantom: PhantomData,
        };

        map.load_assets(crate_dir.as_path());

        Ok(FileMapBuildResult { errors, map })
    }

    /// Start watching assets for changes
    ///
    /// This just diffs every file against itself and populates the tracked assets as it goes
    pub fn load_assets(&mut self, crate_dir: &Path) {
        let keys = self.map.keys().cloned().collect::<Vec<_>>();
        for file in keys {
            _ = self.update_rsx(file.as_path(), crate_dir);
        }
    }

    /// Try to update the rsx in a file
    pub fn update_rsx(
        &mut self,
        file_path: &Path,
        crate_dir: &Path,
    ) -> Result<UpdateResult, HotreloadError> {
        let mut file = File::open(file_path)?;
        let mut src = String::new();
        file.read_to_string(&mut src)?;

        // If we can't parse the contents we want to pass it off to the build system to tell the user that there's a syntax error
        let syntax = syn::parse_file(&src).map_err(|_err| HotreloadError::Parse)?;

        let in_workspace = self.child_in_workspace(crate_dir)?;

        // Get the cached file if it exists, otherwise try to create it
        let Some(old_cached) = self.map.get_mut(file_path) else {
            // if this is a new file, rebuild the project
            let FileMapBuildResult { map, mut errors } =
                FileMap::<Ctx>::create(crate_dir.to_path_buf())?;

            if let Some(err) = errors.pop() {
                return Err(HotreloadError::Failure(err));
            }

            // merge the new map into the old map
            self.map.extend(map.map);

            return Ok(UpdateResult::NeedsRebuild);
        };

        // If the cached file is not a valid rsx file, rebuild the project, forcing errors
        // TODO: in theory the error is simply in the RsxCallbody. We could attempt to parse it using partial expansion
        // And collect out its errors instead of giving up to a full rebuild
        let old = syn::parse_file(&old_cached.raw).map_err(|_e| HotreloadError::Parse)?;

        let instances = match diff_rsx(&syntax, &old) {
            // If the changes were just some rsx, we can just update the template
            //
            // However... if the changes involved code in the rsx itself, this should actually be a CodeChanged
            DiffResult::RsxChanged {
                rsx_calls: instances,
            } => instances,

            // If the changes were some code, we should insert the file into the map and rebuild
            // todo: not sure we even need to put the cached file into the map, but whatever
            DiffResult::CodeChanged(_) => {
                let cached_file = CachedSynFile {
                    raw: src.clone(),
                    path: file_path.to_path_buf(),
                    templates: HashMap::new(),
                    tracked_assets: HashSet::new(),
                };

                self.map.insert(file_path.to_path_buf(), cached_file);
                return Ok(UpdateResult::NeedsRebuild);
            }
        };

        let mut messages: Vec<Template> = Vec::new();

        for calls in instances.into_iter() {
            let ChangedRsx { old, new } = calls;

            let old_start = old.span().start();

            let old_parsed = syn::parse2::<CallBody>(old.tokens);
            let new_parsed = syn::parse2::<CallBody>(new);
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

            // We leak the template since templates are a compiletime value
            // This is not ideal, but also not a huge deal for hot reloading
            // TODO: we could consider arena allocating the templates and dropping them when the connection is closed
            let leaked_location = Box::leak(template_location(old_start, file).into_boxed_str());

            // Retuns Some(template) if the template is hotreloadable
            // dynamic changes are not hot reloadable and force a rebuild
            let hotreloadable_template =
                new_call_body.update_template::<Ctx>(Some(old_call_body), leaked_location);

            // if the template is not hotreloadable, we need to do a full rebuild
            let Some(template) = hotreloadable_template else {
                return Ok(UpdateResult::NeedsRebuild);
            };

            // dioxus cannot handle empty templates...
            // todo: I think it can? or we just skip them nowa
            if template.roots.is_empty() {
                continue;
            }

            // if the template is the same, don't send it
            if let Some(old_template) = old_cached.templates.get(template.name) {
                if old_template == &template {
                    continue;
                }
            };

            // update the cached file
            old_cached.templates.insert(template.name, template);

            // Track any new assets
            old_cached
                .tracked_assets
                .extend(Self::populate_assets(template));

            messages.push(template);
        }

        Ok(UpdateResult::UpdatedRsx(messages))
    }

    fn populate_assets(template: Template) -> HashSet<PathBuf> {
        fn collect_assetlike_attrs(node: &TemplateNode, asset_urls: &mut HashSet<PathBuf>) {
            if let TemplateNode::Element {
                attrs, children, ..
            } = node
            {
                for attr in attrs.iter() {
                    if let TemplateAttribute::Static { name, value, .. } = attr {
                        if *name == "src" || *name == "href" {
                            asset_urls.insert(PathBuf::from(*value));
                        }
                    }
                }

                for child in children.iter() {
                    collect_assetlike_attrs(child, asset_urls);
                }
            }
        }

        let mut asset_urls = HashSet::new();

        for node in template.roots {
            collect_assetlike_attrs(node, &mut asset_urls);
        }

        asset_urls
    }

    /// add the template to an existing file in the filemap if it exists
    /// create a new file if it doesn't exist
    pub fn insert(&mut self, path: PathBuf, template: Template) {
        let tracked_assets = Self::populate_assets(template);

        if self.map.contains_key(&path) {
            let entry = self.map.get_mut(&path).unwrap();
            entry.tracked_assets.extend(tracked_assets);
            entry.templates.insert(template.name, template);
        } else {
            self.map.insert(
                path.clone(),
                CachedSynFile {
                    raw: String::new(),
                    path,
                    tracked_assets,
                    templates: HashMap::from([(template.name, template)]),
                },
            );
        }
    }

    pub fn tracked_assets(&self) -> HashSet<PathBuf> {
        self.map
            .values()
            .flat_map(|file| file.tracked_assets.iter().cloned())
            .collect()
    }

    pub fn is_tracking_asset(&self, path: &PathBuf) -> Option<&CachedSynFile> {
        self.map
            .values()
            .find(|file| file.tracked_assets.contains(path))
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
    let location = file.display().to_string()
                                + ":"
                                + &line.to_string()
                                + ":"
                                + &column.to_string()
                                // the byte index doesn't matter, but dioxus needs it
                                + ":0";
    location
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
                        path: root.clone(),
                        templates: HashMap::new(),
                        tracked_assets: HashSet::new(),
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
    NoPreviousBuild,
}

impl std::fmt::Display for HotreloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failure(err) => write!(f, "Failed to parse file: {}", err),
            Self::Parse => write!(f, "Failed to parse file"),
            Self::NoPreviousBuild => write!(f, "No previous build found"),
        }
    }
}

impl From<io::Error> for HotreloadError {
    fn from(err: io::Error) -> Self {
        HotreloadError::Failure(err)
    }
}
