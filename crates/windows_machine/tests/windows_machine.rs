// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 371: the program held to the runs already made. `lap-20261127T210820791Z-b5a7cf3d` was sent by hand with the
//! Python's exact steps; `lap-20261017T040446837Z-dc268176` by the Python itself. Their stored `job.json` and the
//! tree.tar it names are what the program writes, and the store step keeps a laptop run and refuses a runner's.

use std::fs;
use std::path::{Path, PathBuf};

use windows_machine::{
    Batch, BatchItem, CODE_INTEGRITY, EXECUTOR, Game, HOST, JOINED, Job, KNOWN, Kind, LAPTOP_PROFILE, LINK, Py,
    Request, STEAM_ABSENT, batch_inputs, batch_items, code_integrity, declared_temp, laptop_profile, run_id, sha256,
    split_batch, split_game, stamp, steam_free, store, store_game, stored_run, temp_declared, tree_tar, verdict,
    write_job,
};

/// The TEMP `games/hail-to-the-rainbow/state.json` declares for its state `empty`, the reference runs' (Sprint 376).
const USER_TEMP: &str = "C:\\Users\\w2m\\AppData\\Local\\Temp";

/// Sprint 372's references: the October batch the Python sent (195 binaries, its job.json and the runs split from
/// it), and one execution of the reference set's game job.
const BATCH: &str = "lap-20261013T231059422Z-136bec80";
const GAME_RUN: &str = "lap-20261014T032442034Z-c8814547";
/// A run's state.json proof (run-job.ps1) with the user's TEMP, the path the reference runs read
const USER_TEMP_PROOF: &str = "{\"state\": \"empty\", \"temp_files_made\": \"cur*.tmp\", \"seen\": {\"temp_path\": \
                                \"C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\", \"temp_game_files\": 0}, \"differs\": []}";
/// The nine families whose rows the runner made, each by its runner run (`docs/postmortem-windows-runs-off-the-laptop-
/// 2026-11-27.md`, "What the wrong machine reached"), as Sprint 372's batch names them.
const RUNNER_FAMILIES: [(&str, &str); 9] = [
    ("locked_race", "41326239822"),
    ("thread_values_race", "45760720489"),
    ("thread_slots_race", "45763850660"),
    ("tls_free_reach", "45780352907"),
    ("tls_expansion_first", "45783238565"),
    ("wait_closed_handle", "45797800625"),
    ("wait_closed_kinds", "45813040124"),
    ("mutex_abandoned", "45871497648"),
    ("named_sync_objects", "46053401854"),
];

/// Sprint 372's batch is exactly the nine runner runs' binaries, each its run's probe.sha256, then the control's two;
/// named_sync_objects by its table row is the same binary as by its run, the row naming its laptop run from Sprint 374.
#[test]
fn the_batch_is_the_nine_runner_binaries_and_the_control() {
    let mut names: Vec<String> = RUNNER_FAMILIES.iter().map(|(f, r)| format!("{f}@{r}")).collect();
    names.push("sync_objects".into());
    let items = batch_items(&root(), &names).unwrap();
    assert_eq!(items.len(), 11);
    for (item, (family, run)) in items.iter().zip(RUNNER_FAMILIES) {
        let sha = fs::read_to_string(root().join("store/probe-runs").join(run).join("probe.sha256")).unwrap();
        assert_eq!(
            (
                item.family.as_str(),
                item.probe.as_str(),
                item.binary.as_str(),
                item.runs.as_str()
            ),
            (family, family, sha.trim(), run)
        );
        assert_eq!(item.dll, None);
    }
    let control: Vec<&str> = items[9..].iter().map(|i| i.binary.as_str()).collect();
    assert_eq!(
        control,
        [
            "a564e428b9e44d62b809d084179a261b08927fad516d862a6f8339a29bf0bbf6",
            "530e281c3c7800ab121fa13cf1ed9f1d15e95bbef4bb7e806ff04bd961c4482c"
        ]
    );
    let by_table = batch_items(&root(), &["named_sync_objects".to_string()]).unwrap();
    assert_eq!(
        by_table,
        [BatchItem {
            runs: "lap-20261127T221622950Z-62b931c2-009".into(),
            ..items[8].clone()
        }]
    );
}

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
        why.contains("not the laptop's (laptop-x64-10.0.26200.<revision>-Client): only a laptop run is stored"),
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

