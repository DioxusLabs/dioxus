use super::*;
use crate::Workspace;
use anyhow::Context;
use dioxus_autofmt::{IndentOptions, IndentType};
use rayon::prelude::*;
use std::{borrow::Cow, fs, path::Path};

#[derive(Clone, Debug, Parser)]
pub struct McpServer {}

impl McpServer {
    pub async fn mcp_server(self) -> Result<StructuredOutput> {
        todo!()
    }
}
