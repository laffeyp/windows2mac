// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The Mac's half of the Windows reference machine, the laptop (`docs/windows-reference-machine-2026-10-12.md` §3,
//! §4; Sprint 371). `scripts/windows_machine.py` and `scripts/store_probe_runs.py` did this until Sprint 226 removed
//! them with the Python; this crate does their `probe` and `binary` kinds and their store step, read at the tag
//! frozen-python-2026-10-29, step for step.
//!
//! A job is data: the committed tree (`git archive`, `store/` left out) or a stored binary, the executor scripts from
//! `tools/windows_machine/` at the same commit, and `job.json` naming every input by sha256. It goes to
//! `C:/w2m/jobs/<run>/` by `scp`; `run-in-desktop.ps1` runs `run-job.ps1` in w2m's desktop session by `ssh`; the `out/`
//! folder comes back to `build/probe-runs/<run>/traces-<execution>/`, and only then is the laptop's job folder removed.
//! A run whose results are all back and whose executions all exited 0 is stored: its files to `store/probe-runs/<run>/`,
//! its binary to `store/binaries/<sha256>.exe`, and only a run whose `profile.txt` is the laptop's.
//!
//! Sprint 372 adds the Python's other two kinds, `run_batch` with `batch_items` and `run_game`. A batch runs many stored
//! binaries in one job, its bundle split into one run per binary, `lap-<stamp>-<commit>-<n>`, each naming the batch in
//! `batch.txt`, the batch's `profile.json`, `job.json` and `run-job.log` kept once in `store/probe-runs/<batch>/`. A game
//! job runs the game `executions` times in the desktop session, each execution stored as `store/game-runs/<run>-<i>/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::Value;

/// The laptop in `~/.ssh/config` (HostName `windowsmachine.local`, user `w2m`, key `~/.ssh/w2m_laptop`).
pub const HOST: &str = "w2m-laptop";
/// The laptop's job folders.
pub const JOBS: &str = "C:/w2m/jobs";
/// The executor scripts a job carries, from the committed tree: the Python's list less `pmc_summary.py` and
/// `pmc_images.py`, which left the tree in Sprint 226 and which a probe job never runs.
pub const EXECUTOR: [&str; 7] = [
    "run-job.ps1",
    "profile.ps1",
    "cpusets.ps1",
    "run-in-desktop.ps1",
    "input.cs",
    "pmc.wprp",
    "pmc_sample.wprp",
];
/// The link's options, the Python's: a dead connection ends in a minute instead of hanging (2026-10-13: one scp of a
/// finished bundle stalled 6 minutes; the same copy by hand took 0.9 s). The host key is on record under the laptop's
/// alias, [`HOST`] (Sprint 377: it was under the laptop's first address, 192.168.4.55, and the laptop moved to
/// 192.168.4.82), so the key is checked by the machine's name whatever address it answers at.
pub const LINK: [&str; 8] = [
    "-o",
    "ConnectTimeout=15",
    "-o",
    "ServerAliveInterval=15",
    "-o",
    "ServerAliveCountMax=4",
    "-o",
    "HostKeyAlias=w2m-laptop",
];
/// The laptop's profile as `run-job.ps1` writes it (Sprint 377): its machine, CPU class and Windows build, the revision
/// free (7840 in October, 9974 from 2026-11-27). Only a run whose `profile.txt` is this, the revision one or more
/// digits, is stored or held as the laptop's.
pub const LAPTOP_PROFILE: &str = "laptop-x64-10.0.26200.<revision>-Client";

/// Whether `profile`, a `profile.txt`'s whole text, is [`LAPTOP_PROFILE`].
pub fn laptop_profile(profile: &str) -> bool {
    let (head, tail) = LAPTOP_PROFILE
        .split_once("<revision>")
        .expect("the pattern names its revision");
    profile
        .strip_prefix(head)
        .and_then(|rest| rest.strip_suffix(tail))
        .is_some_and(|revision| !revision.is_empty() && revision.bytes().all(|b| b.is_ascii_digit()))
}

/// A job's ssh ends after this even if the link stays alive (audit 2026-10-13); its code is then 124.
pub const JOB_SECONDS: u64 = 2 * 3600;
/// The lock that holds one job at a time to the laptop's one workspace.
pub const LOCK: &str = "build/windows_machine.lock";
/// A stored run's files besides its executions' (`store_probe_runs.py` `KEPT`), each kept when the bundle holds it; from
/// Sprint 377 with the job's code-integrity window ([`CODE_INTEGRITY`]).
pub const KEPT: [&str; 8] = [
    "cpu.txt",
    "probe.sha256",
    "profile.txt",
    "profile.json",
    "job.json",
    "run-job.log",
    "batch.txt",
    CODE_INTEGRITY,
];
/// The family runs' lookup: family, run, binary, contract, the beside DLL as `name:sha256`, ...
pub const KNOWN: &str = "store/family_runs/known-2026-10-29.tsv";
/// The families that joined on the Rust host (Sprint 369), in `KNOWN`'s first five columns; read after it.
pub const JOINED: &str = "store/family_runs/joined-2026-11-27.tsv";
/// The frozen families held to a contract re-taken on the laptop (Sprint 378): family, contract@version, the laptop run
/// whose rows that contract holds; read after `JOINED`, the run's binary from its `probe.sha256` (Sprint 379).
pub const RETAKEN: &str = "store/family_runs/retaken-2026-11-27.tsv";
/// Where a batch's or a game job's bundle comes back: the job's `out/`, `job.json` beside it.
pub const BATCHES: &str = "build/batches";
/// A batch's files, kept once in `store/probe-runs/<batch>/` (`store_probe_runs.py` `BATCH_FILES`).
pub const BATCH_FILES: [&str; 3] = ["profile.json", "job.json", "run-job.log"];

/// The kind of job: a probe built from the committed tree, or a stored binary run unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Probe,
    Binary,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Probe => "probe",
            Kind::Binary => "binary",
        }
    }
}

/// `job.json`, in the Python's order and form (`json.dumps(spec, indent=1)`).
#[derive(Clone, Debug)]
pub struct Job {
    pub kind: Kind,
    pub probe: String,
    pub execution_id: String,
    pub repeat: u32,
    pub commit: String,
    pub cpu_class: Option<String>,
    /// Each input file's name and sha256, in the order the Python added them.
    pub inputs: Vec<(String, String)>,
}

impl Job {
    /// The text the Python wrote: one space an indent level, `": "` and `","`, no final newline.
    pub fn json(&self) -> String {
        let q = |s: &str| serde_json::to_string(s).expect("a string serializes");
        let mut lines = vec![
            format!(" \"kind\": {}", q(self.kind.name())),
            format!(" \"probe\": {}", q(&self.probe)),
            format!(" \"execution_id\": {}", q(&self.execution_id)),
            format!(" \"repeat\": {}", self.repeat),
            format!(" \"commit\": {}", q(&self.commit)),
            " \"session\": \"desktop\"".to_string(),
        ];
        if let Some(class) = &self.cpu_class {
            lines.push(format!(" \"cpu_class\": {}", q(class)));
        }
        let inputs: Vec<String> = self
            .inputs
            .iter()
            .map(|(name, sha)| format!("  {}: {}", q(name), q(sha)))
            .collect();
        lines.push(if inputs.is_empty() {
            " \"inputs\": {}".to_string()
        } else {
            format!(" \"inputs\": {{\n{}\n }}", inputs.join(",\n"))
        });
        format!("{{\n{}\n}}", lines.join(",\n"))
    }
}

