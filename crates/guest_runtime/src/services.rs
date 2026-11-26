// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The services the runtime's C defines, found as `src/w2m/guest_runtime.py` finds them: a definition
//! `u64 <dll prefix>_<import name>(...) {` at a line's start, `W2M_VOID` before it marking a function Windows declares
//! void. The Python matched `^(W2M_VOID\s+)?u64\s+(ws2_32|[a-z0-9]+)_(\w+)\([^;{]*\)\s*\{` (re.M); this scanner takes
//! the same text, with no regex crate (no crate but serde_json, Decision 2026-10-24).

use std::collections::BTreeMap;

/// One service: its DLL ("KERNEL32", "WS2_32.DLL", ...), import name, C symbol, whether it sets RAX, and its lock
/// class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub dll: String,
    pub name: String,
    pub symbol: String,
    pub sets_rax: bool,
    pub lock: Lock,
}

/// What a service's bridge holds around its call into the C (Change 1 of the host design,
/// `docs/design-rust-host-2026-10-24.md` 95-140, "a runtime call takes the lock of what it touches, or none"): the
/// runtime word, which every service took until Sprint 342; none; the process heap's own lock, which the C takes
/// itself (Sprint 343, `heap.c` `hold`), so its bridge holds nothing, as for none; or the TLS and FLS slots' lock, which
/// the C takes itself too (Sprint 345, `thread_storage.c` `hold_slots`), its bridge holding nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    RuntimeWord,
    None,
    Heap,
    Slots,
}

/// KERNEL32's services of lock class none (Sprint 342; Sprint 339's proof, re-read on Sprint 341's tree): each stores
/// only to its calling thread's own state (its TEB's last error and TLS slots, `last_error.c` 8, `thread_storage.c`
/// 58-65 and 108) or its caller's memory (QueryPerformanceCounter's `*out`, `process.c` 55), and to the call ring by
/// atomic add (`w2m_logged`, `runtime.c` 84-94); its one host request, `w2m_unimplemented`'s, takes the word itself.
/// FlsGetValue and FlsSetValue take the TLS and FLS slots' lock (`SLOTS_LOCK`, Sprint 345): FlsSetValue's first set
/// links a block into the process-wide list, and FlsAlloc and FlsFree write every thread's block.
pub const NO_LOCK: [&str; 7] = [
    "GetCurrentProcessId",
    "GetCurrentThreadId",
    "GetLastError",
    "QueryPerformanceCounter",
    "SetLastError",
    "TlsGetValue",
    "TlsSetValue",
];

/// KERNEL32's services of lock class heap (Sprint 343, "a lock per heap"): the process heap's, all in `heap.c`. Each
/// stores to the heap's shared state, its free list, its bump pointer and its set-up, only under the heap's lock
/// (`hold` and `let_go`, in `take`, `give_back` and GetProcessHeap); to a block's own header and payload, which its
/// holder owns; to the calling thread's last error (`set_error`); and to the call ring by atomic add (`w2m_logged`).
/// Its one host request, `w2m_unimplemented`'s, takes the word itself and is never made with the heap's lock held.
pub const HEAP_LOCK: [&str; 5] = ["GetProcessHeap", "HeapAlloc", "HeapFree", "HeapReAlloc", "HeapSize"];

/// KERNEL32's services of lock class slots (Sprint 345, "the TLS and FLS slots' lock"), all in `thread_storage.c`. Each
/// stores to the process's slot state, the TLS and FLS bitmaps (`tls_used`, `fls_used`), the FLS callbacks, the list
/// of every thread's FLS block and each block's values, and the TLS set-up (`tls_ready`), only under the slots' lock
/// (`hold_slots` and `let_slots_go`); FlsGetValue and FlsGetValue2 read a block FlsAlloc and FlsFree write, so they
/// take it too. Their other stores are the calling thread's TEB (its TLS slots, its expansion pointer, its FLS block's
/// address, its last error) and the call ring by atomic add (`w2m_logged`). The heap's lock is taken under it
/// (FlsSetValue's first set, `w2m_heap_take`), slots then heap, as the design's order has it; `w2m_unimplemented` and
/// `w2m_logged` are called only with it let go.
pub const SLOTS_LOCK: [&str; 7] = [
    "FlsAlloc",
    "FlsFree",
    "FlsGetValue",
    "FlsGetValue2",
    "FlsSetValue",
    "TlsAlloc",
    "TlsFree",
];

