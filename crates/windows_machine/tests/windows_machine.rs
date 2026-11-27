// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 371: the program held to the runs already made. `lap-20261127T210820791Z-b5a7cf3d` was sent by hand with the
//! Python's exact steps; `lap-20261017T040446837Z-dc268176` by the Python itself. Their stored `job.json` and the
//! tree.tar it names are what the program writes, and the store step keeps a laptop run and refuses a runner's.

use std::fs;
use std::path::{Path, PathBuf};

use windows_machine::{
    EXECUTOR, Job, Kind, Request, run_id, sha256, stamp, store, stored_run, tree_tar, verdict, write_job,
};

const HAND_SENT: &str = "lap-20261127T210820791Z-b5a7cf3d";
const HAND_SENT_COMMIT: &str = "b5a7cf3da095c8121e2d4b5a5d33b64a66941bbf";
const HAND_SENT_TREE: &str = "33cd2ed5401605c70fbfc516f32a74ba53b2b7367e43f1f0fe39d0ad4d4a68b8";
const HAND_SENT_BINARY: &str = "4d655151ad84481958a92a383bcd98a646ecdb1ab80f9835c2e743c9fe20b950";
const OCTOBER: &str = "lap-20261017T040446837Z-dc268176";
/// A runner's run: `gha-windows-2025-...-Server` in its profile.txt.
const RUNNER: &str = "46053401854";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch folder for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("w2m-windows-machine-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `store/probe-runs/<run>`'s files copied to `<at>/build/probe-runs/<as_run>/traces-<execution>/`, as a bundle the
/// program fetched; that folder.
fn bundle(at: &Path, run: &str, as_run: &str, execution: &str) -> PathBuf {
    let dst = at
        .join("build/probe-runs")
        .join(as_run)
        .join(format!("traces-{execution}"));
    fs::create_dir_all(&dst).unwrap();
    for e in fs::read_dir(root().join("store/probe-runs").join(run)).unwrap() {
        let p = e.unwrap().path();
        fs::copy(&p, dst.join(p.file_name().unwrap())).unwrap();
    }
    dst
}

/// The hand-sent run's tree.tar is `git archive` of its commit less `store/`, byte for byte: job.json names its sha256.
#[test]
fn the_hand_sent_runs_tree_is_the_programs() {
    let tree = tree_tar(&root(), HAND_SENT_COMMIT, Kind::Probe, "apcs_and_waitable_timers").unwrap();
    assert_eq!(sha256(&tree), HAND_SENT_TREE);
}

/// The program's job.json for each stored run's job is that run's job.json: the October run's byte for byte (the
/// Python wrote it), the hand-sent run's but for the final newline its writer added.
#[test]
fn job_json_is_the_stored_runs() {
    let hand = Job {
        kind: Kind::Probe,
        probe: "apcs_and_waitable_timers".into(),
        execution_id: "apcs_and_waitable_timers-20261127T210820791Z".into(),
        repeat: 3,
        commit: HAND_SENT_COMMIT.into(),
        cpu_class: None,
        inputs: vec![("tree.tar".into(), HAND_SENT_TREE.into())],
    };
    let stored = fs::read_to_string(root().join("store/probe-runs").join(HAND_SENT).join("job.json")).unwrap();
    assert_eq!(format!("{}\n", hand.json()), stored);
    let october = Job {
        kind: Kind::Probe,
        probe: "wait_handoff".into(),
        execution_id: "wait_handoff-20261017T040446837Z".into(),
        repeat: 2,
        commit: "dc2681761074566dafc273b1b544468b21e55f12".into(),
        cpu_class: None,
        inputs: vec![(
            "tree.tar".into(),
            "238320e067a3a98d3a54ed2f062c4390f38fa8235f4a0e65e9eba9e4a68c58d8".into(),
        )],
    };
    let stored = fs::read_to_string(root().join("store/probe-runs").join(OCTOBER).join("job.json")).unwrap();
    assert_eq!(october.json(), stored);
    let pinned = Job {
        cpu_class: Some("performance".into()),
        ..october
    };
    assert!(
        pinned
            .json()
            .contains(",\n \"cpu_class\": \"performance\",\n \"inputs\": {")
    );
}

/// The stamp and the run id: the two stored runs', a leap day's last second, the epoch.
#[test]
fn stamps_and_run_ids() {
    assert_eq!(stamp(1_795_813_700_791), "20261127T210820791Z");
    assert_eq!(run_id(&stamp(1_795_813_700_791), HAND_SENT_COMMIT), HAND_SENT);
    assert_eq!(stamp(1_792_209_886_837), "20261017T040446837Z");
    assert_eq!(stamp(1_709_251_199_999), "20240229T235959999Z");
    assert_eq!(stamp(0), "19700101T000000000Z");
}

/// The verdict as the Python's: results missing, run-job's code, fewer executions than asked, a non-zero execution.
#[test]
fn verdicts() {
    let hand = root().join("store/probe-runs").join(HAND_SENT);
    assert_eq!(verdict(0, true, &hand, 3), (0, "3 of 3 executions".into()));
    assert_eq!(verdict(0, true, &hand, 4), (1, "3 of 4 executions".into()));
    assert_eq!(
        verdict(1, true, &hand, 3),
        (1, "run-job exit 1, 3 of 3 executions".into())
    );
    let kept = "bundle not copied back; the laptop's job folder is kept".to_string();
    assert_eq!(verdict(0, false, &hand, 3), (1, kept.clone()));
    assert_eq!(verdict(124, false, &hand, 3), (124, kept));
    let dir = scratch("verdict");
    fs::write(dir.join("exec-1.json"), r#"{"exit_code": 0, "timed_out": false}"#).unwrap();
    fs::write(dir.join("exec-2.json"), r#"{"exit_code": -1, "timed_out": true}"#).unwrap();
    assert_eq!(
        verdict(0, true, &dir, 2),
        (1, "2 of 2 executions; non-zero exit: exec-2".into())
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A runner's run is refused before anything is stored.
#[test]
fn the_store_refuses_a_runners_run() {
    let at = scratch("runner");
    bundle(&at, RUNNER, RUNNER, "named_sync_objects-46053401854");
    let why = store(&at, RUNNER).unwrap_err();
    assert!(
        why.contains("not the laptop's (laptop-...): only a laptop run is stored"),
        "{why}"
    );
    assert!(why.contains("gha-windows-2025"), "{why}");
    assert!(!at.join("store").exists(), "a refused run wrote under store/");
    let _ = fs::remove_dir_all(&at);
}

/// A laptop run is stored whole: its twelve files equal, its binary under its sha256.
#[test]
fn the_store_keeps_a_laptop_run() {
    let at = scratch("laptop");
    let src = bundle(
        &at,
        HAND_SENT,
        HAND_SENT,
        "apcs_and_waitable_timers-20261127T210820791Z",
    );
    let exe = fs::read(root().join(format!("store/binaries/{HAND_SENT_BINARY}.exe"))).unwrap();
    fs::write(src.join("probe.exe"), &exe).unwrap();
    let line = store(&at, HAND_SENT).unwrap();
    assert!(
        line.contains("12 files, 3 executions, profile laptop-x64-10.0.26200.9974-Client"),
        "{line}"
    );
    let stored = root().join("store/probe-runs").join(HAND_SENT);
    let mut n = 0;
    for e in fs::read_dir(&stored).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap();
        assert_eq!(
            fs::read(&p).unwrap(),
            fs::read(at.join("store/probe-runs").join(HAND_SENT).join(name)).unwrap()
        );
        n += 1;
    }
    assert_eq!(n, 12);
    assert_eq!(
        fs::read_dir(at.join("store/probe-runs").join(HAND_SENT))
            .unwrap()
            .count(),
        12
    );
    assert_eq!(
        fs::read(at.join(format!("store/binaries/{HAND_SENT_BINARY}.exe"))).unwrap(),
        exe
    );
    let _ = fs::remove_dir_all(&at);
}

/// A probe.exe that is not the binary its probe.sha256 names is not stored under that name.
#[test]
fn the_store_refuses_a_binary_not_its_sha256() {
    let at = scratch("binary");
    let src = bundle(
        &at,
        HAND_SENT,
        HAND_SENT,
        "apcs_and_waitable_timers-20261127T210820791Z",
    );
    let mut exe = fs::read(root().join(format!("store/binaries/{HAND_SENT_BINARY}.exe"))).unwrap();
    exe[0] ^= 1;
    fs::write(src.join("probe.exe"), &exe).unwrap();
    let why = store(&at, HAND_SENT).unwrap_err();
    assert!(why.contains(&format!("probe.sha256 names {HAND_SENT_BINARY}")), "{why}");
    assert!(!at.join("store").exists());
    let _ = fs::remove_dir_all(&at);
}

/// A family's stored run: the October run of wait_handoff, its probe from its job.json; an unknown family refused.
#[test]
fn a_familys_stored_run() {
    let r = stored_run(&root(), "wait_handoff", None).unwrap();
    assert_eq!(r.run, OCTOBER);
    assert_eq!(
        r.binary,
        "c45bd6f706ee778fe35f45c6d81a006c5e517822a65d9c1efb3eb9f2aaacb7f6"
    );
    assert_eq!(r.dll, None);
    assert_eq!(r.probe, "wait_handoff");
    assert_eq!(stored_run(&root(), "wait_handoff", Some(OCTOBER)).unwrap(), r);
    assert!(stored_run(&root(), "wait_handoff", Some("38044061149")).is_err());
    assert!(stored_run(&root(), "no_such_family", None).is_err());
}

/// A binary job's folder: the probe's folder as tree.tar, the executor scripts, the stored binary, job.json naming
/// each by sha256.
#[test]
fn a_binary_jobs_folder() {
    let at = scratch("job");
    let job = at.join("lap-x");
    let request = Request {
        kind: Kind::Binary,
        probe: "wait_handoff".into(),
        binary: Some("c45bd6f706ee778fe35f45c6d81a006c5e517822a65d9c1efb3eb9f2aaacb7f6".into()),
        dll: None,
        repeat: 2,
        pin: Some("efficiency".into()),
    };
    let head = String::from_utf8(windows_machine::git(&root(), &["rev-parse", "HEAD"]).unwrap()).unwrap();
    let head = head.trim();
    let spec = write_job(&root(), &job, &request, head, "wait_handoff-x").unwrap();
    assert_eq!(fs::read_to_string(job.join("job.json")).unwrap(), spec.json());
    let tree = fs::read(job.join("tree.tar")).unwrap();
    assert_eq!(tree, tree_tar(&root(), head, Kind::Binary, "wait_handoff").unwrap());
    assert_eq!(spec.inputs[0], ("tree.tar".to_string(), sha256(&tree)));
    assert_eq!(
        spec.inputs[1],
        (
            "probe.exe".to_string(),
            sha256(&fs::read(job.join("probe.exe")).unwrap())
        )
    );
    for script in EXECUTOR {
        assert_eq!(
            fs::read(job.join(script)).unwrap(),
            fs::read(root().join("tools/windows_machine").join(script)).unwrap(),
            "{script}"
        );
    }
    assert!(spec.json().contains("\"kind\": \"binary\""));
    assert!(spec.json().contains("\"cpu_class\": \"efficiency\""));
    let _ = fs::remove_dir_all(&at);
}
