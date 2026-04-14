pub struct DataTransfer {
    inner: Box<dyn NativeDataTransfer>,
}

impl DataTransfer {
    pub fn new(inner: impl NativeDataTransfer + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    #[cfg(feature = "serialize")]
    pub fn store(&self, item: impl Serialize) -> Result<(), String> {
        let serialized = serde_json::to_string(&item).map_err(|e| e.to_string())?;
        self.set_data("application/json", &serialized)
    }

    #[cfg(feature = "serialize")]
    pub fn retrieve<T: for<'de> serde::Deserialize<'de>>(&self) -> Result<Option<T>, String> {
        if let Some(data) = self.get_data("application/json") {
            let deserialized = serde_json::from_str(&data).map_err(|e| e.to_string())?;
            Ok(Some(deserialized))
        } else {
            Ok(None)
        }
    }

    pub fn get_data(&self, format: &str) -> Option<String> {
        self.inner.get_data(format)
    }

    pub fn get_as_text(&self) -> Option<String> {
        self.get_data("text/plain")
    }

    pub fn set_data(&self, format: &str, data: &str) -> Result<(), String> {
        self.inner.set_data(format, data)
    }

    pub fn clear_data(&self, format: Option<&str>) -> Result<(), String> {
        self.inner.clear_data(format)
    }

    pub fn effect_allowed(&self) -> String {
        self.inner.effect_allowed()
    }

    pub fn set_effect_allowed(&self, effect: &str) {
        self.inner.set_effect_allowed(effect)
    }

    pub fn drop_effect(&self) -> String {
        self.inner.drop_effect()
    }

    pub fn set_drop_effect(&self, effect: &str) {
        self.inner.set_drop_effect(effect)
    }

    pub fn files(&self) -> Vec<crate::file_data::FileData> {
        self.inner.files()
    }
}

pub trait NativeDataTransfer: Send + Sync {
    fn get_data(&self, format: &str) -> Option<String>;
    fn set_data(&self, format: &str, data: &str) -> Result<(), String>;
    fn clear_data(&self, format: Option<&str>) -> Result<(), String>;
    fn effect_allowed(&self) -> String;
    fn set_effect_allowed(&self, effect: &str);
    fn drop_effect(&self) -> String;
    fn set_drop_effect(&self, effect: &str);
    fn files(&self) -> Vec<crate::file_data::FileData>;
}

pub trait HasDataTransferData {
    fn data_transfer(&self) -> DataTransfer;
}

#[cfg(feature = "serialize")]
pub use ser::*;
#[cfg(feature = "serialize")]
use serde::Serialize;

#[cfg(feature = "serialize")]
mod ser {
    use crate::DragData;

    use super::*;
    use serde::{Deserialize, Serialize};

    /// A serialized version of DataTransfer
    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedDataTransfer {
        pub items: Vec<SerializedDataTransferItem>,
        pub files: Vec<crate::file_data::SerializedFileData>,
        pub effect_allowed: String,
        pub drop_effect: String,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedDataTransferItem {
        pub kind: String,
        pub type_: String,
        pub data: String,
    }

    impl NativeDataTransfer for SerializedDataTransfer {
        fn get_data(&self, format: &str) -> Option<String> {
            self.items
                .iter()
                .find(|item| item.type_ == format)
                .map(|item| item.data.clone())
        }

        fn set_data(&self, _format: &str, _data: &str) -> Result<(), String> {
            // todo!()
            // Err("Cannot set data on serialized DataTransfer".into())
            Ok(())
        }

        fn clear_data(&self, _format: Option<&str>) -> Result<(), String> {
            // todo!()
            // Err("Cannot clear data on serialized DataTransfer".into())
            Ok(())
        }

        fn effect_allowed(&self) -> String {
            self.effect_allowed.clone()
        }

        fn set_effect_allowed(&self, _effect: &str) {
            // No-op
        }

        fn drop_effect(&self) -> String {
            self.drop_effect.clone()
        }

        fn set_drop_effect(&self, _effect: &str) {
            // No-op
        }

        fn files(&self) -> Vec<crate::file_data::FileData> {
            self.files
                .iter()
                .map(|f| crate::file_data::FileData::new(f.clone()))
                .collect()
        }
    }

    impl From<&DragData> for SerializedDataTransfer {
        fn from(_drag: &DragData) -> Self {
            // todo!()
            Self {
                items: vec![],
                files: vec![],
                effect_allowed: "all".into(),
                drop_effect: "none".into(),
            }
        }
    }
}
