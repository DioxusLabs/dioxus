use super::{
    util::{FileType, MessageSeverity, OutputMessage},
    PipelineContext, PipelineStep,
};

/// Builds an index.html file
pub struct ImageOpt {}

impl ImageOpt {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for ImageOpt {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        if !config.build_config.release {
            return Ok(());
        }

        let mut output_messages = Vec::new();
        
        config.set_message("Optimizing images");
        for file in &mut config.processed_files {
            if file.file_type != FileType::Image {
                continue;
            }

            let img = match image::open(&file.path) {
                Ok(i) => i,
                Err(e) => {
                    output_messages.push(OutputMessage::new(
                        MessageSeverity::Warn,
                        format!("failed to optmize image: {}", e.to_string()),
                    ));
                    continue;
                }
            };
            let path = file.path.join(format!("../{}.webp", file.name));
            if let Err(e) = img.save_with_format(path, image::ImageFormat::WebP) {
                output_messages.push(OutputMessage::new(
                    MessageSeverity::Warn,
                    format!("failed to optmize image: {}", e.to_string()),
                ));
                continue;
            }

            file.file_type = FileType::Webp
        }

        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::Medium
    }
}
