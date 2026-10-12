//! Exact parser acceptance, rejection, and cursor-consumption contracts.

use std::io;

use proptest::prelude::*;
use rstest::rstest;

use super::{EnvironmentValue, GitHubToken, RecordCursor, parse_invocations};

/// Correlated wire fields used to construct one emitter-compatible record.
struct RecordInput<'a> {
    environment_state: &'a str,
    raw_environment: &'a str,
    token_state: &'a str,
    arguments: &'a [&'a str],
}

/// A named environment-state case keeps protocol input with its expected value.
#[derive(Debug)]
struct EnvironmentCase {
    state: &'static str,
    raw: &'static str,
    expected: EnvironmentValue,
}

/// A named token-state case keeps redaction input with its expected presence.
#[derive(Debug)]
struct TokenCase {
    state: &'static str,
    expected: GitHubToken,
}

/// A malformed field and its expected parser diagnostic.
#[derive(Debug)]
struct InvalidFieldCase {
    index: usize,
    value: &'static str,
    message: &'static str,
}

/// An invalid argument-count spelling used by a parser regression.
#[derive(Debug)]
struct InvalidCountCase {
    value: &'static str,
}

/// A declared count whose value disagrees with the actual argv fields.
#[derive(Debug)]
struct CountMismatchCase {
    count: &'static str,
    arguments: Vec<&'static str>,
    kind: io::ErrorKind,
    message: &'static str,
}

/// Builds the fixed emitter field sequence with controlled protocol constituents.
fn fields(input: &RecordInput<'_>) -> Vec<String> {
    let mut result = ["invocation-v1", "executor", "/workspace"]
        .map(str::to_owned)
        .to_vec();
    for name in [
        "WITH_ACT",
        "ACT",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    ] {
        result.extend(
            ["env", name, input.environment_state, input.raw_environment].map(str::to_owned),
        );
    }
    result.extend(["secret", "GITHUB_TOKEN", input.token_state, "argc"].map(str::to_owned));
    result.push(input.arguments.len().to_string());
    result.push("argv".to_owned());
    result.extend(input.arguments.iter().map(|value| (*value).to_owned()));
    result.push("end-invocation".to_owned());
    result
}

/// Builds the common empty invocation used by marker and stream regressions.
fn empty_invocation_fields() -> Vec<String> {
    fields(&RecordInput {
        environment_state: "unset",
        raw_environment: "",
        token_state: "unset",
        arguments: &[],
    })
}

/// Joins record fields with the emitter's final NUL terminator.
fn log(fields: &[String]) -> String { format!("{}\0", fields.join("\0")) }

/// All environment states retain historical raw-value acceptance.
#[rstest]
#[case::unset(EnvironmentCase { state: "unset", raw: "ignored", expected: EnvironmentValue::Unset })]
#[case::empty(EnvironmentCase { state: "empty", raw: "ignored", expected: EnvironmentValue::Empty })]
#[case::empty_value(EnvironmentCase { state: "value", raw: "", expected: EnvironmentValue::Value(String::new()) })]
#[case::raw_value(EnvironmentCase { state: "value", raw: "a\tb\nc", expected: EnvironmentValue::Value("a\tb\nc".to_owned()) })]
fn environment_states_preserve_raw_value_semantics(#[case] case: EnvironmentCase) {
    let records = parse_invocations(&log(&fields(&RecordInput {
        environment_state: case.state,
        raw_environment: case.raw,
        token_state: "unset",
        arguments: &[],
    })))
    .expect("valid environment records must parse");
    let record = records.first().expect("one invocation must be returned");
    assert_eq!(
        record.environment.len(),
        7,
        "all seven ordered environment fields must be retained"
    );
    for actual in record.environment.values() {
        assert_eq!(
            actual, &case.expected,
            "the parser must preserve the selected environment state"
        );
    }
}

/// Token states are redacted independently from environment values.
#[rstest]
#[case::unset(TokenCase { state: "unset", expected: GitHubToken::Unset })]
#[case::empty(TokenCase { state: "empty", expected: GitHubToken::Empty })]
#[case::present(TokenCase { state: "present", expected: GitHubToken::Present })]
fn token_states_remain_distinct(#[case] case: TokenCase) {
    let records = parse_invocations(&log(&fields(&RecordInput {
        environment_state: "unset",
        raw_environment: "",
        token_state: case.state,
        arguments: &[],
    })))
    .expect("valid token records must parse");
    assert_eq!(
        records.first().map(|record| &record.github_token),
        Some(&case.expected),
        "the token presence state must remain distinct"
    );
}

