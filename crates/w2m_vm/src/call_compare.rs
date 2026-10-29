// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run's calls judged against the game's on Windows, call by call (Sprint 214; census row 7): the frozen
//! host's `scripts/call_compare.py` (Sprint 060, the reference a set from Sprint 067), rule for rule. The Rust host's
//! stream was held to the frozen host's only, which conforms to Windows over the reference's 168 log lines; a defect the
//! frozen host and the Rust host share passed. This holds the Rust stream to Windows itself.
//!
//! Each (image, function) pair's calls are compared thread by thread, in order, as far as both go, each value by the
//! relation its type in specs/signatures.json allows (`judge`): a value equal in the type's bytes; a pointer null on
//! both or neither, an argument below 0x10000 equal; a handle null on both or neither, a pseudo-handle equal; a float
//! or void not compared. A last error is compared where the function sets one and its return says it failed
//! (`failed`). Values the system assigns are held through one-to-one maps (`ASSIGNED`). Threads pair by the names the
//! game gives them, then by agreement (`thread_map`); a job pool's workers pair per function (`pools`). The reference
//! is a set of Windows runs (`compare_set`): a value conforms when some run shows it, a value some runs show and some
//! do not is one Windows varies. A presence divergence on a thread whose earlier calls all conform is the game's own
//! scheduling (`by_cause`).
//!
//! Where the frozen host is followed and where not is named at each rule; this module reads the contract
//! (contracts/windows.game.calls 0.1.0) and the Windows runs first.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

/// A traced call: seq, thread, label (the calling image, or "GetProcAddress"), DLL, function, eight arguments, return
/// value, last error (`write_tsv`'s columns).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Call {
    pub seq: u64,
    pub tid: u64,
    pub image: String,
    pub dll: String,
    pub function: String,
    pub args: [u64; 8],
    pub ret: u64,
    pub error: u64,
}

/// A trace in call_trace.h's text: seq, tid and the last error in decimal, the arguments and the return in hex.
pub fn read_tsv(text: &str) -> Result<Vec<Call>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        let bad = || format!("line {}: {line:?} is not a traced call", n + 1);
        if f.len() < 15 {
            return Err(bad());
        }
        let hex = |s: &str| u64::from_str_radix(s, 16).map_err(|_| bad());
        let mut args = [0u64; 8];
        for (i, a) in f[5..13].iter().enumerate() {
            args[i] = hex(a)?;
        }
        out.push(Call {
            seq: f[0].parse().map_err(|_| bad())?,
            tid: f[1].parse().map_err(|_| bad())?,
            image: f[2].into(),
            dll: f[3].into(),
            function: f[4].into(),
            args,
            ret: hex(f[13])?,
            error: f[14].parse().map_err(|_| bad())?,
        });
    }
    Ok(out)
}

/// A string argument: the call's seq, the argument's index, its text (`<capture>.strings.tsv`).
pub type Str = (u64, u64, String);

pub fn read_strings(text: &str) -> Vec<Str> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some((f.next()?.parse().ok()?, f.next()?.parse().ok()?, f.next()?.to_string()))
        })
        .collect()
}

const INVALID_HANDLE_VALUE: u64 = u64::MAX;
const MEM_RESERVE: u64 = 0x2000;

/// A parameter's or return's kind, type and size in specs/signatures.json.
#[derive(Debug, Clone)]
struct Slot {
    kind: String,
    type_name: String,
    bytes: Option<u64>,
}

/// A function's signature: its parameters, return, and whether it sets a last error.
#[derive(Debug, Clone)]
pub struct Signature {
    params: Vec<Slot>,
    ret: Slot,
    sets_last_error: bool,
}

/// specs/signatures.json's `functions`, by name.
pub fn signatures(json: &str) -> Result<BTreeMap<String, Signature>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("signatures: {e}"))?;
    let slot = |s: &Value| Slot {
        kind: s["kind"].as_str().unwrap_or("value").into(),
        type_name: s["type"].as_str().unwrap_or("").into(),
        bytes: s["bytes"].as_u64(),
    };
    let functions = v["functions"].as_object().ok_or("signatures: no functions")?;
    Ok(functions
        .iter()
        .map(|(name, f)| {
            let params = f["params"]
                .as_array()
                .map_or(Vec::new(), |p| p.iter().map(slot).collect());
            (
                name.clone(),
                Signature {
                    params,
                    ret: slot(&f["return"]),
                    sets_last_error: f["sets_last_error"].as_bool().unwrap_or(false),
                },
            )
        })
        .collect())
}

