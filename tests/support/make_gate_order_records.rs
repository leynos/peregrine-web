//! Parse NUL-framed observations from controlled Make executors.

use std::{collections::BTreeMap, error::Error, io};

/// Boxed errors reported for malformed or incomplete executor records.
type Read<T> = Result<T, Box<dyn Error>>;

/// A selected environment variable, retaining whether it was absent or empty.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum EnvironmentValue {
    /// The variable was not present in the executor's environment.
    Unset,
    /// The variable was present with an empty value.
    Empty,
    /// The variable was present with this value.
    Value(String),
}

/// Presence of a secret variable without retaining its value.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum SecretPresence {
    /// No secret variable reached the executor.
    Absent,
    /// A secret variable reached the executor; its value is redacted.
    Present,
}

/// One complete observation from a Make-controlled executable.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Invocation {
    /// Executable filename selected by Make.
    pub(super) executable: String,
    /// Process working directory.
    pub(super) working_directory: String,
    /// Selected environment variables, including unset and empty states.
    pub(super) environment: BTreeMap<String, EnvironmentValue>,
    /// Secret variables are represented by presence only.
    pub(super) secrets: BTreeMap<String, SecretPresence>,
    /// Gate stage assigned by the controlled executable.
    pub(super) stage: String,
    /// Whitaker driver's private cache state, when applicable.
    pub(super) cache_state: String,
    /// Exact argument vector passed to the executable.
    pub(super) arguments: Vec<String>,
    /// Optional output captured from the real Cargo configuration query.
    pub(super) result: Option<String>,
}

/// Parses byte-oriented records while preserving every internal empty field.
pub(super) fn parse_invocations(bytes: &[u8]) -> Read<Vec<Invocation>> {
    let fields = invocation_fields(bytes)?;

    let mut index = 0;
    let mut invocations = Vec::new();
    while index < fields.len() {
        invocations.push(parse_invocation(&fields, &mut index)?);
    }
    Ok(invocations)
}

/// Splits the log and requires its final NUL framing delimiter.
fn invocation_fields(bytes: &[u8]) -> Read<Vec<&[u8]>> {
    let mut fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    if fields.last().is_some_and(|field| field.is_empty()) {
        fields.pop();
    } else if !fields.is_empty() {
        return invalid("NUL-framed invocation log must end with one NUL delimiter");
    }

    Ok(fields)
}

/// Parses one complete invocation and requires its closing marker.
fn parse_invocation(fields: &[&[u8]], index: &mut usize) -> Read<Invocation> {
    require_field(fields, index, "invocation-v1")?;
    let executable = next_field(fields, index)?.to_owned();
    let working_directory = next_field(fields, index)?.to_owned();
    let environment = parse_environment(fields, index)?;
    let secrets = parse_secrets(fields, index)?;

    require_field(fields, index, "stage")?;
    let stage = next_field(fields, index)?.to_owned();
    require_field(fields, index, "cache-state")?;
    let cache_state = next_field(fields, index)?.to_owned();
    let arguments = parse_arguments(fields, index)?;
    let result = parse_result(fields, index)?;
    require_field(fields, index, "end-invocation")?;
    Ok(Invocation {
        executable,
        working_directory,
        environment,
        secrets,
        stage,
        cache_state,
        arguments,
        result,
    })
}

/// Parses exactly the declared argument count, preserving empty arguments.
fn parse_arguments(fields: &[&[u8]], index: &mut usize) -> Read<Vec<String>> {
    require_field(fields, index, "argc")?;
    let argument_count = next_field(fields, index)?
        .parse::<usize>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    require_field(fields, index, "argv")?;
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(next_field(fields, index)?.to_owned());
    }
    Ok(arguments)
}

/// Parses optional query output while rejecting inconsistent result states.
fn parse_result(fields: &[&[u8]], index: &mut usize) -> Read<Option<String>> {
    require_field(fields, index, "result-state")?;
    let result_state = next_field(fields, index)?;
    require_field(fields, index, "result")?;
    let result_value = next_field(fields, index)?.to_owned();
    let result = match result_state {
        "none" if result_value.is_empty() => None,
        "value" => Some(result_value),
        other => return invalid(format!("invalid result state {other}")),
    };
    Ok(result)
}

/// Parses every required selected environment field in recorder order.
fn parse_environment(
    fields: &[&[u8]],
    index: &mut usize,
) -> Read<BTreeMap<String, EnvironmentValue>> {
    let mut environment = BTreeMap::new();
    for expected_name in ENVIRONMENT_NAMES {
        require_field(fields, index, "env")?;
        let name = next_field(fields, index)?;
        if name != *expected_name {
            return invalid(format!(
                "expected environment record {expected_name}, found {name}"
            ));
        }
        let state = next_field(fields, index)?;
        let value = next_field(fields, index)?;
        environment.insert(name.to_owned(), environment_value(state, value)?);
    }

    Ok(environment)
}

/// Parses redacted secret-presence fields in recorder order.
fn parse_secrets(fields: &[&[u8]], index: &mut usize) -> Read<BTreeMap<String, SecretPresence>> {
    let mut secrets = BTreeMap::new();
    for expected_name in ["CS_ACCESS_TOKEN", "GITHUB_TOKEN"] {
        require_field(fields, index, "secret")?;
        let name = next_field(fields, index)?;
        if name != expected_name {
            return invalid(format!(
                "expected secret record {expected_name}, found {name}"
            ));
        }
        let presence = match next_field(fields, index)? {
            "absent" => SecretPresence::Absent,
            "present" => SecretPresence::Present,
            other => return invalid(format!("unknown secret state {other}")),
        };
        secrets.insert(name.to_owned(), presence);
    }

    Ok(secrets)
}

/// Reads a field value and advances the byte-slice parser.
fn next_field<'a>(fields: &[&'a [u8]], index: &mut usize) -> Read<&'a str> {
    let field = fields.get(*index).ok_or_else(|| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete NUL-framed record")
    })?;
    *index += 1;
    Ok(std::str::from_utf8(field)?)
}

/// Requires a record marker before consuming its value.
fn require_field(fields: &[&[u8]], index: &mut usize, expected: &str) -> Read<()> {
    let found = next_field(fields, index)?;
    if found == expected {
        Ok(())
    } else {
        invalid(format!("expected record marker {expected}, found {found}"))
    }
}

/// Converts a selected environment triple without conflating empty and absent.
fn environment_value(state: &str, value: &str) -> Read<EnvironmentValue> {
    match state {
        "unset" if value.is_empty() => Ok(EnvironmentValue::Unset),
        "empty" if value.is_empty() => Ok(EnvironmentValue::Empty),
        "value" if !value.is_empty() => Ok(EnvironmentValue::Value(value.to_owned())),
        other => invalid(format!("invalid environment state {other}")),
    }
}

/// Creates a typed parse error for malformed or ambiguous executor records.
fn invalid<T>(message: impl Into<String>) -> Read<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, message.into()).into())
}

/// Environment fields emitted in the same fixed order by every recorder.
const ENVIRONMENT_NAMES: &[&str] = &[
    "WITH_ACT",
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
    "CARGO_BUILD_TARGET",
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
    "CFLAGS",
    "LDFLAGS",
    "DYLINT_RUSTFLAGS",
    "DYLINT_DRIVER_PATH",
    "PATH",
    "BUILD_TOOLS_PREFIX",
    "CURDIR",
    "GATE_FAIL_AT",
    "CARGO",
    "CHECK_BUILD_TOOLS",
    "WHITAKER",
    "MDTABLEFIX",
    "MDLINT",
    "ACT",
    "TYPOS_CONFIG_BUILDER",
];
