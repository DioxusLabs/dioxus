use std::fs;

use super::{PipelineContext, PipelineStep};
use crate::{
    pipeline::util::{File, FileType},
    tools,
};

const STAGING_OUT: &str = "./sass";

/// Generates CSS files from any SASS or SCSS files.
pub struct SassBuild {}

impl SassBuild {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for SassBuild {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        // Get sass
        config.set_message("Installing dart-sass");
        let sass = tools::Sass::get()?.source_map(false);
        config.set_message("Building sass/scss");
        
        // Get sass staging
        let staging = config.staging_path().join(STAGING_OUT);
        fs::create_dir(&staging)?;

        let mut new_files = Vec::new();
        for file in config.raw_files.iter() {
            // Check if file isn't sass. If true, skip.
            if file.file_type != FileType::SassType {
                continue;
            }

            // Run sass
            let out_path = staging.join(format!("{}.{}", file.name.clone(), "css"));
            let in_path = &file.path;
            sass.run(in_path.clone(), out_path.clone())?;

            new_files.push(File {
                name: file.name.clone(),
                path: out_path,
                file_type: FileType::Css,
            });
        }

        config.processed_files.append(&mut new_files);

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineContext) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}