/// `failed`: whether a return says the call failed, so its last error is the function's to set: a NULL or invalid
/// handle, a NULL pointer, a BOOL of FALSE.
fn failed(kind: &str, type_name: &str, value: u64) -> bool {
    match kind {
        "handle" => value == 0 || value == INVALID_HANDLE_VALUE,
        "pointer" => value == 0,
        _ => matches!(type_name, "BOOL" | "BOOLEAN") && value & 0xFFFF_FFFF == 0,
    }
}

/// `judge`: whether a Windows value and the w2m value agree by the relation the type allows.
fn judge(kind: &str, bytes: Option<u64>, windows: u64, w2m: u64, argument: bool) -> bool {
    match kind {
        "float" | "void" => true,
        "pointer" => {
            if argument && (windows < 0x10000 || w2m < 0x10000) {
                windows == w2m
            } else {
                (windows == 0) == (w2m == 0)
            }
        }
        "handle" => {
            if windows >> 63 != 0 || w2m >> 63 != 0 {
                windows == w2m
            } else {
                (windows == 0) == (w2m == 0)
            }
        }
        _ => {
            let mask = match bytes {
                Some(n) if n < 8 => (1u64 << (8 * n)) - 1,
                _ => u64::MAX,
            };
            windows & mask == w2m & mask
        }
    }
}

/// `ASSIGNED`: values the system assigns, each with the relation that holds instead of equality.
fn assigned(function: &str, field: &str) -> Option<&'static str> {
    Some(match (function, field) {
        ("GetCurrentThreadId", "ret") | ("OpenThread", "a2") | ("GetThreadId", "ret") => "thread",
        ("GetCurrentProcessId", "ret") | ("OpenProcess", "a2") => "process",
        ("TlsAlloc", "ret") | ("TlsGetValue", "a0") | ("TlsSetValue", "a0") | ("TlsFree", "a0") => "tls",
        ("FlsAlloc", "ret") | ("FlsGetValue", "a0") | ("FlsSetValue", "a0") | ("FlsFree", "a0") => "fls",
        ("RegisterClassExW", "ret")
        | ("RegisterClassW", "ret")
        | ("RegisterClassExA", "ret")
        | ("CreateWindowExW", "a1")
        | ("CreateWindowExA", "a1")
        | ("UnregisterClassW", "a0") => "atom",
        ("timeGetTime", "ret") | ("GetTickCount", "ret") | ("GetTickCount64", "ret") => "clock",
        _ => return None,
    })
}

const WINDOW_LONG_ADDRESSES: [u64; 3] = [
    (-4i64 as u64) & 0xFFFF_FFFF,
    (-6i64 as u64) & 0xFFFF_FFFF,
    (-21i64 as u64) & 0xFFFF_FFFF,
];
const POINTER_LPARAM: [u64; 14] = [
    0x01, 0x0C, 0x0D, 0x1A, 0x24, 0x46, 0x47, 0x4A, 0x4E, 0x7C, 0x7D, 0x81, 0x83, 0x2E0,
];
const WINDOW_WPARAM: [u64; 2] = [0x07, 0x08];
const MESSAGE_FUNCTIONS: [&str; 8] = [
    "DefWindowProcW",
    "DefWindowProcA",
    "SendMessageW",
    "SendMessageA",
    "CallWindowProcW",
    "CallWindowProcA",
    "PostMessageW",
    "PostMessageA",
];

/// `carries_address`: integers that carry an address by the function's documentation, judged as pointers.
fn carries_address(function: &str, field: &str, args: &[u64; 8]) -> bool {
    match (function, field) {
        ("RtlAddGrowableFunctionTable", "a4" | "a5") | ("EnumDisplayMonitors", "a3") => true,
        ("SetWindowLongPtrW" | "SetWindowLongPtrA", "a2") => {
            let index = args[1] & 0xFFFF_FFFF;
            WINDOW_LONG_ADDRESSES.contains(&index) || index < 0x8000_0000
        }
        (f, "a3") if MESSAGE_FUNCTIONS.contains(&f) => POINTER_LPARAM.contains(&(args[1] & 0xFFFF_FFFF)),
        _ => false,
    }
}

