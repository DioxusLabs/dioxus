use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Input rsx (selection)
    #[clap(short, long)]
    pub raw: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub file: Option<String>,
}

impl Autoformat {
    pub fn autoformat(self) -> Result<()> {
        if let Some(raw) = self.raw {
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw, 0) {
                println!("{}", inner);
            } else {
                // exit process with error
                eprintln!("error formatting codeblock");
                exit(1);
            }
        }

        if let Some(file) = self.file {
            let edits = dioxus_autofmt::fmt_file(&file);
            let as_json = serde_json::to_string(&edits).unwrap();
            println!("{}", as_json);
        }

        Ok(())
    }
}

#[test]
fn spawn_properly() {
    let out = Command::new("dioxus")
        .args([
            "fmt",
            "-f",
            r#"
//

rsx! {

    div {}
}

//
//
//

        "#,
        ])
        .output()
        .expect("failed to execute process");

    dbg!(out);
}