/// The run's UTC stamp with milliseconds, `YYYYMMDDTHHMMSSmmmZ`, from milliseconds since the epoch: the Python's
/// `time.strftime("%Y%m%dT%H%M%S", time.gmtime(now)) + f"{int(now * 1000) % 1000:03d}Z"`. The date is Howard
/// Hinnant's `civil_from_days` (https://howardhinnant.github.io/date_algorithms.html#civil_from_days).
pub fn stamp(millis: u128) -> String {
    let secs = i64::try_from(millis / 1000).expect("a time within i64 seconds");
    let ms = millis % 1000;
    let (days, day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}{ms:03}Z",
        day / 3600,
        day % 3600 / 60,
        day % 60
    )
}

/// The run id, `lap-<stamp>-<commit's first 8>`.
pub fn run_id(stamp: &str, commit: &str) -> String {
    format!("lap-{stamp}-{}", &commit[..8])
}

/// `git -C <root> <args>`, its standard output.
pub fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// The job's `tree.tar`: `git archive --format=tar <commit> -- <paths>`, the whole tree less `store/` for a probe
/// build, the probe's folder alone for a stored binary.
pub fn tree_tar(root: &Path, commit: &str, kind: Kind, probe: &str) -> Result<Vec<u8>, String> {
    let folder = format!("probes/{probe}");
    let paths: Vec<&str> = match kind {
        Kind::Probe => vec![".", ":!store"],
        Kind::Binary => vec![&folder],
    };
    let mut args = vec!["archive", "--format=tar", commit, "--"];
    args.extend(paths);
    git(root, &args)
}

/// SHA-256 as 64 lowercase hex digits.
pub fn sha256(data: &[u8]) -> String {
    guest_runtime::sha256::hex(data)
}

/// The run's exit status and its line (the Python's `verdict`): a missing bundle, then run-job's code, then a short
/// count, then an execution that exited non-zero (audit 2026-10-13: a crashed second execution counted as one).
pub fn verdict(code: i32, fetched: bool, bundle: &Path, repeat: u32) -> (i32, String) {
    if !fetched {
        let status = if code == 0 { 1 } else { code };
        return (
            status,
            "bundle not copied back; the laptop's job folder is kept".to_string(),
        );
    }
    let mut execs: Vec<PathBuf> = fs::read_dir(bundle)
        .map(|d| {
            d.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| named(p, "exec-", ".json"))
                .collect()
        })
        .unwrap_or_default();
    execs.sort();
    let n = execs.len();
    if code != 0 {
        return (code, format!("run-job exit {code}, {n} of {repeat} executions"));
    }
    if n != repeat as usize {
        return (1, format!("{n} of {repeat} executions"));
    }
    let failed: Vec<String> = execs
        .iter()
        .filter(|p| {
            let exit = fs::read_to_string(p)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v.get("exit_code").and_then(Value::as_i64));
            exit != Some(0)
        })
        .map(|p| {
            p.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .collect();
    if !failed.is_empty() {
        return (
            1,
            format!("{n} of {repeat} executions; non-zero exit: {}", failed.join(", ")),
        );
    }
    (0, format!("{n} of {repeat} executions"))
}

/// A file whose name begins `prefix` and ends `suffix`.
fn named(path: &Path, prefix: &str, suffix: &str) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy())
        .is_some_and(|n| n.starts_with(prefix) && n.ends_with(suffix))
}

/// A family's stored run: its id, its binary and the DLL beside it (sha256s under `store/binaries/`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRun {
    pub run: String,
    pub binary: String,
    pub dll: Option<String>,
    pub probe: String,
}

/// The run a `binary` job runs: the family's row in `KNOWN` named by `run`, or its first, as the Python took the
/// family's runs from `family_run.FAMILIES`. Its probe is the one the run's stored `job.json` names, else the folder
/// `probes/<family>`; a family whose probe neither gives is refused, since the frozen index's family-to-probe map
/// left with the Python.
pub fn stored_run(root: &Path, family: &str, run: Option<&str>) -> Result<StoredRun, String> {
    let text = fs::read_to_string(root.join(KNOWN)).map_err(|e| format!("{KNOWN}: {e}"))?;
    let rows: Vec<Vec<&str>> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|c| c.len() > 4 && c[0] == family)
        .collect();
    if rows.is_empty() {
        return Err(format!("{KNOWN} names no family {family}"));
    }
    let row = match run {
        Some(id) => rows
            .iter()
            .find(|c| c[1] == id)
            .ok_or(format!("{KNOWN}: family {family} has no run {id}"))?,
        None => &rows[0],
    };
    let dll = row[4].split_once(':').map(|(_, sha)| sha.to_string());
    Ok(StoredRun {
        run: row[1].to_string(),
        binary: row[2].to_string(),
        dll,
        probe: run_probe(root, family, row[1])?,
    })
}

/// A family run's probe: the one its stored `job.json` names, else the folder `probes/<family>`.
fn run_probe(root: &Path, family: &str, run: &str) -> Result<String, String> {
    let job = root.join("store/probe-runs").join(run).join("job.json");
    let named = fs::read_to_string(&job)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("probe").and_then(Value::as_str).map(str::to_string));
    match named {
        Some(p) => Ok(p),
        None if root.join("probes").join(family).is_dir() => Ok(family.to_string()),
        None => Err(format!(
            "family {family}: run {run}'s store has no job.json and there is no probes/{family}, so its probe is not \
             known"
        )),
    }
}

/// `probes/<probe>/probe.json`, or null for a probe without one.
fn probe_config(root: &Path, probe: &str) -> Result<Value, String> {
    match fs::read_to_string(root.join("probes").join(probe).join("probe.json")) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("probes/{probe}/probe.json: {e}")),
        Err(_) => Ok(Value::Null),
    }
}

/// What one job sends: its kind, its probe, and for a stored binary the binary and its DLL.
#[derive(Clone, Debug)]
pub struct Request {
    pub kind: Kind,
    pub probe: String,
    pub binary: Option<String>,
    pub dll: Option<String>,
    pub repeat: u32,
    pub pin: Option<String>,
}

/// The job folder's files, written to `job` (a folder named by the run): `tree.tar`, the executor scripts, a stored
/// binary and its DLL, and `job.json`. Returns the job's spec.
pub fn write_job(root: &Path, job: &Path, request: &Request, commit: &str, execution_id: &str) -> Result<Job, String> {
    let probe = &request.probe;
    let config: Value = match fs::read_to_string(root.join("probes").join(probe).join("probe.json")) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("probes/{probe}/probe.json: {e}"))?,
        Err(_) => Value::Null,
    };
    if request.kind == Kind::Probe && config.get("generate").is_some() {
        return Err(format!(
            "probes/{probe} has a generate step, which runs Python on the laptop; no Python runs (Decision 2026-10-29)"
        ));
    }
    fs::create_dir_all(job).map_err(|e| format!("{}: {e}", job.display()))?;
    let write = |name: &str, bytes: &[u8]| {
        fs::write(job.join(name), bytes).map_err(|e| format!("{}/{name}: {e}", job.display()))
    };
    let tree = tree_tar(root, commit, request.kind, probe)?;
    write("tree.tar", &tree)?;
    for script in EXECUTOR {
        write(
            script,
            &git(root, &["show", &format!("{commit}:tools/windows_machine/{script}")])?,
        )?;
    }
    let mut inputs = vec![("tree.tar".to_string(), sha256(&tree))];
    if let Some(binary) = &request.binary {
        let exe = root.join("store/binaries").join(format!("{binary}.exe"));
        write(
            "probe.exe",
            &fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?,
        )?;
        inputs.push(("probe.exe".to_string(), binary.clone()));
        if let Some(dll) = &request.dll {
            let name = config
                .get("dll")
                .and_then(|d| d.get("name"))
                .and_then(Value::as_str)
                .ok_or(format!("probes/{probe}/probe.json names no dll, and the run has one"))?;
            let file = root.join("store/binaries").join(format!("{dll}.dll"));
            let dll_name = format!("{name}.dll");
            write(
                &dll_name,
                &fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?,
            )?;
            inputs.push((dll_name, dll.clone()));
        }
    }
    let spec = Job {
        kind: request.kind,
        probe: probe.clone(),
        execution_id: execution_id.to_string(),
        repeat: request.repeat,
        commit: commit.to_string(),
        cpu_class: request.pin.clone(),
        inputs,
    };
    write("job.json", spec.json().as_bytes())?;
    Ok(spec)
}

