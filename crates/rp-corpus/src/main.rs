//! Corpus round-trip tool: reads each .aseprite file with rp-io, rewrites it,
//! and compares Aseprite renders of the original and our output.

mod aseprite;
mod compare;
mod meta;
mod render;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use aseprite::{AsepriteRun, Diagnostics, diff_diagnostics};
use compare::first_diff;
use render::{Mode, RenderDiff};

const USAGE: &str =
    "usage: rp-corpus --aseprite <path> --out <dir> <file>... | --list <file-with-paths>";

struct Args {
    aseprite: PathBuf,
    out: PathBuf,
    inputs: Vec<PathBuf>,
}

/// Which counter a file lands in.
enum Bucket {
    Ok,
    ReadFail,
    ModelDiff,
    RenderDiff,
    MetaDiff,
    AsepriteDiff,
    Error,
    OrigFail,
}

/// Running totals behind the final counts line and the exit code.
#[derive(Default)]
struct Counts {
    ok: usize,
    read_fail: usize,
    model_diff: usize,
    render_diff: usize,
    meta_diff: usize,
    aseprite_diff: usize,
    error: usize,
    orig_fail: usize,
}

impl Counts {
    fn record(&mut self, bucket: Bucket) {
        match bucket {
            Bucket::Ok => self.ok += 1,
            Bucket::ReadFail => self.read_fail += 1,
            Bucket::ModelDiff => self.model_diff += 1,
            Bucket::RenderDiff => self.render_diff += 1,
            Bucket::MetaDiff => self.meta_diff += 1,
            Bucket::AsepriteDiff => self.aseprite_diff += 1,
            Bucket::Error => self.error += 1,
            Bucket::OrigFail => self.orig_fail += 1,
        }
    }

    /// The single summary line the run prints at the end.
    fn line(&self) -> String {
        format!(
            "{} ok / {} read-fail / {} model-diff / {} render-diff / {} meta-diff / {} aseprite-diff / {} error / {} orig-fail",
            self.ok,
            self.read_fail,
            self.model_diff,
            self.render_diff,
            self.meta_diff,
            self.aseprite_diff,
            self.error,
            self.orig_fail
        )
    }

    /// Aseprite failing on an original file is not ours to fix, so it alone
    /// never fails the run.
    fn failed(&self) -> bool {
        self.read_fail
            + self.model_diff
            + self.render_diff
            + self.meta_diff
            + self.aseprite_diff
            + self.error
            > 0
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the round-trip over every input. False when a record was not OK.
fn run() -> Result<bool> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&argv)?;
    for dir in ["ours", "render", "meta"] {
        std::fs::create_dir_all(args.out.join(dir))
            .with_context(|| format!("creating {}", args.out.join(dir).display()))?;
    }

    let mut counts = Counts::default();
    for (index, path) in args.inputs.iter().enumerate() {
        counts.record(process_file(index + 1, path, &args.aseprite, &args.out));
    }
    println!("{}", counts.line());
    Ok(passed(&counts))
}

/// The exit decision after the counts line; false means exit 1. Orig failures
/// alone never fail a run, but one with no OK file and an ORIG-FAIL proved
/// nothing, so it fails and says why.
fn passed(counts: &Counts) -> bool {
    if counts.ok == 0 && counts.orig_fail > 0 {
        println!("aseprite failed on every original");
        return false;
    }
    !counts.failed()
}

/// Runs one input file through every stage and prints its record line.
/// The first failing stage decides the record; the rest stay untried. Any
/// per-file error becomes an ERROR record, so one file never stops the run.
fn process_file(n: usize, path: &Path, aseprite: &Path, out: &Path) -> Bucket {
    match process_inner(n, path, aseprite, out) {
        Ok(bucket) => bucket,
        Err(err) => {
            println!("ERROR {}: {err:#}", path.display());
            Bucket::Error
        }
    }
}

