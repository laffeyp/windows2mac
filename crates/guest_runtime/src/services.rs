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
/// itself (Sprint 343, `heap.c` `hold`), so its bridge holds nothing, as for none; the TLS and FLS slots' lock, which
/// the C takes itself too (Sprint 345, `thread_storage.c` `hold_slots`), its bridge holding nothing; the handle
/// table's lock, which the C takes itself (Sprint 355, `sync_objects.c` `hold_handles`), its bridge holding nothing; or
/// the sync objects' lock, which the C takes itself with the runtime word to park, to ready and for the host's objects
/// (Sprint 356, `sync_objects.c` `hold_sync`), its bridge holding nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    RuntimeWord,
    None,
    Heap,
    Slots,
    Handles,
    SyncObjects,
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

/// KERNEL32's services of lock class handles (Sprint 355; card 4 of `docs/design-handles-and-sync-objects-2026-11-26.md`,
/// "Where each service's class lands"), all in `sync_objects.c`. Each takes a slot and a new object, or looks a slot up,
/// and stores to the handle table's state, the slots and their flags, the install of a parent's handles, an entry's
/// allocation and both its counts, only under the handle table's lock (`hold_handles` and `let_handles_go`); it reaches
/// no sync object's state, a create writing its object before the handle is published. Its other stores are the calling
/// thread's last error (`set_error`), DuplicateHandle's `*lpTargetHandle`, and the call ring by atomic add
/// (`w2m_logged`). A named event's EVENT_OPEN, and DuplicateHandle's look at the file table and the standard handles
/// for a source that names no object, take the runtime word in the C with no other lock held (`w2m_take_runtime_word`,
/// `runtime.c`); `w2m_unimplemented` and `w2m_logged` are called only with the table's lock let go. The waits and
/// releases take it with no word too (`SYNC_LOCK`, Sprint 356); every other caller of the table, CloseHandle among them,
/// stays on the runtime word and takes the table's lock inside it, a leaf. From Sprint 369 the named sync objects the
/// game declares, CreateMutexW, OpenMutexW, OpenEventW, OpenEventA and OpenSemaphoreW (and CreateSemaphoreW's name):
/// a named mutex's or semaphore's make or open takes the runtime word in the C with no other lock held, for the host's
/// name request, and holds it to the handle's publication, the table's lock taken under it (`named`); a named event's
/// open takes it for the request alone, as CreateEventW's.
pub const HANDLES_LOCK: [&str; 15] = [
    "CreateEventA",
    "CreateEventExW",
    "CreateEventW",
    "CreateMutexA",
    "CreateMutexW",
    "CreateSemaphoreExW",
    "CreateSemaphoreW",
    "DuplicateHandle",
    "GetProcessId",
    "GetThreadId",
    "OpenEventA",
    "OpenEventW",
    "OpenMutexW",
    "OpenSemaphoreW",
    "OpenThread",
];

/// KERNEL32's services of lock class sync objects (Sprint 356; card 5 of
/// `docs/design-handles-and-sync-objects-2026-11-26.md`, "Where each service's class lands"), defined in
/// `sync_objects.c` but WaitForMultipleObjectsEx, `processes.c`'s, which calls WaitForMultipleObjects. Each looks its
/// handles up under the handle table's lock and lets it go; stores to a semaphore's, an event's and a mutex's count and
/// owner, the waiters on one object and the waits for all only under the sync objects' lock (`hold_sync` and
/// `let_sync_go`), never with the table's lock held; and takes the runtime word itself, with neither held
/// (`w2m_take_runtime_word`, `runtime.c`), to park (`w2m_block`: the run table, the census, the woken mark), to ready
/// (`w2m_ready_thread`, `w2m_signal`), and for the host's objects, a thread, a process or a shared event, whose wait
/// holds the word from its check to its park. Its other stores are the calling thread's TEB (its last error, its wait's
/// deadline), ReleaseSemaphore's `*lpPreviousCount`, its run entry's woken mark (armed under the sync objects' lock), its
/// vCPU's state block (`schedule`), and the call ring by atomic add (`w2m_logged`); `w2m_unimplemented` and `w2m_logged`
/// are called only with both leaves let go. From Sprint 369 SignalObjectAndWait, whose signal is a release's and whose
/// wait is WaitForSingleObject's.
pub const SYNC_LOCK: [&str; 9] = [
    "ReleaseMutex",
    "ReleaseSemaphore",
    "ResetEvent",
    "SetEvent",
    "SignalObjectAndWait",
    "WaitForMultipleObjects",
    "WaitForMultipleObjectsEx",
    "WaitForSingleObject",
    "WaitForSingleObjectEx",
];

