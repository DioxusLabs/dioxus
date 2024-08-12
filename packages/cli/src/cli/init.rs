use super::*;
use crate::cli::create::DEFAULT_TEMPLATE;
use cargo_generate::{GenerateArgs, TemplatePath};
use path_absolutize::Absolutize;

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "init")]
pub struct Init {
    /// Create a new Dioxus project at specified path.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Project name. Defaults to directory name
    #[arg(short, long)]
    name: Option<String>,

    /// Template path
    #[clap(default_value = DEFAULT_TEMPLATE, short, long)]
    template: String,

    /// Pass <option>=<value> for the used template (e.g., `foo=bar`)
    #[clap(short, long)]
    option: Vec<String>,

    /// Specify a sub-template within the template repository to be used as the actual template
    #[clap(long)]
    subtemplate: Option<String>,

    /// Skip user interaction by using the default values for the used template.
    /// Default values can be overridden with `--option`
    #[clap(short, long)]
    yes: bool,
}

impl Init {
    pub fn init(mut self) -> Result<()> {
        // Project name defaults to directory name.
        if self.name.is_none() {
            let dir_name = self
                .path
                .absolutize()?
                .to_path_buf()
                .file_name()
                .ok_or("Current path does not include directory name".to_string())?
                .to_str()
                .ok_or("Current directory name is not a valid UTF-8 string".to_string())?
                .to_string();
            self.name = Some(dir_name);
        }

        let args = GenerateArgs {
            define: self.option,
            destination: Some(self.path),
            // https://github.com/cargo-generate/cargo-generate/issues/1250
            init: true,
            name: self.name,
            silent: self.yes,
            template_path: TemplatePath {
                auto_path: Some(self.template),
                subfolder: self.subtemplate,
                ..Default::default()
            },
            ..Default::default()
        };
        let path = cargo_generate::generate(args)?;
        create::post_create(&path)
    }
}
