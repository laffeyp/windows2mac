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
/// finished bundle stalled 6 minutes; the same copy by hand took 0.9 s). The host key on record is under the laptop's
/// first address, 192.168.4.55; the alias names it while the host is reached by name.
pub const LINK: [&str; 8] = [
    "-o",
    "ConnectTimeout=15",
    "-o",
    "ServerAliveInterval=15",
    "-o",
    "ServerAliveCountMax=4",
    "-o",
    "HostKeyAlias=192.168.4.55",
];
/// A job's ssh ends after this even if the link stays alive (audit 2026-10-13); its code is then 124.
pub const JOB_SECONDS: u64 = 2 * 3600;
/// The lock that holds one job at a time to the laptop's one workspace.
pub const LOCK: &str = "build/windows_machine.lock";
/// A stored run's files besides its executions' (`store_probe_runs.py` `KEPT`, less a batch's `batch.txt`).
pub const KEPT: [&str; 6] = [
    "cpu.txt",
    "probe.sha256",
    "profile.txt",
    "profile.json",
    "job.json",
    "run-job.log",
];
/// The family runs' lookup: family, run, binary, contract, the beside DLL as `name:sha256`, ...
pub const KNOWN: &str = "store/family_runs/known-2026-10-29.tsv";

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
    let job = root.join("store/probe-runs").join(row[1]).join("job.json");
    let named = fs::read_to_string(&job)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("probe").and_then(Value::as_str).map(str::to_string));
    let probe = match named {
        Some(p) => p,
        None if root.join("probes").join(family).is_dir() => family.to_string(),
        None => {
            return Err(format!(
                "family {family}: run {}'s store has no job.json and there is no probes/{family}, so its probe is not \
                 known",
                row[1]
            ));
        }
    };
    Ok(StoredRun {
        run: row[1].to_string(),
        binary: row[2].to_string(),
        dll,
        probe,
    })
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
/// `store/binaries/<sha256>.exe` unless that binary is stored. A run whose `profile.txt` does not start `laptop-` is
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
    if !profile.starts_with("laptop-") {
        return Err(format!(
            "{run}: profile.txt is {:?}, not the laptop's (laptop-...): only a laptop run is stored",
            profile.trim()
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
    let dst = root.join("store/probe-runs").join(run);
    fs::create_dir_all(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
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
    Ok(format!(
        "{run}: stored store/probe-runs/{run}, {} files, {executions} executions, profile {}, cpu {}, binary \
         store/binaries/{sha}.exe",
        files.len(),
        profile.trim(),
        cpu.trim()
    ))
}
