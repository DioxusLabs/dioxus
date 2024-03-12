use crate::lock::DioxusLock;
use crate::plugin::convert::Convert;
// use crate::plugin::convert::Convert;
use crate::plugin::interface::{PluginRuntimeState, PluginWorld};
use crate::server::WsMessage;
use cargo_toml::Manifest;
use dioxus_cli_config::{ApplicationConfig, DioxusConfig, PluginConfigInfo};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{self, DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder};
use wasmtime_wasi::{ambient_authority, Dir};

// use self::convert::ConvertWithState;
// use self::interface::plugins::main::toml::Toml;
use self::interface::plugins::main::types::{
    CommandEvent, PluginInfo, ResponseEvent, RuntimeEvent,
};

pub mod convert;
pub mod interface;
lazy_static::lazy_static!(
  static ref ENGINE: Engine = {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    Engine::new(&config).unwrap()
  };

  pub static ref PLUGINS: Mutex<Vec<CliPlugin>> = Default::default();
  pub static ref PLUGINS_CONFIG: Mutex<DioxusConfig> = Default::default();
);

pub trait ChangeFold {
    fn fold_changes(self) -> ResponseEvent;
}

impl ChangeFold for Vec<ResponseEvent> {
    fn fold_changes(self) -> ResponseEvent {
        let mut option = ResponseEvent::None;
        for change in self.into_iter() {
            match (&mut option, change) {
                (ResponseEvent::Rebuild, _) | (_, ResponseEvent::Rebuild) => break,
                (ResponseEvent::Refresh(assets), ResponseEvent::Refresh(new_assets)) => {
                    assets.extend(new_assets);
                }
                (a, b) if *a < b => *a = b,
                _ => (),
            }
        }
        option
    }
}

impl PartialEq for ResponseEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Refresh(l0), Self::Refresh(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for ResponseEvent {}

impl Ord for ResponseEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (ResponseEvent::Refresh(_), ResponseEvent::Refresh(_)) => std::cmp::Ordering::Equal,
            (Self::Rebuild, Self::Rebuild) => std::cmp::Ordering::Equal,
            (Self::Reload, Self::Reload) => std::cmp::Ordering::Equal,
            (Self::None, Self::None) => std::cmp::Ordering::Equal,
            (_, Self::None) => std::cmp::Ordering::Greater,
            (Self::None, _) => std::cmp::Ordering::Less,
            (Self::Rebuild, _) => std::cmp::Ordering::Greater,
            (_, Self::Rebuild) => std::cmp::Ordering::Less,
            (Self::Reload, Self::Refresh(_)) => std::cmp::Ordering::Greater,
            (Self::Refresh(_), Self::Reload) => std::cmp::Ordering::Less,
        }
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::min_by(self, other, Ord::cmp)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
        Self: PartialOrd,
    {
        self.max(min).min(max)
    }
}

impl PartialOrd for ResponseEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Calls the global plugins with the function given
/// It will return a Vec of the results of the function
macro_rules! call_plugins {
    ($func:ident $event:expr) => {{
        let mut successful = vec![];
        for plugin in $crate::plugin::PLUGINS.lock().await.iter_mut() {
            let Ok(success) = plugin.$func($event).await else {
                log::warn!(
                    "Could not call {} {:?} on: {}!",
                    stringify!($func),
                    $event,
                    plugin.metadata.name
                );
                continue;
            };
            log::info!(
                "Called {} {:?} on: {}",
                stringify!($func),
                $event,
                plugin.metadata.name
            );
            successful.push(success);
        }
        let successful = successful.into_iter().flatten().collect::<Vec<_>>();
        successful
    }};
}

pub async fn plugins_before_command(compile_event: CommandEvent) {
    call_plugins!(before_command_event compile_event);
}
pub async fn plugins_after_command(compile_event: CommandEvent) {
    call_plugins!(after_command_event compile_event);
}
pub async fn plugins_before_runtime(runtime_event: RuntimeEvent) -> ResponseEvent {
    call_plugins!(before_runtime_event runtime_event).fold_changes()
}
pub async fn plugins_after_runtime(runtime_event: RuntimeEvent) -> ResponseEvent {
    call_plugins!(after_runtime_event runtime_event).fold_changes()
}

