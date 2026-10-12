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
    let mut cursor = RecordCursor {
        fields: text.split_terminator('\0').collect(),
        index: 0,
    };
    let mut invocations = Vec::new();
    while cursor.index < cursor.fields.len() {
        invocations.push(parse_invocation(&mut cursor)?);
    }
    Ok(invocations)
}

/// Parses one complete invocation and requires its closing marker.
fn parse_invocation(cursor: &mut RecordCursor<'_>) -> io::Result<Invocation> {
    cursor.require_field("invocation-v1")?;
    let executable = cursor.next_field()?.to_owned();
    let working_directory = cursor.next_field()?.to_owned();
    let environment = parse_environment(cursor)?;
    let github_token = parse_token(cursor)?;
    let arguments = parse_arguments(cursor)?;
    cursor.require_field("end-invocation")?;
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
    cursor: &mut RecordCursor<'_>,
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
        cursor.require_field("env")?;
        let name = cursor.next_field()?;
        if name != expected_name {
            return Err(io::Error::other(format!(
                "expected environment record {expected_name}, found {name}"
            )));
        }
        let state = cursor.next_field()?;
        let raw_value = cursor.next_field()?;
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
fn parse_token(cursor: &mut RecordCursor<'_>) -> io::Result<GitHubToken> {
    cursor.require_field("secret")?;
    cursor.require_field("GITHUB_TOKEN")?;
    let token = match cursor.next_field()? {
        "unset" => GitHubToken::Unset,
        "empty" => GitHubToken::Empty,
        "present" => GitHubToken::Present,
        other => return Err(io::Error::other(format!("unknown token state {other}"))),
    };
    Ok(token)
}

/// Parses exactly the declared argument count and preserves internal empty fields.
fn parse_arguments(cursor: &mut RecordCursor<'_>) -> io::Result<Vec<String>> {
    cursor.require_field("argc")?;
    let argument_count = cursor
        .next_field()?
        .parse::<usize>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    cursor.require_field("argv")?;
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(cursor.next_field()?.to_owned());
    }
    Ok(arguments)
}

/// Owns borrowed log fields and their current parsing position.
struct RecordCursor<'a> {
    /// Fields split with the existing NUL terminator semantics.
    fields: Vec<&'a str>,
    /// Position of the next unconsumed field.
    index: usize,
}

impl<'a> RecordCursor<'a> {
    /// Reads an existing field and advances; exhaustion leaves the position intact.
    fn next_field(&mut self) -> io::Result<&'a str> {
        let field = self.fields.get(self.index).ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete command record")
        })?;
        self.index += 1;
        Ok(field)
    }

    /// Consumes a marker, including an existing marker that fails comparison.
    fn require_field(&mut self, expected: &str) -> io::Result<()> {
        let found = self.next_field()?;
        if found == expected {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "expected command record marker {expected}, found {found}"
            )))
        }
    }
}

/// Protocol and cursor contracts beside their private parser implementation.
#[cfg(test)]
#[path = "act_make_parser_tests.rs"]
mod parser_tests;
