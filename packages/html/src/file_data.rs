pub trait HasFileData: std::any::Any {
    // NOTE: The methods of this trait are config'ed out instead of the trait
    // itself because several other traits inherit from this trait and there isn't a clean way to
    // conditionally inherit from a trait based on a config.
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        None
    }
}

#[cfg(feature = "serialize")]
#[cfg(feature = "file-engine")]
/// A file engine that serializes files to bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedFileEngine {
    pub files: std::collections::HashMap<String, Vec<u8>>,
}

#[cfg(feature = "serialize")]
#[cfg(feature = "file-engine")]
#[async_trait::async_trait(?Send)]
impl FileEngine for SerializedFileEngine {
    fn files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    async fn file_size(&self, file: &str) -> Option<u64> {
        let file = self.files.get(file)?;
        Some(file.len() as u64)
    }

    async fn read_file(&self, file: &str) -> Option<Vec<u8>> {
        self.files.get(file).cloned()
    }

    async fn read_file_to_string(&self, file: &str) -> Option<String> {
        self.read_file(file)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }

    async fn get_native_file(&self, file: &str) -> Option<Box<dyn std::any::Any>> {
        self.read_file(file)
            .await
            .map(|val| Box::new(val) as Box<dyn std::any::Any>)
    }
}

#[cfg(feature = "file-engine")]
#[async_trait::async_trait(?Send)]
pub trait FileEngine {
    // get a list of file names
    fn files(&self) -> Vec<String>;

    // get the size of a file
    async fn file_size(&self, file: &str) -> Option<u64>;

    // read a file to bytes
    async fn read_file(&self, file: &str) -> Option<Vec<u8>>;

    // read a file to string
    async fn read_file_to_string(&self, file: &str) -> Option<String>;

    // returns a file in platform's native representation
    async fn get_native_file(&self, file: &str) -> Option<Box<dyn std::any::Any>>;
}
