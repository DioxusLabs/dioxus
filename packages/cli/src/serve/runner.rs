use super::{hot_reloading_file_map::HotreloadError, AppHandle, ServeUpdate};
use crate::{serve::hot_reloading_file_map::FileMap, DioxusCrate};
use crate::{AppBundle, Platform, Result, TraceSrc};
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
    pub(crate) file_map: FileMap,
    pub(crate) applied_hot_reload_message: HotReloadMsg,
    pub(crate) builds_opened: usize,
}

impl AppRunner {
    pub(crate) fn start(krate: &DioxusCrate, ignore: &Gitignore) -> Self {
        // Probe the entire project looking for our rsx calls
        // Whenever we get an update from the file watcher, we'll try to hotreload against this file map
        let file_map = FileMap::create_with_filter::<HtmlCtx>(krate.crate_dir(), |path| {
            ignore.matched(path, path.is_dir()).is_ignore()
        })
        .unwrap();

        Self {
            running: Default::default(),
            file_map,
            applied_hot_reload_message: Default::default(),
            krate: krate.clone(),
            builds_opened: 0,
        }
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
                        tracing::info!("Child process exited with status: {status:?}");
                        match status {
                            Ok(status) => ProcessExited { status, platform },
                            Err(_err) => todo!("handle error in process joining?"),
                        }
                    }
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.server_stdout.as_mut().map(|f| f.next_line())) => {
                        StdoutReceived { platform, msg }
                    },
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.server_stderr.as_mut().map(|f| f.next_line())) => {
                        StderrReceived { platform, msg }
                    },
                    Some(status) = OptionFuture::from(handle.server_child.as_mut().map(|f| f.wait())) => {
                        tracing::info!("Child process exited with status: {status:?}");
                        match status {
                            Ok(status) => ProcessExited { status, platform },
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
    pub(crate) async fn open_existing(&self) {
        tracing::debug!("todo: open existing app");
    }

    pub(crate) fn attempt_hot_reload(
        &mut self,
        modified_files: Vec<PathBuf>,
    ) -> Option<HotReloadMsg> {
        // If we have any changes to the rust files, we need to update the file map
        let crate_dir = self.krate.crate_dir();
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
                    let asset_relative = PathBuf::from("/assets/").join(bundled_name);
                    assets.push(asset_relative);
                }
            }
        }

        // Multiple runners might have queued the same asset, so dedup them
        assets.dedup();

        // Process the rust files
        for rust_file in edited_rust_files {
            match self.file_map.update_rsx::<HtmlCtx>(&rust_file, &crate_dir) {
                Ok(hotreloaded_templates) => {
                    templates.extend(hotreloaded_templates);
                }

                // If the file is not reloadable, we need to rebuild
                Err(HotreloadError::Notreloadable) => return None,

                // The rust file may have failed to parse, but that is most likely
                // because the user is in the middle of adding new code
                // We just ignore the error and let Rust analyzer warn about the problem
                Err(HotreloadError::Parse) => {}

                // Otherwise just log the error
                Err(err) => {
                    tracing::error!(dx_src = ?TraceSrc::Dev, "Error hotreloading file {rust_file:?}: {err}")
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
        let mut templates: HashMap<String, _> = std::mem::take(&mut applied.templates)
            .into_iter()
            .map(|template| (template.location.clone(), template))
            .collect();
        let mut assets: HashSet<PathBuf> =
            std::mem::take(&mut applied.assets).into_iter().collect();
        let mut unknown_files: HashSet<PathBuf> = std::mem::take(&mut applied.unknown_files)
            .into_iter()
            .collect();
        for template in &msg.templates {
            templates.insert(template.location.clone(), template.clone());
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
}
