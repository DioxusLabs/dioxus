use std::fs;

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};

use crate::Error;

use super::{util::FileType, PipelineContext, PipelineStep};

/// Builds an index.html file
pub struct Minify {}

impl Minify {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for Minify {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        if !config.build_config.release {
            return Ok(());
        }

        // Minify CSS
        config.set_message("Minifying CSS");
        for file in config.processed_files.iter() {
            if file.file_type != FileType::Css {
                continue;
            }

            let raw = fs::read_to_string(&file.path)?;
            let mut stylesheet = StyleSheet::parse(&raw, ParserOptions::default())
                .map_err(|e| Error::BuildFailed(e.to_string()))?;

            stylesheet
                .minify(MinifyOptions::default())
                .map_err(|e| Error::BuildFailed(e.to_string()))?;

            let minified = stylesheet
                .to_css(PrinterOptions {
                    minify: true,
                    ..Default::default()
                })
                .map_err(|e| Error::BuildFailed(e.to_string()))?;

            fs::write(&file.path, minified.code)?;
        }

        // Minify JS
        config.set_message("Minifying JS");
        for file in config.processed_files.iter() {
            if file.file_type != FileType::JavaScript {
                continue;
            }

            let raw = fs::read(&file.path)?;
            let mut minified = Vec::new();
            let session = minify_js::Session::new();

            minify_js::minify(
                &session,
                minify_js::TopLevelMode::Module,
                &raw,
                &mut minified,
            )
            .map_err(|e| Error::ParseError(e.to_string()))?;

            fs::write(&file.path, minified)?;
        }

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineContext) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::Medium
    }
}