/// The October batch's items, read from its stored job.json.
fn october_items() -> (serde_json::Value, Vec<BatchItem>) {
    let text = fs::read_to_string(root().join("store/probe-runs").join(BATCH).join("job.json")).unwrap();
    let spec: serde_json::Value = serde_json::from_str(&text).unwrap();
    let s = |v: &serde_json::Value, k: &str| v[k].as_str().unwrap().to_string();
    let items = spec["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| BatchItem {
            family: s(i, "family"),
            probe: s(i, "probe"),
            binary: s(i, "binary"),
            runs: s(i, "runs"),
            dll: i.get("dll").map(|_| (s(i, "dll"), s(i, "dll_name"))),
            execution_id: s(i, "execution_id"),
        })
        .collect();
    (spec, items)
}

/// The October batch's job.json is the program's, byte for byte: its 195 items in the Python's key order, the binaries
/// and DLLs under bin/ once each in the items' order, `json.dumps(indent=1)`'s form.
#[test]
fn the_october_batchs_job_json_is_the_programs() {
    let (spec, items) = october_items();
    assert_eq!(items.len(), 195);
    let batch = Batch {
        run: BATCH.into(),
        repeat: 2,
        commit: spec["commit"].as_str().unwrap().into(),
        window: "hidden".into(),
        pin: None,
        inputs: batch_inputs(spec["inputs"]["tree.tar"].as_str().unwrap(), &items),
        items,
    };
    let stored = fs::read_to_string(root().join("store/probe-runs").join(BATCH).join("job.json")).unwrap();
    assert_eq!(batch.json(), stored);
    let pinned = Batch {
        pin: Some("performance".into()),
        ..batch
    };
    assert!(
        pinned
            .json()
            .contains("\n ],\n \"cpu_class\": \"performance\",\n \"inputs\": {\n")
    );
}

/// `json.dumps`'s escapes, ensure_ascii's included, and its empty containers.
#[test]
fn python_json_escapes() {
    // the escapes built from their hex digits: e9, 7f, and U+1F600's UTF-16 pair
    let u = |hex: &str| format!("{}u{hex}", '\\');
    let want = format!(r#""a\"b\\c\n\t{}{}{}{}""#, u("00e9"), u("007f"), u("d83d"), u("de00"));
    assert_eq!(Py::str("a\"b\\c\n\t\u{e9}\u{7f}\u{1F600}").dumps(), want);
    assert_eq!(
        Py::Dict(vec![("l".into(), Py::List(vec![])), ("d".into(), Py::Dict(vec![]))]).dumps(),
        "{\n \"l\": [],\n \"d\": {}\n}"
    );
}

/// The tables' items as the Python's batch_items made them for the October batch: late_tls's with their DLLs,
/// sync_objects' four runs as two binaries, in the order the families are named; named_sync_objects (in both tables)
/// once.
#[test]
fn batch_items_as_the_october_batchs() {
    let (_, october) = october_items();
    let names = ["late_tls".to_string(), "sync_objects".to_string()];
    let got = batch_items(&root(), &names).unwrap();
    let want: Vec<BatchItem> = october
        .into_iter()
        .filter(|i| names.contains(&i.family))
        .map(|i| BatchItem {
            execution_id: String::new(),
            ..i
        })
        .collect();
    assert_eq!(want.len(), 4);
    assert_eq!(got, want);
    let joined = batch_items(&root(), &["named_sync_objects".to_string()]).unwrap();
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].runs, "lap-20261127T221622950Z-62b931c2-009");
    assert_eq!(
        joined[0].binary,
        "446f77bc42c8f3f56c54513fff3ed245d2a42135b9683b622cca0a3372db069f"
    );
}

/// `windows_machine batch named_sync_objects` takes its item from `store/family_runs/joined-2026-11-27.tsv` (Sprint
/// 373): the family is in both tables, so at Sprint 372's grade the batch's lookup with the joined table dropped passed
/// every test. In a root whose known table holds none of the joined table's families, the family's one item is the
/// joined row's run and binary, as in the tree; with the joined table's rows gone, the family is not found.
#[test]
fn the_batch_reads_the_joined_table() {
    let at = scratch("joined");
    let joined = fs::read_to_string(root().join(JOINED)).unwrap();
    let rows: Vec<Vec<&str>> = joined
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').collect())
        .collect();
    let families: Vec<&str> = rows.iter().map(|c| c[0]).collect();
    let row = rows.iter().find(|c| c[0] == "named_sync_objects").unwrap();
    let known: String = fs::read_to_string(root().join(KNOWN))
        .unwrap()
        .lines()
        .filter(|l| !families.contains(&l.split('\t').next().unwrap()))
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(!known.contains("named_sync_objects"));
    fs::create_dir_all(at.join("store/family_runs")).unwrap();
    fs::create_dir_all(at.join("probes/named_sync_objects")).unwrap();
    fs::copy(
        root().join("probes/named_sync_objects/probe.json"),
        at.join("probes/named_sync_objects/probe.json"),
    )
    .unwrap();
    fs::write(at.join(KNOWN), &known).unwrap();
    fs::write(at.join(JOINED), &joined).unwrap();
    let name = ["named_sync_objects".to_string()];
    let items = batch_items(&at, &name).unwrap();
    assert_eq!(items, batch_items(&root(), &name).unwrap());
    assert_eq!(items.len(), 1);
    assert_eq!(
        (items[0].runs.as_str(), items[0].binary.as_str(), items[0].dll.as_ref()),
        (row[1], row[2], None)
    );
    let header: String = joined
        .lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect();
    fs::write(at.join(JOINED), header).unwrap();
    let why = batch_items(&at, &name).unwrap_err();
    assert!(why.contains("name no family named_sync_objects"), "{why}");
    let _ = fs::remove_dir_all(&at);
}

