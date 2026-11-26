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
/// stays on the runtime word and takes the table's lock inside it, a leaf.
pub const HANDLES_LOCK: [&str; 10] = [
    "CreateEventA",
    "CreateEventExW",
    "CreateEventW",
    "CreateMutexA",
    "CreateSemaphoreExW",
    "CreateSemaphoreW",
    "DuplicateHandle",
    "GetProcessId",
    "GetThreadId",
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
/// are called only with both leaves let go.
pub const SYNC_LOCK: [&str; 8] = [
    "ReleaseMutex",
    "ReleaseSemaphore",
    "ResetEvent",
    "SetEvent",
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
    /// requests), the thread information calls, and ADVAPI32's token calls.
    #[test]
    fn the_ten_services_of_lock_class_handles_are_the_handle_tables() {
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
                "KERNEL32!CreateSemaphoreExW",
                "KERNEL32!CreateSemaphoreW",
                "KERNEL32!DuplicateHandle",
                "KERNEL32!GetProcessId",
                "KERNEL32!GetThreadId",
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
    /// handle table's lock, Sleep, the critical sections and the slim locks on the word.
    #[test]
    fn the_eight_services_of_lock_class_sync_objects_are_the_waits_and_releases() {
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