/// `scp -q <LINK> <args>`, tried up to three times; whether one attempt exited 0.
pub fn copy(args: &[&str]) -> bool {
    (0..3).any(|_| {
        Command::new("scp")
            .arg("-q")
            .args(LINK)
            .args(args)
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// `ssh <LINK> <HOST> <command>`, its exit code; a run that outlives `seconds` is killed and gives 124.
pub fn ssh(command: &str, seconds: u64) -> i32 {
    let mut child = match Command::new("ssh").args(LINK).arg(HOST).arg(command).spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ssh: {e}");
            return 1;
        }
    };
    let pid = child.id().to_string();
    let (done, wait) = mpsc::channel::<()>();
    let timer = thread::spawn(move || {
        let expired = matches!(
            wait.recv_timeout(Duration::from_secs(seconds)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        if expired {
            let _ = Command::new("kill").args(["-KILL", &pid]).status();
        }
        expired
    });
    let status = child.wait();
    let _ = done.send(());
    if timer.join().unwrap_or(false) {
        return 124;
    }
    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("ssh: {e}");
            1
        }
    }
}

/// Removes the local job folder when the job ends, as the Python's `TemporaryDirectory` did.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One job on the laptop, start to stored run (the Python's `run_job`, then `store_probe_runs.py` on a passing run).
/// Prints the run's line and returns its exit status.
pub fn run_job(root: &Path, request: &Request) -> i32 {
    match run_job_inner(root, request) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("windows_machine: {why}");
            1
        }
    }
}

fn run_job_inner(root: &Path, request: &Request) -> Result<i32, String> {
    let name = &request.probe;
    if !root.join("probes").join(name).is_dir() {
        return Err(format!("no probes/{name}"));
    }
    fs::create_dir_all(root.join("build")).map_err(|e| format!("build: {e}"))?;
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(root.join(LOCK))
        .map_err(|e| format!("{LOCK}: {e}"))?;
    // one job at a time on the laptop's one workspace; the lock is released when `lock` drops, at the function's end
    lock.lock().map_err(|e| format!("{LOCK}: {e}"))?;
    // read under the lock: a queued job names what it ships
    let head = String::from_utf8_lossy(&git(root, &["rev-parse", "HEAD"])?)
        .trim()
        .to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("clock: {e}"))?;
    let stamp = stamp(now.as_millis());
    let run = run_id(&stamp, &head);
    let execution = format!("{name}-{stamp}");
    let relative = format!("build/probe-runs/{run}/traces-{execution}");
    let dst = root.join(&relative);
    let scratch = Scratch(std::env::temp_dir().join(format!("windows_machine-{}", std::process::id())));
    let job = scratch.0.join(&run);
    let spec = write_job(root, &job, request, &head, &execution)?;
    if !copy(&["-r", &job.to_string_lossy(), &format!("{HOST}:{JOBS}/")]) {
        return Err(format!("{run}: the job could not be copied to the laptop"));
    }
    let code = ssh(
        &format!("{JOBS}/{run}/run-in-desktop.ps1 -Job {JOBS}/{run}"),
        JOB_SECONDS,
    );
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let fetched = copy(&["-r", &format!("{HOST}:{JOBS}/{run}/out"), &dst.to_string_lossy()]);
    if fetched {
        fs::write(dst.join("job.json"), spec.json()).map_err(|e| format!("{relative}/job.json: {e}"))?;
        let _ = Command::new("ssh")
            .args(LINK)
            .arg(HOST)
            .arg(format!("Remove-Item -Recurse -Force {JOBS}/{run}"))
            .status();
    }
    drop(scratch);
    // a block in the job's window is named in place of the verdict on a missing output (Sprint 372: probe.exe, 3077)
    if fetched {
        match code_integrity(&dst) {
            Ok(line) => println!("{run}: {line}"),
            Err(why) => {
                println!("{run}: {why}; nothing stored; bundle {relative}");
                return Ok(1);
            }
        }
    }
    let (status, line) = verdict(code, fetched, &dst, request.repeat);
    println!("{run}: {line}; bundle {relative}");
    if status != 0 {
        return Ok(status);
    }
    println!("{}", store(root, &run)?);
    Ok(0)
}

