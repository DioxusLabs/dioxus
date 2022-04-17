use crate::tools;

use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "tool")]
pub enum Tool {
    /// Return all dioxus-cli support tools.
    List {},
    /// Get default app install path.
    AppPath {},
    /// Install a new tool.
    Add { name: String },
}

impl Tool {
    pub async fn tool(self) -> Result<()> {
        match self {
            Tool::List {} => {
                for item in tools::tool_list() {
                    if tools::Tool::from_str(item).unwrap().is_installed() {
                        println!("{item} [installed]");
                    } else {
                        println!("{item}");
                    }
                }
            }
            Tool::AppPath {} => {
                println!("{}", tools::tools_path().to_str().unwrap());
            }
            Tool::Add { name } => {
                let tool_list = tools::tool_list();

                if !tool_list.contains(&name.as_str()) {
                    log::error!("Tool {name} not found.");
                    return Ok(());
                }
                let target_tool = tools::Tool::from_str(&name).unwrap();

                if target_tool.is_installed() {
                    log::warn!("Tool {name} is installed.");
                    return Ok(());
                }

                log::info!("Start to download tool package...");
                if let Err(e) = target_tool.download_package().await {
                    log::error!("Tool download failed: {e}");
                    return Ok(());
                }

                log::info!("Start to install tool package...");
                if let Err(e) = target_tool.install_package().await {
                    log::error!("Tool install failed: {e}");
                    return Ok(());
                }

                log::info!("Tool {name} install successfully!");
            }
        }

        Ok(())
    }
}
