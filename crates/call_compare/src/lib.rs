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
//! (contracts/windows.game.calls 0.1.0) and the Windows runs first. From Sprint 375 the set's runs are chosen from
//! store/game-runs by the frozen host's membership rules (`reference_set`), where before they were the frozen host's last
//! choice, read from its record.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use events::{
    ExecutionCompleted, ExecutionStarted, ExitStatus, FiniteF64, HarnessI64, HarnessU64, Id, ProfileKind, Sha256,
    SubjectKind,
};
use serde_json::Value;

/// A traced call: seq, thread, label (the calling image, or "GetProcAddress"), DLL, function, eight arguments, return
/// value, last error (`write_tsv`'s columns).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Call {
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
fn read_tsv(text: &str) -> Result<Vec<Call>, String> {
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
type Str = (u64, u64, String);

fn read_strings(text: &str) -> Vec<Str> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some((f.next()?.parse().ok()?, f.next()?.parse().ok()?, f.next()?.to_string()))
        })
        .collect()
}

const INVALID_HANDLE_VALUE: u64 = u64::MAX;
const MEM_RESERVE: u64 = 0x2000;

/// A value's kind in specs/signatures.json, which picks the relation `judge` holds it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Value,
    Pointer,
    Handle,
    Float,
    Void,
}

impl Kind {
    /// A kind the file names; any other is refused, as `call_compare.py`'s `p["kind"]` raises on a missing one
    /// (reviewer-3's Low 1 of its review of Sprints 212-214: an unknown kind was read as a value).
    fn of(slot: &Value, function: &str) -> Result<Kind, String> {
        Ok(match slot["kind"].as_str() {
            Some("value") => Kind::Value,
            Some("pointer") => Kind::Pointer,
            Some("handle") => Kind::Handle,
            Some("float") => Kind::Float,
            Some("void") => Kind::Void,
            other => return Err(format!("signatures: {function} has a value of kind {other:?}")),
        })
    }
}

/// A parameter's or return's kind, type and size in specs/signatures.json.
#[derive(Debug, Clone)]
struct Slot {
    kind: Kind,
    type_name: String,
    bytes: Option<u64>,
}

/// A function's signature: its parameters, return, and whether it sets a last error.
#[derive(Debug, Clone)]
struct Signature {
    params: Vec<Slot>,
    ret: Slot,
    sets_last_error: bool,
}

/// specs/signatures.json's `functions`, by name; a value of no known kind refuses the file.
fn signatures(json: &str) -> Result<BTreeMap<String, Signature>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("signatures: {e}"))?;
    let slot = |s: &Value, function: &str| -> Result<Slot, String> {
        Ok(Slot {
            kind: Kind::of(s, function)?,
            type_name: s["type"].as_str().unwrap_or("").into(),
            bytes: s["bytes"].as_u64(),
        })
    };
    let functions = v["functions"].as_object().ok_or("signatures: no functions")?;
    functions
        .iter()
        .map(|(name, f)| {
            let params = f["params"]
                .as_array()
                .map_or(Ok(Vec::new()), |p| p.iter().map(|s| slot(s, name)).collect())?;
            Ok((
                name.clone(),
                Signature {
                    params,
                    ret: slot(&f["return"], name)?,
                    sets_last_error: f["sets_last_error"].as_bool().unwrap_or(false),
                },
            ))
        })
        .collect()
}

/// `failed`: whether a return says the call failed, so its last error is the function's to set: a NULL or invalid
/// handle, a NULL pointer, a BOOL of FALSE.
fn failed(kind: Kind, type_name: &str, value: u64) -> bool {
    match kind {
        Kind::Handle => value == 0 || value == INVALID_HANDLE_VALUE,
        Kind::Pointer => value == 0,
        _ => matches!(type_name, "BOOL" | "BOOLEAN") && value & 0xFFFF_FFFF == 0,
    }
}

/// `judge`: whether a Windows value and the w2m value agree by the relation the type allows.
fn judge(kind: Kind, bytes: Option<u64>, windows: u64, w2m: u64, argument: bool) -> bool {
    match kind {
        Kind::Float | Kind::Void => true,
        Kind::Pointer => {
            if argument && (windows < 0x10000 || w2m < 0x10000) {
                windows == w2m
            } else {
                (windows == 0) == (w2m == 0)
            }
        }
        Kind::Handle => {
            if windows >> 63 != 0 || w2m >> 63 != 0 {
                windows == w2m
            } else {
                (windows == 0) == (w2m == 0)
            }
        }
        Kind::Value => {
            let mask = match bytes {
                Some(n) if n < 8 => (1u64 << (8 * n)) - 1,
                _ => u64::MAX,
            };
            windows & mask == w2m & mask
        }
    }
}

/// A call's value a divergence names, or its presence: `called` (w2m calls it, Windows never does), `missing` (a
/// Windows call w2m does not make).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    Arg(usize),
    Ret,
    Error,
    Called,
    Missing,
}

impl Field {
    fn presence(self) -> bool {
        matches!(self, Field::Called | Field::Missing)
    }
}

impl std::fmt::Display for Field {
    /// The contract's and `call_compare.py`'s names: "a0".."a7", "ret", "error", "called", "missing".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Arg(i) => write!(f, "a{i}"),
            Field::Ret => f.write_str("ret"),
            Field::Error => f.write_str("error"),
            Field::Called => f.write_str("called"),
            Field::Missing => f.write_str("missing"),
        }
    }
}

/// A space of values the system assigns, each held by a relation other than equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Space {
    Thread,
    Process,
    Tls,
    Fls,
    Atom,
    Clock,
    Window,
}

