#![cfg(feature = "server")]

use serde::{Deserialize, Serialize};

/// Supported IDE protocol handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdeKind {
    VsCode,
    Cursor,
    Windsurf,
    CustomScheme { scheme: String },
}

impl IdeKind {
    /// Returns the URI scheme associated with the IDE.
    pub fn scheme(&self) -> String {
        match self {
            IdeKind::VsCode => "vscode".to_string(),
            IdeKind::Cursor => "cursor".to_string(),
            IdeKind::Windsurf => "windsurf".to_string(),
            IdeKind::CustomScheme { scheme } => scheme.clone(),
        }
    }
}

/// Payload emitted by the browser client towards the dev server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorRequest {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub ide: IdeKind,
}

impl InspectorRequest {
    /// Builds a deep-link URI for the selected IDE.
    pub fn ide_uri(&self) -> String {
        format!("{}://file/{}:{}", self.ide.scheme(), self.file, self.line)
    }
}

/// Minimal HTTP client that forwards requests to the inspector middleware.
#[derive(Clone)]
pub struct InspectorServerClient {
    endpoint: String,
    http: reqwest::Client,
}

impl InspectorServerClient {
    /// Creates a new client pointing at the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Sends the inspector payload to the middleware.
    pub async fn send(&self, payload: &InspectorRequest) -> Result<(), reqwest::Error> {
        self.http
            .post(format!("{}/api/inspector/open", self.endpoint))
            .json(payload)
            .send()
            .await?
            .error_for_status()
            .map(|_| ())
    }
}