/// A family neither table holds is named by its stored run; without the run it is refused, as is a run not stored.
#[test]
fn a_run_named_by_id() {
    let got = batch_items(&root(), &["locked_race@41326239822".to_string()]).unwrap();
    assert_eq!(
        got,
        vec![BatchItem {
            family: "locked_race".into(),
            probe: "locked_race".into(),
            binary: "d19052c8d9ebfc8fc8a1992a8761a1ec7389501f2acfc661e552057094e053f9".into(),
            runs: "41326239822".into(),
            dll: None,
            execution_id: String::new(),
        }]
    );
    let why = batch_items(&root(), &["locked_race".to_string()]).unwrap_err();
    assert!(why.contains("name no family locked_race"), "{why}");
    assert!(batch_items(&root(), &["locked_race@no_such_run".to_string()]).is_err());
    let why = batch_items(&root(), &["late_tls@38229484497".to_string()]).unwrap_err();
    assert!(why.contains("loads a DLL"), "{why}");
}

/// A bundle as the laptop leaves it for the October batch's sync_objects binaries (items 154 and 155), under `at`.
fn sync_objects_bundle(at: &Path, batch: &str) -> Vec<BatchItem> {
    let mut items = batch_items(&root(), &["sync_objects".to_string()]).unwrap();
    let bundle = at.join("build/batches").join(batch);
    fs::create_dir_all(&bundle).unwrap();
    for name in ["profile.json", "run-job.log", "job.json"] {
        fs::copy(
            root().join("store/probe-runs").join(BATCH).join(name),
            bundle.join(name),
        )
        .unwrap();
    }
    for (k, item) in items.iter_mut().enumerate() {
        item.execution_id = format!("sync_objects-x-{:03}", k + 1);
        let from = root().join("store/probe-runs").join(format!("{BATCH}-{}", 154 + k));
        let to = bundle.join(&item.execution_id);
        fs::create_dir_all(&to).unwrap();
        for e in fs::read_dir(&from).unwrap() {
            let p = e.unwrap().path();
            if p.file_name().unwrap() != "batch.txt" {
                fs::copy(&p, to.join(p.file_name().unwrap())).unwrap();
            }
        }
    }
    items
}

/// A batch's bundle split into one run a binary and stored as the October batch's runs are: each run's files those of
/// the stored run, batch.txt naming the batch, the batch's three files once beside them.
#[test]
fn a_batch_is_split_and_stored() {
    let at = scratch("batch");
    let items = sync_objects_bundle(&at, "lap-b");
    let runs = split_batch(&at, "lap-b", &items, 2).unwrap();
    let short: Vec<&str> = runs.iter().filter(|r| r.1 != 0).map(|r| r.2.as_str()).collect();
    assert!(short.is_empty(), "short binaries: {short:?}");
    assert_eq!(
        runs.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        ["lap-b-001", "lap-b-002"]
    );
    fs::create_dir_all(at.join("store/binaries")).unwrap();
    for (k, (id, _, _)) in runs.iter().enumerate() {
        let sha = &items[k].binary;
        fs::copy(
            root().join(format!("store/binaries/{sha}.exe")),
            at.join(format!("store/binaries/{sha}.exe")),
        )
        .unwrap();
        let line = store(&at, id).unwrap();
        assert!(
            line.contains("8 files, 2 executions, profile laptop-x64-10.0.26200.7840-Client"),
            "{line}"
        );
        assert!(line.ends_with(", batch lap-b"), "{line}");
        let october = root().join("store/probe-runs").join(format!("{BATCH}-{}", 154 + k));
        let stored = at.join("store/probe-runs").join(id);
        assert_eq!(fs::read_dir(&stored).unwrap().count(), 8);
        for e in fs::read_dir(&october).unwrap() {
            let name = e.unwrap().file_name();
            let got = fs::read(stored.join(&name)).unwrap();
            if name == "batch.txt" {
                assert_eq!(got, b"lap-b", "the frozen Python's write_text(run): no newline");
            } else {
                assert_eq!(got, fs::read(october.join(&name)).unwrap(), "{name:?}");
            }
        }
    }
    for name in ["profile.json", "run-job.log", "job.json"] {
        assert_eq!(
            fs::read(at.join("store/probe-runs/lap-b").join(name)).unwrap(),
            fs::read(root().join("store/probe-runs").join(BATCH).join(name)).unwrap()
        );
    }
    let _ = fs::remove_dir_all(&at);
}