/// `ASSIGNED`: values the system assigns, each with the relation that holds instead of equality.
fn assigned(function: &str, field: Field) -> Option<Space> {
    use Field::{Arg, Ret};
    Some(match (function, field) {
        ("GetCurrentThreadId", Ret) | ("OpenThread", Arg(2)) | ("GetThreadId", Ret) => Space::Thread,
        ("GetCurrentProcessId", Ret) | ("OpenProcess", Arg(2)) => Space::Process,
        ("TlsAlloc", Ret) | ("TlsGetValue", Arg(0)) | ("TlsSetValue", Arg(0)) | ("TlsFree", Arg(0)) => Space::Tls,
        ("FlsAlloc", Ret) | ("FlsGetValue", Arg(0)) | ("FlsSetValue", Arg(0)) | ("FlsFree", Arg(0)) => Space::Fls,
        ("RegisterClassExW", Ret)
        | ("RegisterClassW", Ret)
        | ("RegisterClassExA", Ret)
        | ("CreateWindowExW", Arg(1))
        | ("CreateWindowExA", Arg(1))
        | ("UnregisterClassW", Arg(0)) => Space::Atom,
        ("timeGetTime", Ret) | ("GetTickCount", Ret) | ("GetTickCount64", Ret) => Space::Clock,
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
fn carries_address(function: &str, field: Field, args: &[u64; 8]) -> bool {
    use Field::Arg;
    match (function, field) {
        ("RtlAddGrowableFunctionTable", Arg(4 | 5)) | ("EnumDisplayMonitors", Arg(3)) => true,
        ("SetWindowLongPtrW" | "SetWindowLongPtrA", Arg(2)) => {
            let index = args[1] & 0xFFFF_FFFF;
            WINDOW_LONG_ADDRESSES.contains(&index) || index < 0x8000_0000
        }
        (f, Arg(3)) if MESSAGE_FUNCTIONS.contains(&f) => POINTER_LPARAM.contains(&(args[1] & 0xFFFF_FFFF)),
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
                        let field = Field::Arg(i);
                        if assigned(&key.1, field).is_none()
                            && !carries_address(&key.1, field, &a.args)
                            && judge(p.kind, p.bytes, a.args[i], b.args[i], false)
                        {
                            agree += 1;
                        }
                    }
                    if assigned(&key.1, Field::Ret).is_none() && judge(f.ret.kind, f.ret.bytes, a.ret, b.ret, false) {
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
struct Divergence {
    image: String,
    function: String,
    /// the call's place among this pair's calls
    index: usize,
    field: Field,
    windows: Option<u64>,
    w2m: Option<u64>,
    windows_seq: Option<u64>,
    /// a `called` divergence's first w2m call, which orders two such of one function in two images as they first
    /// appear, as `call_compare.py`'s dict does (reviewer-3's Low 1: they sorted by image name)
    w2m_seq: Option<u64>,
    w2m_thread: Option<u64>,
}

type Key = (String, String, Option<u64>, usize, Field);

impl Divergence {
    fn key(&self) -> Key {
        (
            self.image.to_lowercase(),
            self.function.clone(),
            self.w2m_thread,
            self.index,
            self.field,
        )
    }

    /// The order `judged` reports divergences in: by the Windows call, a `called` one after every other by its first
    /// w2m call.
    fn place(&self) -> (bool, u64, &str, u64) {
        (
            self.windows_seq.is_none(),
            self.windows_seq.unwrap_or(0),
            &self.function,
            self.w2m_seq.unwrap_or(0),
        )
    }

    fn text(&self) -> String {
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
            Some(sig) => field_pairs(sig, a, b, false)
                .iter()
                .filter(|v| assigned(&a.function, v.field).is_none())
                .all(|v| {
                    if carries_address(&a.function, v.field, &a.args) {
                        judge(Kind::Pointer, None, v.windows, v.w2m, v.argument)
                    } else {
                        judge(v.kind, v.bytes, v.windows, v.w2m, v.argument)
                    }
                }),
        })
        .count()
}

/// One field of a call pair: its kind and size, the Windows value and the w2m value, and whether it is an argument.
struct FieldPair {
    field: Field,
    kind: Kind,
    bytes: Option<u64>,
    windows: u64,
    w2m: u64,
    argument: bool,
}

/// A pair's fields by its signature: the arguments, the return, and, with `error`, the last error where the function
/// sets one and its return says it failed.
fn field_pairs(sig: &Signature, a: &Call, b: &Call, error: bool) -> Vec<FieldPair> {
    let mut f: Vec<FieldPair> = sig
        .params
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, p)| FieldPair {
            field: Field::Arg(i),
            kind: p.kind,
            bytes: p.bytes,
            windows: a.args[i],
            w2m: b.args[i],
            argument: true,
        })
        .collect();
    f.push(FieldPair {
        field: Field::Ret,
        kind: sig.ret.kind,
        bytes: sig.ret.bytes,
        windows: a.ret,
        w2m: b.ret,
        argument: false,
    });
    if error && sig.sets_last_error && failed(sig.ret.kind, &sig.ret.type_name, a.ret) {
        f.push(FieldPair {
            field: Field::Error,
            kind: Kind::Value,
            bytes: Some(4),
            windows: a.error,
            w2m: b.error,
            argument: false,
        });
    }
    f
}

/// A compared call, named from w2m's side: image (lower case), function, w2m thread, the call's place.
type Compared = (String, String, u64, usize);

/// One side's calls by (image in lower case, function), then by thread, each in seq order.
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

/// A Windows call w2m does not make, at its place `index` among its thread's calls to the pair.
fn missing(c: &Call, function: &str, index: usize, w2m_thread: Option<u64>) -> Divergence {
    Divergence {
        image: c.image.clone(),
        function: function.into(),
        index,
        field: Field::Missing,
        windows: Some(c.ret),
        w2m: None,
        windows_seq: Some(c.seq),
        w2m_seq: None,
        w2m_thread,
    }
}

/// The threads of the two sides that pair, by name and agreement, and each side's job pools.
struct Threads {
    map: BTreeMap<u64, u64>,
    pool_w: BTreeMap<u64, String>,
    pool_m: BTreeMap<u64, String>,
}

/// One pair's Windows workers of each pool given w2m workers of the same pool, the most agreeing first and, on a tie,
/// the thread map's partner, then the w2m worker that appears first; workers taken in the order they first appear
/// (reviewer-3's Low 1: the w2m workers were tried in thread-id order, where `call_compare.py` tries first appearance).
fn pool_partners(
    theirs: &BTreeMap<u64, Vec<&Call>>,
    ours: &BTreeMap<u64, Vec<&Call>>,
    threads: &Threads,
    sigs: &BTreeMap<String, Signature>,
) -> BTreeMap<u64, u64> {
    let mut chosen = threads.map.clone();
    let pool_names: BTreeSet<&String> = threads.pool_w.values().collect();
    for pool in pool_names {
        let mut free: Vec<u64> = ours
            .keys()
            .filter(|t| threads.pool_m.get(t) == Some(pool))
            .copied()
            .collect();
        free.sort_by_key(|t| ours[t][0].seq);
        let mut workers: Vec<u64> = theirs
            .keys()
            .filter(|t| threads.pool_w.get(t) == Some(pool))
            .copied()
            .collect();
        workers.sort_by_key(|t| theirs[t][0].seq);
        for tw in workers {
            if free.is_empty() {
                break;
            }
            let mut best = 0;
            let mut best_score = (0usize, false);
            for (i, tm) in free.iter().enumerate() {
                let s = (
                    agreeing(&theirs[&tw], &ours[tm], sigs),
                    threads.map.get(&tw) == Some(tm),
                );
                if i == 0 || s > best_score {
                    best_score = s;
                    best = i;
                }
            }
            chosen.insert(tw, free.remove(best));
        }
    }
    chosen
}

/// Which calls meet: the pairs of calls compared value by value, `(place, Windows call, w2m call)` in Windows seq
/// order, and the presence divergences, a pair one side never calls and a Windows thread's calls past its partner's.
fn paired<'a>(
    w: &ByPair<'a>,
    m: &ByPair<'a>,
    threads: &Threads,
    sigs: &BTreeMap<String, Signature>,
) -> (Vec<(usize, &'a Call, &'a Call)>, Vec<Divergence>) {
    let mut found = Vec::new();
    let mut pairs = Vec::new();
    for (pair, by_thread) in w.iter().filter(|(pair, _)| !m.contains_key(*pair)) {
        for (tid, calls) in by_thread {
            for (k, c) in calls.iter().enumerate() {
                found.push(missing(c, &pair.1, k, threads.map.get(tid).copied()));
            }
        }
    }
    for (pair, ours) in m {
        let Some(theirs) = w.get(pair) else {
            let first = ours.values().flatten().min().unwrap();
            found.push(Divergence {
                image: first.image.clone(),
                function: pair.1.clone(),
                index: 0,
                field: Field::Called,
                windows: None,
                w2m: Some(first.ret),
                windows_seq: None,
                w2m_seq: Some(first.seq),
                w2m_thread: Some(first.tid),
            });
            continue;
        };
        let chosen = pool_partners(theirs, ours, threads, sigs);
        for (tid, calls) in theirs {
            let mine = chosen.get(tid).and_then(|t| ours.get(t)).map_or(&[][..], Vec::as_slice);
            for (k, (a, b)) in calls.iter().zip(mine).enumerate() {
                pairs.push((k, *a, *b));
            }
            for (k, c) in calls.iter().enumerate().skip(mine.len()) {
                found.push(missing(c, &pair.1, k, chosen.get(tid).copied()));
            }
        }
    }
    pairs.sort_by_key(|p| p.1.seq);
    (pairs, found)
}

/// Per space of assigned values, the Windows-to-w2m map and its inverse: an id maps one to one across the stream; a
/// thread id maps by the thread map, or within its pool; a clock agrees always; an atom above 0x10000 by nullness.
struct OneToOne<'a> {
    threads: &'a Threads,
    maps: BTreeMap<Space, Bijection>,
}

