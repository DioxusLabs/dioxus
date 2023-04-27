use std::path::PathBuf;

use crate::FileEngine;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct NativeFileEngine {
    files: Vec<PathBuf>,
}

impl NativeFileEngine {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self { files }
    }
}

#[async_trait::async_trait(?Send)]
impl FileEngine for NativeFileEngine {
    fn files(&self) -> Vec<String> {
        self.files
            .iter()
            .filter_map(|f| Some(f.to_str()?.to_string()))
            .collect()
    }

    async fn read_file(&self, file: &str) -> Option<Vec<u8>> {
        let mut file = File::open(file).await.ok()?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.ok()?;

        Some(contents)
    }

    async fn read_file_to_string(&self, file: &str) -> Option<String> {
        let mut file = File::open(file).await.ok()?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await.ok()?;

        Some(contents)
    }
}