/// `placed`: the calls without the reservations made at an address the caller chose, and each failed one's retry
/// (mono's hinted VirtualAlloc; whether the address is free is the address space's layout, which ASLR draws per run).
fn placed(calls: &[Call]) -> Vec<Call> {
    let mut sorted = calls.to_vec();
    sorted.sort();
    let mut retry: BTreeMap<u64, u64> = BTreeMap::new();
    let mut out = Vec::new();
    for c in sorted {
        if c.function == "VirtualAlloc" {
            if c.args[0] != 0 && c.args[2] & MEM_RESERVE != 0 {
                if c.ret == 0 {
                    retry.insert(c.tid, c.args[1]);
                }
                continue;
            }
            if c.args[0] == 0 && retry.get(&c.tid) == Some(&c.args[1]) {
                retry.remove(&c.tid);
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// `thread_names`: each thread's first SetThreadDescription name, given to the thread that makes the call.
fn thread_names(calls: &[Call], strings: &[Str]) -> BTreeMap<u64, String> {
    let by_seq: BTreeMap<u64, &Call> = calls
        .iter()
        .filter(|c| c.function == "SetThreadDescription")
        .map(|c| (c.seq, c))
        .collect();
    let mut sorted = strings.to_vec();
    sorted.sort();
    let mut names = BTreeMap::new();
    for (seq, arg, text) in sorted {
        if let Some(c) = by_seq.get(&seq)
            && arg == 1
        {
            names.entry(c.tid).or_insert(text);
        }
    }
    names
}

/// `pools`: each thread of a job pool, a name ending in a number shared but for it by two threads or more, to the pool.
fn pools(names: &BTreeMap<u64, String>) -> BTreeMap<u64, String> {
    let stems: BTreeMap<u64, String> = names
        .iter()
        .filter_map(|(t, n)| {
            let (stem, number) = n.rsplit_once(' ')?;
            (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit())).then(|| (*t, stem.to_string()))
        })
        .collect();
    stems
        .iter()
        .filter(|(_, s)| stems.values().filter(|x| x == s).count() > 1)
        .map(|(t, s)| (*t, s.clone()))
        .collect()
}

/// The threads in the order each first appears.
fn order(calls: &[Call]) -> Vec<u64> {
    let mut sorted: Vec<&Call> = calls.iter().collect();
    sorted.sort();
    let mut seen = Vec::new();
    for c in sorted {
        if !seen.contains(&c.tid) {
            seen.push(c.tid);
        }
    }
    seen
}

type ByFunction<'a> = BTreeMap<u64, BTreeMap<(String, String), Vec<&'a Call>>>;

fn by_function(calls: &[Call]) -> ByFunction<'_> {
    let mut sorted: Vec<&Call> = calls.iter().collect();
    sorted.sort();
    let mut out: ByFunction = BTreeMap::new();
    for c in sorted {
        out.entry(c.tid)
            .or_default()
            .entry((c.image.to_lowercase(), c.function.clone()))
            .or_default()
            .push(c);
    }
    out
}

/// `thread_map`: Windows thread -> w2m thread, by name first, then by agreement in the order threads first appear.
fn thread_map(
    windows: &[Call],
    w2m: &[Call],
    sigs: &BTreeMap<String, Signature>,
    names: (&BTreeMap<u64, String>, &BTreeMap<u64, String>),
) -> BTreeMap<u64, u64> {
    let (w, m) = (by_function(windows), by_function(w2m));
    let empty = BTreeMap::new();
    let agreement = |tw: u64, tm: u64| -> i64 {
        let mut score = 0i64;
        let theirs_all = w.get(&tw).unwrap_or(&empty);
        let ours_all = m.get(&tm).unwrap_or(&empty);
        for (key, theirs) in theirs_all {
            let ours = ours_all.get(key).map_or(&[][..], Vec::as_slice);
            for (a, b) in theirs.iter().zip(ours) {
                let f = sigs.get(&key.1);
                let mut agree = 0i64;
                if let Some(f) = f {
                    for (i, p) in f.params.iter().take(8).enumerate() {
                        let field = format!("a{i}");
                        if assigned(&key.1, &field).is_none()
                            && !carries_address(&key.1, &field, &a.args)
                            && judge(&p.kind, p.bytes, a.args[i], b.args[i], false)
                        {
                            agree += 1;
                        }
                    }
                    if assigned(&key.1, "ret").is_none() && judge(&f.ret.kind, f.ret.bytes, a.ret, b.ret, false) {
                        agree += 1;
                    }
                }
                score += 1 + agree;
            }
        }
        score
    };
    let mut free = order(w2m);
    let mut pairs = BTreeMap::new();
    for tw in order(windows) {
        if let Some(name) = names.0.get(&tw)
            && let Some(i) = free.iter().position(|tm| names.1.get(tm) == Some(name))
        {
            pairs.insert(tw, free.remove(i));
        }
    }
    for tw in order(windows) {
        if pairs.contains_key(&tw) {
            continue;
        }
        if free.is_empty() {
            break;
        }
        // the highest agreement, the first to appear on a tie
        let mut best = 0;
        let mut best_score = i64::MIN;
        for (i, tm) in free.iter().enumerate() {
            let s = agreement(tw, *tm);
            if s > best_score {
                best_score = s;
                best = i;
            }
        }
        pairs.insert(tw, free.remove(best));
    }
    pairs
}

/// One divergence, as w2m's side names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub image: String,
    pub function: String,
    /// the call's place among this pair's calls
    pub index: usize,
    /// "a0".."a7", "ret", "error", "called" (w2m calls it, Windows never does) or "missing"
    pub field: String,
    pub windows: Option<u64>,
    pub w2m: Option<u64>,
    pub windows_seq: Option<u64>,
    pub w2m_thread: Option<u64>,
}

type Key = (String, String, Option<u64>, usize, String);

impl Divergence {
    fn key(&self) -> Key {
        (
            self.image.to_lowercase(),
            self.function.clone(),
            self.w2m_thread,
            self.index,
            self.field.clone(),
        )
    }

    pub fn text(&self) -> String {
        let v = |x: Option<u64>| x.map_or("none".to_string(), |x| format!("0x{x:x}"));
        format!(
            "{}!{} call {} {}: Windows {}, w2m {}",
            self.image,
            self.function,
            self.index,
            self.field,
            v(self.windows),
            v(self.w2m)
        )
    }
}

/// `agreeing`: how many of two threads' calls to one function agree in every value the comparison judges here.
fn agreeing(windows: &[&Call], w2m: &[&Call], sigs: &BTreeMap<String, Signature>) -> usize {
    windows
        .iter()
        .zip(w2m)
        .filter(|(a, b)| match sigs.get(&a.function) {
            None => a.ret == b.ret,
            Some(sig) => {
                let mut fields: Vec<(String, &Slot, u64, u64, bool)> = sig
                    .params
                    .iter()
                    .take(8)
                    .enumerate()
                    .map(|(i, p)| (format!("a{i}"), p, a.args[i], b.args[i], true))
                    .collect();
                fields.push(("ret".into(), &sig.ret, a.ret, b.ret, false));
                fields
                    .iter()
                    .filter(|(f, ..)| assigned(&a.function, f).is_none())
                    .all(|(f, p, x, y, arg)| {
                        if carries_address(&a.function, f, &a.args) {
                            judge("pointer", None, *x, *y, *arg)
                        } else {
                            judge(&p.kind, p.bytes, *x, *y, *arg)
                        }
                    })
            }
        })
        .count()
}

/// A compared call, named from w2m's side: image (lower case), function, w2m thread, the call's place.
type Compared = (String, String, u64, usize);

/// `judged`: every divergence of `w2m` against one Windows run, in the order of the Windows calls, and the calls
/// compared. `excluded` are the run's threads the game does not start (the capture's own); `undecided` the (function,
/// field) values two Windows runs disagree on (the contract's `undetermined`).
pub fn judged(
    windows: &[Call],
    w2m: &[Call],
    sigs: &BTreeMap<String, Signature>,
    excluded: &BTreeSet<u64>,
    strings: (&[Str], &[Str]),
    undecided: &BTreeSet<(String, String)>,
) -> (Vec<Divergence>, Vec<Compared>) {
    let windows: Vec<Call> = placed(
        &windows
            .iter()
            .filter(|c| !excluded.contains(&c.tid))
            .cloned()
            .collect::<Vec<_>>(),
    );
    let w2m = placed(w2m);
    let names = (thread_names(&windows, strings.0), thread_names(&w2m, strings.1));
    let threads = thread_map(&windows, &w2m, sigs, (&names.0, &names.1));
    let (pool_w, pool_m) = (pools(&names.0), pools(&names.1));
    type ByPair<'a> = BTreeMap<(String, String), BTreeMap<u64, Vec<&'a Call>>>;
    fn by_pair(calls: &[Call]) -> ByPair<'_> {
        let mut sorted: Vec<&Call> = calls.iter().collect();
        sorted.sort();
        let mut out: ByPair = BTreeMap::new();
        for c in sorted {
            out.entry((c.image.to_lowercase(), c.function.clone()))
                .or_default()
                .entry(c.tid)
                .or_default()
                .push(c);
        }
        out
    }
    let (w, m) = (by_pair(&windows), by_pair(&w2m));
    let mut found = Vec::new();
    let mut pairs: Vec<(usize, &Call, &Call)> = Vec::new();
    for (pair, by_thread) in &w {
        if !m.contains_key(pair) {
            for (tid, calls) in by_thread {
                for (k, c) in calls.iter().enumerate() {
                    found.push(Divergence {
                        image: c.image.clone(),
                        function: pair.1.clone(),
                        index: k,
                        field: "missing".into(),
                        windows: Some(c.ret),
                        w2m: None,
                        windows_seq: Some(c.seq),
                        w2m_thread: threads.get(tid).copied(),
                    });
                }
            }
        }
    }
    for (pair, ours) in &m {
        let Some(theirs) = w.get(pair) else {
            let first = ours.values().flatten().min().unwrap();
            found.push(Divergence {
                image: first.image.clone(),
                function: pair.1.clone(),
                index: 0,
                field: "called".into(),
                windows: None,
                w2m: Some(first.ret),
                windows_seq: None,
                w2m_thread: Some(first.tid),
            });
            continue;
        };
        let mut chosen = threads.clone();
        let pool_names: BTreeSet<&String> = pool_w.values().collect();
        for pool in pool_names {
            let mut free: Vec<u64> = ours.keys().filter(|t| pool_m.get(t) == Some(pool)).copied().collect();
            let mut workers: Vec<u64> = theirs.keys().filter(|t| pool_w.get(t) == Some(pool)).copied().collect();
            workers.sort_by_key(|t| theirs[t][0].seq);
            for tw in workers {
                if free.is_empty() {
                    break;
                }
                let mut best = 0;
                let mut best_score = (0usize, false);
                for (i, tm) in free.iter().enumerate() {
                    let s = (agreeing(&theirs[&tw], &ours[tm], sigs), threads.get(&tw) == Some(tm));
                    if i == 0 || s > best_score {
                        best_score = s;
                        best = i;
                    }
                }
                chosen.insert(tw, free.remove(best));
            }
        }
        for (tid, calls) in theirs {
            let mine = chosen.get(tid).and_then(|t| ours.get(t)).map_or(&[][..], Vec::as_slice);
            for (k, (a, b)) in calls.iter().zip(mine).enumerate() {
                pairs.push((k, a, b));
            }
            for (k, c) in calls.iter().enumerate().skip(mine.len()) {
                found.push(Divergence {
                    image: c.image.clone(),
                    function: pair.1.clone(),
                    index: k,
                    field: "missing".into(),
                    windows: Some(c.ret),
                    w2m: None,
                    windows_seq: Some(c.seq),
                    w2m_thread: chosen.get(tid).copied(),
                });
            }
        }
    }
    // per space of assigned values, the Windows-to-w2m map and its inverse: an id maps one to one across the stream
    type OneToOne = (BTreeMap<u64, u64>, BTreeMap<u64, u64>);
    let mut maps: BTreeMap<&'static str, OneToOne> = BTreeMap::new();
    let mut same = |space: &'static str, x: u64, y: u64| -> bool {
        match space {
            "clock" => true,
            "thread" => {
                let (x, y) = (x & 0xFFFF_FFFF, y & 0xFFFF_FFFF);
                match pool_w.get(&x) {
                    Some(pool) => pool_m.get(&y) == Some(pool),
                    None => threads.get(&x) == Some(&y),
                }
            }
            "atom" if x >= 0x10000 || y >= 0x10000 => (x == 0) == (y == 0),
            _ => {
                let (x, y) = (x & 0xFFFF_FFFF, y & 0xFFFF_FFFF);
                let (forward, backward) = maps.entry(space).or_default();
                *forward.entry(x).or_insert(y) == y && *backward.entry(y).or_insert(x) == x
            }
        }
    };
    pairs.sort_by_key(|p| p.1.seq);
    for (k, a, b) in &pairs {
        let fn_ = &a.function;
        let fields: Vec<(String, String, Option<u64>, u64, u64, bool)> = match sigs.get(fn_) {
            None => vec![("ret".into(), "value".into(), Some(8), a.ret, b.ret, false)],
            Some(sig) => {
                let mut f: Vec<_> = sig
                    .params
                    .iter()
                    .take(8)
                    .enumerate()
                    .map(|(i, p)| (format!("a{i}"), p.kind.clone(), p.bytes, a.args[i], b.args[i], true))
                    .collect();
                f.push(("ret".into(), sig.ret.kind.clone(), sig.ret.bytes, a.ret, b.ret, false));
                if sig.sets_last_error && failed(&sig.ret.kind, &sig.ret.type_name, a.ret) {
                    f.push(("error".into(), "value".into(), Some(4), a.error, b.error, false));
                }
                f
            }
        };
        for (name, kind, size, x, y, argument) in fields {
            if undecided.contains(&(fn_.clone(), name.clone()))
                || undecided.contains(&(format!("{}!{fn_}", a.image), name.clone()))
            {
                continue;
            }
            let mut space = assigned(fn_, &name);
            if MESSAGE_FUNCTIONS.contains(&fn_.as_str())
                && (name == "a0" || name == "a2" && WINDOW_WPARAM.contains(&(a.args[1] & 0xFFFF_FFFF)))
            {
                space = Some("window");
            }
            let ok = if let Some(space) = space {
                same(space, x, y)
            } else if carries_address(fn_, &name, &a.args) {
                judge("pointer", None, x, y, argument)
            } else {
                judge(&kind, size, x, y, argument)
            };
            if !ok {
                found.push(Divergence {
                    image: a.image.clone(),
                    function: fn_.clone(),
                    index: *k,
                    field: name,
                    windows: Some(x),
                    w2m: Some(y),
                    windows_seq: Some(a.seq),
                    w2m_thread: Some(b.tid),
                });
            }
        }
    }
    found.sort_by(|a, b| {
        (a.windows_seq.is_none(), a.windows_seq.unwrap_or(0), &a.function).cmp(&(
            b.windows_seq.is_none(),
            b.windows_seq.unwrap_or(0),
            &b.function,
        ))
    });
    let compared = pairs
        .iter()
        .map(|(k, a, b)| (a.image.to_lowercase(), a.function.clone(), b.tid, *k))
        .collect();
    (found, compared)
}

