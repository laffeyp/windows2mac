// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run against Windows (Sprint 214; census row 7). Two judgements, both against the reference set the frozen
//! host's run of 2026-10-21 names (game run 38216081440 and its ten laps):
//!
//! - the port: the frozen host's whole stream gives the verdict its record holds, "conforms", no divergence that is a
//!   defect, and the 24 values Windows varies its comparison counted (docs/postmortem-2026-10-21.md 56);
//! - the run: the Rust host's stream, as far as it reaches, has the divergences the frozen host's stream has over the
//!   same rows, and no defect past the cut a stopped run makes (`call_compare::verdict`).

use std::fs;
use std::path::{Path, PathBuf};

use call_compare::{Stream, Verdict, Windows};

/// The frozen host's recorded run of 2026-10-21 (`batch_run.rs`'s STREAM) and its string arguments
const STREAM: &str = "store/frontier-calls/frontier-643a6283510c-1792623423346201000";
/// The values Windows varies over the frozen host's stream of 2026-10-21 (docs/postmortem-2026-10-21.md 56)
const VARIES: usize = 24;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A trace's calls, its non-empty lines
fn calls(text: &str) -> Vec<&str> {
    text.lines().filter(|l| !l.is_empty()).collect()
}

/// Every divergence a verdict holds, the scheduling ones too: a stopped run's paired threads end early, so their tails
/// read as scheduling, where a defect that stopped a thread early would also land (reviewer-3's Medium 2 of its review
/// of Sprints 212-214); held whole to the frozen host's same rows, none can hide
fn texts(v: &Verdict) -> Vec<String> {
    v.defects()
        .iter()
        .chain(v.past_cut())
        .chain(v.scheduling())
        .map(|d| d.text())
        .collect()
}

#[test]
fn the_frozen_hosts_stream_conforms_to_windows_as_its_record_says() {
    let root = root();
    let windows = Windows::load(&root).unwrap();
    let trace = fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap();
    let strings = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv"))).unwrap();
    let stream = |calls, strings, stopped| Stream {
        calls,
        strings,
        stopped,
        dropped: 0,
    };
    let v = windows.judge(&stream(&trace, &strings, false)).unwrap();
    assert_eq!((v.verdict().as_str(), v.first_divergence()), ("conforms", None));
    let left_out = windows
        .judge(&Stream {
            dropped: 3,
            ..stream(&trace, &strings, false)
        })
        .unwrap();
    assert_eq!(
        (left_out.verdict().as_str(), left_out.first_divergence().as_deref()),
        ("inconclusive", Some("the trace left out 3 calls")),
        "a trace that left calls out cannot show the calls conform"
    );
    println!(
        "the frozen host's {} calls: {}, {} defects, {} scheduling, {} values Windows varies",
        calls(&trace).len(),
        v.verdict().as_str(),
        v.defects().len(),
        v.scheduling().len(),
        v.varies().len()
    );
    assert!(
        v.defects().is_empty(),
        "the frozen host's stream against Windows:\n{:#?}",
        texts(&v)
    );
    // a capture thread left in, or a varying value held fixed, moves the count
    assert_eq!(
        v.varies().len(),
        VARIES,
        "values Windows varies over the frozen host's stream"
    );
}