/// The clock mode in which QueryPerformanceCounter reads `CNTVCT_EL0` alone (`process.c` 42-47): mode 1 asks the host
/// through the shared transport page (`w2m_host`, `runtime.c` 26-35), and mode 2 reads the shared warp word.
pub const GUEST_COUNTER: u64 = 0;

/// The clock mode in which the counter is a host request (`process.c` `counter_now`, `W2M_HOST_CLOCK`), which a wait of
/// lock class sync objects reads with no word held.
pub const HOST_CLOCK_MODE: u64 = 1;

impl Service {
    /// The lock its bridge holds in a run of clock mode `clock_mode` (the monitor's `CLOCK_MODE` word, set once at the
    /// load): its class, but QueryPerformanceCounter's on any clock but the guest's counter is the runtime word, and a
    /// service of the sync objects' class on the host's clock is the runtime word (Sprint 356: a timed wait reads the
    /// counter, there a host request, with no leaf held; the word held, the C's own takes keep it).
    pub fn lock_in(&self, clock_mode: u64) -> Lock {
        if self.name == "QueryPerformanceCounter" && clock_mode != GUEST_COUNTER {
            return Lock::RuntimeWord;
        }
        if self.lock == Lock::SyncObjects && clock_mode == HOST_CLOCK_MODE {
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
                } else if dll == "KERNEL32" && HANDLES_LOCK.contains(&name.as_str()) {
                    Lock::Handles
                } else if dll == "KERNEL32" && SYNC_LOCK.contains(&name.as_str()) {
                    Lock::SyncObjects
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

/// The runtime's W2M_API_ ids, fixed (Sprint 368): `api_ids.txt`, "id name" a line, `#` lines comments. Ids 1-299 are
/// the names in name order as `guest_runtime.api_ids` gave them, the frozen tag's and HEAD's at Sprint 368's card; a
/// new service takes the next id after the largest, added to the table with it. Numbered in name order, the new
/// SleepConditionVariableCS renumbered the 69 names after it, and the ring's records and per-API counts, which the
/// exit ledger hashes, moved from the first call of any of them.
pub const API_IDS: &str = include_str!("api_ids.txt");

/// A table's (id, name) rows: ids from 1, each once and none skipped, below the ring's `W2M_RING_APIS` (512).
fn api_table(table: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for line in table.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let (id, name) = line
            .split_once(' ')
            .ok_or_else(|| format!("api_ids.txt: {line:?} is not \"id name\""))?;
        let id: usize = id.parse().map_err(|_| format!("api_ids.txt: {line:?}: no id"))?;
        if id != names.len() + 1 {
            return Err(format!(
                "api_ids.txt: {name} has id {id}, where {} is next",
                names.len() + 1
            ));
        }
        if names.iter().any(|n| n == name) {
            return Err(format!("api_ids.txt: {name} has two ids"));
        }
        names.push(name.to_string());
    }
    if names.len() >= 512 {
        return Err(format!(
            "api_ids.txt: {} ids, where W2M_RING_APIS counts 511",
            names.len()
        ));
    }
    Ok(names)
}

/// The runtime's API names in id order: id n is the n-th, from 1, as `table` numbers them; every service's import
/// name and the ring's internal entries must have a row, a missing one an error naming the id it takes next; two
/// services of one name are an error, their ids colliding.
fn api_names_in(services: &[Service], table: &str) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    names.extend(INTERNAL_APIS.iter().map(|n| n.to_string()));
    let count = names.len();
    names.sort();
    names.dedup();
    if names.len() != count {
        return Err("two services share an import name: their W2M_API_ ids would collide".to_string());
    }
    let ids = api_table(table)?;
    if let Some(missing) = names.iter().find(|n| !ids.contains(n)) {
        return Err(format!(
            "{missing} has no W2M_API_ id: add \"{} {missing}\" to crates/guest_runtime/src/api_ids.txt",
            ids.len() + 1
        ));
    }
    Ok(ids)
}

/// The runtime's API names in id order, `API_IDS`'s numbering.
pub fn api_names(services: &[Service]) -> Result<Vec<String>, String> {
    api_names_in(services, API_IDS)
}

/// `api_ids.h`: `#define W2M_API_<name> <n>`, each name's id in `API_IDS` (`guest_runtime.api_header` numbered them in
/// name order, which ids 1-299 are).
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
        // CreateEventW until Sprint 355, which moves it to the handle table's lock
        assert_eq!(
            named("CloseHandle").lock_in(GUEST_COUNTER),
            Lock::RuntimeWord,
            "CloseHandle's lock"
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

    /// Sprint 355: in the runtime's own C, the services of lock class handles are the ten `HANDLES_LOCK` names, each
    /// KERNEL32's and defined in `sync_objects.c`, of class handles on every clock; the other callers of the handle
    /// table, which take its lock inside the runtime word, stay on the word: CloseHandle (its file path), the waits and
    /// releases (card 5's), Get/SetHandleInformation (the file table), the process, pipe and thread services (host
    /// requests), the thread information calls, and ADVAPI32's token calls. Sprint 369: fifteen, with the named sync
    /// objects' make and opens, CreateMutexW, OpenEventA, OpenEventW, OpenMutexW and OpenSemaphoreW.
    #[test]
    fn the_fifteen_services_of_lock_class_handles_are_the_handle_tables() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let texts = crate::texts(&root).unwrap();
        let all = services(&texts).unwrap();
        let mut handles: Vec<String> = all
            .iter()
            .filter(|s| s.lock == Lock::Handles)
            .map(|s| format!("{}!{}", s.dll, s.name))
            .collect();
        handles.sort();
        assert_eq!(
            handles,
            [
                "KERNEL32!CreateEventA",
                "KERNEL32!CreateEventExW",
                "KERNEL32!CreateEventW",
                "KERNEL32!CreateMutexA",
                "KERNEL32!CreateMutexW",
                "KERNEL32!CreateSemaphoreExW",
                "KERNEL32!CreateSemaphoreW",
                "KERNEL32!DuplicateHandle",
                "KERNEL32!GetProcessId",
                "KERNEL32!GetThreadId",
                "KERNEL32!OpenEventA",
                "KERNEL32!OpenEventW",
                "KERNEL32!OpenMutexW",
                "KERNEL32!OpenSemaphoreW",
                "KERNEL32!OpenThread",
            ],
            "the services of lock class handles"
        );
        let named = |dll: &str, n: &str| all.iter().find(|s| s.dll == dll && s.name == n).unwrap();
        for n in HANDLES_LOCK {
            assert!(
                texts["sync_objects.c"].contains(&format!("u64 k32_{n}(")),
                "{n} defined in sync_objects.c"
            );
            assert_eq!(
                [0, 1, 2].map(|mode| named("KERNEL32", n).lock_in(mode)),
                [Lock::Handles; 3],
                "{n}'s lock in clock modes 0, 1 and 2"
            );
        }
        // Sprint 356 moves the waits and releases to the sync objects' lock (its own test below)
        for (dll, n) in [
            ("KERNEL32", "CloseHandle"),
            ("KERNEL32", "GetHandleInformation"),
            ("KERNEL32", "SetHandleInformation"),
            ("KERNEL32", "CreateFileMappingW"),
            ("KERNEL32", "MapViewOfFile"),
            ("KERNEL32", "CreatePipe"),
            ("KERNEL32", "CreateProcessW"),
            ("KERNEL32", "OpenProcess"),
            ("KERNEL32", "GetExitCodeProcess"),
            ("KERNEL32", "CreateThread"),
            ("KERNEL32", "GetThreadPriority"),
            ("ADVAPI32.DLL", "OpenProcessToken"),
            ("ADVAPI32.DLL", "GetTokenInformation"),
        ] {
            assert_eq!(
                named(dll, n).lock_in(GUEST_COUNTER),
                Lock::RuntimeWord,
                "{dll}!{n}'s lock"
            );
        }
    }

