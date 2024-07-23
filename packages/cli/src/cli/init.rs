use super::*;
use crate::cli::create::DEFAULT_TEMPLATE;
use cargo_generate::{GenerateArgs, TemplatePath};

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "init")]
pub struct Init {
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
    // TODO: turn on/off cargo-generate's output (now is invisible)
    // #[clap(default_value = "false", short, long)]
    // silent: bool,
}

impl Init {
    pub fn init(self) -> Result<()> {
        // Get directory name.
        let name = std::env::current_dir()?
            .file_name()
            .map(|f| f.to_str().unwrap().to_string());
        // https://github.com/console-rs/dialoguer/issues/294
        ctrlc::set_handler(move || {
            let _ = console::Term::stdout().show_cursor();
            std::process::exit(0);
        })
        .expect("ctrlc::set_handler");
        let args = GenerateArgs {
            define: self.option,
            init: true,
            name,
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
