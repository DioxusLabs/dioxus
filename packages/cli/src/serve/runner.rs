use super::{AppHandle, ServeUpdate, WebServer};
use crate::{AppBundle, DioxusCrate, Platform, ReloadKind, Result, TraceSrc};
use dioxus_core::internal::{
    HotReloadTemplateWithLocation, HotReloadedTemplate, TemplateGlobalKey,
};
use dioxus_core_types::HotReloadingContext;
use dioxus_devtools_types::HotReloadMsg;
use dioxus_html::HtmlCtx;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{ChangedRsx, HotReloadResult};
use futures_util::future::OptionFuture;
use ignore::gitignore::Gitignore;
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
};
use syn::spanned::Spanned;

pub(crate) struct AppRunner {
    pub(crate) running: Option<AppHandle>,
    pub(crate) krate: DioxusCrate,
    pub(crate) ignore: Gitignore,
    pub(crate) applied_hot_reload_message: HotReloadMsg,
    pub(crate) builds_opened: usize,
    pub(crate) automatic_rebuilds: bool,
    pub(crate) file_map: HashMap<PathBuf, CachedFile>,
}

pub enum HotReloadKind {
    Rsx(HotReloadMsg),
    Patch,
    Full,
}

struct CachedFile {
    contents: String,
    most_recent: Option<String>,
    templates: HashMap<TemplateGlobalKey, HotReloadedTemplate>,
}

impl AppRunner {
    /// Create the AppRunner and then initialize the filemap with the crate directory.
    pub(crate) fn start(krate: &DioxusCrate) -> Self {
        let mut runner = Self {
            running: Default::default(),
            file_map: Default::default(),
            applied_hot_reload_message: Default::default(),
            ignore: krate.workspace_gitignore(),
            krate: krate.clone(),
            builds_opened: 0,
            automatic_rebuilds: true,
        };

        // todo(jon): this might take a while so we should try and background it, or make it lazy somehow
        // we could spawn a thread to search the FS and then when it returns we can fill the filemap
        // in testing, if this hits a massive directory, it might take several seconds with no feedback.
        for krate in krate.all_watched_crates() {
            runner.fill_filemap(krate);
        }

        // Ensure the session cache dir exists and is empty
        runner.flush_session_cache();

        runner
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        // If there are no running apps, we can just return pending to avoid deadlocking
        let Some(handle) = self.running.as_mut() else {
            return futures_util::future::pending().await;
        };

        use ServeUpdate::*;
        let platform = handle.app.build.build.platform();
        tokio::select! {
            Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stdout.as_mut().map(|f| f.next_line())) => {
                StdoutReceived { platform, msg }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stderr.as_mut().map(|f| f.next_line())) => {
                StderrReceived { platform, msg }
            },
            Some(status) = OptionFuture::from(handle.app_child.as_mut().map(|f| f.wait())) => {
                match status {
                    Ok(status) => {
                        handle.app_child = None;
                        ProcessExited { status, platform }
                    },
                    Err(_err) => todo!("handle error in process joining?"),
                }
            }
            Some(Ok(Some(msg))) = OptionFuture::from(handle.server_stdout.as_mut().map(|f| f.next_line())) => {
                StdoutReceived { platform: Platform::Server, msg }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(handle.server_stderr.as_mut().map(|f| f.next_line())) => {
                StderrReceived { platform: Platform::Server, msg }
            },
            Some(status) = OptionFuture::from(handle.server_child.as_mut().map(|f| f.wait())) => {
                match status {
                    Ok(status) => {
                        handle.server_child = None;
                        ProcessExited { status, platform }
                    },
                    Err(_err) => todo!("handle error in process joining?"),
                }
            }
            else => futures_util::future::pending().await
        }
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
        should_open_web: bool,
    ) -> Result<&AppHandle> {
        // Drop the old handle
        // This is a more forceful kill than soft_kill since the app entropy will be wiped
        self.cleanup().await;

        // Add some cute logging
        if self.builds_opened == 0 {
            tracing::info!(
                "Build completed successfully in {:?}ms, launching app! 💫",
                app.app.time_taken.as_millis()
            );
        } else {
            tracing::info!("Build completed in {:?}ms", app.app.time_taken.as_millis());
        }

        // Start the new app before we kill the old one to give it a little bit of time
        let mut handle = AppHandle::new(app).await?;
        handle
            .open(
                devserver_ip,
                fullstack_address,
                self.builds_opened == 0 && should_open_web,
            )
            .await?;

        self.builds_opened += 1;
        self.running = Some(handle);

        Ok(self.running.as_ref().unwrap())
    }