/// One Windows run of the reference set.
pub struct Reference {
    pub run: String,
    pub calls: Vec<Call>,
    pub excluded: BTreeSet<u64>,
    pub strings: Vec<Str>,
}

/// One w2m value judged against every run of the set (`SetDivergence`).
#[derive(Debug, Clone)]
pub struct SetDivergence {
    pub divergence: Divergence,
    pub run: String,
    pub disagree: Vec<String>,
    pub agree: Vec<String>,
    pub of: usize,
    pub cause: String,
}

impl SetDivergence {
    pub fn varies(&self) -> bool {
        !self.agree.is_empty()
    }

    pub fn text(&self) -> String {
        let silent = self.of - self.disagree.len() - self.agree.len();
        let mut t = format!(
            "{}: {} of {} reference executions disagree",
            self.divergence.text(),
            self.disagree.len(),
            self.of
        );
        if !self.agree.is_empty() {
            t += &format!(", {} agree", self.agree.len());
        }
        if silent > 0 {
            t += &format!(", {silent} do not make the call");
        }
        t += &format!(
            " (run {} seq {})",
            self.run,
            self.divergence.windows_seq.map_or("None".into(), |s| s.to_string())
        );
        if !self.cause.is_empty() {
            t += &format!(" [{}]", self.cause);
        }
        t
    }
}