    /// Sprint 356: in the runtime's own C, the services of lock class sync objects are the eight `SYNC_LOCK` names, each
    /// KERNEL32's, defined in `sync_objects.c` but WaitForMultipleObjectsEx, `processes.c`'s; of class sync objects on
    /// the guest's counter and the instruction clock, and the runtime word's on the host's clock, where a timed wait's
    /// counter read is a host request; the other services that reach these kinds' state or a wait stay where they were:
    /// CloseHandle and the pipe and process calls on the word, the creates, DuplicateHandle and the id lookups on the
    /// handle table's lock, Sleep, the critical sections and the slim locks on the word. Sprint 369: nine, with
    /// SignalObjectAndWait, a release's signal and WaitForSingleObject's wait.
    #[test]
    fn the_nine_services_of_lock_class_sync_objects_are_the_waits_and_releases() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let texts = crate::texts(&root).unwrap();
        let all = services(&texts).unwrap();
        let mut sync: Vec<String> = all
            .iter()
            .filter(|s| s.lock == Lock::SyncObjects)
            .map(|s| format!("{}!{}", s.dll, s.name))
            .collect();
        sync.sort();
        assert_eq!(
            sync,
            [
                "KERNEL32!ReleaseMutex",
                "KERNEL32!ReleaseSemaphore",
                "KERNEL32!ResetEvent",
                "KERNEL32!SetEvent",
                "KERNEL32!SignalObjectAndWait",
                "KERNEL32!WaitForMultipleObjects",
                "KERNEL32!WaitForMultipleObjectsEx",
                "KERNEL32!WaitForSingleObject",
                "KERNEL32!WaitForSingleObjectEx",
            ],
            "the services of lock class sync objects"
        );
        let named = |n: &str| all.iter().find(|s| s.dll == "KERNEL32" && s.name == n).unwrap();
        for n in SYNC_LOCK {
            let file = if n == "WaitForMultipleObjectsEx" {
                "processes.c"
            } else {
                "sync_objects.c"
            };
            assert!(texts[file].contains(&format!("u64 k32_{n}(")), "{n} defined in {file}");
            assert_eq!(
                [0, 1, 2].map(|mode| named(n).lock_in(mode)),
                [Lock::SyncObjects, Lock::RuntimeWord, Lock::SyncObjects],
                "{n}'s lock in clock modes 0, 1 and 2"
            );
        }
        for (n, lock) in [
            ("CloseHandle", Lock::RuntimeWord),
            ("CreateSemaphoreW", Lock::Handles),
            ("CreateEventA", Lock::Handles),
            ("DuplicateHandle", Lock::Handles),
            ("ReadFile", Lock::RuntimeWord),
            ("WriteFile", Lock::RuntimeWord),
            ("GetExitCodeProcess", Lock::RuntimeWord),
            ("Sleep", Lock::RuntimeWord),
            ("EnterCriticalSection", Lock::RuntimeWord),
            ("SleepConditionVariableSRW", Lock::RuntimeWord),
        ] {
            assert_eq!(named(n).lock_in(GUEST_COUNTER), lock, "{n}'s lock");
        }
    }

    /// Sprint 357 (vocabulary 0.14, P31): the vocabulary's enum `lock`, which MEASUREMENT_RECORDED's `lock` takes, names
    /// `Lock`'s leaf classes, every variant but the runtime word, which keeps its own metrics, and none, which is no
    /// lock, each as its snake case and in the enum's order; read from this file's text, so a class added to `Lock`
    /// fails here until the vocabulary names it.
    #[test]
    fn the_vocabularys_lock_values_are_the_leaf_classes_of_lock() {
        let source = include_str!("services.rs");
        let body = source
            .split_once("pub enum Lock {")
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(body, _)| body)
            .expect("services.rs defines `pub enum Lock`");
        let snake = |camel: &str| {
            let mut out = String::new();
            for (i, c) in camel.chars().enumerate() {
                if c.is_ascii_uppercase() && i > 0 {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            }
            out
        };
        let leaves: Vec<String> = body
            .lines()
            .map(|l| l.trim().trim_end_matches(','))
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .filter(|l| *l != "RuntimeWord" && *l != "None")
            .map(snake)
            .collect();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |p: &std::path::Path| -> serde_json::Value {
            serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
        };
        let current = read(&root.join("signals/rust-current.json"));
        let version = current["version"].as_str().expect("rust-current.json names a version");
        let vocabulary = read(&root.join(format!("signals/{version}.json")));
        let values: Vec<String> = vocabulary["enums"]["lock"]
            .as_array()
            .expect("the vocabulary has an enum lock")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            leaves, values,
            "Lock's leaf classes (services.rs) and vocabulary {version}'s enum lock"
        );
    }

    #[test]
    fn a_service_defined_twice_is_refused_by_name() {
        let t = texts(&[("a.c", "u64 k32_Beep(void) {\n"), ("b.c", "u64 k32_Beep(u64 x) {\n")]);
        assert_eq!(services(&t).unwrap_err(), "b.c: k32_Beep is defined twice");
    }

    /// The ids are the table's, the services' and the ring's internal entries each in it (Sprint 368): a service the
    /// table lacks is named with the id it takes next; a table that skips an id or names one twice is refused.
    #[test]
    fn api_ids_are_the_tables_and_a_service_it_lacks_is_named() {
        let t = texts(&[("a.c", "u64 k32_Sleep(u64 ms) {\nu64 user32_Beep(void) {\n")]);
        let s = services(&t).unwrap();
        // HOST_UNIMPLEMENTED's id n names the n-th of `api_names` (Sprint 219): the list `api_ids.h` is written from
        let table = "# a comment\n1 Sleep\n2 Cpuid\n3 Exception\n4 Beep\n";
        assert_eq!(
            api_names_in(&s, table).unwrap(),
            ["Sleep", "Cpuid", "Exception", "Beep"]
        );
        assert_eq!(
            api_names_in(&s, "1 Sleep\n2 Cpuid\n3 Exception\n").unwrap_err(),
            "Beep has no W2M_API_ id: add \"4 Beep\" to crates/guest_runtime/src/api_ids.txt"
        );
        assert_eq!(
            api_names_in(&s, "1 Sleep\n3 Cpuid\n").unwrap_err(),
            "api_ids.txt: Cpuid has id 3, where 2 is next"
        );
        assert_eq!(
            api_names_in(&s, "1 Sleep\n2 Sleep\n").unwrap_err(),
            "api_ids.txt: Sleep has two ids"
        );
        // an id given twice is not the next one
        assert_eq!(
            api_names_in(&s, "1 Sleep\n2 Cpuid\n2 Exception\n").unwrap_err(),
            "api_ids.txt: Exception has id 2, where 3 is next"
        );
    }

    /// Sprint 368: no service's id moves. The table's ids 1-299 are HEAD's at the sprint's card, the names then in name
    /// order (the frozen tag's numbering too), so its first 299 rows are in name order; the frozen numbers below are
    /// written here apart from the table; and `api_header` over the runtime's own C gives every one of those 299 its id,
    /// with SleepConditionVariableCS, the first service after them, 300. Numbered in name order again, every name
    /// sorting after SleepConditionVariableCS moves by one, and the first is named.
    #[test]
    fn the_api_ids_at_head_are_unchanged_and_a_new_service_takes_the_next() {
        let head = api_table(API_IDS).unwrap();
        // rows 1-299 whole, the names one a line: the sha256 of HEAD's list and the frozen tag's, so no row is renamed
        // or reused in place and a new name takes an id above 299
        let text: String = head.iter().take(299).map(|n| format!("{n}\n")).collect();
        assert_eq!(
            crate::sha256::hex(text.as_bytes()),
            "ab180912c4dd52e895e00044b2fbeedf8446a21d5d64fa4d21e15c818d0dd512",
            "api_ids.txt's rows 1-299 are not HEAD's"
        );
        let first: Vec<&String> = head.iter().take(299).collect();
        let mut sorted = first.clone();
        sorted.sort();
        assert_eq!(
            first, sorted,
            "api_ids.txt's ids 1-299 are HEAD's, the names in name order"
        );
        for (id, name) in [
            (1, "AcquireSRWLockExclusive"),
            (15, "Cpuid"),
            (43, "Exception"),
            (230, "Sleep"),
            (231, "SleepConditionVariableSRW"),
            (293, "WideCharToMultiByte"),
            (299, "timeGetTime"),
            (300, "SleepConditionVariableCS"),
            // Sprint 369: the named sync objects, each the next id after the largest
            (301, "CreateMutexW"),
            (306, "SignalObjectAndWait"),
        ] {
            assert_eq!(head[id - 1], name, "the frozen id {id}");
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let header = api_header(&crate::texts(&root).unwrap()).unwrap();
        let ids: BTreeMap<&str, usize> = header
            .lines()
            .filter_map(|l| l.strip_prefix("#define W2M_API_")?.split_once(' '))
            .map(|(n, id)| (n, id.parse().unwrap()))
            .collect();
        for (i, name) in head.iter().enumerate() {
            assert_eq!(
                ids.get(name.as_str()),
                Some(&(i + 1)),
                "{name}'s W2M_API_ id moved from {}, its id at HEAD (api_ids.txt)",
                i + 1
            );
        }
    }
}