    /// Open an existing app bundle, if it exists
    pub(crate) async fn open_existing(&mut self, devserver: &WebServer) -> Result<()> {
        let fullstack_address = devserver.proxied_server_address();

        if let Some(runner) = self.running.as_mut() {
            runner.soft_kill().await;
            runner
                .open(devserver.devserver_address(), fullstack_address, true)
                .await?;
        }

        Ok(())
    }

    /// Shutdown all the running processes
    pub(crate) async fn cleanup(&mut self) {
        if let Some(mut process) = self.running.take() {
            process.cleanup().await;
        }
    }

    /// Attempt to hotreload the given files
    pub(crate) async fn hotreload(&mut self, modified_files: Vec<PathBuf>) -> HotReloadKind {
        let file = modified_files[0].display().to_string();
        let file = file.trim_start_matches(&self.krate.crate_dir().display().to_string());

        // If we have any changes to the rust files, we need to update the file map
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut assets = Vec::new();
        let mut needs_full_rebuild = false;

        // We attempt to hotreload rsx blocks without a full rebuild
        for path in modified_files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };

            // If it's a rust file, we want to hotreload it using the filemap
            if ext == "rs" {
                // Strip the prefix before sending it to the filemap
                let Ok(path) = path.strip_prefix(self.krate.workspace_dir()) else {
                    tracing::error!(
                        "Hotreloading file outside of the crate directory: {:?}",
                        path
                    );
                    continue;
                };

                // And grabout the contents
                let Ok(contents) = std::fs::read_to_string(&path) else {
                    tracing::debug!("Failed to read rust file while hotreloading: {:?}", path);
                    continue;
                };

                match self.rsx_changed::<HtmlCtx>(path, contents) {
                    Some(new) => templates.extend(new),
                    None => needs_full_rebuild = true,
                }

                continue;
            }