/// The store step (`store_probe_runs.py` for one run): `build/probe-runs/<run>/traces-<execution>/` to
/// `store/probe-runs/<run>/` (its executions' bodies and exec files and the `KEPT` files), its `probe.exe` to
/// `store/binaries/<sha256>.exe` unless that binary is stored. A run whose `profile.txt` is not [`LAPTOP_PROFILE`] is
/// refused before anything is written, as is a `probe.exe` whose sha256 is not its `probe.sha256`. Returns the line
/// that names what was stored.
pub fn store(root: &Path, run: &str) -> Result<String, String> {
    let base = root.join("build/probe-runs").join(run);
    let mut found: Vec<PathBuf> = fs::read_dir(&base)
        .map(|d| {
            d.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir() && named(p, "traces-", ""))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    if found.len() != 1 {
        return Err(format!(
            "{run}: {} trace folders under build/probe-runs/{run}",
            found.len()
        ));
    }
    let src = &found[0];
    let read = |name: &str| fs::read_to_string(src.join(name)).map_err(|e| format!("{run}: {name}: {e}"));
    let profile = read("profile.txt")?;
    if !laptop_profile(&profile) {
        return Err(format!(
            "{run}: profile.txt is {profile:?}, not the laptop's ({LAPTOP_PROFILE}): only a laptop run is stored"
        ));
    }
    let sha = read("probe.sha256")?.trim().to_string();
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err(format!("{run}: probe.sha256 is {sha:?}, not a sha256"));
    }
    let binary = root.join("store/binaries").join(format!("{sha}.exe"));
    let exe = if binary.exists() {
        None
    } else {
        let bytes = fs::read(src.join("probe.exe"))
            .map_err(|e| format!("{run}: store/binaries/{sha}.exe is not stored and probe.exe: {e}"))?;
        let got = sha256(&bytes);
        if got != sha {
            return Err(format!("{run}: probe.exe's sha256 is {got}, probe.sha256 names {sha}"));
        }
        Some(bytes)
    };
    // a batch's run: the batch's three files, from its bundle, once in store/probe-runs/<batch>/
    let batch = fs::read_to_string(src.join("batch.txt"))
        .ok()
        .map(|b| b.trim().to_string());
    let mut once: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Some(b) = &batch {
        let kept = root.join("store/probe-runs").join(b);
        for name in BATCH_FILES {
            if !kept.join(name).exists() {
                let from = root.join(BATCHES).join(b).join(name);
                if !from.is_file() {
                    return Err(format!(
                        "{run}: batch {b}'s {name} is not stored and not in {BATCHES}/{b}"
                    ));
                }
                once.push((from, kept.join(name)));
            }
        }
        // the batch's code-integrity window, once beside them, when its bundle holds one (Sprint 377)
        let window = root.join(BATCHES).join(b).join(CODE_INTEGRITY);
        if !kept.join(CODE_INTEGRITY).exists() && window.is_file() {
            once.push((window, kept.join(CODE_INTEGRITY)));
        }
    }
    let dst = root.join("store/probe-runs").join(run);
    fs::create_dir_all(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    for (from, to) in &once {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::copy(from, to).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    let mut files: Vec<PathBuf> = fs::read_dir(src)
        .map_err(|e| format!("{}: {e}", src.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && (named(p, "body-", ".jsonl")
                    || named(p, "exec-", ".json")
                    || p.file_name().is_some_and(|n| KEPT.iter().any(|k| n == *k)))
        })
        .collect();
    files.sort();
    for f in &files {
        let to = dst.join(f.file_name().expect("a file has a name"));
        fs::copy(f, &to).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    if let Some(bytes) = exe {
        fs::create_dir_all(root.join("store/binaries")).map_err(|e| format!("store/binaries: {e}"))?;
        fs::write(&binary, bytes).map_err(|e| format!("{}: {e}", binary.display()))?;
    }
    let executions = files.iter().filter(|p| named(p, "exec-", ".json")).count();
    let cpu = read("cpu.txt").unwrap_or_default();
    let of_batch = batch.map(|b| format!(", batch {b}")).unwrap_or_default();
    Ok(format!(
        "{run}: stored store/probe-runs/{run}, {} files, {executions} executions, profile {}, cpu {}, binary \
         store/binaries/{sha}.exe{of_batch}",
        files.len(),
        profile.trim(),
        cpu.trim()
    ))
}

/// A JSON value in the order a Python dict kept its keys, written as `json.dumps(value, indent=1)` writes it: one
/// space a level, `", "` closing a line and `": "` after a key, an empty list `[]` and an empty dict `{}`, no final
/// newline, every character outside space to tilde escaped (`ensure_ascii`, the default).
#[derive(Clone, Debug, PartialEq)]
pub enum Py {
    Str(String),
    Int(i64),
    Bool(bool),
    Null,
    List(Vec<Py>),
    Dict(Vec<(String, Py)>),
}

impl Py {
    pub fn str(s: &str) -> Py {
        Py::Str(s.to_string())
    }

    /// `json.dumps(self, indent=1)`.
    pub fn dumps(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, level: usize) {
        match self {
            Py::Str(s) => quote(s, out),
            Py::Int(n) => out.push_str(&n.to_string()),
            Py::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Py::Null => out.push_str("null"),
            Py::List(items) if items.is_empty() => out.push_str("[]"),
            Py::Dict(items) if items.is_empty() => out.push_str("{}"),
            Py::List(items) => {
                out.push('[');
                for (k, v) in items.iter().enumerate() {
                    out.push_str(if k == 0 { "\n" } else { ",\n" });
                    out.push_str(&" ".repeat(level + 1));
                    v.write(out, level + 1);
                }
                out.push('\n');
                out.push_str(&" ".repeat(level));
                out.push(']');
            }
            Py::Dict(items) => {
                out.push('{');
                for (k, (key, v)) in items.iter().enumerate() {
                    out.push_str(if k == 0 { "\n" } else { ",\n" });
                    out.push_str(&" ".repeat(level + 1));
                    quote(key, out);
                    out.push_str(": ");
                    v.write(out, level + 1);
                }
                out.push('\n');
                out.push_str(&" ".repeat(level));
                out.push('}');
            }
        }
    }
}

/// A string as `json.dumps` writes it with `ensure_ascii`: `"` and `\` escaped, `\n \r \t \b \f` by letter, any other
/// character outside space to tilde as `\uXXXX` in lowercase hex, a character past U+FFFF as its UTF-16 pair.
fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// One binary of a batch (the Python's `batch_items` item): its family and probe, its sha256 under `store/binaries/`,
/// the stored runs that used it, the DLL beside it, and the execution id the batch gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchItem {
    pub family: String,
    pub probe: String,
    pub binary: String,
    /// The stored runs that used the binary, space-separated, in the tables' order.
    pub runs: String,
    /// The DLL's sha256 and the name it is loaded by (`probe.json` `dll.name`).
    pub dll: Option<(String, String)>,
    /// `<family>-<stamp>-<n>`, set when the batch is made; empty before.
    pub execution_id: String,
}

impl BatchItem {
    /// The item as the Python's dict held it: family, probe, binary, runs, the DLL's two keys, then the execution id
    /// `run_batch` added.
    fn py(&self) -> Py {
        let mut d = vec![
            ("family".to_string(), Py::str(&self.family)),
            ("probe".to_string(), Py::str(&self.probe)),
            ("binary".to_string(), Py::str(&self.binary)),
            ("runs".to_string(), Py::str(&self.runs)),
        ];
        if let Some((sha, name)) = &self.dll {
            d.push(("dll".to_string(), Py::str(sha)));
            d.push(("dll_name".to_string(), Py::str(name)));
        }
        d.push(("execution_id".to_string(), Py::str(&self.execution_id)));
        Py::Dict(d)
    }
}

/// The batch's items (the Python's `batch_items`): for each name, one item per distinct binary of that family in
/// `KNOWN` and then `JOINED` (a row of both counted once), in the tables' order, a repeated binary adding its run to the
/// first item's `runs`; no name, every family the tables hold, sorted. From Sprint 379 a family `RETAKEN` names adds its
/// laptop run there, by the binary `store/probe-runs/<run>/probe.sha256` names: to the item of that binary, or as an item
/// of its own when no table row runs it, refused for a probe that loads a DLL, since the table records none. A name
/// `<family>@<run>` is one item, the binary `store/probe-runs/<run>/probe.sha256` names, for a family whose runs neither
/// table holds; a probe that loads a DLL is refused there too.
pub fn batch_items(root: &Path, names: &[String]) -> Result<Vec<BatchItem>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for table in [KNOWN, JOINED] {
        let text = fs::read_to_string(root.join(table)).map_err(|e| format!("{table}: {e}"))?;
        for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let c: Vec<String> = line.split('\t').map(str::to_string).collect();
            if c.len() > 4 && !rows.iter().any(|r| r[0] == c[0] && r[1] == c[1]) {
                rows.push(c);
            }
        }
    }
    // the re-taken table's laptop runs, in KNOWN's first five columns
    let text = fs::read_to_string(root.join(RETAKEN)).map_err(|e| format!("{RETAKEN}: {e}"))?;
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let [family, contract, run] = line.split('\t').collect::<Vec<_>>()[..] else {
            return Err(format!("{RETAKEN}: a row of 3 columns, not {line:?}"));
        };
        let at = format!("store/probe-runs/{run}/probe.sha256");
        let sha = fs::read_to_string(root.join(&at))
            .map_err(|e| format!("{RETAKEN}: {family}'s run {run}: {at}: {e}"))?
            .trim()
            .to_string();
        let beside = match rows.iter().find(|r| r[0] == family && r[2] == sha) {
            Some(r) => r[4].clone(),
            None if probe_config(root, &run_probe(root, family, run)?)?.get("dll").is_some() => {
                return Err(format!(
                    "{RETAKEN}: {family}'s run {run} ran binary {sha}, which no table row runs with its DLL"
                ));
            }
            None => "-".to_string(),
        };
        if !rows.iter().any(|r| r[0] == family && r[1] == run) {
            rows.push(vec![family.into(), run.into(), sha, contract.into(), beside]);
        }
    }
    let names: Vec<String> = if names.is_empty() {
        let mut all: Vec<String> = rows.iter().map(|c| c[0].clone()).collect();
        all.sort();
        all.dedup();
        all
    } else {
        names.to_vec()
    };
    let mut items = Vec::new();
    for name in &names {
        if let Some((family, run)) = name.split_once('@') {
            items.push(run_item(root, family, run)?);
            continue;
        }
        let mine: Vec<&Vec<String>> = rows.iter().filter(|c| c[0] == *name).collect();
        let Some(first) = mine.first() else {
            return Err(format!(
                "{KNOWN} and {JOINED} name no family {name}; name one of its stored runs as {name}@<run>"
            ));
        };
        let probe = run_probe(root, name, &first[1])?;
        let mut seen: Vec<BatchItem> = Vec::new();
        for c in mine {
            if let Some(item) = seen.iter_mut().find(|i| i.binary == c[2]) {
                item.runs.push(' ');
                item.runs.push_str(&c[1]);
                continue;
            }
            let dll = match c[4].split_once(':') {
                Some((_, sha)) => {
                    let config = probe_config(root, &probe)?;
                    let dll_name = config
                        .get("dll")
                        .and_then(|d| d.get("name"))
                        .and_then(Value::as_str)
                        .ok_or(format!(
                            "probes/{probe}/probe.json names no dll, and run {} has one",
                            c[1]
                        ))?;
                    Some((sha.to_string(), dll_name.to_string()))
                }
                None => None,
            };
            seen.push(BatchItem {
                family: name.clone(),
                probe: probe.clone(),
                binary: c[2].clone(),
                runs: c[1].clone(),
                dll,
                execution_id: String::new(),
            });
        }
        items.extend(seen);
    }
    Ok(items)
}

