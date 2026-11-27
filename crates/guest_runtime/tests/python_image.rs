// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 133's judge: the runtime image Rust builds equals, byte for byte, the one the frozen Python host's build
//! cached under the same key in `build/guest_runtime/`. Xcode's clang and `ld` build it, so the test runs on macOS.
//! From Sprint 271 it builds the C as it stood at `FROZEN`, exported from git, so the working tree's C may change
//! (plan step 6, "in the C runtime, changed only there") and the judge still proves the Rust build reproduces the
//! Python build.
#![cfg(target_os = "macos")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use guest_runtime::layout::{RUNTIME_IMAGE_VA, runtime_defines};
use guest_runtime::{FROZEN_TAG as FROZEN, build, cache_key, frozen_root, services, texts};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The frozen sources' key, and the root to build them at: the repository's while its C keys the same, so its build
/// cache serves, else the export (`guest_runtime::frozen_root`).
fn frozen_build_root(root: &Path) -> (String, PathBuf) {
    let frozen = frozen_root(root).unwrap();
    let key = cache_key(&frozen, &texts(&frozen).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let here = cache_key(root, &texts(root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    (key.clone(), if here == key { root.to_path_buf() } else { frozen })
}

#[test]
fn the_image_rust_builds_is_the_python_builds_byte_for_byte() {
    let root = root();
    let (frozen_key, at) = frozen_build_root(&root);
    let (key, image) = build(&at, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_eq!(key, frozen_key);
    let python = root.join(format!("build/guest_runtime/{key}.bin"));
    assert!(
        python.exists(),
        "no Python image under key {key}: a define, a header or a source differs from the Python build's"
    );
    let ours = fs::read(&image.file).unwrap();
    let theirs = fs::read(&python).unwrap();
    assert_eq!(ours.len(), theirs.len(), "sizes differ");
    assert!(
        ours == theirs,
        "the images differ at byte {:?}",
        ours.iter().zip(&theirs).position(|(a, b)| a != b)
    );
    let texts = texts(&at).unwrap();
    let missing: Vec<String> = services::services(&texts)
        .unwrap()
        .into_iter()
        .filter(|s| !image.symbols.contains_key(&s.symbol))
        .map(|s| s.symbol)
        .collect();
    assert!(missing.is_empty(), "the image does not define {missing:?}");
    println!(
        "python_image: key {key}, {} bytes, {} segments, {} symbols",
        ours.len(),
        image.segments.len(),
        image.symbols.len()
    );
}

/// The C files the working tree changes from `FROZEN`'s, each with the sprint and the reason; the image the VM runs
/// is the frozen one with these changes, which their own tests judge (Sprint 272).
const DEPARTURES: [(&str, &str); 14] = [
    (
        "tools/guest_runtime/block_compiler.c",
        "Sprint 284: a dispatch pair stored whole by one stp; Sprint 286: way 0 read whole by one ldp",
    ),
    (
        "tools/guest_runtime/critical_section.c",
        "Sprint 368: w2m_critical_section_leave_once and w2m_critical_section_enter, SleepConditionVariableCS's two \
         halves (slim_locks.c): the section left by its owner entered once, and entered again",
    ),
    (
        "tools/guest_runtime/file.c",
        "Sprint 355: Get/SetHandleInformation on a pipe read and change its flags through w2m_object_flags, which does \
         both under the handle table's lock, not through a pointer into the slots' flags used after the lookup returned",
    ),
    (
        "tools/guest_runtime/heap.c",
        "Sprint 343: the process heap's own lock around its free list, bump pointer and set-up, for its five \
         services, which take no runtime word; Sprint 358: its take and release timed in an image built with \
         W2M_TIME_LOCKS (runtime.h), and w2m_lock_timing_areas, the symbol by which the host knows that image; \
         without the define the image is the one built before, byte for byte; Sprint 359: the heap's take a scoped \
         guard, W2M_HELD(heap) (runtime.h), whose clang cleanup lets the lock go at its scope's end, hold and let_go \
         renamed hold_heap and let_heap_go, no release by hand left",
    ),
    (
        "tools/guest_runtime/processes.c",
        "Sprint 355: w2m_object_flags declared with its new form, the flags read and changed under the handle table's \
         lock; Sprint 370: WaitForMultipleObjectsEx's bAlertable read, its wait sync_objects.c's w2m_wait_multiple_ex",
    ),
    (
        "tools/guest_runtime/runtime.c",
        "Sprint 341: w2m_logged's counts by atomic add and each record at the index its add reserved, for callers on \
         several vCPUs at once; Sprint 342: w2m_unimplemented holds the runtime word before its host request, which \
         a service of lock class none makes without it; Sprint 355: w2m_take_runtime_word and \
         w2m_let_runtime_word_go, the word taken and let go by a service whose bridge holds none, for a named event's \
         EVENT_OPEN and DuplicateHandle's look at the file table; Sprint 358's amendment: in an image built with \
         W2M_TIME_LOCKS, w2m_take_runtime_word's takings timed and counted in the runtime word's timing area as the \
         site runtime, their wait start in the vCPU's own monitor data words; Sprint 359: its compare-and-swap take \
         renamed claim_runtime_word, the name hold_runtime_word now the runtime word's guard's take (runtime.h)",
    ),
    (
        "tools/guest_runtime/runtime.h",
        "Sprint 358: the leaf locks' timed take and release (w2m_lock_take, w2m_lock_let_go) and their four timing \
         areas at the monitor region's end, declared only under W2M_TIME_LOCKS, which the host defines for a run \
         that times its runtime word: a taking's wait start and taken time in its vCPU's own monitor data words, its \
         count by an atomic add, its whole entry written while the lock is held; Sprint 359: the locks' scoped \
         guard, W2M_HELD(lock), a variable whose clang cleanup attribute lets its lock go at its scope's end, and \
         the two named releases by hand, W2M_LET_GO_EARLY before a call of w2m_unimplemented or w2m_logged in the \
         guard's scope and W2M_LET_GO_BEFORE_STOP in a function its caller's guard holds the lock for; the runtime \
         word's guard (hold_runtime_word, runtime_word_guard_end), which lets it go only when its take took it, and \
         w2m_take_runtime_word and w2m_let_runtime_word_go declared here; Sprint 368: \
         w2m_critical_section_leave_once and w2m_critical_section_enter declared; Sprint 369: W2M_KERNEL_NAMED_OPEN, \
         the host's named open by kind (kernel.rs), its kinds, create flag and answers; Sprint 370: W2M_NAMED_TIMER, \
         and the thread scratch's wait_timeout, a wait's own deadline while a timer's time comes first",
    ),
    (
        "tools/guest_runtime/slim_locks.c",
        "Sprint 368: SleepConditionVariableCS, which the game's batch run on two vCPUs called and the runtime lacked \
         (the run stopped \"import KERNELBASE.dll!SleepConditionVariableCS\"), on the SRW form's phases and variables",
    ),
    (
        "tools/guest_runtime/sync_objects.c",
        "Sprint 350: a blocked wait's entry holds a reference to its object, so CloseHandle of its last handle leaves \
         it alive and its index unused until the wait ends, its handles counted apart so the host's closes stay at the \
         last handle's close; a retried WaitForSingleObject waits on its entry's object; the WAIT_FAILED path clears \
         the deadline; Sprint 352: a lookup holds a reference its caller drops, every blocked wait of any kind keeps \
         its lookups' references in its entry and its retry, WaitForMultipleObjects's too, reads its objects there, \
         and WaitForMultipleObjects of a handle naming no object answers WAIT_FAILED and ERROR_INVALID_HANDLE; \
         Sprint 355: the handle table's own lock around its slots, their flags, the parent's handles' install, an \
         entry's allocation and both its counts, for the ten creates, duplicates and id lookups that take no runtime \
         word and inside the word for every other caller; a named event's EVENT_OPEN takes the word in the C; \
         Sprint 356: the sync objects' own lock around a semaphore's, an event's and a mutex's count and owner, the \
         waiters and the waits for all, for the eight waits and releases, which take no runtime word from their \
         bridges and take it in the C to park, to ready and for the host's objects, a wait on one of those holding it \
         from its check to its park; a wait checked under the lock arms the woken mark; a release readies a wait for \
         all naming one of the host's objects to check again, never taking for it; CloseHandle and w2m_object_kind \
         read a count only of the kinds fixed at creation; Sprint 356's amendment: a mutex whose owner thread ends \
         is abandoned (w2m_mutexes_abandoned), granted to its first waiter or taken by the next wait, which answers \
         WAIT_ABANDONED (WAIT_ABANDONED_0 + index for several) once; Sprint 358: the handle table's and the sync \
         objects' takes and releases timed in an image built with W2M_TIME_LOCKS (runtime.h); Sprint 359: every take \
         of the handle table, the sync objects and the runtime word a scoped guard (W2M_HELD, runtime.h; \
         WORD_WHEN_NEEDED for a release that may take the word), each holding a block and let go at its end, the \
         counter's read and the park of a wait moved after its holding's end, and a release by hand only before \
         w2m_unimplemented or w2m_logged, named; Sprint 369: the named sync objects the game declares, CreateMutexW, \
         OpenMutexW, OpenEventW, OpenEventA, OpenSemaphoreW and SignalObjectAndWait, CreateSemaphoreW named, and a \
         name of a mutex or semaphore refused to CreateEventW, held to probes/named_sync_objects' Windows rows; the \
         three releases' state change one function, signal_object, which SignalObjectAndWait's signal shares; Sprint \
         370: QueueUserAPC, CreateWaitableTimerA and SetWaitableTimer, the APC queue under the sync objects' lock, a \
         timer's state brought to the counter when read, and the alertable flag of WaitForSingleObjectEx, \
         WaitForMultipleObjectsEx and SignalObjectAndWait made real, a due APC run through guest_calls.c's CALL stub \
         and the wait answering WAIT_IO_COMPLETION, held to probes/apcs_and_waitable_timers' Windows rows",
    ),
    (
        "tools/guest_runtime/thread_storage.c",
        "Sprint 345: the TLS and FLS slots' own lock around the slot bitmaps, the callbacks, the FLS block list and \
         the TLS set-up, for its seven services, which take no runtime word; Sprint 347: TlsAlloc and TlsFree make \
         their index 0 in every live thread's TEB or expansion array, as Windows reads another thread's value, and \
         give the first thread its expansion array, so it can use an index past 63 another thread allocated; Sprint \
         348: the run table's count read with acquire (LDAPR), pairing with the host's release store of it after a new entry's \
         words, since the slots' lock is not the runtime word the host's writer holds; Sprint 358: the slots' take \
         and release timed in an image built with W2M_TIME_LOCKS (runtime.h); Sprint 359: the slots' take a scoped \
         guard (W2M_HELD, runtime.h) let go at its scope's end, a release by hand only before w2m_unimplemented or \
         w2m_logged, named",
    ),
    (
        "tools/guest_runtime/threads.c",
        "Sprint 353: slice_until answers 0 while another vCPU holds a thread, since a readying or put-down in the VM \
         writes only its own vCPU's SLICE_UNTIL, so a vCPU's slice ends at each TIME_SLICE and its switch looks again; \
         Sprint 354: the woken mark, the run entry's first reserved word, which w2m_ready_thread and w2m_signal set on \
         a thread another vCPU runs and not BLOCKED, and w2m_block finds and clears, leaving the thread READY, so a \
         readying between a wait's check and its park is not lost once card 5 parts them; the switch's load clears it; \
         Sprint 356: the mark armed by a wait at its check (w2m_wait_armed) and set by w2m_ready_thread and w2m_signal \
         only on an armed thread not BLOCKED, by compare-and-swap, w2m_block clearing it by one exchange, so a thread \
         running guest code on another vCPU takes no mark; w2m_wait_again marks the running thread; Sprint 356's \
         amendment: W2mThreadEnd abandons every mutex its thread owns before the host's THREAD_END; Sprint 370: SleepEx \
         alertable with an APC queued or for INFINITE runs the thread's due APCs (sync_objects.c), and W2mThreadEnd \
         drops the thread's APCs and cancels a timer it set with a completion routine",
    ),
    (
        "tools/vm_compiler/vm_compiler.c",
        "Sprint 272: a memory XCHG declined to the host's atomic lowering",
    ),
    ("tools/vm_compiler/x86_decode.c", "Sprint 272: x86_is_xchg"),
    ("tools/vm_compiler/x86_decode.h", "Sprint 272: x86_is_xchg"),
];

/// Sprint 271, from Sprint 272 a list: the working tree's C differs from `FROZEN`'s in the files `DEPARTURES` names
/// and no others, so a change to the C is named with its reason, the frozen judge unmoved.
#[test]
fn the_working_trees_c_departs_from_the_frozen_c_only_where_named() {
    let root = root();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "diff",
            "--name-only",
            FROZEN,
            "--",
            "tools/guest_runtime",
            "tools/vm_compiler",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "git diff {FROZEN}");
    let changed: Vec<&str> = std::str::from_utf8(&out.stdout).unwrap().lines().collect();
    let named: Vec<&str> = DEPARTURES.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        changed, named,
        "the C files changed from {FROZEN}'s, against those named with a reason"
    );
    let (frozen_key, _) = frozen_build_root(&root);
    let here = cache_key(&root, &texts(&root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(
        here, frozen_key,
        "the named departures leave the image's key the frozen one's"
    );
}

#[test]
fn a_define_changed_by_one_names_an_image_python_never_built() {
    let root = root();
    let mut defines = runtime_defines();
    for (k, v) in &mut defines {
        if k == "W2M_TIME_SLICE" {
            *v += 1;
        }
    }
    let key = cache_key(&root, &texts(&root).unwrap(), &defines, RUNTIME_IMAGE_VA).unwrap();
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
    let right = cache_key(&root, &texts(&root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
}

#[test]
fn a_source_byte_changed_names_another_key() {
    let root = root();
    let mut t = texts(&root).unwrap();
    let right = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    t.get_mut("runtime.c").unwrap().push(' ');
    let key = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
}