#[test]
fn the_rust_hosts_stream_conforms_to_windows_as_far_as_it_runs() {
    let root = root();
    // a stream of other sources is not judged (Sprint 288): the stop file's stamp stands for its run's stream
    if !w2m_vm::built_from::fresh(&root, &root.join("build/batch_run/first-stop.tsv")) {
        return;
    }
    let Ok(ours) = fs::read_to_string(root.join("build/batch_run/first-stop.calls.tsv")) else {
        let what = "no build/batch_run/first-stop.calls.tsv (scripts/gates.sh writes it)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the Rust host's run is not compared here");
        return;
    };
    let windows = Windows::load(&root).unwrap();
    let stopped = !fs::read_to_string(root.join("build/batch_run/first-stop.tsv"))
        .unwrap()
        .starts_with("stop\tExitProcess");
    let our_strings = fs::read_to_string(root.join("build/batch_run/first-stop.calls.strings.tsv")).unwrap();
    let stream = |calls, strings| Stream {
        calls,
        strings,
        stopped,
        dropped: 0,
    };
    let v = windows.judge(&stream(&ours, &our_strings)).unwrap();
    let ours = calls(&ours);
    // the frozen host's stream over the same rows
    let trace = fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap();
    let theirs: Vec<&str> = calls(&trace).into_iter().take(ours.len()).collect();
    let last: u64 = theirs
        .last()
        .and_then(|l| l.split('\t').next())
        .map_or(0, |s| s.parse().unwrap());
    let theirs: String = theirs.iter().map(|l| format!("{l}\n")).collect();
    let their_strings: String = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv")))
        .unwrap()
        .lines()
        .filter(|l| {
            l.split('\t')
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .is_some_and(|s| s <= last)
        })
        .map(|l| format!("{l}\n"))
        .collect();
    let w = windows.judge(&stream(&theirs, &their_strings)).unwrap();
    println!(
        "the Rust host's {} calls: {}, {} defects, {} past the cut ({} for the frozen host's same rows), {} \
         scheduling, {} values Windows varies",
        ours.len(),
        v.verdict().as_str(),
        v.defects().len(),
        v.past_cut().len(),
        w.past_cut().len(),
        v.scheduling().len(),
        v.varies().len()
    );
    let (ours_all, theirs_all) = (texts(&v), texts(&w));
    let only_ours: Vec<&String> = ours_all.iter().filter(|d| !theirs_all.contains(d)).collect();
    let only_theirs: Vec<&String> = theirs_all.iter().filter(|d| !ours_all.contains(d)).collect();
    assert!(
        only_ours.is_empty() && only_theirs.is_empty(),
        "divergences only the Rust host shows: {only_ours:#?}\nonly the frozen host's same rows show: {only_theirs:#?}"
    );
    assert!(
        v.defects().is_empty(),
        "the Rust host's stream against Windows:\n{:#?}",
        v.defects().iter().map(|d| d.text()).collect::<Vec<_>>()
    );
}

/// The game's batch run on two vCPUs (Sprint 368; census G3), which `scripts/gates.sh` runs beside the one-vCPU run
/// through `scripts/batch-run-record.sh --vcpus 2`, its files `build/batch_run/first-stop-2-vcpus.*`: its stream judged
/// against Windows under the user's Decision of 2026-10-13. A value divergence (an argument, a return or a last error
/// of the same call on the paired thread) is a defect and fails here; a presence or count divergence on a thread whose
/// earlier calls conform is the game's scheduling, printed with its thread and call and not failed; the calls of a
/// Windows thread no w2m thread pairs with lie past the stopped run's cut, as on one vCPU. Not held to the frozen host's
/// same rows, as the one-vCPU stream is: two vCPUs interleave the threads' calls in another order than one vCPU does.
#[test]
fn the_rust_hosts_stream_on_two_vcpus_has_no_value_divergence() {
    let root = root();
    let stop_file = root.join("build/batch_run/first-stop-2-vcpus.tsv");
    // a stream of other sources is not judged (Sprint 288)
    if !w2m_vm::built_from::fresh(&root, &stop_file) {
        return;
    }
    let Ok(stop) = fs::read_to_string(&stop_file) else {
        let what = "no build/batch_run/first-stop-2-vcpus.tsv (scripts/gates.sh writes it)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the two-vCPU run is not compared here");
        return;
    };
    assert!(
        stop.lines().any(|l| l == "vcpus\t2"),
        "build/batch_run/first-stop-2-vcpus.tsv names no run on two vCPUs:\n{stop}"
    );
    let trace = fs::read_to_string(root.join("build/batch_run/first-stop-2-vcpus.calls.tsv")).unwrap();
    let strings = fs::read_to_string(root.join("build/batch_run/first-stop-2-vcpus.calls.strings.tsv")).unwrap();
    let windows = Windows::load(&root).unwrap();
    let v = windows
        .judge(&Stream {
            calls: &trace,
            strings: &strings,
            stopped: !stop.starts_with("stop\tExitProcess"),
            dropped: 0,
        })
        .unwrap();
    let thread = |t: Option<u64>| t.map_or("no w2m thread".to_string(), |t| format!("w2m thread {t}"));
    println!(
        "the Rust host's {} calls on two vCPUs: {}, {} defects, {} past the cut, {} scheduling, {} values Windows varies",
        calls(&trace).len(),
        v.verdict().as_str(),
        v.defects().len(),
        v.past_cut().len(),
        v.scheduling().len(),
        v.varies().len()
    );
    for (class, list) in [("past the cut", v.past_cut()), ("scheduling", v.scheduling())] {
        for d in list {
            assert!(!d.is_value(), "a value divergence sorted as {class}: {}", d.text());
            println!("{class}: {}: {}", thread(d.w2m_thread()), d.text());
        }
    }
    assert!(
        v.defects().is_empty(),
        "the Rust host's stream on two vCPUs against Windows:\n{:#?}",
        v.defects()
            .iter()
            .map(|d| format!(
                "{} divergence, {}: {}",
                if d.is_value() { "value" } else { "presence" },
                thread(d.w2m_thread()),
                d.text()
            ))
            .collect::<Vec<_>>()
    );
}

