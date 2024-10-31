use super::{AppHandle, ServeUpdate, WebServer};
use crate::{
    AppBundle, DioxusCrate, HotreloadFilemap, HotreloadResult, Platform, Result, TraceSrc,
};
use dioxus_core::internal::TemplateGlobalKey;
use dioxus_devtools_types::HotReloadMsg;
use dioxus_html::HtmlCtx;
use futures_util::{future::OptionFuture, stream::FuturesUnordered};
use ignore::gitignore::Gitignore;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
};
use tokio_stream::StreamExt;

pub(crate) struct AppRunner {
    pub(crate) running: HashMap<Platform, AppHandle>,
    pub(crate) krate: DioxusCrate,
    pub(crate) file_map: HotreloadFilemap,
    pub(crate) ignore: Gitignore,
    pub(crate) applied_hot_reload_message: HotReloadMsg,
    pub(crate) builds_opened: usize,
    pub(crate) should_full_rebuild: bool,
}

impl AppRunner {
    /// Create the AppRunner and then initialize the filemap with the crate directory.
    pub(crate) fn start(krate: &DioxusCrate) -> Self {
        let mut runner = Self {
            running: Default::default(),
            file_map: HotreloadFilemap::new(),
            applied_hot_reload_message: Default::default(),
            ignore: krate.workspace_gitignore(),
            krate: krate.clone(),
            builds_opened: 0,
            should_full_rebuild: true,
        };

        // todo(jon): this might take a while so we should try and background it, or make it lazy somehow
        // we could spawn a thread to search the FS and then when it returns we can fill the filemap
        // in testing, if this hits a massive directory, it might take several seconds with no feedback.
        for krate in krate.all_watched_crates() {
            runner.fill_filemap(krate);
        }

        runner
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        // If there are no running apps, we can just return pending to avoid deadlocking
        if self.running.is_empty() {
            return futures_util::future::pending().await;
        }

        self.running
            .iter_mut()
            .map(|(platform, handle)| async {
                use ServeUpdate::*;
                let platform = *platform;
                tokio::select! {
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stdout.as_mut().map(|f| f.next_line())) => {
                        StdoutReceived { platform, msg }
                    },
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stderr.as_mut().map(|f| f.next_line())) => {
                        StderrReceived { platform, msg }
                    },
                    Some(status) = OptionFuture::from(handle.app_child.as_mut().map(|f| f.wait())) => {
                        match status {
                            Ok(status) => ProcessExited { status, platform },
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
                            Ok(status) => ProcessExited { status, platform: Platform::Server },
                            Err(_err) => todo!("handle error in process joining?"),
                        }
                    }
                    else => futures_util::future::pending().await
                }
            })
            .collect::<FuturesUnordered<_>>()
            .next()
            .await
            .expect("Stream to pending if not empty")
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
        should_open_web: bool,
    ) -> Result<&AppHandle> {
        let platform = app.build.build.platform();

        // Drop the old handle
        // todo(jon): we should instead be sending the kill signal rather than dropping the process
        // This would allow a more graceful shutdown and fix bugs like desktop not retaining its size
        self.kill(platform);

        // wait a tiny sec for the processes to die so we don't have fullstack servers on top of each other
        // todo(jon): we should allow rebinding to the same port in fullstack itself
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

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
        self.running.insert(platform, handle);

        Ok(self.running.get(&platform).unwrap())
    }

    pub(crate) fn kill(&mut self, platform: Platform) {
        self.running.remove(&platform);
    }

    /// Open an existing app bundle, if it exists
    pub(crate) async fn open_existing(&self, devserver: &WebServer) {
        if let Some(address) = devserver.server_address() {
            let url = format!("http://{address}");
            tracing::debug!("opening url: {url}");
            _ = open::that(url);
        }
    }

    pub(crate) fn attempt_hot_reload(
        &mut self,
        modified_files: Vec<PathBuf>,
    ) -> Option<HotReloadMsg> {
        // If we have any changes to the rust files, we need to update the file map
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut edited_rust_files = Vec::new();
        let mut assets = Vec::new();

        for path in modified_files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };

            // If it's a rust file, we want to hotreload it using the filemap
            if ext == "rs" {
                edited_rust_files.push(path);
                continue;
            }

            // Otherwise, it might be an asset and we should look for it in all the running apps
            for runner in self.running.values() {
                if let Some(bundled_name) = runner.hotreload_bundled_asset(&path) {
                    // todo(jon): don't hardcode this here
                    let asset_relative = PathBuf::from("/assets/").join(bundled_name);
                    assets.push(asset_relative);
                }
            }
        }

        // Multiple runners might have queued the same asset, so dedup them
        assets.dedup();

        // Process the rust files
        for rust_file in edited_rust_files {
            // Strip the prefix before sending it to the filemap
            let Ok(path) = rust_file.strip_prefix(self.krate.workspace_dir()) else {
                tracing::error!(
                    "Hotreloading file outside of the crate directory: {:?}",
                    rust_file
                );
                continue;
            };

            // And grabout the contents
            let Ok(contents) = std::fs::read_to_string(&rust_file) else {
                tracing::debug!(
                    "Failed to read rust file while hotreloading: {:?}",
                    rust_file
                );
                continue;
            };

            match self.file_map.update_rsx::<HtmlCtx>(path, contents) {
                HotreloadResult::Rsx(new) => templates.extend(new),

                // The rust file may have failed to parse, but that is most likely
                // because the user is in the middle of adding new code
                // We just ignore the error and let Rust analyzer warn about the problem
                HotreloadResult::Notreloadable => return None,
                HotreloadResult::NotParseable => {
                    tracing::debug!(dx_src = ?TraceSrc::Dev, "Error hotreloading file - not parseable {rust_file:?}")
                }
            }
        }

        let msg = HotReloadMsg {
            templates,
            assets,
            unknown_files: vec![],
        };

        self.add_hot_reload_message(&msg);

        Some(msg)
    }

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
        let mut unknown_files: HashSet<PathBuf> = std::mem::take(&mut applied.unknown_files)
            .into_iter()
            .collect();
        for template in &msg.templates {
            templates.insert(template.key.clone(), template.clone());
        }
        assets.extend(msg.assets.iter().cloned());
        unknown_files.extend(msg.unknown_files.iter().cloned());
        applied.templates = templates.into_values().collect();
        applied.assets = assets.into_iter().collect();
        applied.unknown_files = unknown_files.into_iter().collect();
    }

    pub(crate) async fn client_connected(&mut self) {
        for (platform, runner) in self.running.iter_mut() {
            // Assign the runtime asset dir to the runner
            if *platform == Platform::Ios {
                // xcrun simctl get_app_container booted com.dioxuslabs
                let res = tokio::process::Command::new("xcrun")
                    .arg("simctl")
                    .arg("get_app_container")
                    .arg("booted")
                    .arg("com.dioxuslabs")
                    .output()
                    .await;

                if let Ok(res) = res {
                    tracing::debug!("Using runtime asset dir: {:?}", res);

                    if let Ok(out) = String::from_utf8(res.stdout) {
                        let out = out.trim();

                        tracing::debug!("Setting Runtime asset dir: {out:?}");
                        runner.runtime_asst_dir = Some(PathBuf::from(out));
                    }
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
                    self.file_map.add_file(path.to_path_buf(), contents);
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
}
