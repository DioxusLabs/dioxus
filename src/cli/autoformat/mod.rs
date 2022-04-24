use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Input file
    #[clap(short, long)]
    pub raw: Option<String>,
}

impl Autoformat {
    pub fn autoformat(self) -> Result<()> {
        if let Some(raw) = self.raw {
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw) {
                println!("{}", inner);
            } else {
                // exit process with error
                eprintln!("error formatting codeblock");
                exit(1);
            }
        }

        Ok(())
    }
}