/// The clock mode in which QueryPerformanceCounter reads `CNTVCT_EL0` alone (`process.c` 42-47): mode 1 asks the host
/// through the shared transport page (`w2m_host`, `runtime.c` 26-35), and mode 2 reads the shared warp word.
pub const GUEST_COUNTER: u64 = 0;

impl Service {
    /// The lock its bridge holds in a run of clock mode `clock_mode` (the monitor's `CLOCK_MODE` word, set once at the
    /// load): its class, but QueryPerformanceCounter's on any clock but the guest's counter is the runtime word.
    pub fn lock_in(&self, clock_mode: u64) -> Lock {
        if self.name == "QueryPerformanceCounter" && clock_mode != GUEST_COUNTER {
            return Lock::RuntimeWord;
        }
        self.lock
    }
}

/// Python's `\s`: space, tab, newline, carriage return, form feed, vertical tab.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b)
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The definition starting at `start` (a line's start), and where the match ends.
fn match_at(text: &[u8], start: usize) -> Option<(bool, String, String, usize)> {
    let mut p = start;
    let mut void = false;
    if text[p..].starts_with(b"W2M_VOID") {
        let mut q = p + 8;
        let spaces = text[q..].iter().take_while(|b| is_space(**b)).count();
        q += spaces;
        if spaces > 0 && text[q..].starts_with(b"u64") {
            void = true;
            p = q;
        }
    }
    if !text[p..].starts_with(b"u64") {
        return None;
    }
    p += 3;
    let spaces = text[p..].iter().take_while(|b| is_space(**b)).count();
    if spaces == 0 {
        return None;
    }
    p += spaces;
    // (ws2_32|[a-z0-9]+)_ : the alternation tries ws2_32 first
    let prefix_len = if text[p..].starts_with(b"ws2_32_") {
        6
    } else {
        text[p..]
            .iter()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            .count()
    };
    if prefix_len == 0 || text.get(p + prefix_len) != Some(&b'_') {
        return None;
    }
    let prefix = String::from_utf8_lossy(&text[p..p + prefix_len]).into_owned();
    p += prefix_len + 1;
    let name_len = text[p..].iter().take_while(|b| is_word(**b)).count();
    if name_len == 0 || text.get(p + name_len) != Some(&b'(') {
        return None;
    }
    let name = String::from_utf8_lossy(&text[p..p + name_len]).into_owned();
    p += name_len + 1;
    // \([^;{]*\)\s*\{ : the first ';' or '{' after the parenthesis must be the '{', with only spaces between it and
    // the last ')'
    let stop = p + text[p..].iter().position(|b| *b == b';' || *b == b'{')?;
    if text[stop] != b'{' {
        return None;
    }
    let inner = &text[p..stop];
    let trimmed = inner.len() - inner.iter().rev().take_while(|b| is_space(**b)).count();
    if trimmed == 0 || inner[trimmed - 1] != b')' {
        return None;
    }
    Some((void, prefix, name, stop + 1))
}

/// Every service in `texts` (file name -> text), files in name order: `guest_runtime.service_table`, refusing a
/// service defined twice by its DLL and name.
pub fn services(texts: &BTreeMap<String, String>) -> Result<Vec<Service>, String> {
    let mut out: Vec<Service> = Vec::new();
    for (file, text) in texts {
        let bytes = text.as_bytes();
        let mut p = 0;
        while p < bytes.len() {
            let at_line_start = p == 0 || bytes[p - 1] == b'\n';
            if at_line_start && let Some((void, prefix, name, end)) = match_at(bytes, p) {
                p = end;
                if prefix == "w2m" {
                    continue;
                }
                let dll = if prefix == "k32" {
                    "KERNEL32".to_string()
                } else {
                    format!("{}.DLL", prefix.to_ascii_uppercase())
                };
                if out.iter().any(|s| s.dll == dll && s.name == name) {
                    return Err(format!("{file}: {prefix}_{name} is defined twice"));
                }
                let lock = if dll == "KERNEL32" && NO_LOCK.contains(&name.as_str()) {
                    Lock::None
                } else if dll == "KERNEL32" && HEAP_LOCK.contains(&name.as_str()) {
                    Lock::Heap
                } else if dll == "KERNEL32" && SLOTS_LOCK.contains(&name.as_str()) {
                    Lock::Slots
                } else {
                    Lock::RuntimeWord
                };
                out.push(Service {
                    dll,
                    symbol: format!("{prefix}_{name}"),
                    name,
                    sets_rax: !void,
                    lock,
                });
                continue;
            }
            p += 1;
        }
    }
    Ok(out)
}

/// The ring's entries that are not services (`guest_runtime.py` `INTERNAL_APIS`).
const INTERNAL_APIS: [&str; 2] = ["Cpuid", "Exception"];

/// The runtime's API names in id order: id n is the n-th, from 1 (`guest_runtime.api_ids`), the services' import names
/// and the ring's internal entries, sorted; two services of one name are an error, their ids colliding.
pub fn api_names(services: &[Service]) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    names.extend(INTERNAL_APIS.iter().map(|n| n.to_string()));
    let count = names.len();
    names.sort();
    names.dedup();
    if names.len() != count {
        return Err("two services share an import name: their W2M_API_ ids would collide".to_string());
    }
    Ok(names)
}