/// A binary whose folder did not come back is named with no executions, and fails the batch.
#[test]
fn a_binary_with_no_executions_is_named() {
    let at = scratch("short");
    let items = sync_objects_bundle(&at, "lap-s");
    fs::remove_dir_all(at.join("build/batches/lap-s").join(&items[1].execution_id)).unwrap();
    let runs = split_batch(&at, "lap-s", &items, 2).unwrap();
    assert_eq!(runs[0].1, 0);
    assert_eq!(runs[1].1, 1);
    assert_eq!(
        runs[1].2,
        "lap-s-002 sync_objects (38051400336, binary 530e281c3c7800ab121fa13cf1ed9f1d15e95bbef4bb7e806ff04bd961c4482c): \
         0 of 2 executions"
    );
    let _ = fs::remove_dir_all(&at);
}

/// The game job's limit is the Python's `executions * (seconds + 180) // 60 + 10`: 26 minutes for this sprint's job of
/// three 150-s executions, the floor of the division kept (3 × 330 = 990, 16.5 minutes).
#[test]
fn the_game_limit_is_the_pythons() {
    let job = |executions, seconds| Game {
        executions,
        seconds,
        audio: "off".into(),
        ..Game::default()
    };
    for (executions, seconds, limit) in [(3, 150, 26), (3, 180, 28), (10, 180, 70), (1, 150, 15), (10, 150, 65)] {
        assert_eq!(
            job(executions, seconds).limit_minutes(),
            limit,
            "{executions} x {seconds} s"
        );
    }
    assert!(
        job(3, 150)
            .json("lap-g", "c", "t")
            .contains("\n \"limit_minutes\": 26,\n")
    );
}

/// Each of these jobs' limits outlasts one execution, so none is refused before it is sent; a limit below one
/// execution's seconds would be (the game job's second plant).
#[test]
fn a_limit_below_one_execution_is_refused() {
    for (executions, seconds) in [(3, 150), (3, 180), (10, 180), (1, 150), (1, 3600)] {
        let game = Game {
            executions,
            seconds,
            ..Game::default()
        };
        assert_eq!(
            game.checked_limit(),
            Ok(game.limit_minutes()),
            "{executions} x {seconds} s: limit {} minutes",
            game.limit_minutes()
        );
    }
}

/// The game job's job.json in the Python's key order, its limit the Python's formula.
#[test]
fn a_game_jobs_json() {
    let game = Game {
        executions: 3,
        ..Game::default()
    };
    assert_eq!(game.limit_minutes(), 28);
    let want = "{\n \"kind\": \"game\",\n \"execution_id\": \"lap-g\",\n \"game\": \"hail-to-the-rainbow\",\n \
                \"exe\": \"Hail to the Rainbow\",\n \"seconds\": 180,\n \"mode\": \"batch\",\n \"audio\": \"on\",\n \
                \"input_script\": null,\n \"record\": false,\n \"pmc\": false,\n \"pmc_sample\": false,\n \
                \"capture\": \"on\",\n \"person\": false,\n \"frame_seconds\": 10,\n \"initial_state\": \"empty\",\n \
                \"plant_state\": false,\n \"executions\": 3,\n \"repeat\": 0,\n \"commit\": \"c0ffee\",\n \
                \"session\": \"desktop\",\n \"limit_minutes\": 28,\n \"inputs\": {\n  \"tree.tar\": \"t\"\n }\n}";
    assert_eq!(game.json("lap-g", "c0ffee", "t"), want);
    assert_eq!(Game::default().limit_minutes(), 70);
    let person = Game {
        person: true,
        input_script: Some("round-trip".into()),
        executions: 1,
        seconds: 150,
        ..Game::default()
    };
    assert_eq!(person.limit_minutes(), 15);
    let text = person.json("lap-p", "c", "t");
    assert!(text.contains("\"input_script\": \"round-trip\","), "{text}");
    assert!(text.contains("\"person\": true,\n \"frame_seconds\": 30,"), "{text}");
}