/// The frozen host's stream with one UnityPlayer.dll HeapReAlloc's size (a3) changed to `to`, at the first row whose
/// size is `size`.
fn with_size(trace: &str, size: &str, to: &str) -> String {
    let mut done = false;
    trace
        .lines()
        .map(|l| {
            let mut c: Vec<&str> = l.split('\t').collect();
            if !done && c.len() > 8 && c[2] == "UnityPlayer.dll" && c[4] == "HeapReAlloc" && c[8] == size {
                c[8] = to;
                done = true;
            }
            format!("{}\n", c.join("\t"))
        })
        .collect()
}

/// The contract's entry scoped to a thread (0.2.0, Sprint 368): "UnityPlayer.dll!HeapReAlloc@BatchDeleteObjects a3"
/// leaves the size uncompared on the threads named BatchDeleteObjects alone. In the frozen host's stream a main-thread
/// HeapReAlloc size changed (0x100 to 0x101, its first call) still diverges as a value; the BatchDeleteObjects
/// thread's first size changed (0x817 to 0x818, on its w2m thread 88, named so) does not.
#[test]
fn a_size_left_out_on_batchdeleteobjects_is_still_judged_on_the_main_thread() {
    let root = root();
    let windows = Windows::load(&root).unwrap();
    let trace = fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap();
    let strings = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv"))).unwrap();
    let judge = |calls: &str| {
        windows
            .judge(&Stream {
                calls,
                strings: &strings,
                stopped: false,
                dropped: 0,
            })
            .unwrap()
    };
    let main = judge(&with_size(&trace, "100", "101"));
    let named: Vec<String> = main.defects().iter().map(|d| d.text()).collect();
    assert!(
        named
            .iter()
            .any(|d| d.starts_with("UnityPlayer.dll!HeapReAlloc call 0 a3: Windows 0x100, w2m 0x101")),
        "a main-thread HeapReAlloc size changed is a value divergence: {named:#?}"
    );
    let batch = judge(&with_size(&trace, "817", "818"));
    assert_eq!(
        (batch.verdict().as_str(), batch.defects().len()),
        ("conforms", 0),
        "a BatchDeleteObjects HeapReAlloc size is undetermined: {:#?}",
        batch.defects().iter().map(|d| d.text()).collect::<Vec<_>>()
    );
}