/// `<family>@<run>`'s item: the run's stored binary, the family's probe.
fn run_item(root: &Path, family: &str, run: &str) -> Result<BatchItem, String> {
    let at = format!("store/probe-runs/{run}/probe.sha256");
    let sha = fs::read_to_string(root.join(&at))
        .map_err(|e| format!("{family}@{run}: {at}: {e}"))?
        .trim()
        .to_string();
    if !root.join("store/binaries").join(format!("{sha}.exe")).is_file() {
        return Err(format!("{family}@{run}: store/binaries/{sha}.exe is not stored"));
    }
    let probe = run_probe(root, family, run)?;
    if probe_config(root, &probe)?.get("dll").is_some() {
        return Err(format!(
            "{family}@{run}: probes/{probe} loads a DLL and a run named by id records none; its family's table row \
             names it"
        ));
    }
    Ok(BatchItem {
        family: family.to_string(),
        probe,
        binary: sha,
        runs: run.to_string(),
        dll: None,
        execution_id: String::new(),
    })
}

/// A batch's inputs: `tree.tar`, then each item's binary and DLL as `bin/<sha256>.exe` and `.dll`, once each, in the
/// items' order (the Python's loop over items and kinds).
pub fn batch_inputs(tree_sha: &str, items: &[BatchItem]) -> Vec<(String, String)> {
    let mut inputs = vec![("tree.tar".to_string(), tree_sha.to_string())];
    for item in items {
        let files = [
            Some(format!("bin/{}.exe", item.binary)).map(|f| (f, item.binary.clone())),
            item.dll
                .as_ref()
                .map(|(sha, _)| (format!("bin/{sha}.dll"), sha.clone())),
        ];
        for (file, sha) in files.into_iter().flatten() {
            if !inputs.iter().any(|(f, _)| *f == file) {
                inputs.push((file, sha));
            }
        }
    }
    inputs
}

/// A batch's `job.json` (the Python's `spec`): kind, execution id (the batch's run), repeat, commit, session, the
/// console window, the items, the core class when pinned, the inputs.
#[derive(Clone, Debug)]
pub struct Batch {
    pub run: String,
    pub repeat: u32,
    pub commit: String,
    pub window: String,
    pub pin: Option<String>,
    pub items: Vec<BatchItem>,
    pub inputs: Vec<(String, String)>,
}

impl Batch {
    pub fn json(&self) -> String {
        let mut d = vec![
            ("kind".to_string(), Py::str("batch")),
            ("execution_id".to_string(), Py::str(&self.run)),
            ("repeat".to_string(), Py::Int(i64::from(self.repeat))),
            ("commit".to_string(), Py::str(&self.commit)),
            ("session".to_string(), Py::str("desktop")),
            ("console_window".to_string(), Py::str(&self.window)),
            (
                "items".to_string(),
                Py::List(self.items.iter().map(BatchItem::py).collect()),
            ),
        ];
        if let Some(class) = &self.pin {
            d.push(("cpu_class".to_string(), Py::str(class)));
        }
        d.push(("inputs".to_string(), inputs_py(&self.inputs)));
        Py::Dict(d).dumps()
    }
}

fn inputs_py(inputs: &[(String, String)]) -> Py {
    Py::Dict(inputs.iter().map(|(f, sha)| (f.clone(), Py::str(sha))).collect())
}

/// The lock held for the whole job, HEAD read under it, and the run's stamp (the Python's first lines of each kind).
fn begin(root: &Path) -> Result<(fs::File, String, String), String> {
    fs::create_dir_all(root.join("build")).map_err(|e| format!("build: {e}"))?;
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(root.join(LOCK))
        .map_err(|e| format!("{LOCK}: {e}"))?;
    lock.lock().map_err(|e| format!("{LOCK}: {e}"))?;
    let head = String::from_utf8_lossy(&git(root, &["rev-parse", "HEAD"])?)
        .trim()
        .to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("clock: {e}"))?;
    Ok((lock, head, stamp(now.as_millis())))
}

/// The executor scripts from the committed tree, into the job folder.
fn write_executor(root: &Path, job: &Path, commit: &str) -> Result<(), String> {
    for script in EXECUTOR {
        let bytes = git(root, &["show", &format!("{commit}:tools/windows_machine/{script}")])?;
        fs::write(job.join(script), bytes).map_err(|e| format!("{}/{script}: {e}", job.display()))?;
    }
    Ok(())
}

/// A job folder sent and run in the desktop session, its `out/` brought back to `bundle` with `job.json` beside it,
/// then the laptop's job folder removed (the steps `run_batch` and `run_game` share). Returns run-job's code and
/// whether the bundle came back.
fn send(job: &Path, run: &str, spec: &str, seconds: u64, bundle: &Path) -> Result<(i32, bool), String> {
    if !copy(&["-r", &job.to_string_lossy(), &format!("{HOST}:{JOBS}/")]) {
        return Err(format!("{run}: the job could not be copied to the laptop"));
    }
    let code = ssh(&format!("{JOBS}/{run}/run-in-desktop.ps1 -Job {JOBS}/{run}"), seconds);
    if let Some(parent) = bundle.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let fetched = copy(&["-r", &format!("{HOST}:{JOBS}/{run}/out"), &bundle.to_string_lossy()]);
    if fetched {
        fs::write(bundle.join("job.json"), spec).map_err(|e| format!("{}/job.json: {e}", bundle.display()))?;
        let _ = Command::new("ssh")
            .args(LINK)
            .arg(HOST)
            .arg(format!("Remove-Item -Recurse -Force {JOBS}/{run}"))
            .status();
    }
    Ok((code, fetched))
}

/// `src`'s files and folders, copied into `dst` (the Python's `shutil.copytree(src, dst, dirs_exist_ok=True)`).
/// Returns the count of files copied.
pub fn copy_tree(src: &Path, dst: &Path) -> Result<usize, String> {
    fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    let mut n = 0;
    for e in fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let p = e.map_err(|e| format!("{}: {e}", src.display()))?.path();
        let to = dst.join(p.file_name().expect("an entry has a name"));
        if p.is_dir() {
            n += copy_tree(&p, &to)?;
        } else {
            fs::copy(&p, &to).map_err(|e| format!("{}: {e}", to.display()))?;
            n += 1;
        }
    }
    Ok(n)
}

