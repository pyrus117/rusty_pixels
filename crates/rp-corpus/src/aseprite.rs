//! Aseprite CLI exit classification for the corpus round-trip tool.

use std::collections::BTreeSet;
use std::process::Output;

/// The X11 auth complaint headless Aseprite prints when it cannot reach a
/// display authorization. Rendering still works, so lines holding it are
/// noise: they never reach a record line or a WARN.
const HARMLESS: &str = "Authorization required, but no authorization protocol specified";

/// One Aseprite invocation, already classified.
pub enum AsepriteRun {
    /// Aseprite exited zero. The payload is the run's diagnostic lines, empty
    /// when the run was quiet.
    Ok(Vec<String>),
    /// Aseprite did not exit zero. The payload is its filtered output on one
    /// line, or its status when that stayed empty.
    Fail(String),
}

/// Diagnostic lines from a file's Aseprite runs, one list per side.
#[derive(Default)]
pub struct Diagnostics {
    pub orig: Vec<String>,
    pub ours: Vec<String>,
}

/// What one file's two diagnostic lists report. Lines both sides printed are
/// expected noise and say nothing.
pub struct DiagDiff {
    /// The first line ours printed that orig never did: an ASEPRITE-DIFF.
    pub ours_only: Option<String>,
    /// Lines only orig printed: WARNs that never fail the run.
    pub orig_only: Vec<String>,
}

/// Classifies one Aseprite exit. Batch Aseprite prints load warnings and load
/// errors on stdout and still exits 0, so both streams are captured.
pub fn classify(output: &Output) -> AsepriteRun {
    let lines = filter_diagnostics(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    );
    if output.status.success() {
        return AsepriteRun::Ok(lines);
    }
    if lines.is_empty() {
        return AsepriteRun::Fail(output.status.to_string());
    }
    AsepriteRun::Fail(lines.join(" "))
}

/// The diagnostic lines of one run: both streams, minus empty lines and the
/// harmless auth complaint.
pub fn filter_diagnostics(stdout: &str, stderr: &str) -> Vec<String> {
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains(HARMLESS))
        .map(str::to_string)
        .collect()
}

/// Compares the diagnostic lines our runs printed against the originals'.
/// Aseprite warns about the same content in both rewrites, so only a line
/// unique to ours marks a real difference. Both sides compare as sets; the
/// reported lines come out sorted.
pub fn diff_diagnostics(orig: &[String], ours: &[String]) -> DiagDiff {
    let orig: BTreeSet<&str> = orig.iter().map(String::as_str).collect();
    let ours: BTreeSet<&str> = ours.iter().map(String::as_str).collect();
    DiagDiff {
        ours_only: ours.difference(&orig).next().map(|line| line.to_string()),
        orig_only: orig
            .difference(&ours)
            .map(|line| line.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_diagnostics_keeps_real_lines_from_both_streams() {
        assert!(filter_diagnostics("", "").is_empty());

        // Empty lines and the auth complaint are noise; what stays is kept as
        // separate lines, stdout first.
        let stdout = "Warning: Unexpected chunk type 123 (skipping)\n\n";
        let stderr = "Authorization required, but no authorization protocol specified\n\
                      Error: second problem\n";
        assert_eq!(
            filter_diagnostics(stdout, stderr),
            vec![
                "Warning: Unexpected chunk type 123 (skipping)",
                "Error: second problem"
            ]
        );
    }

    #[test]
    fn a_line_only_ours_printed_marks_a_diff() {
        let orig = vec!["Warning: shared".to_string()];
        let ours = vec!["Warning: shared".to_string(), "Warning: new".to_string()];
        let diff = diff_diagnostics(&orig, &ours);
        assert_eq!(diff.ours_only.as_deref(), Some("Warning: new"));
        assert!(diff.orig_only.is_empty());
    }

    #[test]
    fn a_line_only_orig_printed_is_a_warn() {
        let orig = vec!["Warning: shared".to_string(), "Warning: old".to_string()];
        let ours = vec!["Warning: shared".to_string()];
        let diff = diff_diagnostics(&orig, &ours);
        assert_eq!(diff.ours_only, None);
        assert_eq!(diff.orig_only, vec!["Warning: old".to_string()]);
    }
}