/// Contract 0.1.0 stays as its last in-place edit, 91f6e329f, left it (Sprint 368: the scoped entry is 0.2.0's, a
/// minor bump, the old file kept, by the Decision of 2026-10-07).
#[test]
fn the_game_calls_contract_0_1_0_is_unchanged() {
    let root = root();
    let path = "contracts/windows.game.calls/0.1.0.json";
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["show", &format!("91f6e329f:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success(), "git show 91f6e329f:{path}");
    assert_eq!(
        fs::read(root.join(path)).unwrap(),
        out.stdout,
        "{path} differs from its last edit, 91f6e329f"
    );
    assert_eq!(call_compare::CONTRACT, "contracts/windows.game.calls/0.2.0.json");
}

/// The laptop runs of 2026-10-14 01:10 that ran the game in a window, as their exec.json records the arguments
const WINDOWED: &str = "other arguments (['-screen-fullscreen', '0', '-screen-width', '1280', '-screen-height', '720', \
                        '-logFile', '\"C:\\\\w2m\\\\unity.log\"'])";
const TWELVE: &str = "a machine of 12 processors, w2m reports 4";

/// Each stored game run the rules leave out of the reference set on 2026-11-27, with the reason `call_compare.py`'s
/// `outside_set` gives (its text, rule for rule)
fn left_out() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [
        ("37982448071", "no call trace with thread names"),
        ("38006665419", "no call trace with thread names"),
        ("38006671518", "no call trace with thread names"),
        ("38113342549", "no call trace with thread names"),
        ("38113343972", "no call trace with thread names"),
        (
            "38176899195",
            "the capture names no threads (SetThreadDescription without names)",
        ),
        (
            "38203364352",
            "the capture names no threads (SetThreadDescription without names)",
        ),
        (
            "38206263976",
            "the capture names no threads (SetThreadDescription without names)",
        ),
        (
            "38275716702",
            "the contract names no capture thread for it (another capture or arguments)",
        ),
        ("lap-20261014T041748433Z-73c66e6a-01", "no call trace with thread names"),
    ]
    .iter()
    .map(|(r, why)| (r.to_string(), why.to_string()))
    .collect();
    for i in 1..=3 {
        out.push((format!("lap-20261014T011035282Z-e61fca06-{i:02}"), WINDOWED.into()));
    }
    for batch in ["lap-20261014T012302244Z-cb542225", "lap-20261014T023701004Z-3f1cf08d"] {
        for i in 1..=10 {
            out.push((format!("{batch}-{i:02}"), TWELVE.into()));
        }
    }
    out.sort();
    out
}

/// G3's reference set chosen by its rules (Sprint 375): from the stored game runs, `call_compare.py`'s `reference_set`
/// gives the eleven runs the frozen host's run of 2026-10-21 names, and leaves out the 33 others, each for its reason;
/// `Windows`, which judges the batch runs, holds that set and cites it in `reference_ref`.
#[test]
fn the_rules_choose_the_frozen_hosts_reference_set() {
    let root = root();
    let set = call_compare::reference_set_of(&root).unwrap();
    let frozen = call_compare::frozen_reference_set(&root).unwrap();
    let joined: Vec<&String> = set.members.iter().filter(|r| !frozen.contains(r)).collect();
    let left: Vec<(&String, Option<&String>)> = frozen
        .iter()
        .filter(|r| !set.members.contains(r))
        .map(|r| (r, set.outside.get(r)))
        .collect();
    assert!(
        joined.is_empty() && left.is_empty(),
        "runs the rules let in that the frozen host left out: {joined:#?}\nruns the frozen host chose that the rules \
         leave out, with the rules' reason: {left:#?}"
    );
    assert_eq!(set.members, frozen, "the set in name order");
    let outside: Vec<(String, String)> = set.outside.into_iter().collect();
    assert_eq!(outside, left_out(), "each run left out, with its reason");
    let windows = Windows::load(&root).unwrap();
    let cited: Vec<String> = frozen.iter().map(|r| format!("game-run-{r}")).collect();
    assert_eq!(windows.reference_ref(), cited.join(" + "));
}

/// The rules the store's runs do not reach, each on a copy of laptop run c8814547-01 under a root of the test's own:
/// the run as stored joins; its arguments as one command line join; no arguments, no profile.json, no record, another
/// profile kind or another game build on the record each leave it out, for the reason `outside_set` gives.
#[test]
fn each_rule_leaves_a_laptop_run_out_for_its_reason() {
    let repo = root();
    let at = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reference-set-{}", std::process::id()));
    let _ = fs::remove_dir_all(&at);
    let copy = |from: &str, to: &str| {
        let to = at.join(to);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(repo.join(from), to).unwrap();
    };
    copy(
        "games/hail-to-the-rainbow/files.json",
        "games/hail-to-the-rainbow/files.json",
    );
    copy(call_compare::CONTRACT, call_compare::CONTRACT);
    let source = "lap-20261014T032442034Z-c8814547-01";
    let record =
        fs::read_to_string(repo.join(format!("store/runs/game-run-{source}/events-000001.open.jsonl"))).unwrap();
    let build = "643a6283510c88e3483212c9d0358de925ac415b83fa0302dced6381f412f974";
    let run = |name: &str, exec: Option<&str>, profile_json: bool, record_text: Option<String>| {
        for file in [
            "jit_capture.bin.trace.tsv",
            "jit_capture.bin.strings.tsv",
            "profile.txt",
            "exec.json",
        ]
        .iter()
        .chain(profile_json.then_some(&"profile.json"))
        {
            copy(
                &format!("store/game-runs/{source}/{file}"),
                &format!("store/game-runs/{name}/{file}"),
            );
        }
        if let Some(exec) = exec {
            fs::write(at.join(format!("store/game-runs/{name}/exec.json")), exec).unwrap();
        }
        if let Some(text) = record_text {
            let dir = at.join(format!("store/runs/game-run-{name}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("events-000001.open.jsonl"), text).unwrap();
        }
    };
    run("a-as-stored", None, true, Some(record.clone()));
    let line = r#"{"arguments": "-batchmode -nographics -logFile \"C:\\w2m\\unity.log\"", "exited_by_itself": false}"#;
    run("b-one-command-line", Some(line), true, Some(record.clone()));
    run(
        "c-no-arguments",
        Some(r#"{"exited_by_itself": false}"#),
        true,
        Some(record.clone()),
    );
    run("d-no-profile", None, false, Some(record.clone()));
    run("e-no-record", None, true, None);
    run(
        "f-another-kind",
        None,
        true,
        Some(record.replace("\"windows_reference\"", "\"native_process\"")),
    );
    run(
        "g-another-build",
        None,
        true,
        Some(record.replace(build, &"0".repeat(64))),
    );
    let set = call_compare::reference_set_of(&at).unwrap();
    let _ = fs::remove_dir_all(&at);
    assert_eq!(set.members, ["a-as-stored", "b-one-command-line"]);
    let outside: Vec<(&str, &str)> = set.outside.iter().map(|(r, why)| (r.as_str(), why.as_str())).collect();
    assert_eq!(
        outside,
        [
            ("c-no-arguments", "a laptop run whose exec.json records no arguments"),
            ("d-no-profile", "a machine of None processors, w2m reports 4"),
            ("e-no-record", "not on the record (game-run-e-no-record)"),
            ("f-another-kind", "no windows_reference EXECUTION_STARTED on the record"),
            (
                "g-another-build",
                "another game build (sha256:0000000000000000000000000000000000000000000000000000000000000000)"
            ),
        ]
    );
}

/// Each stored game run on the record, as `record_game_run.py` put it there: the EXECUTION_STARTED and
/// EXECUTION_COMPLETED that `call_compare::game_run_execution` makes from the run's files are the record's, field for
/// field, for all 38 the frozen host recorded (Sprint 375).
#[test]
fn the_record_step_writes_the_frozen_hosts_executions() {
    use events::{Event, ExecutionCompleted, ExecutionStarted};
    let root = root();
    let mut held = 0;
    for entry in fs::read_dir(root.join(call_compare::GAME_RUNS)).unwrap().flatten() {
        let run = entry.file_name().to_string_lossy().to_string();
        let record = root.join(call_compare::RECORDS).join(format!("game-run-{run}"));
        if !record.is_dir() {
            continue;
        }
        let mut found: Vec<serde_json::Value> = Vec::new();
        for segment in fs::read_dir(&record).unwrap().flatten() {
            let name = segment.file_name().to_string_lossy().to_string();
            if name.starts_with("events-") && name.ends_with(".jsonl") {
                for line in fs::read_to_string(segment.path()).unwrap().lines() {
                    let frame: serde_json::Value = serde_json::from_str(line).unwrap();
                    if frame["kind"].as_str().is_some_and(|k| k.starts_with("EXECUTION_")) {
                        found.push(frame);
                    }
                }
            }
        }
        let payload = |kind: &str| {
            let of: Vec<&serde_json::Value> = found.iter().filter(|f| f["kind"] == kind).collect();
            assert_eq!(of.len(), 1, "{run}: one {kind} on its record");
            of[0]["payload"].clone()
        };
        let (started, completed) = call_compare::game_run_execution(&root, &run).unwrap();
        assert_eq!(
            started,
            ExecutionStarted::from_payload(&payload("EXECUTION_STARTED")).unwrap(),
            "{run}"
        );
        assert_eq!(
            completed,
            ExecutionCompleted::from_payload(&payload("EXECUTION_COMPLETED")).unwrap(),
            "{run}"
        );
        held += 1;
    }
    assert_eq!(held, 38, "the stored game runs on the record");
}

/// The main-thread calls `w2m-vm --game` stops by are the ones `call_compare` derives from the ending run (Sprint 219):
/// the stored file cannot drift from the reference it is read from.
#[test]
fn the_stored_main_thread_calls_are_the_ending_runs() {
    let root = root();
    let stored: Vec<(String, String, u64)> =
        fs::read_to_string(root.join("store/batch_run/main-thread-calls-2026-10-29.tsv"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                (c[0].to_string(), c[1].to_string(), c[2].parse().unwrap())
            })
            .collect();
    let derived = call_compare::main_thread_calls(&root).unwrap();
    assert_eq!(stored, derived);
    assert_eq!(
        derived.iter().map(|r| r.2).sum::<u64>(),
        2999,
        "the Windows main thread's calls"
    );
}
