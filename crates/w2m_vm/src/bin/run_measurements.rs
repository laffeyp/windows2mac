// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `run_measurements` (Sprint 163): two of step 3's measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended
//! 2026-10-27) on a substrate record, as a Producer (`harness/w2m-vm-measurements.toml`). One VM a process, so it runs
//! each measurement as a `w2m-vm` child through `scripts/sign_and_run.sh`:
//! - `qpc` and `sleeper` (`tests/data/timed_waits`), each to ExitProcess with `--serve-times`: per exit kind, the
//!   served exits' host time as `exit_ns_p50` and `exit_ns_p99` (nearest rank) and their count as `vm_exits`;
//! - each guest again with `--runtime-word` (Sprint 164): the runtime word's hold and wait per taking, by the site
//!   that took it, as `runtime_word_hold_ns_p50/p99` and `runtime_word_wait_ns_p50/p99` (nearest rank over the exact
//!   ticks), the takings an exit interrupted apart;
//! - `--wfe-probe`: `wfe_exits` for `wfe` and `wfi`, the WFx traps of 1,000 each.
//! - step 6's two-vCPU run (Sprint 337): `probes/locked_race`, the binary its laptop run's `probe.sha256` names
//!   (`LOCKED_RACE_RUN`, the runner's run 41326239822 until Sprint 374), with `--vcpus 2 --vcpu-figures`: `locked_model_steps` (`count`) and `turn_start_ns_p50` and
//!   `turn_start_ns_p99` (`ns`, nearest rank over each turn's start), each with `vcpus` 2;
//! - the runtime word on two vCPUs while the waits and releases run (Sprint 356, its amendment 2):
//!   `tests/data/timed_waits/semaphore_units` with `--vcpus 2 --runtime-word`, its output "done": the word's hold and
//!   wait per taking by site, each with `vcpus` 2, over the run's first `LOCK_TAKINGS` takings, the takings counted
//!   named in each variant; from Sprint 358 its waits trusted, each wait's start its vCPU's own word, and a wait or a
//!   hold that ran backward fails the run; and `runtime_word_takings` for each timed run;
//! - each leaf lock's takings, hold and wait (Sprint 358): `lock_takings`, `lock_hold_ns_p50/p99` and
//!   `lock_wait_ns_p50/p99`, each with its `lock`, on the guest that drives it, on two vCPUs (`vcpus` 2) and on one:
//!   the sync objects' on `semaphore_units`, the handle table's on `handle_values`, the heap's on `heap_blocks`, the
//!   slots' on `probes/thread_slots_race`. A lock's takings below the bound its guest's own counts of its calls make, an
//!   entry written with its lock let go, a wait or a hold that ran backward, a wait on one vCPU, or a lock its guest's
//!   threads never met at on two vCPUs (its 99th-percentile wait 0) fails the run by name;
//! - the capabilities `vcpus_allowed` and the run-loop design read (Sprint 337), one CAPABILITY_PROBED each:
//!   `feat_lse2` and `feat_bbm_level2` from `w2m-vm --capabilities`, `guest_wfe_exit` from the WFE probe's run.
//!
//! The record: SESSION_INIT (purpose measurement), a MEASUREMENT_RECORDED per number, the three CAPABILITY_PROBED,
//! SESSION_COMPLETED. A child that fails, a guest whose served exits do not add up to its own exit tally less its last,
//! a timed run with more takings than the timing area holds or a time that ran backward, or a two-vCPU run whose rows
//! are not the Windows run's, one of whose vCPUs never ran, or which timed no turn fails the Producer by name.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use events::{
    Capability, CapabilityOutcome, CapabilityProbed, FiniteF64, HarnessU64, Id, Metric, SessionCompleted, SessionInit,
    SessionOutcome, SessionPurpose, VOCAB_VERSION, Version, harness_line,
};
use w2m_vm::monitor_code::Site;
use w2m_vm::record::{lock_measurement_line, measurement_line, measurement_line_on, quantile};

const GUESTS: [&str; 2] = ["qpc", "sleeper"];
/// The Windows run whose `locked_race` binary and rows the two-vCPU run is held to (Sprint 331): from Sprint 374 the
/// laptop's, its binary and rows the runner's run 41326239822's by `cmp`.
const LOCKED_RACE_RUN: &str = "store/probe-runs/lap-20261127T221622950Z-62b931c2-001";
/// The vCPUs of step 6's run.
const TWO: u32 = 2;

