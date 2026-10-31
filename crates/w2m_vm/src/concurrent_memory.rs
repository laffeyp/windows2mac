// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Guest memory that other vCPUs write while the host reads or writes it (Sprint 276,
//! `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`). A `&[u8]` or `&mut [u8]` over such memory breaks Rust's
//! rule that nothing changes what a reference covers, so this type makes none, as `vm-memory`'s `VolatileSlice` makes
//! none for Firecracker and Cloud Hypervisor. Bytes no running vCPU writes at the time (the guest's protocol says
//! which: under the runtime word, or not yet published) are copied through the pointer; a word running guest code may
//! write is reached as an atomic. `vm-memory` copies by volatile accesses, which Rust's documentation says change
//! nothing for concurrency, so the copies here are plain. A slice of guest memory stays sound only while every vCPU
//! that may write it is stopped, as the one-vCPU run loop's are at each exit.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, AtomicU64};

/// A view of `len` bytes of guest memory from `ptr`, shared by copy among the threads that serve vCPUs.
#[derive(Clone, Copy)]
pub struct ConcurrentGuestMemory<'a> {
    ptr: *mut u8,
    len: usize,
    _memory: PhantomData<&'a mut [u8]>,
}

// SAFETY: the view makes no reference into the memory; every access is a copy through the raw pointer or an atomic,
// and the memory outlives `'a`, so several threads may hold copies of it as several vCPUs reach the memory at once
unsafe impl Send for ConcurrentGuestMemory<'_> {}
unsafe impl Sync for ConcurrentGuestMemory<'_> {}

impl<'a> ConcurrentGuestMemory<'a> {
    /// The view of `memory`, which it borrows for `'a`: the host makes no other reference to it meanwhile.
    pub fn new(memory: &'a mut [u8]) -> ConcurrentGuestMemory<'a> {
        ConcurrentGuestMemory {
            ptr: memory.as_mut_ptr(),
            len: memory.len(),
            _memory: PhantomData,
        }
    }

    /// The view of `len` bytes from `ptr`.
    ///
    /// # Safety
    ///
    /// The bytes stay mapped, readable and writable for `'a`, and no reference to them is used while a copy of the view
    /// is: a slice of them is made only while no thread holds the view and every vCPU that may write them is stopped.
    pub unsafe fn from_raw_parts(ptr: *mut u8, len: usize) -> ConcurrentGuestMemory<'a> {
        ConcurrentGuestMemory {
            ptr,
            len,
            _memory: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn at(&self, offset: u64, size: usize) -> *mut u8 {
        let off = usize::try_from(offset)
            .ok()
            .filter(|o| o.checked_add(size).is_some_and(|end| end <= self.len));
        let off = off.unwrap_or_else(|| panic!("guest memory 0x{offset:x}+{size} past its 0x{:x} bytes", self.len));
        // SAFETY: `off + size` is within the `len` bytes the view covers
        unsafe { self.ptr.add(off) }
    }

    /// Copies `out.len()` bytes from `offset` into `out`.
    pub fn read(&self, offset: u64, out: &mut [u8]) {
        let from = self.at(offset, out.len());
        // SAFETY: in bounds by `at`; `out` is the host's own buffer, never inside guest memory
        unsafe { std::ptr::copy_nonoverlapping(from, out.as_mut_ptr(), out.len()) }
    }

    /// Copies `bytes` to `offset`.
    pub fn write(&self, offset: u64, bytes: &[u8]) {
        let to = self.at(offset, bytes.len());
        // SAFETY: in bounds by `at`; `bytes` is the host's own buffer, never inside guest memory
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), to, bytes.len()) }
    }

    pub fn read_u32(&self, offset: u64) -> u32 {
        let mut b = [0u8; 4];
        self.read(offset, &mut b);
        u32::from_le_bytes(b)
    }

    pub fn read_u64(&self, offset: u64) -> u64 {
        let mut b = [0u8; 8];
        self.read(offset, &mut b);
        u64::from_le_bytes(b)
    }

    pub fn write_u32(&self, offset: u64, v: u32) {
        self.write(offset, &v.to_le_bytes());
    }

    pub fn write_u64(&self, offset: u64, v: u64) {
        self.write(offset, &v.to_le_bytes());
    }

    /// The 32-bit word at `offset`, 4-byte aligned, as an atomic: a word running guest code reads or writes by its own
    /// atomics (`CASA`, `LDAR`, `STLR`) is the same physical word.
    pub fn atomic_u32(&self, offset: u64) -> &'a AtomicU32 {
        let p = self.at(offset, 4);
        assert!(
            p.cast::<u32>().is_aligned(),
            "an atomic word at 0x{offset:x}, not 4-byte aligned"
        );
        // SAFETY: in bounds, aligned, and live for `'a`; the word is reached only by atomics of its own size while
        // threads share it
        unsafe { AtomicU32::from_ptr(p.cast()) }
    }

    /// Stores `(low, high)` at `offset`, 16-byte aligned, as one access after a release fence: with FEAT_LSE2 an aligned
    /// `STP` of two 64-bit registers to Normal write-back memory is single-copy atomic (DDI 0487 B2.2.1.1; QEMU's
    /// `atomic128-ldst.h` relies on the same), so a guest `LDP` of the pair sees the old pair or the new, never half.
    /// This host reports `hw.optional.arm.FEAT_LSE2: 1`, and the guest maps its memory Normal write-back (`MAIR`).
    #[cfg(target_arch = "aarch64")]
    pub fn store_pair_release(&self, offset: u64, low: u64, high: u64) {
        let p = self.at(offset, 16);
        assert!(
            p.cast::<u128>().is_aligned(),
            "a pair at 0x{offset:x}, not 16-byte aligned"
        );
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
        // SAFETY: in bounds and 16-byte aligned; the pair is reached only by whole-pair accesses while threads share it
        unsafe {
            std::arch::asm!("stp {low}, {high}, [{p}]", low = in(reg) low, high = in(reg) high, p = in(reg) p,
                options(nostack, preserves_flags));
        }
    }

    /// The 64-bit word at `offset`, 8-byte aligned, as an atomic.
    pub fn atomic_u64(&self, offset: u64) -> &'a AtomicU64 {
        let p = self.at(offset, 8);
        assert!(
            p.cast::<u64>().is_aligned(),
            "an atomic word at 0x{offset:x}, not 8-byte aligned"
        );
        // SAFETY: as `atomic_u32`'s
        unsafe { AtomicU64::from_ptr(p.cast()) }
    }
}

