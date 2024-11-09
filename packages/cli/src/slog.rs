use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::Platform;

/// The structured output for the CLI
///
/// This is designed such that third party tools can reliably consume the output of the CLI when
/// outputting json.
///
/// Not every log outputted will be parsable, but all structued logs should be.
///
/// This means the debug format of this log needs to be parsable json, not the default debug format.
///
/// We guarantee that the last line of the command represents the success of the command, such that
/// tools can simply parse the last line of the output.
///
/// There might be intermediate lines that are parseable as structured logs (which you can put here)
/// but they are not guaranteed to be, such that we can provide better error messages for the user.
#[derive(Serialize, Deserialize)]
pub enum StructuredOutput {
    BuildFinished {},
    BundleOutput {
        platform: Platform,
        bundles: Vec<PathBuf>,
    },
    GenericSuccess,
    Error {
        message: String,
    },
}

impl std::fmt::Debug for StructuredOutput {
    // todo(jon): I think to_string can write directly to the formatter?
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = serde_json::to_string(self).unwrap();
        f.write_str(&out)
    }
}
