use super::{PipelineContext, PipelineStep};

const INDEX_FILE: &str = "./index.html";

/// Builds an index.html file
pub struct IndexFile {}

impl IndexFile {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for IndexFile {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}
