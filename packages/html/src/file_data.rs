pub trait HasFileData: std::any::Any {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        None
    }
}

#[cfg(feature = "serialize")]
/// A file engine that serializes files to bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedFileEngine {
    #[cfg(feature = "file-engine")]
    pub files: std::collections::HashMap<String, Vec<u8>>,
}

#[cfg(feature = "serialize")]
#[cfg_attr(feature="file-engine", async_trait::async_trait(?Send))]
impl FileEngine for SerializedFileEngine {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    #[cfg(feature = "file-engine")]
    async fn file_size(&self, file: &str) -> Option<u64> {
        let file = self.files.get(file)?;
        Some(file.len() as u64)
    }

    #[cfg(feature = "file-engine")]
    async fn read_file(&self, file: &str) -> Option<Vec<u8>> {
        self.files.get(file).cloned()
    }

    #[cfg(feature = "file-engine")]
    async fn read_file_to_string(&self, file: &str) -> Option<String> {
        self.read_file(file)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }

    #[cfg(feature = "file-engine")]
    async fn get_native_file(&self, file: &str) -> Option<Box<dyn std::any::Any>> {
        self.read_file(file)
            .await
            .map(|val| Box::new(val) as Box<dyn std::any::Any>)
    }
}

#[cfg_attr(feature="file-engine", async_trait::async_trait(?Send))]
pub trait FileEngine {
    #[cfg(feature = "file-engine")]
    // get a list of file names
    fn files(&self) -> Vec<String>;

    #[cfg(feature = "file-engine")]
    // get the size of a file
    async fn file_size(&self, file: &str) -> Option<u64>;

    #[cfg(feature = "file-engine")]
    // read a file to bytes
    async fn read_file(&self, file: &str) -> Option<Vec<u8>>;

    #[cfg(feature = "file-engine")]
    // read a file to string
    async fn read_file_to_string(&self, file: &str) -> Option<String>;

    #[cfg(feature = "file-engine")]
    // returns a file in platform's native representation
    async fn get_native_file(&self, file: &str) -> Option<Box<dyn std::any::Any>>;
}