/// A game job's bundle split into executions and stored as the reference set's: the execution's files, batch.txt naming
/// the job, the job's profile.json; an execution without exec.json skipped; a runner's profile refused.
#[test]
fn a_game_job_is_split_and_stored() {
    let at = scratch("game");
    let bundle = at.join("build/batches/lap-g");
    let reference = root().join("store/game-runs").join(format!("{GAME_RUN}-01"));
    fs::create_dir_all(bundle.join("1")).unwrap();
    fs::create_dir_all(bundle.join("2")).unwrap();
    fs::copy(reference.join("profile.json"), bundle.join("profile.json")).unwrap();
    fs::copy(reference.join("unity.log"), bundle.join("2/unity.log")).unwrap();
    // the proof a run of Sprint 376 on writes, the TEMP the reference run read (its strings rows 3423 and 3496)
    fs::write(bundle.join("1/state.json"), USER_TEMP_PROOF).unwrap();
    for e in fs::read_dir(&reference).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap();
        if name != "batch.txt" && name != "profile.json" {
            fs::copy(&p, bundle.join("1").join(name)).unwrap();
        }
    }
    let (done, lines) = split_game(&at, "lap-g", 2).unwrap();
    assert_eq!(done, 1);
    assert_eq!(lines, ["lap-g-02: no exec.json"]);
    let line = store_game(&at, "lap-g-01", USER_TEMP).unwrap();
    assert_eq!(
        line,
        "lap-g-01: stored store/game-runs/lap-g-01, 20 files, profile laptop-x64-10.0.26200.7840-Client, unity.log 180 \
         lines, trace 3499 lines"
    );
    for e in fs::read_dir(&reference).unwrap() {
        let name = e.unwrap().file_name();
        let got = fs::read(at.join("store/game-runs/lap-g-01").join(&name)).unwrap();
        if name == "batch.txt" {
            assert_eq!(got, b"lap-g");
        } else {
            assert_eq!(got, fs::read(reference.join(&name)).unwrap(), "{name:?}");
        }
    }
    let runner = at.join("build/game-runs/lap-r");
    fs::create_dir_all(&runner).unwrap();
    fs::write(
        runner.join("profile.txt"),
        "gha-windows-2025-x64-10.0.26100.33828-Server",
    )
    .unwrap();
    let why = store_game(&at, "lap-r", USER_TEMP).unwrap_err();
    assert!(why.contains("only a laptop run is stored"), "{why}");
    assert!(!at.join("store/game-runs/lap-r").exists());
    let _ = fs::remove_dir_all(&at);
}

/// Sprint 376: an execution whose unity.log speaks of Steam without the reference runs' "SteamAPI_Init() failed" is not
/// stored, since the game's Steam lines with the client signed in name the account; the reference run's log passes, as
/// does a log that never names Steam. The refused log here is the reference log less that one line.
#[test]
fn a_log_with_the_steam_client_running_is_not_stored() {
    let at = scratch("steam");
    let reference = root().join("store/game-runs").join(format!("{GAME_RUN}-01"));
    let log = fs::read_to_string(reference.join("unity.log")).unwrap();
    assert!(log.contains(STEAM_ABSENT));
    let without: String = log
        .lines()
        .filter(|l| !l.contains(STEAM_ABSENT))
        .map(|l| format!("{l}\n"))
        .collect();
    let quiet: String = log
        .lines()
        .filter(|l| !l.to_lowercase().contains("steam"))
        .map(|l| format!("{l}\n"))
        .collect();
    for (id, text) in [
        ("lap-s-01", log.as_str()),
        ("lap-s-02", without.as_str()),
        ("lap-s-03", quiet.as_str()),
    ] {
        let dir = at.join("build/game-runs").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::copy(reference.join("profile.txt"), dir.join("profile.txt")).unwrap();
        fs::write(dir.join("unity.log"), text).unwrap();
    }
    assert_eq!(steam_free(&at.join("build/game-runs/lap-s-01")), Ok(()));
    let why = store_game(&at, "lap-s-02", USER_TEMP).unwrap_err();
    assert_eq!(
        why,
        "lap-s-02: unity.log speaks of Steam and has no \"SteamAPI_Init() failed\": the Steam client ran, and the game's \
         Steam lines may name its account, so the execution stays in build/"
    );
    assert!(!at.join("store/game-runs/lap-s-02").exists());
    assert!(store_game(&at, "lap-s-01", USER_TEMP).is_ok());
    assert!(store_game(&at, "lap-s-03", USER_TEMP).is_ok());
    assert!(steam_free(&at.join("build/game-runs/no-such-run")).is_ok());
    let _ = fs::remove_dir_all(&at);
}

