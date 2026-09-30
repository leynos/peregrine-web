//! Typed fields used when checking the committed Act workflow.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Parsed workflow jobs.
#[derive(Deserialize)]
pub(super) struct Workflow {
    /// Jobs indexed by their workflow identifiers.
    pub(super) jobs: BTreeMap<String, Job>,
}

/// Steps belonging to one workflow job.
#[derive(Deserialize)]
pub(super) struct Job {
    /// Steps in execution order.
    pub(super) steps: Vec<Step>,
}

/// Fields needed to check provisioning and command order.
#[derive(Deserialize)]
pub(super) struct Step {
    /// Human-readable step label.
    pub(super) name: Option<String>,
    /// GitHub expression guarding the step.
    #[serde(rename = "if")]
    pub(super) condition: Option<String>,
    /// Shell script executed by the step.
    pub(super) run: Option<String>,
    /// Referenced external action, if any.
    pub(super) uses: Option<String>,
    /// Action inputs, retained as YAML values.
    #[serde(default)]
    pub(super) with: BTreeMap<String, serde_yaml::Value>,
}