/// A map and its inverse.
type Bijection = (BTreeMap<u64, u64>, BTreeMap<u64, u64>);

impl OneToOne<'_> {
    fn same(&mut self, space: Space, x: u64, y: u64) -> bool {
        match space {
            Space::Clock => true,
            Space::Thread => {
                let (x, y) = (x & 0xFFFF_FFFF, y & 0xFFFF_FFFF);
                match self.threads.pool_w.get(&x) {
                    Some(pool) => self.threads.pool_m.get(&y) == Some(pool),
                    None => self.threads.map.get(&x) == Some(&y),
                }
            }
            Space::Atom if x >= 0x10000 || y >= 0x10000 => (x == 0) == (y == 0),
            _ => {
                let (x, y) = (x & 0xFFFF_FFFF, y & 0xFFFF_FFFF);
                let (forward, backward) = self.maps.entry(space).or_default();
                *forward.entry(x).or_insert(y) == y && *backward.entry(y).or_insert(x) == x
            }
        }
    }
}

/// The value divergences of the pairs, in their order: each field by the relation its kind allows, an assigned value
/// by its space's map, an integer that carries an address as a pointer; a (function, field) the contract leaves
/// `undetermined` not judged, one scoped to a thread name (from contract 0.2.0) only where the Windows call's thread and
/// the w2m call's thread both carry that name (`names`, each side's SetThreadDescription names).
fn value_divergences(
    pairs: &[(usize, &Call, &Call)],
    threads: &Threads,
    sigs: &BTreeMap<String, Signature>,
    undecided: &Undecided,
    names: (&BTreeMap<u64, String>, &BTreeMap<u64, String>),
) -> Vec<Divergence> {
    let mut ids = OneToOne {
        threads,
        maps: BTreeMap::new(),
    };
    let mut found = Vec::new();
    for (k, a, b) in pairs {
        let fn_ = &a.function;
        let fields = match sigs.get(fn_) {
            None => vec![FieldPair {
                field: Field::Ret,
                kind: Kind::Value,
                bytes: Some(8),
                windows: a.ret,
                w2m: b.ret,
                argument: false,
            }],
            Some(sig) => field_pairs(sig, a, b, true),
        };
        for v in fields {
            let name = v.field.to_string();
            let scoped = format!("{}!{fn_}", a.image);
            let undetermined = undecided.iter().any(|(key, field, thread)| {
                (*key == *fn_ || *key == scoped)
                    && *field == name
                    && thread
                        .as_ref()
                        .is_none_or(|t| names.0.get(&a.tid) == Some(t) && names.1.get(&b.tid) == Some(t))
            });
            if undetermined {
                continue;
            }
            let mut space = assigned(fn_, v.field);
            if MESSAGE_FUNCTIONS.contains(&fn_.as_str())
                && (v.field == Field::Arg(0)
                    || v.field == Field::Arg(2) && WINDOW_WPARAM.contains(&(a.args[1] & 0xFFFF_FFFF)))
            {
                space = Some(Space::Window);
            }
            let ok = if let Some(space) = space {
                ids.same(space, v.windows, v.w2m)
            } else if carries_address(fn_, v.field, &a.args) {
                judge(Kind::Pointer, None, v.windows, v.w2m, v.argument)
            } else {
                judge(v.kind, v.bytes, v.windows, v.w2m, v.argument)
            };
            if !ok {
                found.push(Divergence {
                    image: a.image.clone(),
                    function: fn_.clone(),
                    index: *k,
                    field: v.field,
                    windows: Some(v.windows),
                    w2m: Some(v.w2m),
                    windows_seq: Some(a.seq),
                    w2m_seq: Some(b.seq),
                    w2m_thread: Some(b.tid),
                });
            }
        }
    }
    found
}

/// `judged`: every divergence of `w2m` against one Windows run, in the order of the Windows calls, and the calls
/// compared. `excluded` are the run's threads the game does not start (the capture's own); `undecided` the (function,
/// field) values two Windows runs disagree on (the contract's `undetermined`).
fn judged(
    windows: &[Call],
    w2m: &[Call],
    sigs: &BTreeMap<String, Signature>,
    excluded: &BTreeSet<u64>,
    strings: (&[Str], &[Str]),
    undecided: &Undecided,
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
    let threads = Threads {
        map: thread_map(&windows, &w2m, sigs, (&names.0, &names.1)),
        pool_w: pools(&names.0),
        pool_m: pools(&names.1),
    };
    let (w, m) = (by_pair(&windows), by_pair(&w2m));
    let (pairs, mut found) = paired(&w, &m, &threads, sigs);
    found.extend(value_divergences(
        &pairs,
        &threads,
        sigs,
        undecided,
        (&names.0, &names.1),
    ));
    found.sort_by(|a, b| a.place().cmp(&b.place()));
    let compared = pairs
        .iter()
        .map(|(k, a, b)| (a.image.to_lowercase(), a.function.clone(), b.tid, *k))
        .collect();
    (found, compared)
}

/// One Windows run of the reference set.
struct Reference {
    run: String,
    calls: Vec<Call>,
    excluded: BTreeSet<u64>,
    strings: Vec<Str>,
}

/// Why a presence divergence is what `by_cause` calls it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cause {
    /// a Windows thread's calls with no w2m thread paired to it
    NoPartner,
    /// a presence divergence on a thread whose earlier value diverged, at that value's Windows seq
    AfterValue(Option<u64>),
    /// a presence divergence on a thread whose earlier calls all conform: the game's own scheduling
    Scheduling,
}

impl std::fmt::Display for Cause {
    /// `call_compare.py`'s words.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cause::NoPartner => f.write_str("a Windows thread with no w2m partner"),
            Cause::AfterValue(Some(seq)) => write!(f, "after value divergence {seq}"),
            Cause::AfterValue(None) => f.write_str("after value divergence None"),
            Cause::Scheduling => f.write_str("scheduling"),
        }
    }
}

/// One w2m value judged against every run of the set (`SetDivergence`).
#[derive(Debug, Clone)]
pub struct SetDivergence {
    divergence: Divergence,
    run: String,
    disagree: Vec<String>,
    agree: Vec<String>,
    of: usize,
    cause: Option<Cause>,
}

impl SetDivergence {
    fn varies(&self) -> bool {
        !self.agree.is_empty()
    }

    /// The w2m thread the divergence is on, None for a Windows thread with no w2m partner (Sprint 368: the two-vCPU
    /// runs' divergences sorted with their thread)
    pub fn w2m_thread(&self) -> Option<u64> {
        self.divergence.w2m_thread
    }

    /// Whether the divergence is a value's (an argument, the return or the last error), not a call's presence or
    /// count (Sprint 368; the user's Decision of 2026-10-13)
    pub fn is_value(&self) -> bool {
        !self.divergence.field.presence()
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
        if let Some(cause) = self.cause {
            t += &format!(" [{cause}]");
        }
        t
    }
}