/// `compare_set`: w2m's calls against every run of the set: the divergences, the values Windows varies, and the calls
/// compared per run.
pub fn compare_set(
    references: &[Reference],
    w2m: &[Call],
    sigs: &BTreeMap<String, Signature>,
    strings: &[Str],
    undecided: &BTreeSet<(String, String)>,
) -> (Vec<SetDivergence>, Vec<SetDivergence>, BTreeMap<String, usize>) {
    let mut found: BTreeMap<String, Vec<(Key, Divergence)>> = BTreeMap::new();
    let mut compared: BTreeMap<String, BTreeSet<Compared>> = BTreeMap::new();
    for r in references {
        let (divergences, pairs) = judged(&r.calls, w2m, sigs, &r.excluded, (&r.strings, strings), undecided);
        let mut seen: Vec<(Key, Divergence)> = Vec::new();
        for d in divergences {
            let k = d.key();
            if !seen.iter().any(|(x, _)| *x == k) {
                seen.push((k, d));
            }
        }
        found.insert(r.run.clone(), seen);
        compared.insert(r.run.clone(), pairs.into_iter().collect());
    }
    let runs: Vec<String> = references.iter().map(|r| r.run.clone()).collect();
    let holds = |run: &str, key: &Key| found[run].iter().any(|(k, _)| k == key);
    let agrees = |run: &str, key: &Key| -> bool {
        if holds(run, key) {
            return false;
        }
        if key.4 == "missing" || key.4 == "called" {
            return true;
        }
        key.2
            .is_some_and(|t| compared[run].contains(&(key.0.clone(), key.1.clone(), t, key.3)))
    };
    let mut keys: Vec<Key> = Vec::new();
    for run in &runs {
        for (k, _) in &found[run] {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
    }
    let mut set: Vec<SetDivergence> = keys
        .iter()
        .map(|key| {
            let disagree: Vec<String> = runs.iter().filter(|r| holds(r, key)).cloned().collect();
            let agree: Vec<String> = runs.iter().filter(|r| agrees(r, key)).cloned().collect();
            let first = &disagree[0];
            let d = found[first].iter().find(|(k, _)| k == key).unwrap().1.clone();
            SetDivergence {
                divergence: d,
                run: first.clone(),
                disagree,
                agree,
                of: runs.len(),
                cause: String::new(),
            }
        })
        .collect();
    set.sort_by(|a, b| {
        let at = |s: &SetDivergence| {
            (
                s.divergence.windows_seq.is_none(),
                runs.iter().position(|r| *r == s.run),
                s.divergence.windows_seq.unwrap_or(0),
                s.divergence.function.clone(),
            )
        };
        at(a).cmp(&at(b))
    });
    let counts = runs.iter().map(|r| (r.clone(), compared[r].len())).collect();
    let (varies, divergences): (Vec<_>, Vec<_>) = set.into_iter().partition(SetDivergence::varies);
    (divergences, varies, counts)
}

/// `by_cause`: the divergences that are defects, and those that are the game's own scheduling: a presence divergence
/// on a w2m thread whose earlier calls all conform, the rest defects.
pub fn by_cause(found: Vec<SetDivergence>) -> (Vec<SetDivergence>, Vec<SetDivergence>) {
    let presence = |s: &SetDivergence| s.divergence.field == "missing" || s.divergence.field == "called";
    let values: Vec<SetDivergence> = found.iter().filter(|s| !presence(s)).cloned().collect();
    let (mut defects, mut scheduling) = (Vec::new(), Vec::new());
    for s in found {
        let d = &s.divergence;
        if !presence(&s) {
            defects.push(s);
        } else if d.w2m_thread.is_none() {
            defects.push(SetDivergence {
                cause: "a Windows thread with no w2m partner".into(),
                ..s
            });
        } else {
            let before = values.iter().find(|v| {
                v.divergence.w2m_thread == d.w2m_thread
                    && (d.field == "called"
                        || (v.run == s.run && v.divergence.windows_seq.unwrap_or(0) < d.windows_seq.unwrap_or(0)))
            });
            match before {
                Some(v) => {
                    let cause = format!(
                        "after value divergence {}",
                        v.divergence.windows_seq.map_or("None".into(), |x| x.to_string())
                    );
                    defects.push(SetDivergence { cause, ..s });
                }
                None => scheduling.push(SetDivergence {
                    cause: "scheduling".into(),
                    ..s
                }),
            }
        }
    }
    (defects, scheduling)
}

/// A run's verdict against the set: its defects, the divergences past a stopped run's cut, the game's own scheduling,
/// and the values Windows varies.
pub struct Verdict {
    pub defects: Vec<SetDivergence>,
    pub past_cut: Vec<SetDivergence>,
    pub scheduling: Vec<SetDivergence>,
    pub varies: Vec<SetDivergence>,
    pub compared: BTreeMap<String, usize>,
}

impl Verdict {
    /// "conforms" or "discrepancy", COMPARISON_COMPLETED's verdict.
    pub fn verdict(&self) -> &'static str {
        if self.defects.is_empty() {
            "conforms"
        } else {
            "discrepancy"
        }
    }
}