/// Sprint 376: run-job.ps1's game kind holds the initial state games/<game>/state.json declares. Each item a declared
/// state names is read back in the script's `$seen` map, so the check before each execution covers it; and a state with
/// no Steam process has the script stop the Steam processes it counts before the first execution and start the client
/// again after the job. There is no PowerShell on this Mac, so the script is held by its text.
#[test]
fn the_game_kind_holds_every_declared_item_and_stops_the_steam_client() {
    let script = fs::read_to_string(root().join("tools/windows_machine/run-job.ps1")).unwrap();
    let states: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root().join("games/hail-to-the-rainbow/state.json")).unwrap())
            .unwrap();
    let seen = &script[script
        .find("$seen = [ordered]@{")
        .expect("the game kind reads the state back")..];
    let seen = &seen[..seen.find("\n        }").unwrap()];
    for (name, state) in states["states"].as_object().unwrap() {
        for key in state.as_object().unwrap().keys().filter(|k| *k != "about") {
            assert!(
                seen.contains(&format!("\n          {key} = ")),
                "state {name}'s {key} is not read back before an execution"
            );
        }
    }
    assert_eq!(states["states"]["empty"]["steam_processes"], 0);
    let stop = &script[script
        .find("$running = @(& $fromSteam)")
        .expect("the game kind counts the Steam client")..];
    let stop = &stop[..stop.find("Note \"Steam client: ").unwrap()];
    assert!(
        stop.contains("$running | ForEach-Object {") && stop.contains("Stop-Process -Id $_.Id -Force"),
        "the game kind does not stop the Steam client's processes it counts:\n{stop}"
    );
    assert!(script.contains("Start-Process -FilePath $steamExe -ArgumentList '-silent'"));
    // the game's TEMP: the user's own, as the job found it, which the state declares and w2m gives the game
    let temp = states["states"]["empty"]["temp_path"].as_str().unwrap();
    assert_eq!(temp, "C:\\Users\\w2m\\AppData\\Local\\Temp");
    let w2m = fs::read_to_string(root().join("crates/w2m_vm/src/bin/w2m-vm.rs")).unwrap();
    assert!(
        w2m.contains(&format!("\"TEMP={}\",", temp.replace('\\', "\\\\"))),
        "w2m's TEMP is not {temp}"
    );
    assert!(
        script.contains("$tempDir = $userTemp.TrimEnd('\\')")
            && script.contains("$env:TMP = $tempDir; $env:TEMP = $tempDir")
            && script.contains("temp_path = $tempDir"),
        "the game's TEMP is not the user's TEMP the job found, read back as temp_path"
    );
    assert_eq!(states["temp_files_made"], "cur*.tmp");
    assert!(script.contains("& $fromGameTemp | Remove-Item -Force"));
}

/// Sprint 376, held from Sprint 377 to the declared path (Sprint 376's Rubber Duck Observation 2): an execution whose
/// game read a TEMP other than the one `games/<game>/state.json` declares is not stored, whatever its proof says. The
/// reference run read the user's TEMP (strings rows 3423 and 3496, each after a GetTempPathW call); with a proof naming
/// that path it passes. A run whose proof and strings both name the job's own folder, as Sprint 376's first three
/// executions read, is refused at the row against the declared path; a proof naming that folder beside the reference
/// strings is refused at the proof; and a proof with no TEMP path is refused.
#[test]
fn a_run_whose_game_read_another_temp_is_not_stored() {
    let at = scratch("temp");
    let reference = root().join("store/game-runs").join(format!("{GAME_RUN}-01"));
    let dir = at.join("run");
    fs::create_dir_all(&dir).unwrap();
    for name in ["jit_capture.bin.trace.tsv", "jit_capture.bin.strings.tsv"] {
        fs::copy(reference.join(name), dir.join(name)).unwrap();
    }
    fs::write(dir.join("state.json"), USER_TEMP_PROOF).unwrap();
    assert_eq!(temp_declared(&dir, USER_TEMP), Ok(()));
    // the job's own folder in the proof and the strings alike: agreeing with each other, not with the declaration
    let job_temp_proof = USER_TEMP_PROOF.replace("C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp", "C:\\\\w2m\\\\temp");
    fs::write(dir.join("state.json"), &job_temp_proof).unwrap();
    let strings = fs::read_to_string(reference.join("jit_capture.bin.strings.tsv")).unwrap();
    assert!(strings.contains("\tC:\\Users\\w2m\\AppData\\Local\\Temp\\\n"));
    fs::write(
        dir.join("jit_capture.bin.strings.tsv"),
        strings.replace("C:\\Users\\w2m\\AppData\\Local\\Temp\\", "C:\\w2m\\temp\\"),
    )
    .unwrap();
    assert_eq!(
        temp_declared(&dir, USER_TEMP),
        Err(
            "the game's TEMP is C:\\w2m\\temp\\ (strings row 3423, after GetTempPathW), where its state declares \
             C:\\Users\\w2m\\AppData\\Local\\Temp\\"
                .to_string()
        )
    );
    fs::copy(
        reference.join("jit_capture.bin.strings.tsv"),
        dir.join("jit_capture.bin.strings.tsv"),
    )
    .unwrap();
    assert_eq!(
        temp_declared(&dir, USER_TEMP),
        Err(
            "state.json proves the TEMP C:\\w2m\\temp, where its state declares C:\\Users\\w2m\\AppData\\Local\\Temp\\"
                .to_string()
        )
    );
    fs::write(
        dir.join("state.json"),
        "{\"state\": \"empty\", \"seen\": {\"temp_entries\": 0}, \"differs\": []}",
    )
    .unwrap();
    assert_eq!(
        temp_declared(&dir, USER_TEMP),
        Err("state.json records no TEMP path: the TEMP the game read is not proved".to_string())
    );
    let _ = fs::remove_dir_all(&at);
}