/// `api_ids.h`: `#define W2M_API_<name> <n>`, 1 up in name order (`guest_runtime.api_header`).
pub fn api_header(texts: &BTreeMap<String, String>) -> Result<String, String> {
    Ok(api_names(&services(texts)?)?
        .iter()
        .enumerate()
        .map(|(i, n)| format!("#define W2M_API_{n} {}\n", i + 1))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn definitions_are_found_as_the_python_pattern_found_them() {
        let t = texts(&[(
            "a.c",
            "u64 k32_GetTickCount(void) {\n\
             W2M_VOID\nu64 k32_Sleep(u64 ms)\n{\n\
             u64 ws2_32_WSAStartup(u64 a, u64 b) {\n\
             u64 shell32_X(u64 a);\n\
             static u64 k32_Hidden(void) {\n\
             u64 w2m_internal(void) {\n\
             u64 oleaut32_SysFreeString(u64 s /* a { */) {\n",
        )]);
        let found: Vec<(String, String, bool)> = services(&t)
            .unwrap()
            .into_iter()
            .map(|s| (s.dll, s.name, s.sets_rax))
            .collect();
        assert_eq!(
            found,
            [
                ("KERNEL32".to_string(), "GetTickCount".to_string(), true),
                ("KERNEL32".to_string(), "Sleep".to_string(), false),
                ("WS2_32.DLL".to_string(), "WSAStartup".to_string(), true),
            ]
        );
    }

    /// Sprint 342: in the runtime's own C, the services of lock class none are the seven `NO_LOCK` names, each
    /// KERNEL32's, and every other service takes the runtime word; QueryPerformanceCounter is none on the guest's
    /// counter alone, and a service of the word's class stays on it on every clock.
    #[test]
    fn the_seven_services_of_lock_class_none_are_the_proofs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let all = services(&crate::texts(&root).unwrap()).unwrap();
        let mut none: Vec<String> = all
            .iter()
            .filter(|s| s.lock == Lock::None)
            .map(|s| format!("{}!{}", s.dll, s.name))
            .collect();
        none.sort();
        assert_eq!(
            none,
            [
                "KERNEL32!GetCurrentProcessId",
                "KERNEL32!GetCurrentThreadId",
                "KERNEL32!GetLastError",
                "KERNEL32!QueryPerformanceCounter",
                "KERNEL32!SetLastError",
                "KERNEL32!TlsGetValue",
                "KERNEL32!TlsSetValue",
            ],
            "the services of lock class none"
        );
        let named = |n: &str| all.iter().find(|s| s.name == n).unwrap();
        let qpc = named("QueryPerformanceCounter");
        assert_eq!(
            [0, 1, 2].map(|mode| qpc.lock_in(mode)),
            [Lock::None, Lock::RuntimeWord, Lock::RuntimeWord],
            "QueryPerformanceCounter's lock in clock modes 0, 1 and 2"
        );
        assert_eq!(
            named("CreateEventW").lock_in(GUEST_COUNTER),
            Lock::RuntimeWord,
            "CreateEventW's lock"
        );
    }

    /// Sprint 343: in the runtime's own C, the services of lock class heap are the five `HEAP_LOCK` names, each
    /// KERNEL32's and defined in `heap.c`, of class heap on every clock; LocalAlloc and LocalFree, which reach the heap
    /// through `w2m_heap_take` and `w2m_heap_release`, stay on the runtime word.
    #[test]
    fn the_five_services_of_lock_class_heap_are_the_process_heaps() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let texts = crate::texts(&root).unwrap();
        let all = services(&texts).unwrap();
        let mut heap: Vec<String> = all
            .iter()
            .filter(|s| s.lock == Lock::Heap)
            .map(|s| format!("{}!{}", s.dll, s.name))
            .collect();
        heap.sort();
        assert_eq!(
            heap,
            [
                "KERNEL32!GetProcessHeap",
                "KERNEL32!HeapAlloc",
                "KERNEL32!HeapFree",
                "KERNEL32!HeapReAlloc",
                "KERNEL32!HeapSize",
            ],
            "the services of lock class heap"
        );
        let named = |n: &str| all.iter().find(|s| s.name == n).unwrap();
        for n in HEAP_LOCK {
            assert!(
                texts["heap.c"].contains(&format!("u64 k32_{n}(")),
                "{n} defined in heap.c"
            );
            assert_eq!(
                [0, 1, 2].map(|mode| named(n).lock_in(mode)),
                [Lock::Heap; 3],
                "{n}'s lock in clock modes 0, 1 and 2"
            );
        }
        for n in ["LocalAlloc", "LocalFree"] {
            assert_eq!(named(n).lock_in(GUEST_COUNTER), Lock::RuntimeWord, "{n}'s lock");
        }
    }

    /// Sprint 345: in the runtime's own C, the services of lock class slots are the seven `SLOTS_LOCK` names, each
    /// KERNEL32's and defined in `thread_storage.c`, of class slots on every clock; TlsGetValue, TlsSetValue and
    /// TlsGetValue2, which touch only the calling thread's TEB, are not of it.
    #[test]
    fn the_seven_services_of_lock_class_slots_are_the_tls_and_fls_slots() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let texts = crate::texts(&root).unwrap();
        let all = services(&texts).unwrap();
        let mut slots: Vec<String> = all
            .iter()
            .filter(|s| s.lock == Lock::Slots)
            .map(|s| format!("{}!{}", s.dll, s.name))
            .collect();
        slots.sort();
        assert_eq!(
            slots,
            [
                "KERNEL32!FlsAlloc",
                "KERNEL32!FlsFree",
                "KERNEL32!FlsGetValue",
                "KERNEL32!FlsGetValue2",
                "KERNEL32!FlsSetValue",
                "KERNEL32!TlsAlloc",
                "KERNEL32!TlsFree",
            ],
            "the services of lock class slots"
        );
        let named = |n: &str| all.iter().find(|s| s.name == n).unwrap();
        for n in SLOTS_LOCK {
            assert!(
                texts["thread_storage.c"].contains(&format!("u64 k32_{n}(")),
                "{n} defined in thread_storage.c"
            );
            assert_eq!(
                [0, 1, 2].map(|mode| named(n).lock_in(mode)),
                [Lock::Slots; 3],
                "{n}'s lock in clock modes 0, 1 and 2"
            );
        }
        for (n, lock) in [
            ("TlsGetValue", Lock::None),
            ("TlsSetValue", Lock::None),
            ("TlsGetValue2", Lock::RuntimeWord),
        ] {
            assert_eq!(named(n).lock_in(GUEST_COUNTER), lock, "{n}'s lock");
        }
    }

    #[test]
    fn a_service_defined_twice_is_refused_by_name() {
        let t = texts(&[("a.c", "u64 k32_Beep(void) {\n"), ("b.c", "u64 k32_Beep(u64 x) {\n")]);
        assert_eq!(services(&t).unwrap_err(), "b.c: k32_Beep is defined twice");
    }

    #[test]
    fn api_ids_count_from_one_in_name_order_with_the_internal_entries() {
        let t = texts(&[("a.c", "u64 k32_Sleep(u64 ms) {\nu64 user32_Beep(void) {\n")]);
        assert_eq!(
            api_header(&t).unwrap(),
            "#define W2M_API_Beep 1\n#define W2M_API_Cpuid 2\n#define W2M_API_Exception 3\n#define W2M_API_Sleep 4\n"
        );
        // HOST_UNIMPLEMENTED's id n names the n-th of `api_names` (Sprint 219): the list `api_ids.h` is written from
        let names = api_names(&services(&t).unwrap()).unwrap();
        assert_eq!(names, ["Beep", "Cpuid", "Exception", "Sleep"]);
    }
}