/// The stages of one file. Returns the bucket its record line names.
fn process_inner(n: usize, path: &Path, aseprite: &Path, out: &Path) -> Result<Bucket> {
    let shown = path.display();
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("READ-FAIL {shown}: {err}");
            return Ok(Bucket::ReadFail);
        }
    };
    let model = match rp_io::read(&bytes) {
        Ok(model) => model,
        Err(err) => {
            println!("READ-FAIL {shown}: {err}");
            return Ok(Bucket::ReadFail);
        }
    };
    let written = match rp_io::write(&model) {
        Ok(written) => written,
        Err(err) => {
            println!("MODEL-DIFF {shown}: write failed: {err}");
            return Ok(Bucket::ModelDiff);
        }
    };
    let round_trip = match rp_io::read(&written) {
        Ok(model) => model,
        Err(err) => {
            println!("MODEL-DIFF {shown}: re-read failed: {err}");
            return Ok(Bucket::ModelDiff);
        }
    };
    if let Some(field) = first_diff(&model, &round_trip) {
        println!("MODEL-DIFF {shown}: {field}");
        return Ok(Bucket::ModelDiff);
    }
    let ours = out.join("ours").join(format!("{n}.aseprite"));
    std::fs::write(&ours, &written).with_context(|| format!("writing {}", ours.display()))?;

    let mut diags = Diagnostics::default();
    for mode in [Mode::Visible, Mode::AllLayers] {
        let (diff, run_diags) = render::compare(aseprite, out, n, path, &ours, mode)?;
        diags.orig.extend(run_diags.orig);
        diags.ours.extend(run_diags.ours);
        let Some(diff) = diff else {
            continue;
        };
        return Ok(match diff {
            RenderDiff::Orig(detail) => {
                println!("ORIG-FAIL {shown}: {detail}");
                Bucket::OrigFail
            }
            RenderDiff::Ours(detail) => {
                println!("RENDER-DIFF {shown}: aseprite: {detail}");
                Bucket::RenderDiff
            }
            RenderDiff::Detail(detail) => {
                println!("RENDER-DIFF {shown}: {detail}");
                Bucket::RenderDiff
            }
            RenderDiff::Frame { key, pixels } => {
                match mode {
                    Mode::Visible => println!("RENDER-DIFF {shown} frame {key} ({pixels} pixels)"),
                    Mode::AllLayers => {
                        println!("RENDER-DIFF {shown} all-layers frame {key} ({pixels} pixels)")
                    }
                }
                Bucket::RenderDiff
            }
        });
    }

    for (tag, file) in [("orig", path), ("ours", ours.as_path())] {
        match meta::export(aseprite, out, n, tag, file)? {
            AsepriteRun::Ok(lines) => {
                if tag == "orig" {
                    diags.orig.extend(lines);
                } else {
                    diags.ours.extend(lines);
                }
            }
            AsepriteRun::Fail(detail) => {
                return Ok(if tag == "orig" {
                    println!("ORIG-FAIL {shown}: {detail}");
                    Bucket::OrigFail
                } else {
                    println!("ERROR {shown}: aseprite: {detail}");
                    Bucket::Error
                });
            }
        }
    }
    let orig_meta = meta::load(&out.join("meta").join(format!("{n}-orig.json")))?;
    let ours_meta = meta::load(&out.join("meta").join(format!("{n}-ours.json")))?;
    if let Some(key) = meta::first_meta_diff(&orig_meta, &ours_meta) {
        println!("META-DIFF {shown}: {key}");
        return Ok(Bucket::MetaDiff);
    }

    // Last stage: Aseprite must not complain about our rewrite alone.
    let diff = diff_diagnostics(&diags.orig, &diags.ours);
    for line in &diff.orig_only {
        println!("WARN {shown}: {line}");
    }
    if let Some(line) = &diff.ours_only {
        println!("ASEPRITE-DIFF {shown}: {line}");
        return Ok(Bucket::AsepriteDiff);
    }
    println!("OK {shown}");
    Ok(Bucket::Ok)
}

fn parse_args(argv: &[String]) -> Result<Args> {
    let mut aseprite = None;
    let mut out = None;
    let mut inputs = Vec::new();
    let mut it = argv.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--aseprite" => aseprite = Some(PathBuf::from(need(&mut it, "--aseprite")?)),
            "--out" => out = Some(PathBuf::from(need(&mut it, "--out")?)),
            "--list" => inputs.extend(read_list(Path::new(&need(&mut it, "--list")?))?),
            flag if flag.starts_with("--") => bail!("{flag}: unknown option\n{USAGE}"),
            _ => inputs.push(PathBuf::from(arg)),
        }
    }
    let (Some(aseprite), Some(out)) = (aseprite, out) else {
        bail!("--aseprite and --out are required\n{USAGE}");
    };
    if inputs.is_empty() {
        bail!("no input files\n{USAGE}");
    }
    Ok(Args {
        aseprite,
        out,
        inputs,
    })
}

fn need<'a>(it: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<String> {
    it.next()
        .cloned()
        .with_context(|| format!("{flag} needs a value\n{USAGE}"))
}

fn read_list(path: &Path) -> Result<Vec<PathBuf>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_line_names_every_bucket() {
        let mut counts = Counts::default();
        for _ in 0..3 {
            counts.record(Bucket::Ok);
        }
        counts.record(Bucket::ReadFail);
        counts.record(Bucket::ModelDiff);
        counts.record(Bucket::RenderDiff);
        counts.record(Bucket::MetaDiff);
        counts.record(Bucket::AsepriteDiff);
        counts.record(Bucket::Error);
        counts.record(Bucket::OrigFail);
        assert_eq!(
            counts.line(),
            "3 ok / 1 read-fail / 1 model-diff / 1 render-diff / 1 meta-diff / 1 aseprite-diff / 1 error / 1 orig-fail"
        );
    }

    #[test]
    fn only_our_faults_fail_the_run() {
        for bucket in [Bucket::Ok, Bucket::OrigFail] {
            let mut counts = Counts::default();
            counts.record(bucket);
            assert!(!counts.failed(), "unexpected failure: {}", counts.line());
        }
        for bucket in [
            Bucket::ReadFail,
            Bucket::ModelDiff,
            Bucket::RenderDiff,
            Bucket::MetaDiff,
            Bucket::AsepriteDiff,
            Bucket::Error,
        ] {
            let mut counts = Counts::default();
            counts.record(bucket);
            assert!(counts.failed(), "unexpected pass: {}", counts.line());
        }
    }

    #[test]
    fn every_original_failing_fails_the_run() {
        let mut counts = Counts::default();
        counts.record(Bucket::OrigFail);
        assert!(!passed(&counts), "unexpected pass: {}", counts.line());

        // One OK file keeps an orig failure from failing the run.
        counts.record(Bucket::Ok);
        assert!(passed(&counts), "unexpected failure: {}", counts.line());

        // Without an ORIG-FAIL the rule stays out of it.
        assert!(passed(&Counts::default()), "empty run failed");
    }
}