/// Zero arguments and internal empty, tabbed, or multiline arguments remain exact.
#[rstest]
#[case(vec![])]
#[case(vec!["", "tab\targument", "line\nargument", ""])]
fn argument_boundaries_are_lossless(#[case] arguments: Vec<&str>) {
    let records = parse_invocations(&log(&fields(&RecordInput {
        environment_state: "unset",
        raw_environment: "",
        token_state: "unset",
        arguments: &arguments,
    })))
    .expect("valid argument records must parse");
    assert_eq!(
        records.first().map(|record| &record.arguments),
        Some(
            &arguments
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        ),
        "argument values and empty boundaries must remain exact"
    );
}

/// Empty logs, concatenated records, and missing final NUL retain their acceptance.
#[test]
fn record_stream_boundaries_preserve_acceptance() {
    assert!(
        parse_invocations("")
            .expect("empty logs must parse")
            .is_empty(),
        "empty logs must contain no invocation"
    );
    let record = log(&empty_invocation_fields());
    assert_eq!(
        parse_invocations(&format!("{record}{record}"))
            .expect("two records must parse")
            .len(),
        2,
        "both appended records must be retained"
    );
    assert_eq!(
        parse_invocations(record.trim_end_matches('\0'))
            .expect("historical missing final NUL must remain accepted")
            .len(),
        1,
        "the complete unterminated final record must remain accepted"
    );
}

/// Known protocol corruption retains the exact marker and state diagnostics.
#[rstest]
#[case::invocation_marker(InvalidFieldCase { index: 0, value: "wrong", message: "expected command record marker invocation-v1, found wrong" })]
#[case::environment_marker(InvalidFieldCase { index: 3, value: "wrong", message: "expected command record marker env, found wrong" })]
#[case::environment_name(InvalidFieldCase { index: 4, value: "ACT", message: "expected environment record WITH_ACT, found ACT" })]
#[case::environment_state(InvalidFieldCase { index: 5, value: "unknown", message: "unknown environment state unknown" })]
#[case::secret_marker(InvalidFieldCase { index: 31, value: "wrong", message: "expected command record marker secret, found wrong" })]
#[case::token_name(InvalidFieldCase { index: 32, value: "wrong", message: "expected command record marker GITHUB_TOKEN, found wrong" })]
#[case::token_state(InvalidFieldCase { index: 33, value: "unknown", message: "unknown token state unknown" })]
#[case::count_marker(InvalidFieldCase { index: 34, value: "wrong", message: "expected command record marker argc, found wrong" })]
#[case::arguments_marker(InvalidFieldCase { index: 36, value: "wrong", message: "expected command record marker argv, found wrong" })]
#[case::end_marker(InvalidFieldCase { index: 37, value: "wrong", message: "expected command record marker end-invocation, found wrong" })]
fn invalid_markers_and_states_are_rejected(#[case] case: InvalidFieldCase) {
    let mut record = empty_invocation_fields();
    *record
        .get_mut(case.index)
        .expect("fixture field index must exist") = case.value.to_owned();
    let error = parse_invocations(&log(&record)).expect_err("invalid protocol records must fail");
    assert_eq!(
        error.kind(),
        io::ErrorKind::Other,
        "marker and state diagnostics must retain their error kind"
    );
    assert_eq!(
        error.to_string(),
        case.message,
        "protocol diagnostics must remain exact"
    );
}

/// Invalid declared argument counts retain integer parsing errors.
#[rstest]
#[case::nonnumeric(InvalidCountCase { value: "not-a-number" })]
#[case::negative(InvalidCountCase { value: "-1" })]
fn invalid_argument_counts_are_rejected(#[case] case: InvalidCountCase) {
    let mut record = empty_invocation_fields();
    *record.get_mut(35).expect("argc value field must exist") = case.value.to_owned();
    let error = parse_invocations(&log(&record)).expect_err("invalid argument counts must fail");
    assert_eq!(
        error.kind(),
        io::ErrorKind::InvalidData,
        "integer parse errors must retain InvalidData"
    );
    assert_eq!(
        error.to_string(),
        case.value
            .parse::<usize>()
            .expect_err("fixture count must be invalid")
            .to_string(),
        "integer parsing diagnostics must be preserved"
    );
}

