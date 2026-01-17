use std::process::Command;

use schemars::{schema_for, JsonSchema, Schema};
use serde::{Deserialize, Serialize};

/// A component compatible with the dioxus components system.
/// This may be a "virtual" component which is empty except for a list of members.
#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component_dependencies: Vec<ComponentDependency>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_dependencies: Vec<CargoDependency>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub global_assets: Vec<String>,
}

/// A dependency on another component, either built-in or third-party.
#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ComponentDependency {
    Builtin(String),
    ThirdParty {
        name: String,
        git: String,
        #[serde(default)]
        rev: Option<String>,
    },
}

/// A dependency on a cargo crate required for a component.
#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum CargoDependency {
    Simple(String),
    Detailed {
        name: String,
        #[serde(default)]
        version: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        default_features: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
    },
}

impl CargoDependency {
    /// Get the `cargo add` command for this dependency.
    pub fn add_command(&self) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.arg("add");
        match self {
            CargoDependency::Simple(name) => {
                cmd.arg(name);
            }
            CargoDependency::Detailed {
                name,
                version,
                features,
                default_features,
                git,
                rev,
            } => {
                cmd.arg(format!(
                    "{name}{}",
                    version
                        .as_ref()
                        .map(|version| format!("@{version}"))
                        .unwrap_or_default()
                ));
                if !features.is_empty() {
                    cmd.arg("--features").arg(features.join(","));
                }
                if !*default_features {
                    cmd.arg("--no-default-features");
                }
                if let Some(git) = git {
                    cmd.arg("--git").arg(git);
                }
                if let Some(rev) = rev {
                    cmd.arg("--rev").arg(rev);
                }
            }
        }
        cmd
    }

    /// Get the name of the dependency.
    pub fn name(&self) -> &str {
        match self {
            CargoDependency::Simple(name) => name,
            CargoDependency::Detailed { name, .. } => name,
        }
    }
}

/// Get the JSON schema for the `Component` struct.
pub fn component_manifest_schema() -> Schema {
    schema_for!(Component)
}