/// `compare_set`: w2m's calls against every run of the set: the divergences, the values Windows varies, and the calls
/// compared per run.
fn compare_set(
    references: &[Reference],
    w2m: &[Call],
    sigs: &BTreeMap<String, Signature>,
    strings: &[Str],
    undecided: &Undecided,
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
        if key.4.presence() {
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
                cause: None,
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
fn by_cause(found: Vec<SetDivergence>) -> (Vec<SetDivergence>, Vec<SetDivergence>) {
    let values: Vec<SetDivergence> = found
        .iter()
        .filter(|s| !s.divergence.field.presence())
        .cloned()
        .collect();
    let (mut defects, mut scheduling) = (Vec::new(), Vec::new());
    for s in found {
        let d = &s.divergence;
        if !d.field.presence() {
            defects.push(s);
        } else if d.w2m_thread.is_none() {
            defects.push(SetDivergence {
                cause: Some(Cause::NoPartner),
                ..s
            });
        } else {
            let before = values.iter().find(|v| {
                v.divergence.w2m_thread == d.w2m_thread
                    && (d.field == Field::Called
                        || (v.run == s.run && v.divergence.windows_seq.unwrap_or(0) < d.windows_seq.unwrap_or(0)))
            });
            match before {
                Some(v) => defects.push(SetDivergence {
                    cause: Some(Cause::AfterValue(v.divergence.windows_seq)),
                    ..s
                }),
                None => scheduling.push(SetDivergence {
                    cause: Some(Cause::Scheduling),
                    ..s
                }),
            }
        }
    }
    (defects, scheduling)
}

/// A run's verdict against the set: its defects, the divergences past a stopped run's cut, the game's own scheduling,
/// the values Windows varies, the calls compared per Windows run, and the calls the trace left out.
pub struct Verdict {
    defects: Vec<SetDivergence>,
    past_cut: Vec<SetDivergence>,
    scheduling: Vec<SetDivergence>,
    varies: Vec<SetDivergence>,
    compared: BTreeMap<String, usize>,
    dropped: u64,
}

/// The lists are read, never set, outside the crate, so a verdict's `verdict()` always agrees with them (reviewer-3's
/// Low 2 of its review of Sprints 215-217).
impl Verdict {
    pub fn defects(&self) -> &[SetDivergence] {
        &self.defects
    }

    pub fn past_cut(&self) -> &[SetDivergence] {
        &self.past_cut
    }

    pub fn scheduling(&self) -> &[SetDivergence] {
        &self.scheduling
    }

    pub fn varies(&self) -> &[SetDivergence] {
        &self.varies
    }

    /// The pairs of calls compared, per Windows run of the set.
    pub fn compared(&self) -> &BTreeMap<String, usize> {
        &self.compared
    }
}

impl Verdict {
    /// COMPARISON_COMPLETED's verdict, as `run_frontier.py`'s comparator gives it: "discrepancy" for a defect;
    /// "inconclusive" when the trace left calls out, since a trace that left calls out cannot show that w2m's calls
    /// conform (Sprint 064); else "conforms". Sprint 214's port left the second out (Sprint 217).
    pub fn verdict(&self) -> events::Verdict {
        if !self.defects.is_empty() {
            events::Verdict::Discrepancy
        } else if self.dropped > 0 {
            events::Verdict::Inconclusive
        } else {
            events::Verdict::Conforms
        }
    }

    /// SESSION_COMPLETED's `outcome` for the session this comparison closes: "completed_with_discrepancies" when the
    /// verdict is a discrepancy, as `run_frontier.py` 436 gives it and vocabulary 0.12's `session_outcome` holds it;
    /// else "completed" (reviewer-3's Medium 1 of its review of Sprints 215-217: Sprint 214 wrote "completed" always).
    pub fn outcome(&self) -> events::SessionOutcome {
        match self.verdict() {
            events::Verdict::Discrepancy => events::SessionOutcome::CompletedWithDiscrepancies,
            _ => events::SessionOutcome::Completed,
        }
    }

    /// COMPARISON_COMPLETED's `first_divergence`: the first defect's text, to 500 characters, or the calls left out.
    pub fn first_divergence(&self) -> Option<String> {
        match self.defects.first() {
            Some(d) => Some(d.text().chars().take(500).collect()),
            None => (self.dropped > 0).then(|| format!("the trace left out {} calls", self.dropped)),
        }
    }
}

/// A w2m call stream as `w2m-vm` writes it: the trace's text and its strings' text (`write_tsv`'s and
/// `<capture>.strings.tsv`'s), whether the run stopped before ExitProcess, and the calls the full trace left out.
pub struct Stream<'a> {
    pub calls: &'a str,
    pub strings: &'a str,
    pub stopped: bool,
    pub dropped: u64,
}

/// `calls` against the set, its divergences by cause (`by_cause`). A run `stopped` before ExitProcess has not started
/// every thread Windows started: the calls of a Windows thread with no w2m partner lie past the cut, not defects
/// (Sprint 214: all 76 such at the Rust host's row 3131, as at the frozen host's same rows); every other divergence of a
/// stopped run is a defect. The frozen host compared whole runs and had no cut.
fn verdict(
    references: &[Reference],
    calls: &[Call],
    sigs: &BTreeMap<String, Signature>,
    strings: &[Str],
    undecided: &Undecided,
    stopped: bool,
) -> Verdict {
    let (found, varies, compared) = compare_set(references, calls, sigs, strings, undecided);
    let (defects, scheduling) = by_cause(found);
    let (past_cut, defects): (Vec<_>, Vec<_>) = defects
        .into_iter()
        .partition(|d| stopped && d.cause == Some(Cause::NoPartner));
    Verdict {
        defects,
        past_cut,
        scheduling,
        varies,
        compared,
        dropped: 0,
    }
}

/// The game run whose log and main thread end the batch run (`run_frontier.py` 74, `REFERENCE`)
const ENDING_RUN: &str = "store/game-runs/38216081440";

/// The Windows main thread's calls in the run the batch run ends by (`run_frontier.py` 289-292): the calls of the
/// thread that makes the run's first call, the capture's own thread left out by the contract's `excluded_threads`,
/// counted by (calling image in lower case, function), in that order. Python takes the thread as
/// `min(reference_calls)[1]`, the first call by tuple order, whose first element is the seq; this names the rule
/// (Sprint 219; reviewer-3's Concern 5 of its review of Sprint 218).
pub fn main_thread_calls(root: &Path) -> Result<Vec<(String, String, u64)>, String> {
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let contract: Value = serde_json::from_str(&read(&root.join(CONTRACT))?).map_err(|e| format!("{CONTRACT}: {e}"))?;
    let run = load_reference(&root.join(ENDING_RUN), &contract)?;
    let calls: Vec<&Call> = run.calls.iter().filter(|c| !run.excluded.contains(&c.tid)).collect();
    let main = calls
        .iter()
        .min_by_key(|c| c.seq)
        .ok_or("the ending run traces no call")?
        .tid;
    let mut counts: Vec<(String, String, u64)> = Vec::new();
    for c in calls.iter().filter(|c| c.tid == main) {
        let key = (c.image.to_lowercase(), c.function.clone());
        match counts.iter_mut().find(|(i, f, _)| (i, f) == (&key.0, &key.1)) {
            Some(row) => row.2 += 1,
            None => counts.push((key.0, key.1, 1)),
        }
    }
    counts.sort();
    Ok(counts)
}

/// The frozen host's run of 2026-10-21, whose record names the reference set (`reference_runs`)
const REFERENCE_RECORD: &str = "store/runs/frontier-643a6283510c-1792623423346201000/events-000001.open.jsonl";
/// The contract the calls are judged under
/// (0.2.0 from Sprint 368: an entry scoped to a thread; 0.1.0 kept as it was)
pub const CONTRACT: &str = "contracts/windows.game.calls/0.2.0.json";

/// The Windows side of the comparison: the reference set's runs under store/game-runs, specs/signatures.json and the
/// contract, read from the repository once.
pub struct Windows {
    references: Vec<Reference>,
    sigs: BTreeMap<String, Signature>,
    undecided: Undecided,
    pub contract_id: String,
    pub contract_version: String,
}

impl Windows {
    pub fn load(root: &Path) -> Result<Windows, String> {
        let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
        let contract: Value =
            serde_json::from_str(&read(&root.join(CONTRACT))?).map_err(|e| format!("{CONTRACT}: {e}"))?;
        let field = |name: &str| {
            contract[name]
                .as_str()
                .map(str::to_string)
                .ok_or(format!("{CONTRACT}: no {name}"))
        };
        let references = reference_set(root, &contract)?
            .members
            .iter()
            .map(|r| load_reference(&root.join(GAME_RUNS).join(r), &contract))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Windows {
            references,
            sigs: signatures(&read(&root.join("specs/signatures.json"))?)?,
            undecided: contract_rules(&contract, "").1,
            contract_id: field("contract_id")?,
            contract_version: field("version")?,
        })
    }

    /// A stream's verdict; a run stopped before ExitProcess has its cut set aside (`verdict`).
    pub fn judge(&self, stream: &Stream) -> Result<Verdict, String> {
        Ok(Verdict {
            dropped: stream.dropped,
            ..verdict(
                &self.references,
                &read_tsv(stream.calls)?,
                &self.sigs,
                &read_strings(stream.strings),
                &self.undecided,
                stream.stopped,
            )
        })
    }

    /// COMPARISON_COMPLETED.reference_ref for the set: its executions joined by " + " (`reference_ref`).
    pub fn reference_ref(&self) -> String {
        self.references
            .iter()
            .map(|r| format!("game-run-{}", r.run))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

/// The reference set the frozen host's run of 2026-10-21 names, its last choice, which the set computed by the rules
/// (`reference_set`) is held to (Sprint 375).
pub fn frozen_reference_set(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(REFERENCE_RECORD);
    reference_runs(&std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// The reference set a frozen-host record names: its RunStarted baseline's `reference_set` (`run_frontier.py`), the runs
/// `call_compare.py`'s `reference_set` chose. From Sprint 214 to 374 this was the set the batch run was judged against,
/// the membership rules not ported; from Sprint 375 the rules choose it (`reference_set`), and this names the frozen
/// host's choice they are held to.
fn reference_runs(record_first_segment: &str) -> Result<Vec<String>, String> {
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

/// The contract's `undetermined` values: (function or `Image!Function`, field, the thread name it is scoped to, if any).
type Undecided = BTreeSet<(String, String, Option<String>)>;

/// One `undetermined` key: `Function field`, `Image!Function field`, or from contract 0.2.0 (Sprint 368)
/// `Image!Function@Thread field`, the field the key's last word and the thread the name after `@`, which may hold spaces
/// ("Job.Worker 0").
fn undetermined_key(key: &str) -> Option<(String, String, Option<String>)> {
    let (function, field) = key.rsplit_once(' ')?;
    Some(match function.split_once('@') {
        Some((function, thread)) => (function.to_string(), field.to_string(), Some(thread.to_string())),
        None => (function.to_string(), field.to_string(), None),
    })
}

/// The contract's `excluded_threads` for a run, and its `undetermined` values.
fn contract_rules(contract: &Value, run: &str) -> (BTreeSet<u64>, Undecided) {
    let excluded = contract["excluded_threads"][run]
        .as_object()
        .map_or(BTreeSet::new(), |o| o.keys().filter_map(|k| k.parse().ok()).collect());
    let undetermined = contract["undetermined"].as_object().map_or(BTreeSet::new(), |o| {
        o.keys().filter_map(|k| undetermined_key(k)).collect()
    });
    (excluded, undetermined)
}

/// A run of the set from its directory: its trace and strings; a laptop run's capture thread is the trace's first
/// call's (`capture_thread`), a runner run's the contract's (`load_reference`).
fn load_reference(dir: &Path, contract: &Value) -> Result<Reference, String> {
    let read =
        |name: &str| std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{}: {e}", dir.join(name).display()));
    let calls = read_tsv(&read("jit_capture.bin.trace.tsv")?)?;
    let strings = read_strings(&read("jit_capture.bin.strings.tsv").unwrap_or_default());
    let run = name_of(dir);
    let excluded = if laptop_run(dir) {
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

fn name_of(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

// The reference set by its rules (Sprint 375): `call_compare.py` 492-609 at the tag frozen-python-2026-10-29,
// `GAME`, `GAME_RUNS`, `RECORDS`, `GAME_ARGUMENTS`, `arguments`, `laptop_run`, `game_build`, `outside_set` and
// `reference_set`, rule for rule; and `scripts/record_game_run.py`, whose record `outside_set` reads.

/// The game the set is of, and its executable (`GAME`, `GAME_EXE`)
const GAME: &str = "hail-to-the-rainbow";
const GAME_EXE: &str = "Hail to the Rainbow.exe";
/// The stored game runs, and the records (`GAME_RUNS`, `RECORDS`)
pub const GAME_RUNS: &str = "store/game-runs";
pub const RECORDS: &str = "store/runs";
const TRACE: &str = "jit_capture.bin.trace.tsv";
const STRINGS: &str = "jit_capture.bin.strings.tsv";

/// `GAME_ARGUMENTS`: the game's arguments in the capture the set is of, windows-game.yml's batch mode, as the
/// contract's applicability states them and a laptop run's exec.json records them.
const GAME_ARGUMENTS: [&str; 4] = ["-batchmode", "-nographics", "-logFile", "C:\\w2m\\unity.log"];

/// The processors w2m reports to the game (`w2m.vm_backend.PROCESSORS`, 4), the Rust host's own constant, one
/// definition of the fact (`w2m_vm/tests/layout_constants.rs`): a laptop run's machine must have reported as many, since
/// the game sizes its job workers by them.
pub const PROCESSORS: u64 = w2m_vm::process::PROCESSORS as u64;

/// `laptop_run`: a laptop game run, its profile.txt starting "laptop-" (the runners' start "gha-").
fn laptop_run(run: &Path) -> bool {
    std::fs::read_to_string(run.join("profile.txt")).is_ok_and(|p| p.starts_with("laptop-"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `game_build`: the game build the set is of, the executable's digest in games/<game>/files.json.
pub fn game_build(root: &Path) -> Result<String, String> {
    Ok(format!("sha256:{}", exe_sha256(root)?))
}

fn exe_sha256(root: &Path) -> Result<String, String> {
    let path = root.join("games").join(GAME).join("files.json");
    read_json(&path)?["listing"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|f| f["path"] == GAME_EXE)
        .and_then(|f| f["sha256"].as_str())
        .map(str::to_string)
        .ok_or(format!("{}: no {GAME_EXE} in its listing", path.display()))
}

/// `shlex.split(given, posix=False)`: words split at whitespace, a word that opens with a quote running to its closing
/// quote, the quotes kept; a quote inside a word is a character of it.
fn split_command_line(given: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut chars = given.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut word = String::new();
        if c == '"' || c == '\'' {
            word.push(c);
            chars.next();
            loop {
                match chars.next() {
                    Some(x) if x == c => {
                        word.push(x);
                        break;
                    }
                    Some(x) => word.push(x),
                    None => return Err(format!("{given:?}: no closing quotation")),
                }
            }
        } else {
            while let Some(&x) = chars.peek() {
                if x.is_whitespace() {
                    break;
                }
                word.push(x);
                chars.next();
            }
        }
        words.push(word);
    }
    Ok(words)
}

/// `arguments`: a command line's arguments, each without its quotes (a Windows path keeps its backslashes); the
/// laptop's exec.json records them as a list (run-job.ps1), a command line as one string.
fn arguments(given: &Value) -> Result<Vec<String>, String> {
    let parts = match given {
        Value::Array(list) => list
            .iter()
            .map(|a| {
                a.as_str()
                    .map(str::to_string)
                    .ok_or(format!("an argument that is not a string: {a}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::String(line) => split_command_line(line)?,
        other => return Err(format!("arguments that are neither a list nor a command line: {other}")),
    };
    Ok(parts.iter().map(|a| a.trim_matches('"').to_string()).collect())
}

/// A JSON value as Python's `str` shows it in an f-string: a string as itself, a list of the items' `repr`s.
fn python_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => python_repr(other),
    }
}

/// Python's `repr` of a JSON value: a string in single quotes, or double when it holds a single quote and no double,
/// with `\\`, the quote and control characters escaped; a list as `[a, b]`; None, True, False.
fn python_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let quote = if s.contains('\'') && !s.contains('"') {
                '"'
            } else {
                '\''
            };
            let mut out = String::from(quote);
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c == quote => {
                        out.push('\\');
                        out.push(c);
                    }
                    c if (c as u32) < 0x20 || (0x7f..0xa0).contains(&(c as u32)) => {
                        out.push_str(&format!("\\x{:02x}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push(quote);
            out
        }
        Value::Array(items) => format!("[{}]", items.iter().map(python_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(_) => v.to_string(),
    }
}

/// A record's envelopes in seq order, as substrate's `read_record` yields them: the sealed segments
/// (`events-NNNNNN.jsonl`) by name, then the hot segment (`events-NNNNNN.open.jsonl`) up to its first line that is
/// not a whole JSON line (a torn tail); a seq that does not follow refuses the record, as substrate's reader and
/// `check_records` refuse it. A frame's crc is not checked here, as no Rust reader of the project checks it; the gates'
/// `substrate replay --level 2` does over each new record.
fn read_record(record: &Path) -> Result<Vec<Value>, String> {
    let mut segments: Vec<std::path::PathBuf> = std::fs::read_dir(record)
        .map_err(|e| format!("{}: {e}", record.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events-") && n.ends_with(".jsonl"))
        })
        .collect();
    segments.sort_by_key(|p| (p.to_string_lossy().ends_with(".open.jsonl"), p.clone()));
    let mut out = Vec::new();
    for segment in &segments {
        let hot = segment.to_string_lossy().ends_with(".open.jsonl");
        let text = std::fs::read_to_string(segment).map_err(|e| format!("{}: {e}", segment.display()))?;
        let whole = text.len() - text.rsplit_once('\n').map_or(text.len(), |(_, tail)| tail.len());
        if !hot && whole < text.len() {
            return Err(format!("{}: a sealed segment with a torn tail", segment.display()));
        }
        for line in text[..whole].lines() {
            let frame: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) if hot => break,
                Err(e) => return Err(format!("{}: {e}", segment.display())),
            };
            if frame["seq"].as_u64() != Some(out.len() as u64) {
                return Err(format!(
                    "{}: seq {} where {} follows",
                    record.display(),
                    frame["seq"],
                    out.len()
                ));
            }
            out.push(frame);
        }
    }
    Ok(out)
}

/// `outside_set`: why a stored game run is not in the reference set, or None when it is. A run is in the set when
/// its capture names the game's threads (SetThreadDescription calls with their names in the strings file); when its game
/// arguments are `GAME_ARGUMENTS`, a laptop run's by its exec.json, a runner run's, which records none, by the contract
/// naming its capture thread; when a laptop run's machine reported w2m's processor count (profile.json); and when it is
/// on the record as a windows_reference execution of the same game build (store/runs/game-run-<run>, written by
/// `record_game_run`: EXECUTION_STARTED.subject_sha256 is files.json's digest of the executable).
fn outside_set(run: &Path, build: &str, contract: &Value, records: &Path) -> Result<Option<String>, String> {
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let (trace, names) = (run.join(TRACE), run.join(STRINGS));
    if !(trace.is_file() && names.is_file()) {
        return Ok(Some("no call trace with thread names".into()));
    }
    if thread_names(&read_tsv(&read(&trace)?)?, &read_strings(&read(&names)?)).is_empty() {
        return Ok(Some(
            "the capture names no threads (SetThreadDescription without names)".into(),
        ));
    }
    let name = name_of(run);
    if laptop_run(run) {
        let ended = run.join("exec.json");
        let recorded = if ended.is_file() {
            read_json(&ended)?.get("arguments").cloned().filter(|a| !a.is_null())
        } else {
            None
        };
        let Some(recorded) = recorded else {
            return Ok(Some("a laptop run whose exec.json records no arguments".into()));
        };
        if arguments(&recorded)? != GAME_ARGUMENTS {
            return Ok(Some(format!("other arguments ({})", python_text(&recorded))));
        }
        let profile = run.join("profile.json");
        let logical = if profile.is_file() {
            read_json(&profile)?
                .get("cpu")
                .and_then(|c| c.get("logical"))
                .cloned()
                .ok_or(format!("{}: no cpu.logical", profile.display()))?
        } else {
            Value::Null
        };
        if logical.as_f64() != Some(PROCESSORS as f64) {
            return Ok(Some(format!(
                "a machine of {} processors, w2m reports {PROCESSORS}",
                python_text(&logical)
            )));
        }
    } else if contract["excluded_threads"].get(&name).is_none() {
        return Ok(Some(
            "the contract names no capture thread for it (another capture or arguments)".into(),
        ));
    }
    let record = records.join(format!("game-run-{name}"));
    if !record.is_dir() {
        return Ok(Some(format!("not on the record (game-run-{name})")));
    }
    let started: Vec<Value> = read_record(&record)?
        .into_iter()
        .filter(|e| e["kind"] == "EXECUTION_STARTED")
        .map(|e| e["payload"].clone())
        .collect();
    let field = |key: &str| {
        started[0]
            .get(key)
            .cloned()
            .ok_or(format!("{}: EXECUTION_STARTED with no {key}", record.display()))
    };
    if started.is_empty() || field("profile_kind")? != "windows_reference" {
        return Ok(Some("no windows_reference EXECUTION_STARTED on the record".into()));
    }
    let subject = field("subject_sha256")?;
    if subject != build {
        return Ok(Some(format!("another game build ({})", python_text(&subject))));
    }
    Ok(None)
}

/// The stored game runs in the reference set, in name order, and each run left out with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSet {
    pub members: Vec<String>,
    pub outside: BTreeMap<String, String>,
}

/// `reference_set`: every directory under store/game-runs judged by `outside_set` against the game build, in name
/// order, the records read from store/runs.
fn reference_set(root: &Path, contract: &Value) -> Result<ReferenceSet, String> {
    let build = game_build(root)?;
    let dir = root.join(GAME_RUNS);
    let mut runs: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    runs.sort();
    let mut set = ReferenceSet {
        members: Vec::new(),
        outside: BTreeMap::new(),
    };
    for run in runs {
        match outside_set(&run, &build, contract, &root.join(RECORDS))? {
            None => set.members.push(name_of(&run)),
            Some(reason) => {
                set.outside.insert(name_of(&run), reason);
            }
        }
    }
    Ok(set)
}

/// The reference set the rules choose from the repository's stored game runs, under the contract `CONTRACT`.
pub fn reference_set_of(root: &Path) -> Result<ReferenceSet, String> {
    reference_set(root, &read_json(&root.join(CONTRACT))?)
}

/// The SHA-256 of nothing: what a game run's record gives as its initial state and injected inputs, which the capture
/// records neither of (`record_game_run.py`'s `EMPTY`).
const EMPTY: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// A stored game run's execution as `scripts/record_game_run.py` puts it on the record: EXECUTION_STARTED with the
/// game's executable, its digest in files.json, and the run's profile (profile.txt's first word), its timeout the run's
/// elapsed seconds rounded half to even as Python's `round` does; and EXECUTION_COMPLETED with the run's end
/// (exec.json), the call trace's digest and its non-empty lines.
pub fn game_run_execution(root: &Path, run: &str) -> Result<(ExecutionStarted, ExecutionCompleted), String> {
    let invalid = |e: events::InvalidValue| e.to_string();
    let dir = root.join(GAME_RUNS).join(run);
    let trace_path = format!("{GAME_RUNS}/{run}/{TRACE}");
    let trace = std::fs::read(root.join(&trace_path)).map_err(|e| format!("{trace_path}: {e}"))?;
    let lines = String::from_utf8_lossy(&trace)
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    let ended = read_json(&dir.join("exec.json"))?;
    let profile_path = dir.join("profile.txt");
    let profile = std::fs::read_to_string(&profile_path)
        .map_err(|e| format!("{}: {e}", profile_path.display()))?
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or(format!("{}: empty", profile_path.display()))?;
    let elapsed = ended["elapsed_seconds"]
        .as_f64()
        .ok_or(format!("{run}/exec.json: no elapsed_seconds"))?;
    let exited = ended["exited_by_itself"]
        .as_bool()
        .ok_or(format!("{run}/exec.json: no exited_by_itself"))?;
    let exit_code = match &ended["exit_code"] {
        Value::Null => None,
        code => {
            Some(HarnessI64::new(code.as_i64().ok_or(format!("{run}/exec.json: exit_code {code}"))?).map_err(invalid)?)
        }
    };
    let execution_id = Id::new(format!("game-run-{run}")).map_err(invalid)?;
    let empty = Sha256::new(EMPTY).map_err(invalid)?;
    let started = ExecutionStarted {
        execution_id: execution_id.clone(),
        subject_kind: SubjectKind::Application,
        subject_sha256: Sha256::new(game_build(root)?).map_err(invalid)?,
        profile_kind: ProfileKind::WindowsReference,
        profile_id: Id::new(profile).map_err(invalid)?,
        initial_state_sha256: empty.clone(),
        injected_inputs_sha256: empty,
        timeout_seconds: Some(FiniteF64::new(elapsed.round_ties_even()).map_err(invalid)?),
        kill_grace_seconds: None,
        unobserved: Some("initial state and injected inputs: the capture records neither".into()),
        vcpus: None,
    };
    let completed = ExecutionCompleted {
        execution_id,
        exit_status: if exited {
            ExitStatus::Exited
        } else {
            ExitStatus::TimedOut
        },
        trace_sha256: Sha256::new(format!("sha256:{}", guest_runtime::sha256::hex(&trace))).map_err(invalid)?,
        guest_line_count: HarnessU64::new(lines as u64).map_err(invalid)?,
        exit_code,
        elapsed_seconds: Some(FiniteF64::new(elapsed).map_err(invalid)?),
        unobserved: Some(format!(
            "the trace is {trace_path} in tools/jit_capture/call_trace.h's columns, not a vocabulary guest trace: the \
             capture does not record where threads start"
        )),
        guest_trace: None,
    };
    Ok((started, completed))
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
        assert!(judge(Kind::Value, Some(4), 0x1_0000_0005, 5, true));
        assert!(!judge(Kind::Value, Some(4), 5, 6, true));
        assert!(judge(Kind::Pointer, None, 0x1234_0000, 0x5678_0000, false));
        assert!(!judge(Kind::Pointer, None, 0, 0x5678_0000, false));
        assert!(
            !judge(Kind::Pointer, None, 0x20, 0x21, true),
            "an argument below 0x10000 equal"
        );
        assert!(judge(Kind::Handle, None, 0x44, 0x88, false));
        assert!(!judge(Kind::Handle, None, u64::MAX, u64::MAX - 1, false));
        assert!(failed(Kind::Handle, "HANDLE", u64::MAX) && failed(Kind::Pointer, "LPVOID", 0));
        assert!(failed(Kind::Value, "BOOL", 0x1_0000_0000) && !failed(Kind::Value, "DWORD", 0));
    }

    /// The session's outcome follows the verdict: a defect closes it "completed_with_discrepancies"; calls left out
    /// make the verdict inconclusive and the session "completed".
    #[test]
    fn the_session_closes_with_discrepancies_on_a_defect() {
        let windows = vec![call(1, 1, "GetLastError", 0, 5)];
        let (found, _) = judged(
            &windows,
            &[call(1, 4, "GetLastError", 0, 7)],
            &BTreeMap::new(),
            &BTreeSet::new(),
            (&[], &[]),
            &BTreeSet::new(),
        );
        let defect = SetDivergence {
            divergence: found[0].clone(),
            run: "1".into(),
            disagree: vec!["1".into()],
            agree: Vec::new(),
            of: 1,
            cause: None,
        };
        let verdict = |defects: Vec<SetDivergence>, dropped| Verdict {
            defects,
            past_cut: Vec::new(),
            scheduling: Vec::new(),
            varies: Vec::new(),
            compared: BTreeMap::new(),
            dropped,
        };
        let outcomes = [verdict(vec![defect], 0), verdict(Vec::new(), 3), verdict(Vec::new(), 0)]
            .map(|v| (v.verdict().as_str(), v.outcome().as_str()));
        assert_eq!(
            outcomes,
            [
                ("discrepancy", "completed_with_discrepancies"),
                ("inconclusive", "completed"),
                ("conforms", "completed")
            ]
        );
    }

    /// A value of a kind the file does not know, or of none, refuses the file, where it was read as a value.
    #[test]
    fn a_signature_of_an_unknown_kind_is_refused() {
        let known =
            r#"{"functions": {"Sleep": {"params": [{"kind": "value", "bytes": 4}], "return": {"kind": "void"}}}}"#;
        assert!(signatures(known).is_ok());
        let unknown = known.replace(r#""kind": "value""#, r#""kind": "integer""#);
        assert_eq!(
            signatures(&unknown).err().as_deref(),
            Some(r#"signatures: Sleep has a value of kind Some("integer")"#)
        );
        let none = known.replace(r#"{"kind": "void"}"#, "{}");
        assert!(signatures(&none).is_err(), "a return of no kind");
    }

    /// Two functions w2m alone calls, one name in two images, are named in the order w2m first calls them, not by
    /// image name.
    #[test]
    fn functions_only_w2m_calls_are_named_as_they_first_appear() {
        let windows = vec![call(1, 1, "GetLastError", 0, 5)];
        let mut late = call(2, 4, "Sleep", 0, 0);
        late.image = "a.dll".into();
        let mut early = call(1, 4, "Sleep", 0, 0);
        early.image = "z.dll".into();
        let w2m = vec![late, early, call(3, 4, "GetLastError", 0, 5)];
        let (found, _) = judged(
            &windows,
            &w2m,
            &BTreeMap::new(),
            &BTreeSet::new(),
            (&[], &[]),
            &BTreeSet::new(),
        );
        let named: Vec<String> = found.iter().map(|d| format!("{} {}", d.image, d.field)).collect();
        assert_eq!(named, ["z.dll called", "a.dll called"], "{found:?}");
    }

    fn names(pairs: &[(u64, &str)]) -> BTreeMap<u64, String> {
        pairs.iter().map(|(t, n)| (*t, n.to_string())).collect()
    }

    /// A pool is a name ending in a number that two threads or more share but for it; a lone numbered name, or a name
    /// with no number, is no pool.
    #[test]
    fn a_pool_is_a_numbered_name_two_threads_share() {
        let pools = pools(&names(&[
            (1, "Job.Worker 0"),
            (2, "Job.Worker 1"),
            (3, "Loading 0"),
            (4, "UnityMain"),
            (5, "Job.Worker x"),
        ]));
        assert_eq!(pools, names(&[(1, "Job.Worker"), (2, "Job.Worker")]));
    }

    /// Threads pair by name first, whatever the order they appear in; an unnamed thread then takes the free thread
    /// that agrees most, the first to appear on a tie.
    #[test]
    fn threads_pair_by_name_then_by_agreement() {
        let windows = vec![
            call(1, 10, "GetLastError", 0, 5),
            call(2, 11, "GetLastError", 0, 6),
            call(3, 12, "GetLastError", 0, 7),
        ];
        let w2m = vec![
            call(1, 20, "GetLastError", 0, 7),
            call(2, 21, "GetLastError", 0, 5),
            call(3, 22, "GetLastError", 0, 6),
        ];
        let sigs =
            signatures(r#"{"functions": {"GetLastError": {"params": [], "return": {"kind": "value", "bytes": 4}}}}"#)
                .unwrap();
        let map = thread_map(
            &windows,
            &w2m,
            &sigs,
            (&names(&[(11, "Audio")]), &names(&[(20, "Audio")])),
        );
        assert_eq!(map, BTreeMap::from([(10, 21), (11, 20), (12, 22)]));
        let tie = vec![call(4, 30, "Sleep", 0, 0), call(5, 31, "Sleep", 0, 0)];
        let map = thread_map(&windows[..1], &tie, &sigs, (&BTreeMap::new(), &BTreeMap::new()));
        assert_eq!(map, BTreeMap::from([(10, 30)]), "on a tie the first to appear");
    }

    /// A pool's workers pair with w2m's in the order each first appears, not by thread id.
    #[test]
    fn pool_workers_pair_as_they_first_appear() {
        let windows = vec![call(1, 10, "Sleep", 0, 0), call(2, 11, "Sleep", 0, 0)];
        let w2m = vec![call(1, 29, "Sleep", 0, 0), call(2, 28, "Sleep", 0, 0)];
        let (w, m) = (by_pair(&windows), by_pair(&w2m));
        let key = ("mono-2.0-bdwgc.dll".to_string(), "Sleep".to_string());
        let threads = Threads {
            map: BTreeMap::new(),
            pool_w: names(&[(10, "Job"), (11, "Job")]),
            pool_m: names(&[(28, "Job"), (29, "Job")]),
        };
        let chosen = pool_partners(&w[&key], &m[&key], &threads, &BTreeMap::new());
        assert_eq!(chosen, BTreeMap::from([(10, 29), (11, 28)]));
    }

    /// A reservation at a chosen address leaves the stream, and so does the retry of one that failed, with no address
    /// and the same size on the same thread; any other VirtualAlloc stays.
    #[test]
    fn a_placed_reservation_and_its_retry_leave_the_stream() {
        let alloc = |seq, tid, at, size, ret| Call {
            args: [at, size, MEM_RESERVE, 0, 0, 0, 0, 0],
            ..call(seq, tid, "VirtualAlloc", at, ret)
        };
        let calls = vec![
            alloc(1, 1, 0x7000_0000, 0x10000, 0),
            alloc(2, 1, 0, 0x10000, 0x1234_0000),
            alloc(3, 1, 0, 0x10000, 0x1235_0000),
            alloc(4, 2, 0x7100_0000, 0x10000, 0x7100_0000),
            alloc(5, 2, 0, 0x20000, 0x1236_0000),
        ];
        let kept: Vec<u64> = placed(&calls).iter().map(|c| c.seq).collect();
        assert_eq!(kept, [3, 5]);
    }

    /// An undetermined key is read as (function, field, thread): unscoped, by image, or scoped to a thread whose name
    /// holds a space (Sprint 368).
    #[test]
    fn an_undetermined_key_names_its_function_field_and_thread() {
        let s = |x: &str| x.to_string();
        assert_eq!(
            undetermined_key("TryEnterCriticalSection ret"),
            Some((s("TryEnterCriticalSection"), s("ret"), None))
        );
        assert_eq!(
            undetermined_key("UnityPlayer.dll!HeapReAlloc@BatchDeleteObjects a3"),
            Some((s("UnityPlayer.dll!HeapReAlloc"), s("a3"), Some(s("BatchDeleteObjects"))))
        );
        assert_eq!(
            undetermined_key("UnityPlayer.dll!Sleep@Job.Worker 0 a0"),
            Some((s("UnityPlayer.dll!Sleep"), s("a0"), Some(s("Job.Worker 0"))))
        );
        assert_eq!(undetermined_key("nospace"), None);
    }

    /// A command line splits as `shlex.split(posix=False)` splits it: a word that opens with a quote runs to the closing
    /// one, the quotes kept, and the next word starts after it; a quote inside a word is a character; an unclosed quote
    /// is refused. `arguments` then strips the quotes from each end.
    #[test]
    fn a_command_line_splits_as_shlex_without_posix() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            split_command_line(r#"-logFile "C:\w2m\a b.log"x it's"#).unwrap(),
            s(&["-logFile", r#""C:\w2m\a b.log""#, "x", "it's"])
        );
        assert!(split_command_line(r#"-logFile "C:\w2m"#).is_err());
        let given = Value::String(r#"-batchmode -nographics -logFile "C:\w2m\unity.log""#.into());
        assert_eq!(arguments(&given).unwrap(), GAME_ARGUMENTS);
        let listed = serde_json::json!(["-logFile", "\"\"C:\\w2m\""]);
        assert_eq!(arguments(&listed).unwrap(), s(&["-logFile", "C:\\w2m"]));
    }

    /// A recorded value in a reason reads as Python's f-string shows it: a list by its items' reprs, a string bare.
    #[test]
    fn a_reason_shows_a_value_as_python_does() {
        let v = serde_json::json!(["-logFile", "\"C:\\w2m\\unity.log\"", "it's", 4, null, true]);
        assert_eq!(
            python_text(&v),
            r#"['-logFile', '"C:\\w2m\\unity.log"', "it's", 4, None, True]"#
        );
        assert_eq!(python_text(&Value::String("-batchmode x".into())), "-batchmode x");
        assert_eq!(python_text(&Value::Null), "None");
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
