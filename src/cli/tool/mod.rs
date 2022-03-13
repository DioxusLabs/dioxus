use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "tool")]
pub enum Tool {
    /// Return all dioxus-cli support tools.
    List {},
    Add {
        name: String
    }
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
            Tool::Add { name } => {
                let tool_list = tools::tool_list();
                
                if !tool_list.contains(&name.as_str()) {
                    log::error!("Tool {name} not found.");
                    return Ok(());
                }
                let target_tool = tools::Tool::from_str(&name).unwrap();
                println!("{:?}", target_tool.download_package().await);
            }
        }

        Ok(())
    }
}