pub(crate) fn handle_change(
    change: ResponseEvent,
    reload_tx: &Option<Sender<WsMessage>>,
    needs_full_rebuild: &mut bool,
) {
    match change {
        ResponseEvent::Rebuild if reload_tx.is_some() => {
            if let Err(err) = reload_tx.as_ref().unwrap().send(WsMessage::Reload) {
                log::error!("Failed to send reload message: {}", err);
            }
        }
        ResponseEvent::Refresh(assets) if reload_tx.is_some() => {
            if let Err(err) = reload_tx
                .as_ref()
                .unwrap()
                .send(WsMessage::RefreshAssets { urls: assets })
            {
                log::error!("Failed to send refresh asset message: {}", err);
            }
        }
        ResponseEvent::Rebuild => *needs_full_rebuild = true,
        _ => (),
    }
}

pub async fn plugins_watched_paths_changed(
    paths: &[PathBuf],
    crate_dir: &PathBuf,
) -> ResponseEvent {
    if crate::plugin::PLUGINS.lock().await.is_empty() {
        return ResponseEvent::None;
    }

    let paths: Vec<String> = paths
        .iter()
        .filter_map(|f| match f.strip_prefix(crate_dir) {
            Ok(val) => val.to_str().map(|f| f.to_string()),
            Err(_) => {
                log::warn!(
                    "Path won't be available to plugins: {}! Plugins can only access paths under {}, Skipping..",
                    f.display(),
                    crate_dir.display(),
                );
                None
            }
        })
        .collect();
    call_plugins!(on_watched_paths_change & paths).fold_changes()
}

/// Returns a sorted list of plugins that are loaded in order
/// of priority from the dioxus config
async fn load_plugins(
    config: &DioxusConfig,
    crate_dir: &PathBuf,
    dioxus_lock: &mut DioxusLock,
    dependency_paths: &[PathBuf],
) -> wasmtime::Result<Vec<CliPlugin>> {
    let mut sorted_plugins: Vec<&PluginConfigInfo> = config.plugins.plugins.values().collect();
    // Have some leeway to have some plugins execute before the default priority plugins
    sorted_plugins.sort_by_key(|f| f.priority.unwrap_or(10));
    let mut plugins = Vec::with_capacity(sorted_plugins.len());

    for plugin in sorted_plugins.into_iter() {
        let plugin = load_plugin(
            &plugin.path,
            config,
            plugin.priority,
            crate_dir,
            dioxus_lock,
            dependency_paths,
        )
        .await?;
        plugins.push(plugin);
    }

    dioxus_lock.initialize_new_plugins(&mut plugins).await?;

    Ok(plugins)
}

enum PackageSource {
    Version(String, String),
    Path(String),
}

pub fn get_dependency_paths(crate_dir: &PathBuf) -> crate::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let toml_path = crate_dir.join("Cargo.toml");

    let registry_path = std::fs::read_dir(
        PathBuf::from(
            std::env::var("CARGO_HOME")
                .expect("Cargo Home environment variable should exist if cargo installed"),
        )
        .join("registry/src"),
    )?
    .find_map(|entry| {
        entry.ok().filter(|e| {
            e.file_name()
                .to_str()
                .filter(|f| f.starts_with("index.crates.io"))
                .is_some()
        })
    })
    .map(|e| e.path());

    if let None = registry_path {
        log::warn!("Could not find registry path for dependencies, skipping..");
        return Ok(out);
    }

    let registry_path = registry_path.unwrap();

    if let Ok(mut manifest) = Manifest::<Manifest>::from_path_with_metadata(&toml_path) {
        if let Err(err) = manifest.complete_from_path_and_workspace::<u8>(&toml_path, None) {
            log::warn!("Could not complete cargo manifest: {err}");
            return Ok(out);
        };
        for (name, dependency) in manifest.dependencies.into_iter() {
            let source = match dependency {
                cargo_toml::Dependency::Simple(version) => {
                    PackageSource::Version(name.clone(), version)
                }
                cargo_toml::Dependency::Inherited(_) => {
                    log::warn!("Could not get path for dependency: {name}, inheritted crate from workspace");
                    continue;
                }
                cargo_toml::Dependency::Detailed(detail) => {
                    if let Some(version) = detail.version {
                        PackageSource::Version(name.clone(), version)
                    } else if let Some(_git) = detail.git {
                        log::warn!("Git dependencies not supported yet!");
                        continue;
                    } else if let Some(path) = detail.path {
                        PackageSource::Path(path)
                    } else {
                        log::warn!("Could not get path for dependency: {name}, too complex path");
                        continue;
                    }
                }
            };
            let source_path = match source {
                PackageSource::Version(name, version) => {
                    let source_name = format!("{name}-{version}");
                    registry_path.join(source_name)
                }
                PackageSource::Path(path) => crate_dir.join(path),
            };

            log::info!("Found source dir for {name}: {}", source_path.display());

            out.push(source_path);
        }
    }
    Ok(out)
}