/// `calls` against the set, its divergences by cause (`by_cause`). A run `stopped` before ExitProcess has not started
/// every thread Windows started: the calls of a Windows thread with no w2m partner lie past the cut, not defects
/// (Sprint 214: all 76 such at the Rust host's row 3131, as at the frozen host's same rows); every other divergence of a
/// stopped run is a defect. The frozen host compared whole runs and had no cut.
pub fn verdict(
    references: &[Reference],
    calls: &[Call],
    sigs: &BTreeMap<String, Signature>,
    strings: &[Str],
    undecided: &BTreeSet<(String, String)>,
    stopped: bool,
) -> Verdict {
    let (found, varies, compared) = compare_set(references, calls, sigs, strings, undecided);
    let (defects, scheduling) = by_cause(found);
    let (past_cut, defects): (Vec<_>, Vec<_>) = defects
        .into_iter()
        .partition(|d| stopped && d.cause == "a Windows thread with no w2m partner");
    Verdict {
        defects,
        past_cut,
        scheduling,
        varies,
        compared,
    }
}

/// COMPARISON_COMPLETED.reference_ref for a set: its executions joined by " + " (`reference_ref`).
pub fn reference_ref(references: &[Reference]) -> String {
    references
        .iter()
        .map(|r| format!("game-run-{}", r.run))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The reference set a frozen-host record names: its RunStarted baseline's `reference_set` (`run_frontier.py`), the runs
/// `call_compare.py`'s `reference_set` chose (Sprint 214: the membership rules, which read laptop and runner run files
/// and records, are not ported; the set is the frozen host's last choice).
pub fn reference_runs(record_first_segment: &str) -> Result<Vec<String>, String> {
    let started = record_first_segment
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["kind"] == "substrate.RunStarted")
        .ok_or("no substrate.RunStarted in the record")?;
    started["payload"]["baseline"]["reference_set"]
        .as_array()
        .ok_or("the RunStarted baseline names no reference_set")?
        .iter()
        .map(|r| {
            r.as_str()
                .map(str::to_string)
                .ok_or("a reference run that is not a string".to_string())
        })
        .collect()
}