            // Otherwise, it might be an asset and we should look for it in all the running apps
            if let Some(runner) = self.running.as_mut() {
                if let Some(bundled_name) = runner.hotreload_bundled_asset(&path).await {
                    // todo(jon): don't hardcode this here
                    assets.push(PathBuf::from("/assets/").join(bundled_name));
                }
            }
        }

        match needs_full_rebuild {
            true => HotReloadKind::Patch,
            false => {
                let msg = HotReloadMsg {
                    templates,
                    assets,
                    ..Default::default()
                };

                self.add_hot_reload_message(&msg);

                HotReloadKind::Rsx(msg)
            }
        }
    }

    fn attempt_rsx_hotreload(&mut self) {}

    /// Get any hot reload changes that have been applied since the last full rebuild
    pub(crate) fn applied_hot_reload_changes(&mut self) -> HotReloadMsg {
        self.applied_hot_reload_message.clone()
    }

    /// Clear the hot reload changes. This should be called any time a new build is starting
    pub(crate) fn clear_hot_reload_changes(&mut self) {
        self.applied_hot_reload_message = Default::default();
    }

    /// Store the hot reload changes for any future clients that connect
    fn add_hot_reload_message(&mut self, msg: &HotReloadMsg) {
        let applied = &mut self.applied_hot_reload_message;

        // Merge the assets, unknown files, and templates
        // We keep the newer change if there is both a old and new change
        let mut templates: HashMap<TemplateGlobalKey, _> = std::mem::take(&mut applied.templates)
            .into_iter()
            .map(|template| (template.key.clone(), template))
            .collect();
        let mut assets: HashSet<PathBuf> =
            std::mem::take(&mut applied.assets).into_iter().collect();
        for template in &msg.templates {
            templates.insert(template.key.clone(), template.clone());
        }
        assets.extend(msg.assets.iter().cloned());
        applied.templates = templates.into_values().collect();
        applied.assets = assets.into_iter().collect();
    }

    pub(crate) async fn client_connected(&mut self) {
        let Some(handle) = self.running.as_mut() else {
            return;
        };

        // Assign the runtime asset dir to the runner
        if handle.app.build.build.platform() == Platform::Ios {
            // xcrun simctl get_app_container booted com.dioxuslabs
            let res = tokio::process::Command::new("xcrun")
                .arg("simctl")
                .arg("get_app_container")
                .arg("booted")
                .arg(handle.app.build.krate.bundle_identifier())
                .output()
                .await;

            if let Ok(res) = res {
                tracing::trace!("Using runtime asset dir: {:?}", res);

                if let Ok(out) = String::from_utf8(res.stdout) {
                    let out = out.trim();

                    tracing::trace!("Setting Runtime asset dir: {out:?}");
                    handle.runtime_asst_dir = Some(PathBuf::from(out));
                }
            }
        }
    }

    /// Fill the filemap with files from the filesystem, using the given filter to determine which files to include.
    ///
    /// You can use the filter with something like a gitignore to only include files that are relevant to your project.
    /// We'll walk the filesystem from the given path and recursively search for all files that match the filter.
    ///
    /// The filter function takes a path and returns true if the file should be included in the filemap.
    /// Generally this will only be .rs files
    ///
    /// If a file couldn't be parsed, we don't fail. Instead, we save the error.
    pub fn fill_filemap(&mut self, path: PathBuf) {
        if self.ignore.matched(&path, path.is_dir()).is_ignore() {
            return;
        }

        // If the file is a .rs file, add it to the filemap
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(path) = path.strip_prefix(self.krate.workspace_dir()) {
                    self.file_map.insert(
                        path.to_path_buf(),
                        CachedFile {
                            contents,
                            most_recent: None,
                            templates: Default::default(),
                        },
                    );
                }
            }
            return;
        }

        // If it's not, we'll try to read the directory
        if path.is_dir() {
            if let Ok(read_dir) = std::fs::read_dir(&path) {
                for entry in read_dir.flatten() {
                    self.fill_filemap(entry.path());
                }
            }
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
    pub(crate) fn rsx_changed<Ctx: HotReloadingContext>(
        &mut self,
        path: &Path,
        new_contents: String,
    ) -> Option<Vec<HotReloadTemplateWithLocation>> {
        // Get the cached file if it exists - ignoring if it doesn't exist
        let Some(cached_file) = self.file_map.get_mut(path) else {
            return Some(vec![]);
        };

        // We assume we can parse the old file and the new file
        // We should just ignore hotreloading files that we can't parse
        // todo(jon): we could probably keep the old `File` around instead of re-parsing on every hotreload
        let (Ok(old_file), Ok(new_file)) = (
            syn::parse_file(&cached_file.contents),
            syn::parse_file(&new_contents),
        ) else {
            tracing::debug!("Diff rsx returned not parseable");
            return Some(vec![]);
        };

        // todo(jon): allow server-fn hotreloading
        let Some(changed_rsx) = dioxus_rsx_hotreload::diff_rsx(&new_file, &old_file) else {
            return None;
        };

        // Update the most recent version of the file, so when we force a rebuild, we keep operating on the most recent version
        cached_file.most_recent = Some(new_contents);

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
                return None;
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

        Some(out_templates)
    }

    /// Commit the changes to the filemap, overwriting the contents of the files
    ///
    /// Removes any cached templates and replaces the contents of the files with the most recent
    ///
    /// todo: we should-reparse the contents so we never send a new version, ever
    pub fn clear_cached_rsx(&mut self) {
        for cached_file in self.file_map.values_mut() {
            if let Some(most_recent) = cached_file.most_recent.take() {
                cached_file.contents = most_recent;
            }
            cached_file.templates.clear();
        }
    }

    fn flush_session_cache(&self) {
        let cache_dir = self.krate.session_cache_dir();
        _ = std::fs::remove_dir_all(&cache_dir);
        _ = std::fs::create_dir_all(&cache_dir);
    }
}