/// Sprint 377: the declared TEMP is read from `games/<game>/state.json` at the commit the job sends, HEAD here; a state
/// it does not declare is refused before the job is sent.
#[test]
fn the_declared_temp_is_the_states_at_the_commit() {
    let head = String::from_utf8(windows_machine::git(&root(), &["rev-parse", "HEAD"]).unwrap()).unwrap();
    assert_eq!(
        declared_temp(&root(), head.trim(), "hail-to-the-rainbow", "empty"),
        Ok(USER_TEMP.to_string())
    );
    let why = declared_temp(&root(), head.trim(), "hail-to-the-rainbow", "no_such_state").unwrap_err();
    assert!(why.contains("declares no temp_path for state no_such_state"), "{why}");
}

/// Sprint 377: the one profile a stored Windows run may carry, `laptop-x64-10.0.26200.<revision>-Client`, the revision
/// free; the runner's, another build, another edition, an empty or a lettered revision and a trailing newline refused.
#[test]
fn the_laptop_profile_is_one_pattern() {
    assert_eq!(LAPTOP_PROFILE, "laptop-x64-10.0.26200.<revision>-Client");
    for ok in [
        "laptop-x64-10.0.26200.7840-Client",
        "laptop-x64-10.0.26200.9974-Client",
        "laptop-x64-10.0.26200.1-Client",
    ] {
        assert!(laptop_profile(ok), "{ok}");
    }
    for refused in [
        "gha-windows-2025-x64-10.0.26100.33828-Server",
        "laptop-x64-10.0.26100.1-Server",
        "laptop-x64-10.0.26200.7840-Server",
        "laptop-x64-10.0.26100.7840-Client",
        "laptop-arm64-10.0.26200.7840-Client",
        "laptop-x64-10.0.26200.-Client",
        "laptop-x64-10.0.26200.78a0-Client",
        "laptop-x64-10.0.26200.7840-Client\n",
        "laptop-",
        "",
    ] {
        assert!(!laptop_profile(refused), "{refused:?}");
    }
}

/// Sprint 377: every ssh and scp checks the laptop's host key under the alias the laptop is reached by, not under an
/// address it no longer answers at.
#[test]
fn the_link_checks_the_key_under_the_laptops_alias() {
    assert_eq!(HOST, "w2m-laptop");
    let alias: Vec<&str> = LINK
        .iter()
        .filter(|o| o.starts_with("HostKeyAlias="))
        .copied()
        .collect();
    assert_eq!(alias, [format!("HostKeyAlias={HOST}")]);
    assert!(!LINK.iter().any(|o| o.contains("192.168.")), "{LINK:?}");
}

/// One window as `run-job.ps1` writes it, measured on the laptop 2026-11-27 over 01:13-01:14Z on 2026-11-28: Sprint
/// 376's third block of handler_trace.dll.
const ONE_BLOCK: &str = "{\n  \"from\": \"2026-11-28T01:13:00.0000000Z\",\n  \"to\": \"2026-11-28T01:14:00.0000000Z\",\n  \
                         \"blocks\": [\n    {\n      \"time\": \"2026-11-28T01:13:05.3973818Z\",\n      \"file\": \
                         \"\\\\Device\\\\HarddiskVolume6\\\\w2m\\\\game\\\\handler_trace.dll\",\n      \"process\": \
                         \"\\\\Device\\\\HarddiskVolume6\\\\w2m\\\\game\\\\UnityCrashHandler64.exe\",\n      \"policy\": \
                         \"VerifiedAndReputableDesktop\"\n    }\n  ]\n}\n";