/// Every truncated field sequence, including a missing closing marker, fails at EOF.
#[rstest]
#[case(1)]
#[case(4)]
#[case(32)]
#[case(35)]
#[case(37)]
fn truncated_records_preserve_eof_diagnostics(#[case] length: usize) {
    let mut record = empty_invocation_fields();
    record.truncate(length);
    let error = parse_invocations(&log(&record)).expect_err("truncated records must fail");
    assert_eq!(
        error.kind(),
        io::ErrorKind::UnexpectedEof,
        "truncation must retain UnexpectedEof"
    );
    assert_eq!(
        error.to_string(),
        "incomplete command record",
        "truncation must retain its exact diagnostic"
    );
}

/// Mismatch consumes an existing field; exhaustion never advances the cursor.
#[test]
fn cursor_advances_only_for_existing_fields() {
    let mut cursor = RecordCursor {
        fields: vec!["wrong", "next"],
        index: 0,
    };
    let mismatch = cursor
        .require_field("expected")
        .expect_err("existing wrong markers must fail");
    assert_eq!(
        mismatch.to_string(),
        "expected command record marker expected, found wrong",
        "marker mismatch must retain its diagnostic"
    );
    assert_eq!(
        cursor.index, 1,
        "a mismatched existing marker must still be consumed"
    );
    assert_eq!(
        cursor
            .next_field()
            .expect("the next field must remain accessible"),
        "next",
        "mismatch must not consume the following field"
    );
    let exhausted = cursor.next_field().expect_err("exhausted cursor must fail");
    assert_eq!(
        exhausted.kind(),
        io::ErrorKind::UnexpectedEof,
        "exhaustion must retain its error kind"
    );
    assert_eq!(
        cursor.index, 2,
        "exhaustion must leave the cursor position unchanged"
    );
}

/// Numeric argument counts must consume exactly the declared number of fields.
#[rstest]
#[case::declared_too_many(CountMismatchCase { count: "2", arguments: vec![], kind: io::ErrorKind::UnexpectedEof, message: "incomplete command record" })]
#[case::declared_too_few(CountMismatchCase { count: "0", arguments: vec!["extra"], kind: io::ErrorKind::Other, message: "expected command record marker end-invocation, found extra" })]
fn numeric_argument_count_mismatches_are_rejected(#[case] case: CountMismatchCase) {
    let mut record = fields(&RecordInput {
        environment_state: "unset",
        raw_environment: "",
        token_state: "unset",
        arguments: &case.arguments,
    });
    *record.get_mut(35).expect("argc value field must exist") = case.count.to_owned();
    let error = parse_invocations(&log(&record))
        .expect_err("a mismatched declared argument count must fail");
    assert_eq!(
        error.kind(),
        case.kind,
        "argument count mismatches must retain their error kind"
    );
    assert_eq!(
        error.to_string(),
        case.message,
        "argument count mismatches must retain their exact diagnostic"
    );
}

proptest! {
    /// NUL-free arguments and values round-trip without losing record boundaries.
    #[test]
    fn generated_records_preserve_arguments_and_states(
        arguments in proptest::collection::vec("[^\\x00]{0,32}", 0..12),
        raw in "[^\\x00]{0,32}",
        environment_state in 0u8..3,
        token_state in 0u8..3,
        copies in 1usize..4,
        final_nul in any::<bool>(),
    ) {
        let (state, expected_environment) = match environment_state {
            0 => ("unset", EnvironmentValue::Unset),
            1 => ("empty", EnvironmentValue::Empty),
            _ => ("value", EnvironmentValue::Value(raw.clone())),
        };
        let (token, expected_token) = match token_state {
            0 => ("unset", GitHubToken::Unset),
            1 => ("empty", GitHubToken::Empty),
            _ => ("present", GitHubToken::Present),
        };
        let borrowed_arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let encoded = log(&fields(&RecordInput {
            environment_state: state,
            raw_environment: &raw,
            token_state: token,
            arguments: &borrowed_arguments,
        }))
        .repeat(copies);
        let input = if final_nul { encoded.as_str() } else { encoded.trim_end_matches('\0') };
        let records = parse_invocations(input).expect("generated valid records must parse");
        prop_assert_eq!(records.len(), copies, "every concatenated record must be retained");
        for record in records {
            prop_assert_eq!(&record.arguments, &arguments, "all argument boundaries must round-trip");
            prop_assert_eq!(&record.github_token, &expected_token, "token states must remain redacted");
            prop_assert_eq!(record.environment.len(), 7, "all ordered environment records must remain");
            for observed in record.environment.values() {
                prop_assert_eq!(observed, &expected_environment, "environment states must retain raw-value semantics");
            }
        }
    }
}
