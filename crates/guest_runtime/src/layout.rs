// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The guest-memory layout the C runtime is compiled against, carried from the frozen Python host: each constant is
//! the same expression as the line it cites in `src/w2m/vm_backend.py` (`vm_backend` below) or `src/w2m/scheduler.py`
//! (`scheduler`), so a change there is a change here by hand, and the image built from these must equal the Python
//! build's (Sprint 133's judge).

// vm_backend 80-86
pub const IPA_BASE: u64 = 0x1000_0000;
pub const MONITOR_VA: u64 = 0x7F00_0000_0000;
pub const PAGE: u64 = 0x1000;
pub const BLOCK: u64 = 0x20_0000;
pub const HEAP_VA: u64 = 0x0000_0002_0000_0000;
pub const HEAP_SIZE: u64 = 0x1000_0000;
// vm_backend 88-125: the monitor data page's offsets
pub const MONITOR_DATA: u64 = 0x00000;
pub const RECON_COUNT: u64 = 0xC0; // RUN_TABLE + this
pub const MXCSR_OFF: u64 = 0x50;
pub const CLOCK_WARP: u64 = 0xC8; // RUN_TABLE + this
pub const FCW_OFF: u64 = 0x70;
pub const X87_ST_OFF: u64 = 768; // vm_backend 108
pub const PAUSE_COUNT: u64 = 0xA8;
pub const LOCK_OWNER: u64 = 0xB0;
pub const SLICE_UNTIL: u64 = 0xC0; // vm_backend 119
pub const RUN_END: u64 = 0xC8;
pub const CURRENT: u64 = 0xD0; // vm_backend 122
pub const CURRENT_ENTRY: u64 = 0xD8;
pub const SLICE_COUNT: u64 = 0xE0;
pub const SLICE_PAUSES: u64 = 0xE8;
pub const SAVE_AREA: u64 = 0x100; // vm_backend 123
// vm_backend 131-155
pub const TRANSLATION_TABLES: u64 = 0x12000;
pub const CODE_CACHE: u64 = 0x60000;
pub const CODE_CACHE_SIZE: u64 = 0x400_0000;
pub const RUNTIME_STACK: u64 = CODE_CACHE + CODE_CACHE_SIZE + 0x100000;
pub const RUNTIME_STACK_SIZE: u64 = 0x10000;
pub const RUNTIME_IMAGE: u64 = RUNTIME_STACK + RUNTIME_STACK_SIZE;
pub const RUNTIME_IMAGE_SIZE: u64 = 0x800000;
pub const RECON_TABLE: u64 = RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE;
const _: () = assert!(
    RUNTIME_STACK + RUNTIME_STACK_SIZE == 0x4170000,
    "the stencil table's runtime stack top"
);
pub const RECON_ENTRIES: u64 = 0x40_0000;
pub const DISPATCH_BITS: u64 = 20;
pub const DISPATCH_ENTRIES: u64 = 1 << DISPATCH_BITS;
pub const DISPATCH_TABLE: u64 = RECON_TABLE + 16 * RECON_ENTRIES;
// vm_backend 170-191
pub const SPLIT_TABLES: u64 = DISPATCH_TABLE + 16 * DISPATCH_ENTRIES;
pub const SPLIT_TABLES_SIZE: u64 = 0x40_0000;
pub const VECTOR_SLOTS: u64 = SPLIT_TABLES + SPLIT_TABLES_SIZE;
pub const VECTOR_SLOT: u64 = 0x120;
pub const VECTOR_SLOT_COUNT: u64 = 1024;
pub const CHANNEL: u64 = VECTOR_SLOTS + VECTOR_SLOT * VECTOR_SLOT_COUNT;
pub const RUN_TABLE: u64 = CHANNEL + PAGE;
pub const RUN_THREADS: u64 = VECTOR_SLOT_COUNT;
// scheduler 28-49: the run table
pub const TIME_SLICE: u64 = 20_000;
pub const PER_THREAD_MONITOR: [(u64, u64); 4] = [(0x20, 0x40), (0x50, 0x58), (0x70, 0x72), (0x400, 0x500)];
pub const STATE_SIZE: u64 = 896;
pub const RT_ENTRIES: u64 = 0x100;
pub const E_X: u64 = 0x40;
pub const E_MONITOR: u64 = 0x140;
pub const E_STATE: u64 = 0x280;
pub const E_MONITOR_AT: [u64; 4] = [0x00, 0x20, 0x28, 0x30];
pub const RUN_ENTRY: u64 = 0x600;
// vm_backend 196-223
pub const VM_COMPILER: u64 = (RUN_TABLE + RT_ENTRIES + RUN_ENTRY * RUN_THREADS + PAGE - 1) & !(PAGE - 1);
pub const VMC_LOG: u64 = 0x1000;
pub const VMC_LOG_ENTRY: u64 = 0x80;
pub const VMC_LOG_ENTRIES: u64 = 4096;
pub const VM_COMPILER_SIZE: u64 = VMC_LOG + VMC_LOG_ENTRY * VMC_LOG_ENTRIES;
pub const CODE_PAGE_SET: u64 = VM_COMPILER + VM_COMPILER_SIZE;
pub const CODE_PAGE_BITS: u64 = 20;
pub const CODE_PAGE_SLOTS: u64 = 1 << CODE_PAGE_BITS;
pub const MAX_VCPUS: u64 = 8;
pub const VCPU_AREA: u64 = CODE_PAGE_SET + 8 * CODE_PAGE_SLOTS;
pub const VCPU_STRIDE: u64 = 0x20000;
pub const MONITOR_SIZE: u64 = (VCPU_AREA + MAX_VCPUS * VCPU_STRIDE + BLOCK - 1) & !(BLOCK - 1);
// vm_backend 225-247: the process area
pub const PROCESS_VA: u64 = 0x7FFE_0000_0000;
pub const TEB_OFF: u64 = 0x0000;
pub const PEB_OFF: u64 = 0x2000;
pub const STATE_OFF: u64 = 0x3000;
pub const TRANSPORT_OFF: u64 = 0x4000;
pub const RING_OFF: u64 = 0x5000;
pub const RING_SIZE: u64 = 0x40000;
pub const EXCEPTION_OFF: u64 = RING_OFF + RING_SIZE;
pub const KERNEL32_IMAGE_OFF: u64 = EXCEPTION_OFF + 0x1000;
pub const KERNELBASE_IMAGE_OFF: u64 = KERNEL32_IMAGE_OFF + 0x1000;
pub const TLS_EXPANSION_OFF: u64 = KERNELBASE_IMAGE_OFF + 0x1000;
pub const COMMAND_LINE_OFF: u64 = TLS_EXPANSION_OFF + 0x2000;
pub const ENVIRONMENT_OFF: u64 = COMMAND_LINE_OFF + 0x1000;
pub const ENVIRONMENT_SIZE: u64 = 0x8000;
pub const NTDLL_IMAGE_OFF: u64 = ENVIRONMENT_OFF + ENVIRONMENT_SIZE;
pub const LOADER_DATA_OFF: u64 = NTDLL_IMAGE_OFF + 0x1000;
pub const LOADER_DATA_SIZE: u64 = 0x4000;
pub const SYSTEM_IMAGES_OFF: u64 = LOADER_DATA_OFF + LOADER_DATA_SIZE;
pub const SYSTEM_IMAGES: u64 = 32;
pub const INHERIT_OFF: u64 = SYSTEM_IMAGES_OFF + SYSTEM_IMAGES * 0x1000;
pub const FAULT_LOG_OFF: u64 = INHERIT_OFF + 0x1000;
pub const CURRENT_DIRECTORY_OFF: u64 = FAULT_LOG_OFF + 0x1000;
pub const TRACE_OFF: u64 = CURRENT_DIRECTORY_OFF + 0x1000;
// vm_backend 286, 344, 129
pub const INIT_STUB_SLOT: u64 = 0x80;
pub const TLS_TAKEN: u64 = 16;
pub const AP_READ_ONLY: u64 = 1 << 7;