/// The contract's `excluded_threads` for a run, and its `undetermined` (function, field) values.
pub fn contract_rules(contract: &Value, run: &str) -> (BTreeSet<u64>, BTreeSet<(String, String)>) {
    let excluded = contract["excluded_threads"][run]
        .as_object()
        .map_or(BTreeSet::new(), |o| o.keys().filter_map(|k| k.parse().ok()).collect());
    let undetermined = contract["undetermined"].as_object().map_or(BTreeSet::new(), |o| {
        o.keys()
            .filter_map(|k| k.split_once(' ').map(|(f, field)| (f.to_string(), field.to_string())))
            .collect()
    });
    (excluded, undetermined)
}

/// A run of the set from its directory: its trace and strings; a laptop run's capture thread is the trace's first
/// call's (`capture_thread`), a runner run's the contract's (`load_reference`).
pub fn load_reference(dir: &Path, contract: &Value) -> Result<Reference, String> {
    let read =
        |name: &str| std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{}: {e}", dir.join(name).display()));
    let calls = read_tsv(&read("jit_capture.bin.trace.tsv")?)?;
    let strings = read_strings(&read("jit_capture.bin.strings.tsv").unwrap_or_default());
    let run = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let laptop = std::fs::read_to_string(dir.join("profile.txt")).is_ok_and(|p| p.starts_with("laptop-"));
    let excluded = if laptop {
        calls.iter().min().map(|c| BTreeSet::from([c.tid])).unwrap_or_default()
    } else {
        contract_rules(contract, &run).0
    };
    Ok(Reference {
        run,
        calls,
        excluded,
        strings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(seq: u64, tid: u64, function: &str, a0: u64, ret: u64) -> Call {
        Call {
            seq,
            tid,
            image: "mono-2.0-bdwgc.dll".into(),
            dll: "KERNEL32.dll".into(),
            function: function.into(),
            args: [a0, 0, 0, 0, 0, 0, 0, 0],
            ret,
            error: 0,
        }
    }

    /// `judge`'s relations: a value in its type's bytes, a pointer's nullness, a handle's top bit; `failed`'s three.
    #[test]
    fn each_value_is_judged_by_its_types_relation() {
        assert!(judge("value", Some(4), 0x1_0000_0005, 5, true));
        assert!(!judge("value", Some(4), 5, 6, true));
        assert!(judge("pointer", None, 0x1234_0000, 0x5678_0000, false));
        assert!(!judge("pointer", None, 0, 0x5678_0000, false));
        assert!(
            !judge("pointer", None, 0x20, 0x21, true),
            "an argument below 0x10000 equal"
        );
        assert!(judge("handle", None, 0x44, 0x88, false));
        assert!(!judge("handle", None, u64::MAX, u64::MAX - 1, false));
        assert!(failed("handle", "HANDLE", u64::MAX) && failed("pointer", "LPVOID", 0));
        assert!(failed("value", "BOOL", 0x1_0000_0000) && !failed("value", "DWORD", 0));
    }

    /// A return that differs is named by its row; a function only w2m calls is "called"; a Windows call past w2m's is
    /// "missing"; the capture's thread is left out.
    #[test]
    fn a_divergence_is_named_by_its_call_and_field() {
        let sigs = BTreeMap::new();
        let windows = vec![
            call(1, 9, "TlsGetValue", 1, 0),
            call(2, 1, "GetLastError", 0, 5),
            call(3, 1, "GetLastError", 0, 6),
        ];
        let w2m = vec![call(1, 4, "GetLastError", 0, 5), call(2, 4, "Sleep", 0, 0)];
        let (found, compared) = judged(
            &windows,
            &w2m,
            &sigs,
            &BTreeSet::from([9]),
            (&[], &[]),
            &BTreeSet::new(),
        );
        let texts: Vec<String> = found.iter().map(|d| format!("{} {}", d.function, d.field)).collect();
        assert_eq!(texts, ["GetLastError missing", "Sleep called"], "{found:?}");
        assert_eq!(compared.len(), 1);
        let w2m = vec![call(1, 4, "GetLastError", 0, 7), call(2, 4, "GetLastError", 0, 6)];
        let (found, _) = judged(
            &windows,
            &w2m,
            &sigs,
            &BTreeSet::from([9]),
            (&[], &[]),
            &BTreeSet::new(),
        );
        assert_eq!(
            found[0].text(),
            "mono-2.0-bdwgc.dll!GetLastError call 0 ret: Windows 0x5, w2m 0x7"
        );
    }
}
