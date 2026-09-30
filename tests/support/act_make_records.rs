//! Parses lossless NUL-delimited records emitted by the Make command fixtures.

use std::{collections::BTreeMap, io};

/// The observed state of a selected non-secret process environment variable.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum EnvironmentValue {
    /// The variable was absent from the executor environment.
    Unset,
    /// The variable was present with an empty value.
    Empty,
    /// The variable was present with this non-empty value.
    Value(String),
}

/// Whether the Act stub received a non-empty token, without recording its value.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum GitHubToken {
    /// No token variable reached the executor.
    Unset,
    /// A token variable reached the executor with an empty value.
    Empty,
    /// A non-empty token reached the executor; its value is deliberately redacted.
    Present,
}

/// A complete record of one controlled executor invocation.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Invocation {
    /// Executable filename selected by Make.
    pub(super) executable: String,
    /// Exact argument vector passed to the executable.
    pub(super) arguments: Vec<String>,
    /// Working directory observed by the executable.
    pub(super) working_directory: String,
    /// Captured state of selected environment variables.
    pub(super) environment: BTreeMap<String, EnvironmentValue>,
    /// Token-presence state, with no token value stored.
    pub(super) github_token: GitHubToken,
}

/// Parses one or more versioned executor records from the NUL-delimited log.
pub(super) fn parse_invocations(text: &str) -> io::Result<Vec<Invocation>> {
    let fields = text.split_terminator('\0').collect::<Vec<_>>();
    let mut index = 0;
    let mut invocations = Vec::new();
    while index < fields.len() {
        invocations.push(parse_invocation(&fields, &mut index)?);
    }
    Ok(invocations)
}

/// Parses one complete invocation and requires its closing marker.
fn parse_invocation(fields: &[&str], index: &mut usize) -> io::Result<Invocation> {
    require_field(fields, index, "invocation-v1")?;
    let executable = next_field(fields, index)?.to_owned();
    let working_directory = next_field(fields, index)?.to_owned();
    let environment = parse_environment(fields, index)?;
    let github_token = parse_token(fields, index)?;
    let arguments = parse_arguments(fields, index)?;
    require_field(fields, index, "end-invocation")?;
    Ok(Invocation {
        executable,
        arguments,
        working_directory,
        environment,
        github_token,
    })
}

/// Parses the fixed environment sequence without losing absent or empty values.
fn parse_environment(
    fields: &[&str],
    index: &mut usize,
) -> io::Result<BTreeMap<String, EnvironmentValue>> {
    let mut environment = BTreeMap::new();
    for expected_name in [
        "WITH_ACT",
        "ACT",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    ] {
        require_field(fields, index, "env")?;
        let name = next_field(fields, index)?;
        if name != expected_name {
            return Err(io::Error::other(format!(
                "expected environment record {expected_name}, found {name}"
            )));
        }
        let state = next_field(fields, index)?;
        let raw_value = next_field(fields, index)?;
        let environment_value = parse_environment_value(state, raw_value)?;
        environment.insert(name.to_owned(), environment_value);
    }
    Ok(environment)
}

/// Converts the recorded environment state without changing field consumption.
fn parse_environment_value(state: &str, raw_value: &str) -> io::Result<EnvironmentValue> {
    let value = match state {
        "unset" => EnvironmentValue::Unset,
        "empty" => EnvironmentValue::Empty,
        "value" => EnvironmentValue::Value(raw_value.to_owned()),
        other => {
            return Err(io::Error::other(format!(
                "unknown environment state {other}"
            )));
        }
    };
    Ok(value)
}

/// Parses the redacted GitHub token-presence field.
fn parse_token(fields: &[&str], index: &mut usize) -> io::Result<GitHubToken> {
    require_field(fields, index, "secret")?;
    require_field(fields, index, "GITHUB_TOKEN")?;
    let token = match next_field(fields, index)? {
        "unset" => GitHubToken::Unset,
        "empty" => GitHubToken::Empty,
        "present" => GitHubToken::Present,
        other => return Err(io::Error::other(format!("unknown token state {other}"))),
    };
    Ok(token)
}

/// Parses exactly the declared argument count and preserves internal empty fields.
fn parse_arguments(fields: &[&str], index: &mut usize) -> io::Result<Vec<String>> {
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

/// Reads the next NUL-delimited field and advances the parser position.
fn next_field<'a>(fields: &[&'a str], index: &mut usize) -> io::Result<&'a str> {
    let field = fields
        .get(*index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete command record"))?;
    *index += 1;
    Ok(field)
}

/// Requires a field marker and advances the parser position on success.
fn require_field(fields: &[&str], index: &mut usize, expected: &str) -> io::Result<()> {
    let found = next_field(fields, index)?;
    if found == expected {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "expected command record marker {expected}, found {found}"
        )))
    }
}
