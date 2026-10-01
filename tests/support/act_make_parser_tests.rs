//! Exact parser acceptance, rejection, and cursor-consumption contracts.

use std::io;

use rstest::rstest;

use super::{EnvironmentValue, GitHubToken, RecordCursor, parse_invocations};

/// Builds the fixed emitter field sequence with controlled protocol constituents.
fn fields(state: &str, raw: &str, token: &str, arguments: &[&str]) -> Vec<String> {
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
        result.extend(["env", name, state, raw].map(str::to_owned));
    }
    result.extend(["secret", "GITHUB_TOKEN", token, "argc"].map(str::to_owned));
    result.push(arguments.len().to_string());
    result.push("argv".to_owned());
    result.extend(arguments.iter().map(|value| (*value).to_owned()));
    result.push("end-invocation".to_owned());
    result
}

/// Joins record fields with the emitter's final NUL terminator.
fn log(fields: &[String]) -> String { format!("{}\0", fields.join("\0")) }

/// All environment states retain historical raw-value acceptance.
#[rstest]
#[case("unset", "ignored", EnvironmentValue::Unset)]
#[case("empty", "ignored", EnvironmentValue::Empty)]
#[case("value", "", EnvironmentValue::Value(String::new()))]
#[case("value", "a\tb\nc", EnvironmentValue::Value("a\tb\nc".to_owned()))]
fn environment_states_preserve_raw_value_semantics(
    #[case] state: &str,
    #[case] raw: &str,
    #[case] expected: EnvironmentValue,
) {
    let records = parse_invocations(&log(&fields(state, raw, "unset", &[])))
        .expect("valid environment records must parse");
    let record = records.first().expect("one invocation must be returned");
    assert_eq!(
        record.environment.len(),
        7,
        "all seven ordered environment fields must be retained"
    );
    for actual in record.environment.values() {
        assert_eq!(
            actual, &expected,
            "the parser must preserve the selected environment state"
        );
    }
}

/// Token states are redacted independently from environment values.
#[rstest]
#[case("unset", GitHubToken::Unset)]
#[case("empty", GitHubToken::Empty)]
#[case("present", GitHubToken::Present)]
fn token_states_remain_distinct(#[case] state: &str, #[case] expected: GitHubToken) {
    let records = parse_invocations(&log(&fields("unset", "", state, &[])))
        .expect("valid token records must parse");
    assert_eq!(
        records.first().map(|record| &record.github_token),
        Some(&expected),
        "the token presence state must remain distinct"
    );
}

/// Zero arguments and internal empty, tabbed, or multiline arguments remain exact.
#[rstest]
#[case(vec![])]
#[case(vec!["", "tab\targument", "line\nargument", ""])]
fn argument_boundaries_are_lossless(#[case] arguments: Vec<&str>) {
    let records = parse_invocations(&log(&fields("unset", "", "unset", &arguments)))
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
    let record = log(&fields("unset", "", "unset", &[]));
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
#[case(
    0,
    "wrong",
    "expected command record marker invocation-v1, found wrong"
)]
#[case(3, "wrong", "expected command record marker env, found wrong")]
#[case(4, "ACT", "expected environment record WITH_ACT, found ACT")]
#[case(5, "unknown", "unknown environment state unknown")]
#[case(31, "wrong", "expected command record marker secret, found wrong")]
#[case(
    32,
    "wrong",
    "expected command record marker GITHUB_TOKEN, found wrong"
)]
#[case(33, "unknown", "unknown token state unknown")]
#[case(34, "wrong", "expected command record marker argc, found wrong")]
#[case(36, "wrong", "expected command record marker argv, found wrong")]
#[case(
    37,
    "wrong",
    "expected command record marker end-invocation, found wrong"
)]
fn invalid_markers_and_states_are_rejected(
    #[case] index: usize,
    #[case] value: &str,
    #[case] message: &str,
) {
    let mut record = fields("unset", "", "unset", &[]);
    *record
        .get_mut(index)
        .expect("fixture field index must exist") = value.to_owned();
    let error = parse_invocations(&log(&record)).expect_err("invalid protocol records must fail");
    assert_eq!(
        error.kind(),
        io::ErrorKind::Other,
        "marker and state diagnostics must retain their error kind"
    );
    assert_eq!(
        error.to_string(),
        message,
        "protocol diagnostics must remain exact"
    );
}

/// Invalid declared argument counts retain integer parsing errors.
#[rstest]
#[case("not-a-number")]
#[case("-1")]
fn invalid_argument_counts_are_rejected(#[case] count: &str) {
    let mut record = fields("unset", "", "unset", &[]);
    *record.get_mut(35).expect("argc value field must exist") = count.to_owned();
    let error = parse_invocations(&log(&record)).expect_err("invalid argument counts must fail");
    assert_eq!(
        error.kind(),
        io::ErrorKind::InvalidData,
        "integer parse errors must retain InvalidData"
    );
    assert_eq!(
        error.to_string(),
        count
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
    let mut record = fields("unset", "", "unset", &[]);
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
#[case("2", vec![], io::ErrorKind::UnexpectedEof, "incomplete command record")]
#[case("0", vec!["extra"], io::ErrorKind::Other, "expected command record marker end-invocation, found extra")]
fn numeric_argument_count_mismatches_are_rejected(
    #[case] count: &str,
    #[case] arguments: Vec<&str>,
    #[case] kind: io::ErrorKind,
    #[case] message: &str,
) {
    let mut record = fields("unset", "", "unset", &arguments);
    *record.get_mut(35).expect("argc value field must exist") = count.to_owned();
    let error = parse_invocations(&log(&record))
        .expect_err("a mismatched declared argument count must fail");
    assert_eq!(
        error.kind(),
        kind,
        "argument count mismatches must retain their error kind"
    );
    assert_eq!(
        error.to_string(),
        message,
        "argument count mismatches must retain their exact diagnostic"
    );
}