pub async fn init_plugins(
    config: &DioxusConfig,
    crate_dir: &PathBuf,
    dependency_paths: &[PathBuf],
) -> crate::Result<()> {
    let mut dioxus_lock = DioxusLock::load()?;
    let plugins = load_plugins(config, crate_dir, &mut dioxus_lock, dependency_paths).await?;
    *PLUGINS.lock().await = plugins;
    *PLUGINS_CONFIG.lock().await = config.clone();
    Ok(())
}

pub async fn save_plugin_config(bin: PathBuf) -> crate::Result<()> {
    let crate_root = dioxus_cli_config::crate_root()?.join(bin);

    let toml_path = crate_root.join("Dioxus.toml");

    let toml_string = std::fs::read_to_string(&toml_path)?;
    let mut diox_doc: toml_edit::Document = match toml_string.parse() {
        Ok(doc) => doc,
        Err(err) => {
            return Err(crate::Error::Unique(format!(
                "Could not parse Dioxus toml! {}",
                err
            )));
        }
    };

    let watcher_info = toml::Value::try_from(&PLUGINS_CONFIG.lock().await.watcher)
        .expect("Invalid Watcher Config!");
    diox_doc["watcher"] = watcher_info.convert();

    let plugin_info =
        toml::Value::try_from(&PLUGINS_CONFIG.lock().await.plugins).expect("Invalid Plugin Info!");
    diox_doc["plugins"] = plugin_info.convert();

    let mut dioxus_lock = DioxusLock::load()?;
    dioxus_lock.save(Some(PLUGINS.lock().await.as_ref()))?;

    std::fs::write(toml_path, diox_doc.to_string())?;
    log::info!("✔️  Successfully saved config");
    Ok(())
}

async fn wasi_context(
    crate_dir: &PathBuf,
    config: &ApplicationConfig,
    dependency_paths: &[PathBuf],
) -> crate::Result<WasiCtx> {
    let mut ctx = WasiCtxBuilder::new();

    // Give the plugins access to the terminal as well as crate files
    let mut ctx_pointer = ctx
        .inherit_stderr()
        .inherit_stdin()
        .inherit_stdio()
        .inherit_stdout()
        .preopened_dir(
            Dir::open_ambient_dir(crate_dir, ambient_authority())?,
            DirPerms::all(),
            FilePerms::all(),
            ".",
        );

    // If the application has these directories they might be seperate from the crate root
    if !config.out_dir.is_dir() {
        tokio::fs::create_dir(&config.out_dir).await?;
    }

    ctx_pointer = ctx_pointer.preopened_dir(
        Dir::open_ambient_dir(&config.out_dir, ambient_authority())?,
        DirPerms::all(),
        FilePerms::all(),
        "./dist",
    );

    if !config.asset_dir.is_dir() {
        tokio::fs::create_dir(&config.asset_dir).await?;
    }

    ctx_pointer = ctx_pointer.preopened_dir(
        Dir::open_ambient_dir(&config.asset_dir, ambient_authority())?,
        DirPerms::all(),
        FilePerms::all(),
        "./assets",
    );

    for path in dependency_paths {
        let Some(dep_name) = path.file_name() else {
            log::warn!(
                "Invalid path to add as plugin dependency: {}, skipping..",
                path.display()
            );
            continue;
        };
        ctx_pointer = ctx_pointer.preopened_dir(
            Dir::open_ambient_dir(path, ambient_authority())?,
            DirPerms::all(),
            FilePerms::all(),
            PathBuf::from("/deps")
                .join(dep_name)
                .to_str()
                .unwrap_or("/deps/unknown"), // TODO Check if this is possible
        )
    }

    Ok(ctx_pointer.build())
}

