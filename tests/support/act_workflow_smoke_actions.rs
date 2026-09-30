//! Fixture action interfaces for the production-derived Act smoke.

use std::fmt::Write;

/// Defines the interface that one replaced action exposes to later steps.
pub(super) fn fixture_action(
    index: usize,
    inputs: &[(&str, &str)],
) -> Result<String, std::fmt::Error> {
    let mut source =
        format!("name: Fixture action {index}\ndescription: Controlled workflow interface\n");
    if !inputs.is_empty() {
        source.push_str("inputs:\n");
        for (key, _) in inputs {
            writeln!(source, "  {key}:\n    required: false")?;
        }
    }
    source.push_str("runs:\n  using: composite\n  steps:\n    - shell: bash\n      run: |\n");
    write!(source, "        printf '%s\\0' 'action-{index}' \"$PWD\" ")?;
    for (key, _) in inputs {
        write!(source, "\"${{{{ inputs.{key} }}}}\" ")?;
    }
    source.push_str("'__END__' >> \"$GITHUB_WORKSPACE/smoke.log\"\n");
    if index == 0 {
        source.push_str("        echo \"$GITHUB_WORKSPACE/fixture-bin\" >> \"$GITHUB_PATH\"\n");
    }
    if index == 1 {
        source.push_str("        echo 'RUSTFLAGS=' >> \"$GITHUB_ENV\"\n");
    }
    Ok(source)
}