#[cfg(test)]
mod tests {
    use super::ConcurrentGuestMemory;
    use std::sync::atomic::Ordering;

    /// Sprint 276: a word written through the view reads back as bytes, as a word and as an atomic (plant 2 reads 4
    /// bytes past the offset).
    #[test]
    fn a_word_written_reads_back_through_every_path() {
        let mut buf = vec![0u64; 4];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 32) };
        let mem = ConcurrentGuestMemory::new(bytes);
        mem.write_u64(8, 0x1122_3344_5566_7788);
        mem.write_u32(16, 0xAABB_CCDD);
        assert_eq!(mem.read_u64(8), 0x1122_3344_5566_7788, "the word read at 8");
        assert_eq!(mem.read_u32(16), 0xAABB_CCDD, "the word read at 16");
        assert_eq!(mem.atomic_u64(8).load(Ordering::Acquire), 0x1122_3344_5566_7788);
        mem.atomic_u32(16).store(7, Ordering::Release);
        let mut out = [0u8; 4];
        mem.read(16, &mut out);
        assert_eq!(out, 7u32.to_le_bytes());
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn a_pair_stored_reads_back_low_then_high() {
        let mut buf = vec![0u128; 2];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 32) };
        let mem = ConcurrentGuestMemory::new(bytes);
        mem.store_pair_release(16, 0x1111, 0x2222);
        assert_eq!((mem.read_u64(16), mem.read_u64(24)), (0x1111, 0x2222));
    }

    #[test]
    #[should_panic(expected = "not 8-byte aligned")]
    fn a_misaligned_atomic_is_refused() {
        let mut buf = vec![0u64; 2];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 16) };
        ConcurrentGuestMemory::new(bytes).atomic_u64(4);
    }

    #[test]
    #[should_panic(expected = "past its 0x10 bytes")]
    fn an_access_past_the_end_is_refused() {
        let mut buf = vec![0u8; 16];
        ConcurrentGuestMemory::new(&mut buf).read_u64(12);
    }
}