pub async fn load_plugin(
    path: impl AsRef<Path>,
    config: &DioxusConfig,
    priority: Option<usize>,
    crate_dir: &PathBuf,
    dioxus_lock: &mut DioxusLock,
    dependency_paths: &[PathBuf],
) -> crate::Result<CliPlugin> {
    let path = path.as_ref();
    let component = Component::from_file(&ENGINE, path)?;

    let mut linker = Linker::new(&ENGINE);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut PluginRuntimeState| state)?;

    let ctx = wasi_context(crate_dir, &config.application, dependency_paths).await?;
    let table = ResourceTable::new();

    let mut store = Store::new(
        &ENGINE,
        PluginRuntimeState {
            table,
            ctx,
            // tomls: Slab::new(),
            metadata: PluginInfo {
                name: "".into(),
                version: "".into(),
            },
            map: std::collections::HashMap::new(),
        },
    );

    let (bindings, instance) =
        PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    let metadata = bindings
        .plugins_main_definitions()
        .call_metadata(&mut store)
        .await?;

    if let Some(existing) = dioxus_lock.plugins.remove(&metadata.name) {
        store.data_mut().map = existing.map.into_iter().map(|(a, b)| (a, b.0)).collect();
    }

    let Ok(version) = semver::Version::from_str(&metadata.version) else {
        log::warn!(
            "Couldn't parse version from plugin: {} >> {}",
            metadata.name,
            metadata.version
        );
        return Err(crate::Error::CustomError(
            "couldn't parse plugin version".into(),
        ));
    };

    let config = &mut PLUGINS_CONFIG.lock().await.plugins.plugins;
    if let None = config.get(&metadata.name) {
        config.insert(
            metadata.name.clone(),
            PluginConfigInfo {
                version,
                path: path.to_path_buf(),
                config: HashMap::new(),
                priority,
            },
        );
    }

    store.data_mut().metadata = metadata.clone();

    Ok(CliPlugin {
        bindings,
        instance,
        store,
        metadata,
    })
}

pub struct CliPlugin {
    pub bindings: PluginWorld,
    pub instance: Instance,
    pub store: Store<PluginRuntimeState>,
    pub metadata: PluginInfo,
}

impl AsMut<PluginRuntimeState> for CliPlugin {
    fn as_mut(&mut self) -> &mut PluginRuntimeState {
        self.store.data_mut()
    }
}

impl CliPlugin {
    // pub async fn get_default_config(&mut self) -> wasmtime::Result<toml::Value> {
    //     let default_config = self
    //         .bindings
    //         .plugins_main_definitions()
    //         .call_get_default_config(&mut self.store)
    //         .await?;
    //     let t = self
    //         .store
    //         .data_mut()
    //         .get_toml(default_config)
    //         .convert_with_state(self.store.data_mut())
    //         .await;
    //     Ok(t)
    // }

    // pub async fn apply_config(
    //     &mut self,
    //     config: Resource<Toml>,
    // ) -> wasmtime::Result<Result<(), ()>> {
    //     self.bindings
    //         .plugins_main_definitions()
    //         .call_apply_config(&mut self.store, config)
    //         .await
    // }

    pub async fn register(&mut self) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_register(&mut self.store)
            .await
    }
    pub async fn before_command_event(
        &mut self,
        event: CommandEvent,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_before_command_event(&mut self.store, event)
            .await
    }
    pub async fn after_command_event(
        &mut self,
        event: CommandEvent,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_after_command_event(&mut self.store, event)
            .await
    }
    pub async fn before_runtime_event(
        &mut self,
        event: RuntimeEvent,
    ) -> wasmtime::Result<Result<ResponseEvent, ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_before_runtime_event(&mut self.store, event)
            .await
    }
    pub async fn after_runtime_event(
        &mut self,
        event: RuntimeEvent,
    ) -> wasmtime::Result<Result<ResponseEvent, ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_after_runtime_event(&mut self.store, event)
            .await
    }

    pub async fn on_watched_paths_change(
        &mut self,
        paths: &[String],
    ) -> wasmtime::Result<Result<ResponseEvent, ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_on_watched_paths_change(&mut self.store, paths)
            .await
    }

    // pub fn clone_handle(&mut self, handle: &Resource<Toml>) -> Resource<Toml> {
    //     self.store.data_mut().clone_handle(handle)
    // }

    // pub async fn get(&mut self, value: Resource<Toml>) -> toml::Value {
    //     self.store
    //         .data_mut()
    //         .get_toml(value)
    //         .convert_with_state(self.store.data_mut())
    //         .await
    // }

    // pub async fn insert_toml(&mut self, value: toml::Value) -> Resource<Toml> {
    //     let value = value.convert_with_state(self.store.data_mut()).await;
    //     self.store.data_mut().new_toml(value)
    // }

    // pub async fn set(&mut self, handle: Resource<Toml>, value: toml::Value) {
    //     // Should probably check if there is a Toml in the store
    //     // that is the same as the one we are putting in, currently will just add it to the
    //     // table
    //     let value = value.convert_with_state(self.store.data_mut()).await;
    //     self.store.data_mut().set_toml(handle, value);
    // }
}