/// Sprint 377: a job's code-integrity window names each blocked file, so a job Smart App Control refused is not taken
/// for the program's missing output; a window with no block passes with its line; a missing file or a failed read is
/// refused, the window's blocks not being known.
#[test]
fn a_block_in_the_jobs_window_is_named() {
    let at = scratch("integrity");
    let why = code_integrity(&at).unwrap_err();
    assert!(why.starts_with("code-integrity.json: "), "{why}");
    assert!(why.ends_with("the job's code-integrity blocks are not known"), "{why}");
    fs::write(at.join(CODE_INTEGRITY), ONE_BLOCK).unwrap();
    assert_eq!(
        code_integrity(&at),
        Err(
            "CodeIntegrity blocked \\Device\\HarddiskVolume6\\w2m\\game\\handler_trace.dll (loaded by \
             \\Device\\HarddiskVolume6\\w2m\\game\\UnityCrashHandler64.exe, policy VerifiedAndReputableDesktop, at \
             2026-11-28T01:13:05.3973818Z) from 2026-11-28T01:13:00.0000000Z to 2026-11-28T01:14:00.0000000Z: the job's \
             missing output is the block's, not the program's"
                .to_string()
        )
    );
    let none = "\u{feff}{\n  \"from\": \"2026-11-20T00:00:00.0000000Z\",\n  \"to\": \"2026-11-20T00:05:00.0000000Z\",\n  \
                \"blocks\": []\n}\n";
    fs::write(at.join(CODE_INTEGRITY), none).unwrap();
    assert_eq!(
        code_integrity(&at),
        Ok("CodeIntegrity: no block from 2026-11-20T00:00:00.0000000Z to 2026-11-20T00:05:00.0000000Z".to_string())
    );
    fs::write(
        at.join(CODE_INTEGRITY),
        "{\"from\": \"a\", \"to\": \"b\", \"blocks\": [], \"error\": \"Attempted to perform an unauthorized operation.\"}",
    )
    .unwrap();
    let why = code_integrity(&at).unwrap_err();
    assert!(
        why.contains(
            "the CodeIntegrity log was not read (Attempted to perform an unauthorized operation.) from a to b"
        ),
        "{why}"
    );
    let _ = fs::remove_dir_all(&at);
}

/// Sprint 377: the executor records the code-integrity state each run met, and each job's window. There is no
/// PowerShell on this Mac, so the scripts are held by their text (both were parsed, and profile.ps1 run, on the laptop
/// 2026-11-27: no parse error; `code_integrity` {"verified_and_reputable_policy_state":1, "sac_previous_state":2,
/// "sac_enforcement_reason":1, "smart_app_control":"On"}).
#[test]
fn the_executor_records_the_code_integrity_state_and_window() {
    let profile = fs::read_to_string(root().join("tools/windows_machine/profile.ps1")).unwrap();
    assert!(profile.contains("$ci  = Get-ItemProperty 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\CI\\Policy'"));
    let entry = &profile[profile
        .find("  code_integrity   = [ordered]@{")
        .expect("profile.json carries code_integrity")..];
    let entry = &entry[..entry.find("\n  scheduler").unwrap()];
    for field in [
        "verified_and_reputable_policy_state = $ci.VerifiedAndReputablePolicyState",
        "sac_previous_state = $ci.SAC_PreviousState",
        "sac_enforcement_reason = $ci.SAC_EnforcementReason",
        "smart_app_control = $mp.SmartAppControlState",
    ] {
        assert!(
            entry.contains(field),
            "profile.ps1's code_integrity has no {field}:\n{entry}"
        );
    }
    let job = fs::read_to_string(root().join("tools/windows_machine/run-job.ps1")).unwrap();
    let opened = job
        .find("\n$jobStart = Get-Date")
        .expect("the window opens as the job starts");
    assert!(opened < job.find("Get-Content (Join-Path $Job 'job.json')").unwrap());
    let last = &job[job.rfind("} finally {").unwrap()..];
    for step in [
        "LogName = 'Microsoft-Windows-CodeIntegrity/Operational'; Id = 3077",
        "StartTime = $jobStart; EndTime = $ciEnd",
        "-notlike 'NoMatchingEventsFound*') { $window.error = \"$_\" }",
        "Set-Content -Path (Join-Path $out 'code-integrity.json')",
    ] {
        assert!(last.contains(step), "run-job.ps1's last finally has no {step:?}");
    }
    assert!(last.find("code-integrity.json").unwrap() < last.find("Remove-Item $src, $tree").unwrap());
    assert_eq!(CODE_INTEGRITY, "code-integrity.json");
}
