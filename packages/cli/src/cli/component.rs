use std::{fs::create_dir_all, path::PathBuf};

use crate::{extract_assets_from_file, Result, StructuredOutput};
use clap::Parser;
use dioxus_cli_opt::process_file_to;
use tracing::debug;

#[derive(Clone, Debug, Parser)]
pub struct ComponentRegisteryArgs {
    /// The url of the the component registry
    #[arg(long, conflicts_with = "path")]
    git: Option<String>,
    /// The path to the components directory
    #[arg(long, conflicts_with = "git")]
    path: Option<String>,
}

impl ComponentRegisteryArgs {
    async fn resolve(&self) -> Result<PathBuf> {
        // If a path is provided, use that
        if let Some(path) = &self.path {
            return Ok(PathBuf::from(path));
        }

        todo!()
    }
}

#[derive(Clone, Debug, Parser)]
pub enum Component {
    #[clap(name = "add")]
    Add {
        /// The component to add
        component: String,
        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
    },
    #[clap(name = "remove")]
    Remove {
        /// The component to remove
        component: String,
    },
    #[clap(name = "list")]
    List {
        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
    },
}

impl Component {
    pub async fn run(self) -> Result<StructuredOutput> {
        match self {
            Self::List { registry } => {
                let path = registry.resolve().await?;
                debug!("Listing components in {:?}", path);
                let file = std::fs::File::open(path.join("components.json"))?;
                let mut reader = std::io::BufReader::new(file);
                let components: dioxus_component_manifest::Component =
                    serde_json::from_reader(&mut reader);
                println!("{}", serde_json::to_string_pretty(&components)?);
            }
            _ => todo!(),
        }

        Ok(StructuredOutput::Success)
    }
}
