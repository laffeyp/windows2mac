// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 359's judge, in the form of `run.rs`' `the_word_is_released_only_by_its_guards_drop` (Sprint 338): in the five
//! files of the runtime C's locks (`heap.c`, `thread_storage.c`, `sync_objects.c`, `runtime.c`, `runtime.h`), every
//! take of the heap's, the slots', the handle table's and the sync objects' locks and of the C's own runtime word goes
//! through a guard, `W2M_HELD(lock)` (runtime.h), whose clang `cleanup` attribute lets the lock go when its scope ends,
//! and a release by hand is one of two named forms: `W2M_LET_GO_EARLY(lock)`, on a path that then calls
//! `w2m_unimplemented` or `w2m_logged` in the guard's scope, and `W2M_LET_GO_BEFORE_STOP(lock)`, in a function every
//! caller of which holds the lock, before `w2m_unimplemented`. A leaf lock is never held at a call of either, in the
//! function that takes it or in one it calls. The text is read with its comments, literals and preprocessor lines made
//! blank, so a comment naming a release is not one, and a function called while a guard is armed counts as holding its
//! lock, through every call it makes in the five files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The five files (the Decision "The runtime C's leaf locks released by a scoped guard", 2026-11-26).
const FILES: [&str; 5] = ["heap.c", "thread_storage.c", "sync_objects.c", "runtime.c", "runtime.h"];

/// The four leaf locks by their guards' names: held, none calls `w2m_unimplemented` or `w2m_logged`.
const LEAVES: [&str; 4] = ["heap", "slots", "handles", "sync"];

/// The calls that take a lock by hand: each leaf's take, the runtime word's take as its guard makes it and as
/// runtime.c makes it, and the timed take of runtime.h.
const TAKES: [&str; 8] = [
    "hold_heap",
    "hold_slots",
    "hold_handles",
    "hold_sync",
    "hold_runtime_word",
    "w2m_take_runtime_word",
    "claim_runtime_word",
    "w2m_lock_take",
];

/// The calls that let a lock go by hand; with them a store of 0 by `__atomic_store_n` with `__ATOMIC_RELEASE`.
const RELEASES: [&str; 7] = [
    "let_heap_go",
    "let_slots_go",
    "let_handles_go",
    "let_sync_go",
    "w2m_let_runtime_word_go",
    "let_word_go",
    "w2m_lock_let_go",
];

/// The functions that are a take, a release or a guard's end, whose bodies take and let go by hand.
const PRIMITIVES: [&str; 21] = [
    "hold_heap",
    "hold_slots",
    "hold_handles",
    "hold_sync",
    "hold_runtime_word",
    "w2m_take_runtime_word",
    "take_word",
    "w2m_lock_take",
    "let_heap_go",
    "let_slots_go",
    "let_handles_go",
    "let_sync_go",
    "w2m_let_runtime_word_go",
    "let_word_go",
    "w2m_lock_let_go",
    "heap_guard_end",
    "slots_guard_end",
    "handles_guard_end",
    "sync_guard_end",
    "runtime_word_guard_end",
    "claim_runtime_word",
];

/// A take or a release by hand outside the primitives, named by its file, its function and the call, with its reason.
const NAMED: [(&str, &str, &str, &str); 1] = [(
    "runtime.c",
    "w2m_unimplemented",
    "claim_runtime_word",
    "the run ends at the host request made next under the word, and the host lets the word go",
)];

/// The calls a leaf lock is never held at (heap.c, thread_storage.c, sync_objects.c: a leaf calls neither).
const NOT_UNDER_A_LEAF: [&str; 2] = ["w2m_unimplemented", "w2m_logged"];

/// Words followed by `(` that are not calls.
const KEYWORDS: [&str; 10] = [
    "if",
    "for",
    "while",
    "switch",
    "return",
    "sizeof",
    "_Static_assert",
    "__attribute__",
    "__asm__",
    "__builtin_offsetof",
];

