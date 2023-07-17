use super::{PipelineConfig, PipelineStep};

const INDEX_FILE: &str = "./index.html";

/// Builds an index.html file 
pub struct IndexFile {}

impl IndexFile {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for IndexFile {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building `index.html` file...");

        log::info!("Finished building `index.html` file.");
        Ok(())
    }
}
