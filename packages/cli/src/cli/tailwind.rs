use crate::{Result, StructuredOutput, TailwindCli, Workspace};
use clap::Parser;
use krates::cm::TargetKind;
use std::path::PathBuf;

/// Generate the tailwind CSS for a package without building it.
///
/// Runs the same tailwind step as `dx build`, using `tailwind_input` and `tailwind_output` from
/// `Dioxus.toml`, then exits.
#[derive(Clone, Debug, Parser)]
pub struct BuildTailwind {
    /// The package to generate tailwind CSS for
    #[clap(short, long)]
    pub(crate) package: Option<String>,

    /// The tailwind input file, relative to the current directory. Overrides `tailwind_input` in `Dioxus.toml`
    #[clap(long)]
    pub(crate) input: Option<PathBuf>,

    /// The tailwind output file, relative to the current directory. Overrides `tailwind_output` in `Dioxus.toml`
    #[clap(long)]
    pub(crate) output: Option<PathBuf>,
}

impl BuildTailwind {
    pub async fn run(self) -> Result<StructuredOutput> {
        let workspace = Workspace::current().await?;
        let crate_package = workspace.find_main_package(self.package)?;

        // Read inline `dioxus.toml` config from the binary's source, like `dx build`, when the
        // package has a single binary
        let mut bins = workspace.krates[crate_package]
            .targets
            .iter()
            .filter(|target| target.kind.contains(&TargetKind::Bin));
        let src_path = match (bins.next(), bins.next()) {
            (Some(bin), None) => Some(bin.src_path.as_std_path()),
            _ => None,
        };

        let config = workspace
            .load_dioxus_config(crate_package, src_path)?
            .unwrap_or_default();

        let manifest_dir = workspace.krates[crate_package]
            .manifest_path
            .parent()
            .unwrap()
            .to_path_buf()
            .into();

        let input = match self.input {
            Some(input) => Some(std::path::absolute(input)?),
            None => config.application.tailwind_input,
        };
        let output = match self.output {
            Some(output) => Some(std::path::absolute(output)?),
            None => config.application.tailwind_output,
        };

        TailwindCli::run_once(manifest_dir, input, output).await?;

        Ok(StructuredOutput::Success)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn cli_parses_tools_tailwind() {
        let cli = crate::Cli::try_parse_from([
            "dx",
            "tools",
            "tailwind",
            "--package",
            "app",
            "--input",
            "in.css",
            "--output",
            "out.css",
        ])
        .expect("tools tailwind should parse");

        let crate::Commands::Tools(crate::BuildTools::BuildTailwind(args)) = cli.action else {
            panic!("expected tools tailwind command");
        };
        assert_eq!(args.package.as_deref(), Some("app"));
        assert_eq!(args.input, Some("in.css".into()));
        assert_eq!(args.output, Some("out.css".into()));
    }
}