/// A batch on the laptop, start to stored runs (the Python's `run_batch`, then `store_probe_runs.py` on each run):
/// returns the batch's exit status.
pub fn run_batch(root: &Path, items: Vec<BatchItem>, repeat: u32, pin: Option<String>, window: &str) -> i32 {
    match run_batch_inner(root, items, repeat, pin, window) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("windows_machine: {why}");
            1
        }
    }
}

fn run_batch_inner(
    root: &Path,
    mut items: Vec<BatchItem>,
    repeat: u32,
    pin: Option<String>,
    window: &str,
) -> Result<i32, String> {
    if items.is_empty() {
        return Err("a batch of no binaries".to_string());
    }
    let (_lock, head, stamp) = begin(root)?;
    let run = run_id(&stamp, &head);
    for (n, item) in items.iter_mut().enumerate() {
        item.execution_id = format!("{}-{stamp}-{:03}", item.family, n + 1);
    }
    let scratch = Scratch(std::env::temp_dir().join(format!("windows_machine-{}", std::process::id())));
    let job = scratch.0.join(&run);
    fs::create_dir_all(job.join("bin")).map_err(|e| format!("{}: {e}", job.display()))?;
    let tree = git(root, &["archive", "--format=tar", &head, "--", "probes"])?;
    fs::write(job.join("tree.tar"), &tree).map_err(|e| format!("{run}/tree.tar: {e}"))?;
    write_executor(root, &job, &head)?;
    let inputs = batch_inputs(&sha256(&tree), &items);
    for (file, _) in &inputs[1..] {
        let from = root.join("store/binaries").join(&file["bin/".len()..]);
        fs::copy(&from, job.join(file)).map_err(|e| format!("{}: {e}", from.display()))?;
    }
    let spec = Batch {
        run: run.clone(),
        repeat,
        commit: head.clone(),
        window: window.to_string(),
        pin,
        items: items.clone(),
        inputs,
    }
    .json();
    fs::write(job.join("job.json"), &spec).map_err(|e| format!("{run}/job.json: {e}"))?;
    let bundle = root.join(BATCHES).join(&run);
    let (code, fetched) = send(&job, &run, &spec, JOB_SECONDS, &bundle)?;
    drop(scratch);
    if !fetched {
        println!("{run}: bundle not copied back; the laptop's job folder is kept");
        return Ok(if code == 0 { 1 } else { code });
    }
    match code_integrity(&bundle) {
        Ok(line) => println!("{run}: {line}"),
        Err(why) => {
            println!("{run}: {why}; nothing stored; bundle {BATCHES}/{run}");
            return Ok(1);
        }
    }
    let runs = split_batch(root, &run, &items, repeat)?;
    let short: Vec<&(String, i32, String)> = runs.iter().filter(|r| r.1 != 0).collect();
    for (_, _, line) in &short {
        println!("{line}");
    }
    println!(
        "{run}: {} binaries, {} with {repeat} executions; run-job exit {code}; bundle {BATCHES}/{run}",
        items.len(),
        items.len() - short.len()
    );
    if code != 0 {
        return Ok(code);
    }
    if !short.is_empty() {
        return Ok(1);
    }
    // the batch is one job on one machine: every run's profile is read before any is stored
    for (id, _, _) in &runs {
        let profile = fs::read_to_string(traces(root, id)?.join("profile.txt")).map_err(|e| format!("{id}: {e}"))?;
        if !laptop_profile(&profile) {
            return Err(format!(
                "{id}: profile.txt is {profile:?}, not the laptop's ({LAPTOP_PROFILE}): nothing stored"
            ));
        }
    }
    for (id, _, _) in &runs {
        println!("{}", store(root, id)?);
    }
    Ok(0)
}

/// A run's one `traces-*` folder under `build/probe-runs/<run>/`.
fn traces(root: &Path, run: &str) -> Result<PathBuf, String> {
    let base = root.join("build/probe-runs").join(run);
    let found: Vec<PathBuf> = fs::read_dir(&base)
        .map_err(|e| format!("{}: {e}", base.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && named(p, "traces-", ""))
        .collect();
    match found.as_slice() {
        [one] => Ok(one.clone()),
        _ => Err(format!(
            "{run}: {} trace folders under build/probe-runs/{run}",
            found.len()
        )),
    }
}

/// The batch's bundle split into one run per binary (the Python's loop after the fetch): `build/batches/<batch>/
/// <execution>/` to `build/probe-runs/<batch>-<n>/traces-<execution>/`, with `batch.txt` naming the batch, each held to
/// the verdict. Returns each run's id, status, and a line naming its binary.
pub fn split_batch(
    root: &Path,
    batch: &str,
    items: &[BatchItem],
    repeat: u32,
) -> Result<Vec<(String, i32, String)>, String> {
    let bundle = root.join(BATCHES).join(batch);
    let mut out = Vec::new();
    for (n, item) in items.iter().enumerate() {
        let id = format!("{batch}-{:03}", n + 1);
        let dst = root
            .join("build/probe-runs")
            .join(&id)
            .join(format!("traces-{}", item.execution_id));
        let src = bundle.join(&item.execution_id);
        if src.is_dir() {
            copy_tree(&src, &dst)?;
        }
        fs::create_dir_all(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        // its profile.json, run-job.log and job.json: the batch's
        fs::write(dst.join("batch.txt"), batch).map_err(|e| format!("{id}/batch.txt: {e}"))?;
        let (status, line) = verdict(0, true, &dst, repeat);
        let first = item.runs.split(' ').next().unwrap_or_default();
        let text = format!("{id} {} ({first}, binary {}): {line}", item.family, item.binary);
        out.push((id, status, text));
    }
    Ok(out)
}

/// A game job (the Python's `run_game` arguments, its defaults in `Default`); the counter options are not here, their
/// summaries having run Python on the laptop.
#[derive(Clone, Debug)]
pub struct Game {
    pub game: String,
    pub exe: String,
    pub executions: u32,
    pub seconds: u32,
    pub mode: String,
    pub audio: String,
    pub input_script: Option<String>,
    pub record: bool,
    pub capture: String,
    pub person: bool,
    pub initial_state: String,
    pub plant_state: bool,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            game: "hail-to-the-rainbow".to_string(),
            exe: "Hail to the Rainbow".to_string(),
            executions: 10,
            seconds: 180,
            mode: "batch".to_string(),
            audio: "on".to_string(),
            input_script: None,
            record: false,
            capture: "on".to_string(),
            person: false,
            initial_state: "empty".to_string(),
            plant_state: false,
        }
    }
}

impl Game {
    /// The desktop task's limit in minutes: each execution's tree mirror and check (about a minute), its seconds and
    /// its stop, the Python's `executions * (seconds + 180) // 60 + 10`.
    pub fn limit_minutes(&self) -> u64 {
        u64::from(self.executions) * (u64::from(self.seconds) + 180) / 60 + 10
    }

    /// The limit held against one execution's seconds before the job is sent: a desktop task stopped before its first
    /// execution ends brings back no execution, so such a limit is refused (Sprint 372's second plant). Returns the
    /// limit in minutes.
    pub fn checked_limit(&self) -> Result<u64, String> {
        let limit = self.limit_minutes();
        if limit * 60 < u64::from(self.seconds) {
            return Err(format!(
                "the job's limit, {limit} minutes, is below one execution's {} s: the desktop task would stop it before \
                 its first execution ends",
                self.seconds
            ));
        }
        Ok(limit)
    }

