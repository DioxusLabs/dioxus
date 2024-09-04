use crate::builder::BuildArgs;
use crate::builder::BuildRequest;
use crate::dioxus_crate::DioxusCrate;
use futures_channel::mpsc::UnboundedSender;
use std::io::Write;
use toml_edit::Item;

use super::{Platform, UpdateBuildProgress};

pub static CLIENT_PROFILE: &str = "dioxus-client";
pub static SERVER_PROFILE: &str = "dioxus-server";

// The `opt-level=2` increases build times, but can noticeably decrease time
// between saving changes and being able to interact with an app. The "overall"
// time difference (between having and not having the optimization) can be
// almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
// on setup (hardware, OS, browser, idle load).
// Find or create the client and server profiles in the .cargo/config.toml file
pub fn initialize_profiles(krate: &DioxusCrate) -> crate::Result<()> {
    let config_path = krate.workspace_dir().join(".cargo/config.toml");
    let mut config = match std::fs::read_to_string(&config_path) {
        Ok(config) => config.parse::<toml_edit::DocumentMut>().map_err(|e| {
            crate::Error::Other(anyhow::anyhow!("Failed to parse .cargo/config.toml: {}", e))
        })?,
        Err(_) => Default::default(),
    };

    if let Item::Table(table) = config
        .as_table_mut()
        .entry("profile")
        .or_insert(Item::Table(Default::default()))
    {
        if let toml_edit::Entry::Vacant(entry) = table.entry(CLIENT_PROFILE) {
            let mut client = toml_edit::Table::new();
            client.insert("inherits", Item::Value("dev".into()));
            client.insert("opt-level", Item::Value(2.into()));
            entry.insert(Item::Table(client));
        }

        if let toml_edit::Entry::Vacant(entry) = table.entry(SERVER_PROFILE) {
            let mut server = toml_edit::Table::new();
            server.insert("inherits", Item::Value("dev".into()));
            server.insert("opt-level", Item::Value(2.into()));
            entry.insert(Item::Table(server));
        }
    }

    // Write the config back to the file
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(config_path)?;
    let mut buf_writer = std::io::BufWriter::new(file);
    write!(buf_writer, "{}", config)?;

    Ok(())
}
