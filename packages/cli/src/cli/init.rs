use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "init")]
pub struct Init {
    /// Template path
    #[clap(default_value = "gh:dioxuslabs/dioxus-template", long)]
    template: String,
}

impl Init {
    pub fn init(self) -> Result<()> {
        // get dir name
        let name = std::env::current_dir()?
            .file_name()
            .map(|f| f.to_str().unwrap().to_string());

        let args = GenerateArgs {
            template_path: TemplatePath {
                auto_path: Some(self.template),
                ..Default::default()
            },
            name,
            init: true,
            ..Default::default()
        };

        let path = cargo_generate::generate(args)?;

        create::post_create(&path)
    }
}