    /// The job's `job.json` (the Python's `spec`, `pmc` and `pmc_sample` false).
    pub fn json(&self, run: &str, commit: &str, tree_sha: &str) -> String {
        let s = |k: &str, v: &str| (k.to_string(), Py::str(v));
        let b = |k: &str, v: bool| (k.to_string(), Py::Bool(v));
        let i = |k: &str, v: i64| (k.to_string(), Py::Int(v));
        Py::Dict(vec![
            s("kind", "game"),
            s("execution_id", run),
            s("game", &self.game),
            s("exe", &self.exe),
            i("seconds", i64::from(self.seconds)),
            s("mode", &self.mode),
            s("audio", &self.audio),
            (
                "input_script".to_string(),
                self.input_script.as_deref().map_or(Py::Null, Py::str),
            ),
            b("record", self.record),
            b("pmc", false),
            b("pmc_sample", false),
            s("capture", &self.capture),
            b("person", self.person),
            i("frame_seconds", if self.person { 30 } else { 10 }),
            s("initial_state", &self.initial_state),
            b("plant_state", self.plant_state),
            i("executions", i64::from(self.executions)),
            i("repeat", 0),
            s("commit", commit),
            s("session", "desktop"),
            i(
                "limit_minutes",
                i64::try_from(self.limit_minutes()).expect("a limit within i64"),
            ),
            (
                "inputs".to_string(),
                inputs_py(&[("tree.tar".to_string(), tree_sha.to_string())]),
            ),
        ])
        .dumps()
    }
}

/// A game job on the laptop, start to stored executions (the Python's `run_game`, each execution then stored): returns
/// the job's exit status.
pub fn run_game(root: &Path, game: &Game) -> i32 {
    match run_game_inner(root, game) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("windows_machine: {why}");
            1
        }
    }
}

fn run_game_inner(root: &Path, game: &Game) -> Result<i32, String> {
    let limit = game.checked_limit()?;
    let (_lock, head, stamp) = begin(root)?;
    // the TEMP the job's state declares, at the commit the job sends: what each execution's game must have read
    let declared = declared_temp(root, &head, &game.game, &game.initial_state)?;
    let run = run_id(&stamp, &head);
    let scratch = Scratch(std::env::temp_dir().join(format!("windows_machine-{}", std::process::id())));
    let job = scratch.0.join(&run);
    fs::create_dir_all(&job).map_err(|e| format!("{}: {e}", job.display()))?;
    let folder = format!("games/{}", game.game);
    let tree = git(
        root,
        &["archive", "--format=tar", &head, "--", "tools/jit_capture", &folder],
    )?;
    fs::write(job.join("tree.tar"), &tree).map_err(|e| format!("{run}/tree.tar: {e}"))?;
    write_executor(root, &job, &head)?;
    let spec = game.json(&run, &head, &sha256(&tree));
    fs::write(job.join("job.json"), &spec).map_err(|e| format!("{run}/job.json: {e}"))?;
    let bundle = root.join(BATCHES).join(&run);
    println!(
        "{run}: game {}, {} executions of {} s, mode {}, audio {}, limit {limit} minutes",
        game.game, game.executions, game.seconds, game.mode, game.audio
    );
    let (code, fetched) = send(&job, &run, &spec, limit * 60 + 600, &bundle)?;
    drop(scratch);
    if !fetched {
        println!("{run}: bundle not copied back; the laptop's job folder is kept");
        return Ok(if code == 0 { 1 } else { code });
    }
    // a block in the job's window is the block's, not the game's (Sprint 376's first job: handler_trace.dll, 3077)
    match code_integrity(&bundle) {
        Ok(line) => println!("{run}: {line}"),
        Err(why) => {
            println!("{run}: {why}; nothing stored; bundle {BATCHES}/{run}");
            return Ok(1);
        }
    }
    let (done, lines) = split_game(root, &run, game.executions)?;
    for line in &lines {
        println!("{line}");
    }
    println!(
        "{run}: {done} of {} executions back; run-job exit {code}; bundle {BATCHES}/{run}",
        game.executions
    );
    if code != 0 {
        return Ok(code);
    }
    if done != game.executions {
        return Ok(1);
    }
    let ids: Vec<String> = (1..=game.executions).map(|i| format!("{run}-{i:02}")).collect();
    for id in &ids {
        let profile = fs::read_to_string(root.join("build/game-runs").join(id).join("profile.txt"))
            .map_err(|e| format!("{id}: profile.txt: {e}"))?;
        if !laptop_profile(&profile) {
            return Err(format!(
                "{id}: profile.txt is {profile:?}, not the laptop's ({LAPTOP_PROFILE}): nothing stored"
            ));
        }
        let dir = root.join("build/game-runs").join(id);
        steam_free(&dir).map_err(|why| format!("{id}: {why}: nothing stored"))?;
        temp_declared(&dir, &declared).map_err(|why| format!("{id}: {why}: nothing stored"))?;
    }
    for id in &ids {
        println!("{}", store_game(root, id, &declared)?);
    }
    Ok(0)
}

/// The game job's bundle split into one folder per execution (the Python's loop after the fetch): `build/batches/<run>/
/// <i>/` to `build/game-runs/<run>-<i>/`, with `batch.txt` naming the job, the job's `profile.json` (the machine the
/// game asked) and its [`CODE_INTEGRITY`] file (Sprint 377); an execution without `exec.json` is named and skipped.
/// Returns the count split and the lines naming the skipped.
pub fn split_game(root: &Path, run: &str, executions: u32) -> Result<(u32, Vec<String>), String> {
    let bundle = root.join(BATCHES).join(run);
    let (mut done, mut lines) = (0, Vec::new());
    for i in 1..=executions {
        let src = bundle.join(i.to_string());
        if !src.join("exec.json").is_file() {
            lines.push(format!("{run}-{i:02}: no exec.json"));
            continue;
        }
        let dst = root.join("build/game-runs").join(format!("{run}-{i:02}"));
        copy_tree(&src, &dst)?;
        fs::write(dst.join("batch.txt"), run).map_err(|e| format!("{}/batch.txt: {e}", dst.display()))?;
        // the job's profile and its code-integrity window, the machine and the policy each execution met
        for name in ["profile.json", CODE_INTEGRITY] {
            if bundle.join(name).is_file() {
                fs::copy(bundle.join(name), dst.join(name)).map_err(|e| format!("{}/{name}: {e}", dst.display()))?;
            }
        }
        done += 1;
    }
    Ok((done, lines))
}

/// The file `run-job.ps1` writes to a job's `out/` as the job ends (Sprint 377): the job's window on the laptop's clock
/// (`from`, `to`, UTC) and each event 3077 the `Microsoft-Windows-CodeIntegrity/Operational` log holds in it
/// (`blocks`: `time`, `file`, `process`, `policy`), the event by which Smart App Control's policy
/// `VerifiedAndReputableDesktop` refused a file: the laptop's log held six, each a file of a job (probe.exe under
/// `D:\a\windows2mac\windows2mac` loaded by cmd.exe, Sprint 372; handler_trace.dll loaded by UnityCrashHandler64.exe,
/// Sprint 376's first game job), each beside an event 3033 at the same instant; the log's other 3033s name vendor DLLs
/// refused by protected processes, no file of a job. A read that failed writes `error`.
pub const CODE_INTEGRITY: &str = "code-integrity.json";

