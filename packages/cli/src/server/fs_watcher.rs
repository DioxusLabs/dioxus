use notify::Watcher;

pub struct FileWatcher {}

impl FileWatcher {
    pub fn start(config: &dioxus_cli_config::CrateConfig) -> Self {
        let mut last_update_time = chrono::Local::now().timestamp();

        // file watcher: check file change
        let mut allow_watch_path = config.dioxus_config.web.watcher.watch_path.clone();

        // Extend the watch path to include the assets directory - this is so we can hotreload CSS and other assets by default
        allow_watch_path.push(config.dioxus_config.application.asset_dir.clone());

        // Extend the watch path to include Cargo.toml and Dioxus.toml
        allow_watch_path.push("Cargo.toml".to_string().into());
        allow_watch_path.push("Dioxus.toml".to_string().into());
        allow_watch_path.dedup();

        // Create the file watcher
        let event_handler = {
            let watcher_config = config.clone();
            move |info: notify::Result<notify::Event>| {
                let Ok(e) = info else {
                    return;
                };
                // watch_event(
                //     e,
                //     &mut last_update_time,
                //     &hot_reload,
                //     &watcher_config,
                //     &build_with,
                //     &web_info,
                // );
            }
        };

        let mut watcher = notify::recommended_watcher(event_handler)
            .expect("Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.");

        // Watch the specified paths
        for sub_path in allow_watch_path {
            let path = &config.crate_dir.join(sub_path);
            let mode = notify::RecursiveMode::Recursive;

            if let Err(err) = watcher.watch(path, mode) {
                tracing::warn!("Failed to watch path: {}", err);
            }
        }

        Self {}
    }

    /// A cancel safe handle to the file watcher
    pub async fn wait(&mut self) {
        todo!()
    }
}