/// The runtime image's link address (vm_backend 1183: `MONITOR_VA + RUNTIME_IMAGE`).
pub const RUNTIME_IMAGE_VA: u64 = MONITOR_VA + RUNTIME_IMAGE;

/// The defines the runtime is compiled against: `vm_backend.runtime_defines()` (1075-1104), name for name.
pub fn runtime_defines() -> Vec<(String, u64)> {
    let mut d: Vec<(String, u64)> = [
        ("W2M_TEB_VA", PROCESS_VA + TEB_OFF),
        ("W2M_PEB_VA", PROCESS_VA + PEB_OFF),
        ("W2M_TRANSPORT_VA", PROCESS_VA + TRANSPORT_OFF),
        ("W2M_RING_VA", PROCESS_VA + RING_OFF),
        ("W2M_STATE_VA", PROCESS_VA + STATE_OFF),
        ("W2M_EXCEPTION_VA", PROCESS_VA + EXCEPTION_OFF),
        ("W2M_TLS_EXPANSION_VA", PROCESS_VA + TLS_EXPANSION_OFF),
        ("W2M_TLS_TAKEN", TLS_TAKEN),
        ("W2M_HEAP_VA", HEAP_VA),
        ("W2M_HEAP_SIZE", HEAP_SIZE),
        ("W2M_COMMAND_LINE_VA", PROCESS_VA + COMMAND_LINE_OFF),
        ("W2M_INHERIT_VA", PROCESS_VA + INHERIT_OFF),
        ("W2M_CURRENT_DIRECTORY_VA", PROCESS_VA + CURRENT_DIRECTORY_OFF),
        ("W2M_FAULT_LOG_VA", PROCESS_VA + FAULT_LOG_OFF),
        ("W2M_TRACE_VA", PROCESS_VA + TRACE_OFF),
        ("W2M_ENVIRONMENT_VA", PROCESS_VA + ENVIRONMENT_OFF),
        ("W2M_ENVIRONMENT_SIZE", ENVIRONMENT_SIZE),
        ("W2M_INIT_STUB_SLOT", INIT_STUB_SLOT),
        ("W2M_MONITOR_DATA_VA", MONITOR_VA + MONITOR_DATA),
        ("W2M_SAVE_VA", MONITOR_VA + MONITOR_DATA + SAVE_AREA),
        ("W2M_RECON_VA", MONITOR_VA + RECON_TABLE),
        ("W2M_RECON_COUNT_VA", MONITOR_VA + RUN_TABLE + RECON_COUNT),
        ("W2M_CODE_START", MONITOR_VA + CODE_CACHE),
        ("W2M_CODE_END", MONITOR_VA + CODE_CACHE + CODE_CACHE_SIZE),
        ("W2M_RUN_TABLE_VA", MONITOR_VA + RUN_TABLE),
        ("W2M_RUN_ENTRIES_OFF", RT_ENTRIES),
        ("W2M_RUN_ENTRY", RUN_ENTRY),
        ("W2M_RUN_X_OFF", E_X),
        ("W2M_RUN_MONITOR_OFF", E_MONITOR),
        ("W2M_RUN_STATE_OFF", E_STATE),
        ("W2M_STATE_SIZE", STATE_SIZE),
        ("W2M_TIME_SLICE", TIME_SLICE),
        ("W2M_SLICE_UNTIL_OFF", SLICE_UNTIL),
        ("W2M_RUN_END_OFF", RUN_END),
        ("W2M_CURRENT_OFF", CURRENT),
        ("W2M_CURRENT_ENTRY_OFF", CURRENT_ENTRY),
        ("W2M_LOCK_OWNER_OFF", LOCK_OWNER),
        ("W2M_SLICE_COUNT_OFF", SLICE_COUNT),
        ("W2M_SLICE_PAUSES_OFF", SLICE_PAUSES),
        ("W2M_CLOCK_WARP_VA", MONITOR_VA + RUN_TABLE + CLOCK_WARP),
        ("W2M_PAUSE_COUNT_OFF", PAUSE_COUNT),
        ("W2M_FCW_OFF", FCW_OFF),
        ("W2M_MXCSR_OFF", MXCSR_OFF),
        ("W2M_X87_ST_OFF", X87_ST_OFF),
        ("W2M_MONITOR_VA", MONITOR_VA),
        ("W2M_VMC_VA", MONITOR_VA + VM_COMPILER),
        ("W2M_VMC_LOG_ENTRIES", VMC_LOG_ENTRIES),
        ("W2M_CODE_PAGE_SET_VA", MONITOR_VA + CODE_PAGE_SET),
        ("W2M_CODE_PAGE_BITS", CODE_PAGE_BITS),
        ("W2M_DISPATCH_VA", MONITOR_VA + DISPATCH_TABLE),
        ("W2M_DISPATCH_BITS", DISPATCH_BITS),
        ("W2M_RECON_ENTRIES", RECON_ENTRIES),
        ("W2M_TRANSLATION_TABLES_OFF", TRANSLATION_TABLES),
        ("W2M_IPA_BASE", IPA_BASE),
        ("W2M_PROCESS_VA", PROCESS_VA),
        ("W2M_MONITOR_SIZE", MONITOR_SIZE),
        ("W2M_AP_READ_ONLY", AP_READ_ONLY),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    // vm_backend 1103-1104: each per-thread monitor range's start, end and place, W2M_RUN_MONITOR_0 on
    let ranges = PER_THREAD_MONITOR
        .iter()
        .zip(E_MONITOR_AT)
        .flat_map(|(&(a, b), at)| [a, b, at]);
    d.extend(ranges.enumerate().map(|(i, v)| (format!("W2M_RUN_MONITOR_{i}"), v)));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chained_offsets_land_where_the_python_host_puts_them() {
        assert_eq!(RUNTIME_IMAGE, 0x417_0000);
        assert_eq!(RECON_TABLE, 0x497_0000);
        assert_eq!(DISPATCH_TABLE, 0x897_0000);
        assert_eq!(RUN_TABLE, 0x9DB_9000);
        assert_eq!(VM_COMPILER, 0x9F3_A000);
        assert_eq!(MONITOR_SIZE, 0xAA0_0000);
        assert_eq!(TRACE_OFF, 0x7_B000);
        assert_eq!(runtime_defines().len(), 57 + 12);
    }
}