/// The job's code-integrity window, from `bundle`'s [`CODE_INTEGRITY`]: `Ok` with its line when the window holds no
/// block, `Err` naming each blocked file (and the process that loaded it, the policy and the time) when it holds one, so
/// a job whose binary or DLL Smart App Control refused is not reported as the program's missing output, and when the
/// file is missing or records an error, since then the window's blocks are not known.
pub fn code_integrity(bundle: &Path) -> Result<String, String> {
    let text = fs::read_to_string(bundle.join(CODE_INTEGRITY))
        .map_err(|e| format!("{CODE_INTEGRITY}: {e}: the job's code-integrity blocks are not known"))?;
    let record: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("{CODE_INTEGRITY}: {e}: the job's code-integrity blocks are not known"))?;
    let window = format!(
        "from {} to {}",
        record["from"].as_str().unwrap_or("?"),
        record["to"].as_str().unwrap_or("?")
    );
    if let Some(error) = record["error"].as_str() {
        return Err(format!(
            "{CODE_INTEGRITY}: the CodeIntegrity log was not read ({error}) {window}: the job's blocks are not known"
        ));
    }
    // ConvertTo-Json writes an array, or one object for a lone block
    let blocks: Vec<&Value> = match &record["blocks"] {
        Value::Array(items) => items.iter().collect(),
        Value::Null => Vec::new(),
        one => vec![one],
    };
    if blocks.is_empty() {
        return Ok(format!("CodeIntegrity: no block {window}"));
    }
    let named: Vec<String> = blocks
        .iter()
        .map(|b| {
            let s = |k: &str| b[k].as_str().unwrap_or("?").to_string();
            format!(
                "{} (loaded by {}, policy {}, at {})",
                s("file"),
                s("process"),
                s("policy"),
                s("time")
            )
        })
        .collect();
    Err(format!(
        "CodeIntegrity blocked {} {window}: the job's missing output is the block's, not the program's",
        named.join("; ")
    ))
}

/// What the game writes to `unity.log` when the Steam client is not running, as the reference runs of 2026-10-14 ran.
pub const STEAM_ABSENT: &str = "SteamAPI_Init() failed";

/// An execution's `unity.log` names no Steam account: a log that speaks of Steam at all says [`STEAM_ABSENT`]. With
/// the client running and signed in, the game writes its own Steam lines and the account's player name in that line's
/// place (Sprint 372's three executions; the Decision "A laptop game run's initial state includes the Steam client":
/// no `unity.log` naming the user's account enters `store/`). A folder without a log, or a log that never names Steam,
/// passes. The refusal quotes nothing from the log.
pub fn steam_free(dir: &Path) -> Result<(), String> {
    let Ok(log) = fs::read(dir.join("unity.log")) else {
        return Ok(());
    };
    let log = String::from_utf8_lossy(&log);
    if log.to_lowercase().contains("steam") && !log.contains(STEAM_ABSENT) {
        return Err(format!(
            "unity.log speaks of Steam and has no \"{STEAM_ABSENT}\": the Steam client ran, and the game's Steam lines \
             may name its account, so the execution stays in build/"
        ));
    }
    Ok(())
}

/// The TEMP path `games/<game>/state.json` declares for `state`, at `commit`, the tree the game job sends (Sprint 377):
/// what [`temp_declared`] holds each execution to.
pub fn declared_temp(root: &Path, commit: &str, game: &str, state: &str) -> Result<String, String> {
    let at = format!("games/{game}/state.json");
    let text = git(root, &["show", &format!("{commit}:{at}")])?;
    let states: Value = serde_json::from_slice(&text).map_err(|e| format!("{at} at {commit}: {e}"))?;
    states["states"][state]["temp_path"]
        .as_str()
        .map(str::to_string)
        .ok_or(format!(
            "{at} at {commit} declares no temp_path for state {state}: the TEMP the game must read is not named"
        ))
}

/// An execution's game read the TEMP its declared state names, `declared` ([`declared_temp`]; Sprint 377, Sprint 376's
/// Rubber Duck Observation 2: the check had read the proof the executor wrote, so a proof and a game that agreed on
/// another TEMP passed). Each folder the capture's strings name right after a GetTempPathW call (the row at the call's
/// seq + 1 ending in `\`, as the reference runs' rows 3423 and 3496 do) is that path, and the run's `state.json` proves
/// that path (`seen.temp_path`). Sprint 376's first three executions read `C:\w2m\temp\`, the job's own folder, where the
/// reference runs read the user's TEMP, and nine of the game's values differed with it. A folder without a trace passes;
/// a run whose state.json is missing, records no TEMP path or records another is refused.
pub fn temp_declared(dir: &Path, declared: &str) -> Result<(), String> {
    let Ok(trace) = fs::read_to_string(dir.join("jit_capture.bin.trace.tsv")) else {
        return Ok(());
    };
    let want = format!("{}\\", declared.trim_end_matches('\\'));
    let calls: Vec<u64> = trace
        .lines()
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c.get(4) == Some(&"GetTempPathW")).then(|| c[0].parse().ok()).flatten()
        })
        .collect();
    let strings = fs::read_to_string(dir.join("jit_capture.bin.strings.tsv")).unwrap_or_default();
    for row in strings.lines() {
        let c: Vec<&str> = row.splitn(3, '\t').collect();
        let (Some(seq), Some(text)) = (c.first().and_then(|s| s.parse::<u64>().ok()), c.get(2)) else {
            continue;
        };
        if text.ends_with('\\')
            && seq.checked_sub(1).is_some_and(|s| calls.contains(&s))
            && !text.eq_ignore_ascii_case(&want)
        {
            return Err(format!(
                "the game's TEMP is {text} (strings row {seq}, after GetTempPathW), where its state declares {want}"
            ));
        }
    }
    let proof: Value = fs::read_to_string(dir.join("state.json"))
        .ok()
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .ok_or("state.json is missing or not JSON: the TEMP the game read is not proved")?;
    let Some(path) = proof["seen"]["temp_path"].as_str() else {
        return Err("state.json records no TEMP path: the TEMP the game read is not proved".to_string());
    };
    if !format!("{}\\", path.trim_end_matches('\\')).eq_ignore_ascii_case(&want) {
        return Err(format!(
            "state.json proves the TEMP {path}, where its state declares {want}"
        ));
    }
    Ok(())
}

/// One game execution stored: `build/game-runs/<id>/` to `store/game-runs/<id>/`, refused before anything is written
/// unless its `profile.txt` is [`LAPTOP_PROFILE`], its `unity.log` names no Steam account ([`steam_free`]) and its game
/// read the TEMP its state declares, `declared` ([`temp_declared`]). Returns the line naming what was stored, with its
/// log's and its call trace's line counts.
pub fn store_game(root: &Path, id: &str, declared: &str) -> Result<String, String> {
    let src = root.join("build/game-runs").join(id);
    let profile = fs::read_to_string(src.join("profile.txt")).map_err(|e| format!("{id}: profile.txt: {e}"))?;
    if !laptop_profile(&profile) {
        return Err(format!(
            "{id}: profile.txt is {profile:?}, not the laptop's ({LAPTOP_PROFILE}): only a laptop run is stored"
        ));
    }
    steam_free(&src).map_err(|why| format!("{id}: {why}"))?;
    temp_declared(&src, declared).map_err(|why| format!("{id}: {why}"))?;
    let dst = root.join("store/game-runs").join(id);
    let n = copy_tree(&src, &dst)?;
    let lines = |name: &str| fs::read_to_string(src.join(name)).map_or(0, |t| t.lines().count());
    Ok(format!(
        "{id}: stored store/game-runs/{id}, {n} files, profile {}, unity.log {} lines, trace {} lines",
        profile.trim(),
        lines("unity.log"),
        lines("jit_capture.bin.trace.tsv")
    ))
}
