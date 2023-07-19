use std::fs;

use super::{PipelineConfig, PipelineStep};
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
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building SASS/SCSS files");

        // Get sass
        let sass = tools::Sass::get()?.source_map(false);

        // Get sass staging
        let staging = config.staging_path().join(STAGING_OUT);
        fs::create_dir(&staging)?;

        let mut new_files = Vec::new();
        for file in config.files.iter() {
            // Check if file isn't sass. If true, skip.
            if file.file_type != FileType::Sass && file.file_type != FileType::Scss {
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

        config.files.append(&mut new_files);

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}