/// This host as the frozen Python's probes named it (`scripts/litmus_mp.py` 59 at the tag frozen-python-2026-10-29):
/// `mac-<hw.model>-macos-<build>`, the build `sw_vers -buildVersion`.
fn host_id() -> Result<String, String> {
    let said = |program: &str, args: &[&str]| -> Result<String, String> {
        let out = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| format!("{program}: {e}"))?;
        if !out.status.success() {
            return Err(format!("{program} {args:?}: {}", out.status));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    Ok(format!(
        "mac-{}-macos-{}",
        said("sysctl", &["-n", "hw.model"])?,
        said("sw_vers", &["-buildVersion"])?
    ))
}

/// One CAPABILITY_PROBED of this host.
fn probed(capability: Capability, host: &str, outcome: CapabilityOutcome, detail: String) -> String {
    harness_line(&CapabilityProbed {
        capability,
        host_id: Id::new(host).expect("not empty"),
        outcome,
        detail,
        execution_id: None,
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// One `w2m-vm` child, signed with the Hypervisor entitlement; its standard output.
fn child(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new(root.join("scripts/sign_and_run.sh"))
        .arg(root.join("target/release/w2m-vm"))
        .args(args)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("w2m-vm {args:?}: {e}"))?;
    // the input line `w2m-vm` reads as a Producer would
    c.stdin.take().unwrap().write_all(b"{}\n").map_err(|e| e.to_string())?;
    let out = c.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "w2m-vm {args:?}: {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The Windows run whose `thread_slots_race` binary and rows the slots' lock is measured on (Sprint 345).
const SLOTS_RACE_RUN: &str = "store/probe-runs/45763850660";

/// A probe's rows but `tls.overlap`, which counts how the two loops met and varies run to run (`run_loop.rs`,
/// `two_threads_taking_and_freeing_tls_and_fls_slots_at_once_never_share_one_on_two_vcpus`).
fn but_overlap(body: &str) -> String {
    body.lines()
        .filter(|l| !l.starts_with("{\"s\":\"tls.overlap\","))
        .map(|l| format!("{l}\n"))
        .collect()
}

/// One timed run (Sprint 358): `exe` on `vcpus` vCPUs with `--runtime-word`, which times the runtime word and the leaf
/// locks; its output and its timing file.
fn timed_run(root: &Path, exe: &str, vcpus: u32, name: &str) -> Result<(Vec<u8>, serde_json::Value), String> {
    let (output, word) = (
        format!("build/measurements/{name}-{vcpus}-stdout.txt"),
        format!("build/measurements/{name}-{vcpus}-word.json"),
    );
    let n = vcpus.to_string();
    child(
        root,
        &[exe, "--vcpus", &n, "--output", &output, "--runtime-word", &word],
    )?;
    let out = std::fs::read(root.join(&output)).map_err(|e| format!("{output}: {e}"))?;
    let json = serde_json::from_str(&std::fs::read_to_string(root.join(&word)).map_err(|e| format!("{word}: {e}"))?)
        .map_err(|e| format!("{word}: {e}"))?;
    Ok((out, json))
}

/// A lock's takings as a timed run wrote them: [wait start, taken, released, the lock word at the entry's write].
type LockEntries = Vec<[u64; 4]>;

/// Every leaf lock's takings from a timed run's file, held to the judges every timed run meets (Sprint 358): as many
/// written as the area holds of those counted; every entry written while its holder held the lock, its word read 1
/// (a word of 0 is an entry written after its release); no wait and no hold that ran backward, each taker's three
/// times its own vCPU's counter; and on one vCPU no wait at all, there being no other taker. The locks in
/// `events::Lock::ALL`'s order, each with its count.
fn leaf_locks(
    json: &serde_json::Value,
    label: &str,
    vcpus: u32,
) -> Result<Vec<(events::Lock, u64, LockEntries)>, String> {
    let mut out = Vec::new();
    for &lock in events::Lock::ALL {
        let name = lock.as_str();
        let area = &json["locks"][name];
        let counted = area["counted"]
            .as_u64()
            .ok_or(format!("{label}: no takings counted for the {name} lock"))?;
        let takings: LockEntries =
            serde_json::from_value(area["takings"].clone()).map_err(|e| format!("{label}: the {name} lock: {e}"))?;
        let n = takings.len();
        if n as u64 != counted.min(w2m_vm::monitor_code::LOCK_AREA_TAKINGS) {
            return Err(format!(
                "{label}: the {name} lock counted {counted} takings and wrote {n}"
            ));
        }
        let let_go = takings.iter().filter(|t| t[3] != 1).count();
        if let_go != 0 {
            return Err(format!(
                "{label}: the {name} lock: {let_go} of its first {n} entries written with the lock let go, its word \
                 not 1"
            ));
        }
        let (waits_back, holds_back) = (
            takings.iter().filter(|t| t[1] < t[0]).count(),
            takings.iter().filter(|t| t[2] < t[1]).count(),
        );
        if waits_back + holds_back != 0 {
            return Err(format!(
                "{label}: the {name} lock: of its first {n} takings, {waits_back} waits and {holds_back} holds ran \
                 backward"
            ));
        }
        if vcpus == 1 {
            let waited = takings.iter().filter(|t| t[1] != t[0]).count();
            if waited != 0 {
                return Err(format!(
                    "{label}: the {name} lock: {waited} of its first {n} takings waited, with no other vCPU to take it"
                ));
            }
        }
        out.push((lock, counted, takings));
    }
    Ok(out)
}

/// A lower bound on the takings of the lock a guest drives, from the guest's own counts of the calls it made (Sprint
/// 358), each call that answered taking the lock at least once:
/// - semaphore_units (sync objects): every ReleaseSemaphore, TRUE or FALSE, and every wait on the semaphore, satisfied
///   or timed out (`.counts` released, refused, satisfied, timed_out); each holds `sync_lock` once or more;
/// - handle_values (handles): every value a create or a duplicate gave, and every close, TRUE or FALSE (`.counts`
///   claimed, closed, refused); each publish, duplicate and close holds the table's lock once;
/// - heap_blocks (heap): every HeapAlloc that gave a block and every HeapFree that answered TRUE (`.counts` allocated,
///   freed); each `take` and `give_back` holds the heap once;
/// - thread_slots_race (slots): each of threads A's and B's rounds' HOLD TlsAlloc and HOLD TlsFree (its rows' rounds,
///   none out of indexes or range, HOLD from `probes/thread_slots_race/thread_slots_race.c`), and FlsAlloc, the
///   FlsSetValue calls that answered TRUE and FlsFree (its row `fls`: allocated, set_ok, freed).
fn takings_bound(lock: events::Lock, json: &serde_json::Value, output: &[u8], root: &Path) -> Result<u64, String> {
    let counts: Vec<u64> = serde_json::from_value(json["counts"].clone()).map_err(|e| e.to_string())?;
    let sum = |at: &[usize]| -> Result<u64, String> {
        at.iter()
            .map(|&i| {
                counts
                    .get(i)
                    .copied()
                    .ok_or(format!("the guest's .counts holds no word {i}"))
            })
            .sum()
    };
    match lock {
        events::Lock::SyncObjects => sum(&[0, 1, 2, 3]),
        events::Lock::Handles => sum(&[0, 2, 3]),
        events::Lock::Heap => sum(&[0, 1]),
        events::Lock::Slots => {
            let source = std::fs::read_to_string(root.join("probes/thread_slots_race/thread_slots_race.c"))
                .map_err(|e| e.to_string())?;
            let hold: u64 = source
                .lines()
                .find_map(|l| l.strip_prefix("#define HOLD "))
                .and_then(|v| v.trim().trim_end_matches('u').parse().ok())
                .ok_or("thread_slots_race.c defines no HOLD")?;
            let rows: Vec<serde_json::Value> = String::from_utf8_lossy(output)
                .lines()
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()
                .map_err(|e| format!("thread_slots_race's rows: {e}"))?;
            let row = |s: &str| {
                rows.iter()
                    .find(|r| r["s"] == s)
                    .ok_or(format!("thread_slots_race: no {s} row"))
            };
            let mut bound = 0;
            for s in ["tls.a", "tls.b"] {
                let r = row(s)?;
                if r["out_of_indexes"] != 0 || r["out_of_range"] != 0 {
                    return Err(format!("thread_slots_race: its {s} row ran out of indexes: {r}"));
                }
                bound += 2
                    * hold
                    * r["rounds"]
                        .as_u64()
                        .ok_or(format!("thread_slots_race: {s} has no rounds"))?;
            }
            let fls = row("fls")?;
            let alloc = row("fls.alloc")?;
            for (r, k) in [(alloc, "allocated"), (fls, "set_ok"), (fls, "freed")] {
                bound += r[k].as_u64().ok_or(format!("thread_slots_race: no {k}"))?;
            }
            Ok(bound)
        }
    }
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
    let root = root();
    std::fs::create_dir_all(root.join("build/measurements")).map_err(|e| e.to_string())?;
    let mut lines = vec![harness_line(&SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new("w2m-vm-measurements").expect("not empty"),
        contract_version: Version::new("0.0.0").expect("a version"),
        harness_version: Version::new(env!("CARGO_PKG_VERSION")).expect("Cargo's package version"),
        purpose: SessionPurpose::Measurement,
    })];

    for guest in GUESTS {
        let times = format!("build/measurements/{guest}-serve.json");
        let exe = format!("tests/data/timed_waits/{guest}.exe");
        let output = format!("build/measurements/{guest}-stdout.txt");
        child(&root, &[&exe, "--output", &output, "--serve-times", &times])?;
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(&times)).map_err(|e| format!("{times}: {e}"))?)
                .map_err(|e| format!("{times}: {e}"))?;
        let mut kinds: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (kind, ns) in json["kinds"].as_object().ok_or("no kinds")? {
            let mut v: Vec<u64> = ns
                .as_array()
                .ok_or("kind not a list")?
                .iter()
                .filter_map(|x| x.as_u64())
                .collect();
            v.sort_unstable();
            kinds.insert(kind.clone(), v);
        }
        // the served hvc exits are the run's own tally less its last exit, which ended the run unserved, and less each
        // flush stub's `hvc #15` (Sprint 305), an exit inside a split's service that the run loop tallies (`run.rs`'s
        // turn start) and times as that service's part, not as its own (Sprint 337: sleeper's three flushes failed the
        // run, 22 served of a tally of 26)
        let exits = json["exits"].as_object().ok_or("no exits")?;
        let tally: u64 = exits.values().filter_map(|v| v.as_u64()).sum();
        let flushes = exits
            .get(&w2m_vm::monitor_code::HVC_TLB_FLUSHED.to_string())
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let served: u64 = kinds
            .iter()
            .filter(|(k, _)| k.starts_with("hvc "))
            .map(|(_, v)| v.len() as u64)
            .sum();
        if served + 1 + flushes != tally {
            return Err(format!(
                "{guest}: {served} served hvc exits and {flushes} flushes, the run's tally {tally}"
            ));
        }
        let application = format!("timed-waits-{guest}");
        for (kind, ns) in &kinds {
            let tag = kind.replace(' ', "-");
            lines.push(measurement_line(
                &format!("{guest}-exits-{tag}"),
                Metric::VmExits,
                ns.len() as f64,
                "exits",
                &application,
                Some(kind),
            ));
            for (metric, q, name) in [(Metric::ExitNsP50, 0.5, "p50"), (Metric::ExitNsP99, 0.99, "p99")] {
                lines.push(measurement_line(
                    &format!("{guest}-exit-ns-{name}-{tag}"),
                    metric,
                    quantile(ns, q) as f64,
                    "ns",
                    &application,
                    Some(kind),
                ));
            }
        }

        // the runtime word's hold and wait times (Sprint 164): a second run with the takings timed in the guest, apart
        // from the first, so the timing's few instructions are not in the exit times above
        let word = format!("build/measurements/{guest}-word.json");
        let timed_output = format!("build/measurements/{guest}-timed-stdout.txt");
        child(&root, &[&exe, "--output", &timed_output, "--runtime-word", &word])?;
        let (plain, timed) = (
            std::fs::read(root.join(&output)).map_err(|e| e.to_string())?,
            std::fs::read(root.join(&timed_output)).map_err(|e| e.to_string())?,
        );
        if guest == "sleeper" && plain != timed {
            return Err(format!(
                "{guest}: the timed run's output differs from the untimed run's"
            ));
        }
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(&word)).map_err(|e| format!("{word}: {e}"))?)
                .map_err(|e| format!("{word}: {e}"))?;
        let counted = json["counted"].as_u64().ok_or(format!("{word}: no count"))?;
        let takings: Vec<[u64; 7]> =
            serde_json::from_value(json["takings"].clone()).map_err(|e| format!("{word}: {e}"))?;
        if counted != takings.len() as u64 {
            return Err(format!(
                "{guest}: {counted} takings counted, {} written: the timing area is full",
                takings.len()
            ));
        }
        // [site, wait start, taken, released, exits at each]: a time that ran backward fails the run by name
        let backward = takings.iter().filter(|t| t[2] < t[1] || t[3] < t[2]).count();
        if backward != 0 {
            return Err(format!("{guest}: {counted} takings, {backward} backward"));
        }
        // the leaf locks the same run timed (Sprint 358), held to their judges on one vCPU, and the takings counted
        leaf_locks(&json, guest, 1)?;
        lines.push(measurement_line(
            &format!("{guest}-runtime-word-takings"),
            Metric::RuntimeWordTakings,
            counted as f64,
            "count",
            &application,
            Some(&format!("{} written", takings.len())),
        ));
        let ns_per_tick = 1e9 / w2m_vm::host_state::counter_frequency()? as f64;
        for site in Site::ALL {
            let at: Vec<&[u64; 7]> = takings.iter().filter(|t| t[0] == site as u64).collect();
            // a hold or a wait within one run of the vCPU, apart from one an exit interrupted, which holds the host's
            // time too (a bridge's C service exits by hvc #8 while it holds the word)
            for (which, [p50, p99], start, end) in [
                (
                    "hold",
                    [Metric::RuntimeWordHoldNsP50, Metric::RuntimeWordHoldNsP99],
                    2,
                    3,
                ),
                (
                    "wait",
                    [Metric::RuntimeWordWaitNsP50, Metric::RuntimeWordWaitNsP99],
                    1,
                    2,
                ),
            ] {
                for across in [false, true] {
                    let mut ticks: Vec<u64> = at
                        .iter()
                        .filter(|t| (t[start + 3] != t[end + 3]) == across)
                        .map(|t| t[end] - t[start])
                        .collect();
                    if ticks.is_empty() {
                        continue;
                    }
                    ticks.sort_unstable();
                    let n = ticks.len();
                    let (tag, variant) = if across {
                        ("-across-exits", format!("{}; across an exit; {n} takings", site.name()))
                    } else {
                        ("", format!("{}; {n} takings", site.name()))
                    };
                    for (metric, q, name) in [(p50, 0.5, "p50"), (p99, 0.99, "p99")] {
                        lines.push(measurement_line(
                            &format!("{guest}-runtime-word-{}-{which}-ns-{name}{tag}", site.name()),
                            metric,
                            (quantile(&ticks, q) as f64 * ns_per_tick).round(),
                            "ns",
                            &application,
                            Some(&variant),
                        ));
                    }
                }
            }
        }
    }

    let probe: serde_json::Value =
        serde_json::from_str(child(&root, &["--wfe-probe"])?.trim()).map_err(|e| format!("the WFE probe: {e}"))?;
    for which in ["wfe", "wfi"] {
        let exits = probe[which]["exits"]
            .as_u64()
            .ok_or(format!("the probe gave no {which} exits"))?;
        if probe[which]["ended"] != true {
            return Err(format!("the probe's {which} run did not reach its hvc #0"));
        }
        lines.push(measurement_line(
            &format!("wfe-probe-{which}"),
            Metric::WfeExits,
            exits as f64,
            "count",
            "wfe-probe",
            Some(which),
        ));
    }

    // step 6's two-vCPU run (Sprint 337): locked_race, its rows held to the Windows run's as `run_loop.rs`'s test
    // holds them, so a run that lost an add is not measured
    let binary = std::fs::read_to_string(root.join(LOCKED_RACE_RUN).join("probe.sha256"))
        .map_err(|e| format!("{LOCKED_RACE_RUN}/probe.sha256: {e}"))?;
    let program = format!("store/binaries/{}.exe", binary.trim());
    let (race_output, race_figures) = (
        "build/measurements/locked_race-stdout.txt",
        "build/measurements/locked_race-vcpus.json",
    );
    let two = TWO.to_string();
    child(
        &root,
        &[
            &program,
            "--vcpus",
            &two,
            "--output",
            race_output,
            "--vcpu-figures",
            race_figures,
        ],
    )?;
    let windows = std::fs::read(root.join(LOCKED_RACE_RUN).join("body-1.jsonl")).map_err(|e| e.to_string())?;
    if std::fs::read(root.join(race_output)).map_err(|e| format!("{race_output}: {e}"))? != windows {
        return Err(format!(
            "locked_race on {TWO} vCPUs: its rows are not Windows run {LOCKED_RACE_RUN}'s"
        ));
    }
    let figures: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(race_figures)).map_err(|e| format!("{race_figures}: {e}"))?,
    )
    .map_err(|e| format!("{race_figures}: {e}"))?;
    // the count the run was given, which every two-vCPU row carries; nonzero by type (E14)
    let vcpus = figures["vcpus"]
        .as_u64()
        .and_then(NonZeroU64::new)
        .ok_or(format!("{race_figures}: no vCPU count above 0"))?;
    let runs: Vec<u64> = serde_json::from_value(figures["runs_by_vcpu"].clone()).map_err(|e| e.to_string())?;
    if runs.len() as u64 != vcpus.get() || runs.contains(&0) {
        return Err(format!(
            "locked_race on {vcpus} vCPUs: runs by vCPU {runs:?}, a vCPU that never ran"
        ));
    }
    let locked = figures["locked_steps"]
        .as_u64()
        .ok_or(format!("{race_figures}: no locked steps"))?;
    let mut turns: Vec<u64> = serde_json::from_value(figures["turn_start_ns"].clone()).map_err(|e| e.to_string())?;
    if turns.is_empty() {
        return Err(format!("locked_race on {vcpus} vCPUs: no turn's start timed"));
    }
    turns.sort_unstable();
    let race = "locked-race";
    lines.push(measurement_line_on(
        vcpus,
        "locked-race-locked-model-steps",
        Metric::LockedModelSteps,
        locked as f64,
        "count",
        race,
        None,
    ));
    let n = turns.len();
    for (metric, q, name) in [
        (Metric::TurnStartNsP50, 0.5, "p50"),
        (Metric::TurnStartNsP99, 0.99, "p99"),
    ] {
        lines.push(measurement_line_on(
            vcpus,
            &format!("locked-race-turn-start-ns-{name}"),
            metric,
            quantile(&turns, q) as f64,
            "ns",
            race,
            Some(&format!("{n} turns")),
        ));
    }

    // the runtime word on two vCPUs while the waits and releases run (Sprint 356, its amendment 2):
    // `tests/data/timed_waits/semaphore_units`, its output "done", with the takings timed; its takings pass the timing
    // area's LOCK_TAKINGS, so the figures are over the takings the area holds, the run's first, and each variant names
    // them and the takings counted
    let (units_output, units_word) = (
        "build/measurements/semaphore_units-stdout.txt",
        "build/measurements/semaphore_units-word.json",
    );
    child(
        &root,
        &[
            "tests/data/timed_waits/semaphore_units.exe",
            "--vcpus",
            &two,
            "--output",
            units_output,
            "--runtime-word",
            units_word,
        ],
    )?;
    if std::fs::read(root.join(units_output)).map_err(|e| format!("{units_output}: {e}"))? != b"done\n" {
        return Err(format!("semaphore_units on {TWO} vCPUs: its output is not \"done\""));
    }
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(units_word)).map_err(|e| format!("{units_word}: {e}"))?,
    )
    .map_err(|e| format!("{units_word}: {e}"))?;
    let counted = json["counted"].as_u64().ok_or(format!("{units_word}: no count"))?;
    let takings: Vec<[u64; 7]> =
        serde_json::from_value(json["takings"].clone()).map_err(|e| format!("{units_word}: {e}"))?;
    if takings.len() as u64 != counted.min(w2m_vm::monitor_code::LOCK_TAKINGS) {
        return Err(format!(
            "semaphore_units on {TWO} vCPUs: {counted} takings counted, {} written",
            takings.len()
        ));
    }
    // a hold, taken to released, is written by the word's holder alone, and from Sprint 358 a wait's start is its vCPU's
    // own word (`monitor_code::RUNTIME_WAIT_START`), which no other vCPU writes: so on two vCPUs a hold or a wait that
    // ran backward is a fault, and fails the run by name, as on one (before, the one header word a second waiter wrote
    // over left 4,349 of 16,384 waits, then 146, starting past their taking, and the waits were a lower bound)
    let held_back = takings.iter().filter(|t| t[3] < t[2]).count();
    let waited_back = takings.iter().filter(|t| t[2] < t[1]).count();
    if held_back + waited_back != 0 {
        return Err(format!(
            "semaphore_units on {TWO} vCPUs: of the runtime word's first {} takings, {waited_back} waits and \
             {held_back} holds ran backward",
            takings.len()
        ));
    }
    let ns_per_tick = 1e9 / w2m_vm::host_state::counter_frequency()? as f64;
    let units = "timed-waits-semaphore-units";
    // the runtime C's own takings among them (Sprint 358's amendment, `Site::Runtime`), held to a bound from the
    // guest's own counts: every SetEvent of the manual-reset gate that let its wait through (`.counts` gated) readies
    // that event's waiters under the word (`sync_objects.c` k32_SetEvent, `w2m_signal`), and every 1 ms wait on a
    // thread's handle that timed out (`.counts` watched) checks one of the host's objects under the word
    // (`wait_one_held`); each takes it once at least, in the C, the eight services' bridges taking none
    let in_c = json["counted_in_c"]
        .as_u64()
        .ok_or(format!("{units_word}: no count of the C's takings"))?;
    let guest_counts: Vec<u64> = serde_json::from_value(json["counts"].clone()).map_err(|e| e.to_string())?;
    // `.counts` words 15 and 16 (`semaphore_units.c` struct judged: gated, watched; the section's size rounds the
    // struct's 17 words up)
    let (gated, watched) = match guest_counts.get(15..17) {
        Some(&[g, w]) => (g, w),
        _ => {
            return Err(format!(
                "semaphore_units: {} .counts words, not the 17 it counts",
                guest_counts.len()
            ));
        }
    };
    if in_c < gated + watched || in_c > counted {
        return Err(format!(
            "semaphore_units on {TWO} vCPUs: the runtime C took the word {in_c} times, of {counted} takings, below \
             the {} its {gated} gate releases and {watched} timed-out 1 ms waits make",
            gated + watched
        ));
    }
    lines.push(measurement_line_on(
        vcpus,
        "semaphore-units-runtime-word-takings",
        Metric::RuntimeWordTakings,
        counted as f64,
        "count",
        units,
        Some(&format!(
            "{} written, {waited_back} of them waits run backward; {in_c} of them the runtime C's, at least {} \
             from the guest's {gated} gate releases and {watched} timed-out 1 ms waits",
            takings.len(),
            gated + watched
        )),
    ));
    for site in Site::ALL {
        let at: Vec<&[u64; 7]> = takings.iter().filter(|t| t[0] == site as u64).collect();
        for (which, [p50, p99], start, end) in [
            (
                "hold",
                [Metric::RuntimeWordHoldNsP50, Metric::RuntimeWordHoldNsP99],
                2,
                3,
            ),
            (
                "wait",
                [Metric::RuntimeWordWaitNsP50, Metric::RuntimeWordWaitNsP99],
                1,
                2,
            ),
        ] {
            let mut ticks: Vec<u64> = at.iter().map(|t| t[end] - t[start]).collect();
            if ticks.is_empty() {
                continue;
            }
            ticks.sort_unstable();
            let variant = format!(
                "{}; {} of the run's first {} takings, of {counted} counted",
                site.name(),
                ticks.len(),
                takings.len()
            );
            for (metric, q, name) in [(p50, 0.5, "p50"), (p99, 0.99, "p99")] {
                lines.push(measurement_line_on(
                    vcpus,
                    &format!("semaphore-units-runtime-word-{}-{which}-ns-{name}", site.name()),
                    metric,
                    (quantile(&ticks, q) as f64 * ns_per_tick).round(),
                    "ns",
                    units,
                    Some(&variant),
                ));
            }
        }
    }

    // each leaf lock's takings, hold and wait (Sprint 358; vocabulary 0.14, P30-P32), on the guest that drives it, on two
    // vCPUs and on one: the sync objects' on semaphore_units (its two-vCPU run the one above), the handle table's on
    // handle_values, the heap's on heap_blocks, the slots' on probes/thread_slots_race
    let slots_binary = std::fs::read_to_string(root.join(SLOTS_RACE_RUN).join("probe.sha256"))
        .map_err(|e| format!("{SLOTS_RACE_RUN}/probe.sha256: {e}"))?;
    let slots_program = format!("store/binaries/{}.exe", slots_binary.trim());
    let rows: [(events::Lock, &str, &str, &str); 4] = [
        (
            events::Lock::SyncObjects,
            "semaphore_units",
            "tests/data/timed_waits/semaphore_units.exe",
            units,
        ),
        (
            events::Lock::Handles,
            "handle_values",
            "tests/data/timed_waits/handle_values.exe",
            "timed-waits-handle-values",
        ),
        (
            events::Lock::Heap,
            "heap_blocks",
            "tests/data/timed_waits/heap_blocks.exe",
            "timed-waits-heap-blocks",
        ),
        (
            events::Lock::Slots,
            "thread_slots_race",
            &slots_program,
            "probe-thread-slots-race",
        ),
    ];
    for (lock, name, exe, application) in rows {
        for n in [TWO, 1] {
            let label = format!("{name} on {n} vCPU(s)");
            let (output, timed) = if lock == events::Lock::SyncObjects && n == TWO {
                (b"done\n".to_vec(), json.clone())
            } else {
                timed_run(&root, exe, n, name)?
            };
            // the guest's own judge first: its rows, or "done"
            if lock == events::Lock::Slots {
                let windows = std::fs::read_to_string(root.join(SLOTS_RACE_RUN).join("body-1.jsonl"))
                    .map_err(|e| format!("{SLOTS_RACE_RUN}/body-1.jsonl: {e}"))?;
                if but_overlap(&String::from_utf8_lossy(&output)) != but_overlap(&windows) {
                    return Err(format!(
                        "{label}: its rows but tls.overlap are not Windows run {SLOTS_RACE_RUN}'s"
                    ));
                }
            } else if output != b"done\n" {
                return Err(format!(
                    "{label}: its output is {:?}, not \"done\"",
                    String::from_utf8_lossy(&output)
                ));
            }
            let figures = leaf_locks(&timed, &label, n)?;
            let (counted, takings) = figures
                .iter()
                .find(|(l, _, _)| *l == lock)
                .map(|(_, c, t)| (*c, t.clone()))
                .expect("every lock read");
            let bound = takings_bound(lock, &timed, &output, &root)?;
            if counted < bound {
                return Err(format!(
                    "{label}: the {} lock taken {counted} times, below the {bound} its own counts of its calls make",
                    lock.as_str()
                ));
            }
            let mut holds: Vec<u64> = takings.iter().map(|t| t[2] - t[1]).collect();
            let mut waits: Vec<u64> = takings.iter().map(|t| t[1] - t[0]).collect();
            holds.sort_unstable();
            waits.sort_unstable();
            if n == TWO && quantile(&waits, 0.99) == 0 {
                return Err(format!(
                    "{label}: the {} lock's waits are 0 at the 99th percentile of its first {} takings: its \
                     threads never met at it",
                    lock.as_str(),
                    takings.len()
                ));
            }
            let on = (n == TWO).then_some(vcpus);
            let suffix = if n == TWO { "" } else { "-one-vcpu" };
            let id = |what: &str| {
                format!(
                    "{}-lock-{}-{what}{suffix}",
                    name.replace('_', "-"),
                    lock.as_str().replace('_', "-")
                )
            };
            let over = format!("the run's first {} takings, of {counted} counted", takings.len());
            let mut push = |what: &str, metric: Metric, value: f64, unit: &str, variant: String| {
                lines.push(
                    lock_measurement_line(on, lock, &id(what), metric, value, unit, application, Some(&variant))
                        .expect("a lock metric"),
                );
            };
            push(
                "takings",
                Metric::LockTakings,
                counted as f64,
                "count",
                format!(
                    "{} written; at least {bound} from the guest's own counts of its calls",
                    takings.len()
                ),
            );
            for (what, metric, sorted, q) in [
                ("hold-ns-p50", Metric::LockHoldNsP50, &holds, 0.5),
                ("hold-ns-p99", Metric::LockHoldNsP99, &holds, 0.99),
                ("wait-ns-p50", Metric::LockWaitNsP50, &waits, 0.5),
                ("wait-ns-p99", Metric::LockWaitNsP99, &waits, 0.99),
            ] {
                push(
                    what,
                    metric,
                    (quantile(sorted, q) as f64 * ns_per_tick).round(),
                    "ns",
                    over.clone(),
                );
            }
        }
    }

    // the capabilities `vcpus_allowed` and the run-loop design read (Sprint 337; vocabulary 0.13, P28)
    let host = host_id()?;
    let capabilities: serde_json::Value = serde_json::from_str(child(&root, &["--capabilities"])?.trim())
        .map_err(|e| format!("w2m-vm --capabilities: {e}"))?;
    let (outcome, detail) = match capabilities["feat_lse2"].as_i64() {
        Some(1) => (CapabilityOutcome::Present, "hw.optional.arm.FEAT_LSE2 is 1".to_string()),
        Some(v) => (CapabilityOutcome::Absent, format!("hw.optional.arm.FEAT_LSE2 is {v}")),
        // a kernel that names no such feature shows no absence (v0.2 8.1)
        None => (
            CapabilityOutcome::NotEstablished,
            "sysctlbyname names no hw.optional.arm.FEAT_LSE2".to_string(),
        ),
    };
    lines.push(probed(
        Capability::FeatLse2,
        &host,
        outcome,
        format!("{detail} (sysctlbyname), which vcpus_allowed reads before it admits a second vCPU"),
    ));
    let level = capabilities["feat_bbm_level"]
        .as_u64()
        .ok_or("w2m-vm --capabilities: no FEAT_BBM level")?;
    let register = capabilities["id_aa64mmfr2_el1"].as_str().unwrap_or("unread");
    lines.push(probed(
        Capability::FeatBbmLevel2,
        &host,
        if level >= 2 {
            CapabilityOutcome::Present
        } else {
            CapabilityOutcome::Absent
        },
        format!(
            "FEAT_BBM level {level}: ID_AA64MMFR2_EL1 {register}, bits 55:52, as a vCPU reads it; level 2 or more lets \
             a change of block size alone go without a break"
        ),
    ));
    let wfe = probe["wfe"]["exits"].as_u64().ok_or("the probe gave no wfe exits")?;
    lines.push(probed(
        Capability::GuestWfeExit,
        &host,
        if wfe > 0 {
            CapabilityOutcome::Present
        } else {
            CapabilityOutcome::Absent
        },
        format!(
            "the WFE probe's run of 1,000 WFE ended at its hvc #0 with {wfe} WFx traps (ESR EC 0x01) to the host; its \
             run of 1,000 WFI with {}",
            probe["wfi"]["exits"]
        ),
    ));

    let zero = HarnessU64::new(0).expect("0");
    lines.push(harness_line(&SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        elapsed_seconds: FiniteF64::new((started.elapsed().as_secs_f64() * 1000.0).round() / 1000.0)
            .expect("a measured duration is finite"),
    }));
    let mut out = io::stdout().lock();
    for line in lines {
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("run_measurements: {e}");
            ExitCode::FAILURE
        }
    }
}
