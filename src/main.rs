use dioxus_studio::*;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Trunk::from_args();
    set_up_logging();

    match args.action {
        TrunkSubcommands::Build(_) => {
            //
        }

        TrunkSubcommands::Translate(opts) => {
            // let TranslateOptions {
            //     file,
            //     text,
            //     component,
            // } = cfg;

            // match component {
            //     true => {
            //         let f = helpers::to_component::convert_html_to_component(&text.unwrap())?;
            //         println!("{}", f);
            //     }
            //     false => {
            //         let renderer = match (file, text) {
            //             (None, Some(text)) => translate::translate_from_html_to_rsx(&text, false)?,
            //             (Some(file), None) => translate::translate_from_html_file(&file)?,
            //             _ => panic!("Must select either file or text - not both or none!"),
            //         };

            //         println!("{}", renderer);
            //     }
            // }
        }

        TrunkSubcommands::Clean(_) => {
            //
        }
        TrunkSubcommands::Config(_) => {
            //
        }
        TrunkSubcommands::Serve(_) => {
            //
        }
    }

    Ok(())
}
