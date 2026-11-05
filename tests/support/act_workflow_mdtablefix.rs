//! The mdtablefix version the hosted workflow installs and the manual one must match.

use serde_yaml::Value;

use super::{ANY_VALUE, CI_WORKFLOW, action_input, at, document, text};

/// The mdtablefix action's path, without a revision.
const MDTABLEFIX_ACTION: &str = "leynos/shared-actions/.github/actions/install-mdtablefix";

/// Reads the version the named job gives its mdtablefix install step.
pub(crate) fn version(workflow: &Value, job: &str) -> Option<String> {
    at(workflow, &["jobs", job, "steps"])?
        .as_sequence()?
        .iter()
        .find_map(|item| {
            let uses = text(item, &["uses"])?;
            (uses.split('@').next() == Some(MDTABLEFIX_ACTION)).then_some(())?;
            action_input(item, "version")
        })
}

/// The hosted workflow is the source of the mdtablefix version; the manual
/// Act workflow must install the same one.
pub(crate) fn versions_agree(ci: &Value, act: &Value) -> bool {
    version(ci, "build-test").is_some_and(|hosted| {
        !hosted.is_empty() && version(act, "act-validation").as_deref() == Some(hosted.as_str())
    })
}

/// The value an action fixture must receive for a contract input: a marked
/// input takes the hosted workflow's mdtablefix version.
pub(crate) fn expected_input(expected: &str) -> String {
    if expected != ANY_VALUE {
        return expected.to_owned();
    }
    document(CI_WORKFLOW)
        .ok()
        .and_then(|ci| version(&ci, "build-test"))
        .unwrap_or_default()
}
