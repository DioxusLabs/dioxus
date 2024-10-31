use std::collections::HashMap;

use bytes::Bytes;
use http::HeaderMap;

pub struct RenderChunk {
    pub contents: String,
    pub headers: HeaderMap,
}

impl RenderChunk {
    pub fn from_contents(contents: String) -> Self {
        Self {
            contents,
            headers: Default::default(),
        }
    }
}

// Such that we can stream this directly into the body response
impl Into<Bytes> for RenderChunk {
    fn into(self) -> Bytes {
        self.contents.into()
    }
}