/// `text` with its comments and its string and character literals made spaces, then its preprocessor lines (with
/// their continuations), newlines kept, so a token's line is its line in the file.
fn code_of(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let end = text[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            out.extend(b[i..end].iter().map(|&d| if d == b'\n' { b'\n' } else { b' ' }));
            i = end;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            out.push(quote);
            i += 1;
            while i < b.len() && b[i] != quote && b[i] != b'\n' {
                let skip = (if b[i] == b'\\' { 2 } else { 1 }).min(b.len() - i);
                out.extend(std::iter::repeat_n(b' ', skip));
                i += skip;
            }
            if i < b.len() && b[i] == quote {
                out.push(quote);
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    let code = String::from_utf8(out).unwrap_or_default();
    let mut lines = Vec::new();
    let mut directive = false;
    for line in code.split('\n') {
        directive = directive || line.trim_start().starts_with('#');
        if directive {
            lines.push(" ".repeat(line.len()));
            directive = line.trim_end().ends_with('\\');
        } else {
            lines.push(line.to_string());
        }
    }
    lines.join("\n")
}

/// A word or a mark of the code, at its line.
struct Token {
    text: String,
    line: usize,
}

fn tokens(code: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for (n, line) in code.split('\n').enumerate() {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let start = i;
            if b[i].is_ascii_alphanumeric() || b[i] == b'_' {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
            } else {
                i += 1;
                if b[start].is_ascii_whitespace() {
                    continue;
                }
            }
            out.push(Token {
                text: line[start..i].to_string(),
                line: n + 1,
            });
        }
    }
    out
}

fn is_word(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
}

/// The index of the brace that closes the one at `open`.
fn closing(t: &[Token], open: usize) -> usize {
    let mut depth = 0;
    for (i, tok) in t.iter().enumerate().skip(open) {
        match tok.text.as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    t.len() - 1
}

/// A function's definition: its name, whether it is static, and its body's braces.
struct Function {
    file: usize,
    name: String,
    is_static: bool,
    open: usize,
    close: usize,
}

/// The functions a file defines: a brace at depth 0 whose header since the last `;` or `}` holds a `(` and no `=`, the
/// name the word before the header's first `(`.
fn functions(file: usize, t: &[Token]) -> Vec<Function> {
    let mut out = Vec::new();
    let (mut i, mut header) = (0, 0);
    while i < t.len() {
        match t[i].text.as_str() {
            "{" => {
                let close = closing(t, i);
                let head = &t[header..i];
                let paren = head.iter().position(|x| x.text == "(");
                if let Some(p) = paren
                    && p > 0
                    && !head.iter().any(|x| x.text == "=")
                    && is_word(&head[p - 1].text)
                {
                    out.push(Function {
                        file,
                        name: head[p - 1].text.clone(),
                        is_static: head.iter().any(|x| x.text == "static"),
                        open: i,
                        close,
                    });
                    header = close + 1;
                }
                i = close + 1;
                continue;
            }
            ";" => header = i + 1,
            _ => {}
        }
        i += 1;
    }
    out
}

/// A block of a function's body: the guards it declares, the locks let go early in it, and its closing brace.
struct Frame {
    guards: Vec<String>,
    released: Vec<String>,
    close: usize,
}

/// The leaf locks held at a point: each one a guard in an enclosing block declares, or that the function holds on
/// entry (`entered`), and no early release since in that block or one inside it on the way to the point.
fn armed(stack: &[Frame], entered: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for lock in LEAVES {
        let from = match stack.iter().rposition(|f| f.guards.iter().any(|g| g == lock)) {
            Some(d) => d,
            None if entered.contains(lock) => 0,
            None => continue,
        };
        if !stack[from..].iter().any(|f| f.released.iter().any(|r| r == lock)) {
            out.insert(lock.to_string());
        }
    }
    out
}

/// A call in a function's body, its token and the leaf locks held at it.
struct Call {
    name: String,
    at: usize,
    armed: BTreeSet<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Early {
    /// `W2M_LET_GO_EARLY`
    InScope,
    /// `W2M_LET_GO_BEFORE_STOP`
    BeforeStop,
}

/// A named early release: its form, its lock, its token, the leaf locks held at it, its block's closing brace.
struct Site {
    form: Early,
    lock: String,
    at: usize,
    armed: BTreeSet<String>,
    close: usize,
}

/// A walk of `f`'s body holding `entered` on entry: its calls, its named early releases, and each `struct word`
/// declared with no guard (`WORD_WHEN_NEEDED`, sync_objects.c), by token.
fn walk(t: &[Token], f: &Function, entered: &BTreeSet<String>) -> (Vec<Call>, Vec<Site>, Vec<usize>) {
    let (mut calls, mut sites, mut bare) = (Vec::new(), Vec::new(), Vec::new());
    let mut stack = vec![Frame {
        guards: Vec::new(),
        released: Vec::new(),
        close: f.close,
    }];
    let mut i = f.open + 1;
    while i < f.close {
        let text = t[i].text.as_str();
        let next = |k: usize| t.get(i + k).map_or("", |x| x.text.as_str());
        let named_form = matches!(
            text,
            "W2M_HELD" | "W2M_LET_GO_EARLY" | "W2M_LET_GO_BEFORE_STOP" | "WORD_WHEN_NEEDED"
        ) && next(1) == "("
            && is_word(next(2))
            && next(3) == ")";
        if named_form {
            let lock = next(2).to_string();
            let form = match text {
                "W2M_LET_GO_EARLY" => Some(Early::InScope),
                "W2M_LET_GO_BEFORE_STOP" => Some(Early::BeforeStop),
                _ => None,
            };
            let top = stack.len() - 1;
            if let Some(form) = form {
                sites.push(Site {
                    form,
                    lock: lock.clone(),
                    at: i,
                    armed: armed(&stack, entered),
                    close: stack[top].close,
                });
                stack[top].released.push(lock);
            } else if text == "W2M_HELD" {
                stack[top].guards.push(lock);
            }
            i += 4;
            continue;
        }
        match text {
            "{" => stack.push(Frame {
                guards: Vec::new(),
                released: Vec::new(),
                close: closing(t, i),
            }),
            "}" => {
                stack.pop();
            }
            "struct" if next(1) == "word" && is_word(next(2)) => bare.push(i),
            _ if is_word(text) && next(1) == "(" && !KEYWORDS.contains(&text) => calls.push(Call {
                name: text.to_string(),
                at: i,
                armed: armed(&stack, entered),
            }),
            _ => {}
        }
        i += 1;
    }
    (calls, sites, bare)
}

/// Whether the call at `at` lets a lock go: one of `RELEASES`, or `__atomic_store_n` with `__ATOMIC_RELEASE`.
fn releases(t: &[Token], at: usize) -> bool {
    if RELEASES.contains(&t[at].text.as_str()) {
        return true;
    }
    if t[at].text != "__atomic_store_n" {
        return false;
    }
    let mut depth = 0;
    for tok in &t[at + 1..] {
        match tok.text.as_str() {
            "(" => depth += 1,
            ")" => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            "__ATOMIC_RELEASE" => return true,
            _ => {}
        }
    }
    false
}

/// The next call from token `from` up to `to`, by name.
fn next_call(t: &[Token], from: usize, to: usize) -> Option<&str> {
    (from..to.min(t.len() - 1))
        .find(|&k| is_word(&t[k].text) && t[k + 1].text == "(" && !KEYWORDS.contains(&t[k].text.as_str()))
        .map(|k| t[k].text.as_str())
}

/// What the five files' text breaks of the rules above, each as "file:line: what: the line", and each named early
/// release with the call that follows it in its block, "file:line: lock, before call".
fn judge(sources: &[(&str, &str)]) -> (Vec<String>, Vec<String>) {
    let toks: Vec<Vec<Token>> = sources.iter().map(|(_, text)| tokens(&code_of(text))).collect();
    let funcs: Vec<Function> = toks
        .iter()
        .enumerate()
        .flat_map(|(file, t)| functions(file, t))
        .collect();
    let header = sources.iter().position(|(name, _)| *name == "runtime.h");
    let resolve = |file: usize, name: &str| -> Option<usize> {
        let at = |pred: &dyn Fn(&Function) -> bool| funcs.iter().position(|g| g.name == name && pred(g));
        at(&|g| g.file == file)
            .or_else(|| at(&|g| Some(g.file) == header))
            .or_else(|| at(&|g| !g.is_static))
    };
    let full: BTreeSet<String> = LEAVES.iter().map(|l| l.to_string()).collect();
    // what each function may hold on entry: the union over its calls' armed locks, to a fixed point
    let mut may = vec![BTreeSet::new(); funcs.len()];
    loop {
        let mut changed = false;
        for (k, f) in funcs.iter().enumerate() {
            for c in walk(&toks[f.file], f, &may[k].clone()).0 {
                if let Some(g) = resolve(f.file, &c.name) {
                    for lock in c.armed {
                        changed |= may[g].insert(lock);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    // what each function must hold on entry: a static function's, the intersection over its calls, from all down
    let mut must: Vec<BTreeSet<String>> = funcs
        .iter()
        .map(|f| if f.is_static { full.clone() } else { BTreeSet::new() })
        .collect();
    loop {
        let mut met: Vec<Option<BTreeSet<String>>> = vec![None; funcs.len()];
        for (k, f) in funcs.iter().enumerate() {
            for c in walk(&toks[f.file], f, &must[k]).0 {
                if let Some(g) = resolve(f.file, &c.name) {
                    met[g] = Some(match met[g].take() {
                        None => c.armed,
                        Some(s) => s.intersection(&c.armed).cloned().collect(),
                    });
                }
            }
        }
        let next: Vec<BTreeSet<String>> = funcs
            .iter()
            .zip(met)
            .map(|(f, m)| {
                if f.is_static {
                    m.unwrap_or_default()
                } else {
                    BTreeSet::new()
                }
            })
            .collect();
        if next == must {
            break;
        }
        must = next;
    }
    let (mut found, mut early) = (Vec::new(), Vec::new());
    for (k, f) in funcs.iter().enumerate() {
        let (t, (file, text)) = (&toks[f.file], sources[f.file]);
        let line = |at: usize| {
            let n = t[at].line;
            format!("{file}:{n}: {}", text.lines().nth(n - 1).unwrap_or("").trim())
        };
        let primitive = PRIMITIVES.contains(&f.name.as_str());
        let named = |call: &str| {
            NAMED
                .iter()
                .any(|(nf, nfun, nc, _)| *nf == file && *nfun == f.name && *nc == call)
        };
        let (calls, sites, bare) = walk(t, f, &may[k]);
        for c in &calls {
            if TAKES.contains(&c.name.as_str()) && !primitive && !named(&c.name) {
                found.push(format!("a take with no guard, {}", line(c.at)));
            }
            if releases(t, c.at) && !primitive && !named(&c.name) {
                found.push(format!("a release outside a guard, {}", line(c.at)));
            }
            if NOT_UNDER_A_LEAF.contains(&c.name.as_str()) && !c.armed.is_empty() {
                let held: Vec<&str> = c.armed.iter().map(String::as_str).collect();
                found.push(format!(
                    "{} called holding {}, {}",
                    c.name,
                    held.join(" and "),
                    line(c.at)
                ));
            }
        }
        let held_sites = walk(t, f, &must[k]).1;
        for (s, sure) in sites.iter().zip(&held_sites) {
            if LEAVES.contains(&s.lock.as_str()) && !sure.armed.contains(&s.lock) {
                found.push(format!("{} let go where it is not held, {}", s.lock, line(s.at)));
            }
            let after = match s.form {
                Early::InScope => (s.at + 4..s.close)
                    .find(|&k| NOT_UNDER_A_LEAF.contains(&t[k].text.as_str()) && t[k + 1].text == "(")
                    .map(|k| t[k].text.as_str()),
                Early::BeforeStop => next_call(t, s.at + 4, s.close).filter(|c| *c == "w2m_unimplemented"),
            };
            match after {
                Some(call) => early.push(format!("{file}:{}: {}, before {call}", t[s.at].line, s.lock)),
                None if s.form == Early::InScope => found.push(format!(
                    "an early release on a path that then calls neither w2m_unimplemented nor w2m_logged, {}",
                    line(s.at)
                )),
                None => found.push(format!(
                    "a release before a stop whose next call is not w2m_unimplemented, {}",
                    line(s.at)
                )),
            }
        }
        for at in bare {
            found.push(format!("a runtime word held with no guard, {}", line(at)));
        }
        if CLEANUPS.contains(&f.name.as_str()) && !reads_its_flag(t, f) {
            found.push(format!("a guard's end that does not read its flag, {}", line(f.open)));
        }
    }
    (found, early)
}

/// The functions clang's `cleanup` attribute calls at a guard's scope's end: the five guards' ends and the lazily taken
/// word's (`WORD_WHEN_NEEDED`, sync_objects.c).
const CLEANUPS: [&str; 6] = [
    "heap_guard_end",
    "slots_guard_end",
    "handles_guard_end",
    "sync_guard_end",
    "runtime_word_guard_end",
    "let_word_go",
];

/// Whether a guard's end reads the flag its guard holds (the reviewer's High 1 at the card's amendment: an end that lets
/// its lock go whatever its take answered lets go a word this call did not take, or lets it go twice after an early
/// release): its parameter, the guard's address, read as `*p` or `p->`.
fn reads_its_flag(t: &[Token], f: &Function) -> bool {
    let Some(param) = t[..f.open]
        .iter()
        .rposition(|x| x.text == ")")
        .and_then(|close| t[..close].iter().rev().find(|x| is_word(&x.text)))
        .map(|x| x.text.as_str())
    else {
        return false;
    };
    (f.open + 1..f.close).any(|k| {
        t[k].text == param
            && (t[k - 1].text == "*" || (t[k + 1].text == "-" && t.get(k + 2).is_some_and(|x| x.text == ">")))
    })
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The runtime C's five locks are taken only through their guards and let go only by a guard's end or a named early
/// release before a call that must not run under a leaf (Sprint 359); a take, a release or a call breaking it is named
/// by its line.
#[test]
fn the_runtime_cs_locks_are_released_only_by_their_guards() {
    let texts: Vec<(&str, String)> = FILES
        .iter()
        .map(|f| {
            (
                *f,
                std::fs::read_to_string(root().join("tools/guest_runtime").join(f)).unwrap(),
            )
        })
        .collect();
    let sources: Vec<(&str, &str)> = texts.iter().map(|(f, t)| (*f, t.as_str())).collect();
    let (found, early) = judge(&sources);
    assert_eq!(found, Vec::<String>::new());
    let mut by_lock: BTreeMap<String, usize> = BTreeMap::new();
    for e in &early {
        println!("named early release: {e}");
        let lock = e
            .split(": ")
            .nth(1)
            .and_then(|r| r.split(',').next())
            .unwrap_or("")
            .to_string();
        *by_lock.entry(lock).or_default() += 1;
    }
    println!("named early releases by lock: {by_lock:?}, {} in all", early.len());
    for (file, guard) in [
        ("heap.c", "W2M_HELD(heap)"),
        ("thread_storage.c", "W2M_HELD(slots)"),
        ("sync_objects.c", "W2M_HELD(handles)"),
        ("sync_objects.c", "W2M_HELD(sync)"),
        ("sync_objects.c", "W2M_HELD(runtime_word)"),
    ] {
        let text = &texts.iter().find(|(f, _)| *f == file).unwrap().1;
        assert!(text.contains(guard), "{file} takes no lock by {guard}");
    }
    // every guard's end is there to be read (its reading of its flag is `judge`'s)
    for end in CLEANUPS {
        let defined = texts.iter().any(|(_, t)| {
            let toks = tokens(&code_of(t));
            functions(0, &toks).iter().any(|f| f.name == end)
        });
        assert!(defined, "no guard's end {end}");
    }
}

/// The judge's own cases: what it names and what it passes.
#[test]
fn the_judge_names_a_take_a_release_and_a_call_outside_the_rules() {
    let prelude = "static int hold_sync(void) { return 1; }\n\
                   static void let_sync_go(void) { }\n\
                   static void sync_guard_end(int *held) { if (*held) let_sync_go(); }\n";
    let judge_one = |body: &str| judge(&[("x.c", &format!("{prelude}{body}"))]).0;
    let bare = "u64 f(void) {\n    W2M_HELD(sync);\n    let_sync_go();\n    return 0;\n}\n";
    assert_eq!(
        judge_one(bare),
        vec!["a release outside a guard, x.c:6: let_sync_go();".to_string()]
    );
    let unguarded = "u64 f(void) {\n    hold_sync();\n    return 0;\n}\n";
    assert_eq!(
        judge_one(unguarded),
        vec!["a take with no guard, x.c:5: hold_sync();".to_string()]
    );
    let stop = "u64 f(u64 x) {\n    W2M_HELD(sync);\n    if (x) {\n        w2m_unimplemented(1, x);\n    }\n    return 0;\n}\n";
    assert_eq!(
        judge_one(stop),
        vec!["w2m_unimplemented called holding sync, x.c:7: w2m_unimplemented(1, x);".to_string()]
    );
    let logged = "u64 f(void) {\n    W2M_HELD(sync);\n    return w2m_logged(1, 2, 3);\n}\n";
    assert_eq!(
        judge_one(logged),
        vec!["w2m_logged called holding sync, x.c:6: return w2m_logged(1, 2, 3);".to_string()]
    );
    let returns = "u64 f(u64 x) {\n    W2M_HELD(sync);\n    if (x) {\n        W2M_LET_GO_EARLY(sync);\n        park();\n        return 0;\n    }\n    return 1;\n}\n";
    assert_eq!(
        judge_one(returns),
        vec![
            "an early release on a path that then calls neither w2m_unimplemented nor w2m_logged, x.c:7: \
             W2M_LET_GO_EARLY(sync);"
                .to_string()
        ]
    );
    let callee = "static void inner(void) {\n    w2m_unimplemented(1, 2);\n}\nu64 f(void) {\n    W2M_HELD(sync);\n    inner();\n    return 0;\n}\n";
    assert_eq!(
        judge_one(callee),
        vec!["w2m_unimplemented called holding sync, x.c:5: w2m_unimplemented(1, 2);".to_string()]
    );
    let lone = "static void lone(void) {\n    W2M_LET_GO_BEFORE_STOP(sync);\n    w2m_unimplemented(1, 2);\n}\nu64 f(void) {\n    lone();\n    return 0;\n}\n";
    assert_eq!(
        judge_one(lone),
        vec!["sync let go where it is not held, x.c:5: W2M_LET_GO_BEFORE_STOP(sync);".to_string()]
    );
    let word = "u64 f(void) {\n    struct word w = {0, 0};\n    return 0;\n}\n";
    assert_eq!(
        judge_one(word),
        vec!["a runtime word held with no guard, x.c:5: struct word w = {0, 0};".to_string()]
    );
    let right = "static void stops(void) {\n    W2M_LET_GO_BEFORE_STOP(sync);\n    w2m_unimplemented(1, 2);\n}\n\
                 u64 f(u64 x) {\n    W2M_HELD(sync);\n    if (x) stops();\n    if (x > 1) {\n        W2M_LET_GO_EARLY(sync);\n        \
                 return w2m_logged(1, 2, 3);\n    }\n    return 0;\n}\n\
                 u64 g(void) {\n    {\n        W2M_HELD(sync);\n    }\n    return w2m_logged(1, 2, 3);\n}\n";
    let (found, early) = judge(&[("x.c", &format!("{prelude}{right}"))]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(
        early,
        vec![
            "x.c:5: sync, before w2m_unimplemented".to_string(),
            "x.c:12: sync, before w2m_logged".to_string()
        ]
    );
    let commented = "u64 f(void) {\n    W2M_HELD(sync);\n    /* let_sync_go() is the guard's */\n    return 0;\n}\n";
    assert!(judge_one(commented).is_empty(), "a comment naming a release");
    // a guard's end that lets its lock go whatever its take answered (the card's plant 4)
    let ignored = "static void runtime_word_guard_end(int *taken) { (void)taken; w2m_let_runtime_word_go(1); }\n";
    assert_eq!(
        judge(&[("runtime.h", ignored)]).0,
        vec![
            "a guard's end that does not read its flag, runtime.h:1: static void runtime_word_guard_end(int *taken) \
             { (void)taken; w2m_let_runtime_word_go(1); }"
                .to_string()
        ]
    );
}
