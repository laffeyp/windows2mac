// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Guest memory that other vCPUs write while the host reads or writes it (Sprint 276,
//! `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`). A `&[u8]` or `&mut [u8]` over such memory breaks Rust's
//! rule that nothing changes what a reference covers, so this type makes none, as `vm-memory`'s `VolatileSlice` makes
//! none for Firecracker and Cloud Hypervisor. Bytes are copied by relaxed atomic loads and stores, a word at a time
//! where aligned and a byte at the ends (Sprint 279): a guest thread writing a buffer the host copies, as Windows lets
//! it, gives torn bytes, defined, not a data race, which Rust's model makes undefined for any non-atomic access. This is
//! RFC 3301's "atomic memcpy" (https://github.com/rust-lang/rfcs/pull/3301, open): "All of the data would have to be
//! written and read through atomic operations to prevent a data race … it's fine if there's tearing"; C++'s P1478 is
//! the same. `vm-memory` copies by volatile accesses, which Rust's documentation says change nothing for concurrency.
//! A word whose ordering matters is reached by `atomic_u32` or `atomic_u64` with its ordering; no bulk copy covers such
//! a word while another host thread reaches it, as Rust forbids atomics of different sizes racing on the same bytes. A
//! slice of guest memory stays sound only while every vCPU that may write it is stopped, as the one-vCPU run loop's are
//! at each exit.

use std::marker::PhantomData;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64};

/// A view of `len` bytes of guest memory from `ptr`, shared by copy among the threads that serve vCPUs.
///
/// Code outside the crate's tests builds one from the mapping or its pointer, never from a slice (Sprint 292): `new`
/// is a test's only, so the second sample does not compile. The first, the same but for `from_raw_parts`, compiles
/// and runs, so the second can fail for `new` alone: a bare `compile_fail` passes on any error, and stable rustdoc
/// reads `compile_fail,E0599` as plain text (the rustdoc book, "Unstable features").
///
/// ```
/// let mut buffer = [0u8; 16];
/// // SAFETY: the buffer outlives the view, and nothing else touches it meanwhile
/// let view = unsafe { w2m_vm::concurrent_memory::ConcurrentGuestMemory::from_raw_parts(buffer.as_mut_ptr(), 16) };
/// assert_eq!(view.len(), 16);
/// ```
///
/// ```compile_fail
/// let mut buffer = [0u8; 16];
/// let view = w2m_vm::concurrent_memory::ConcurrentGuestMemory::new(&mut buffer);
/// assert_eq!(view.len(), 16);
/// ```
#[derive(Clone, Copy)]
pub struct ConcurrentGuestMemory<'a> {
    ptr: *mut u8,
    len: usize,
    _memory: PhantomData<&'a mut [u8]>,
}

// SAFETY: the view makes no reference into the memory; every access is an atomic load or store through the pointer,
// and the memory outlives `'a`, so several threads may hold copies of it as several vCPUs reach the memory at once
unsafe impl Send for ConcurrentGuestMemory<'_> {}
unsafe impl Sync for ConcurrentGuestMemory<'_> {}

impl<'a> ConcurrentGuestMemory<'a> {
    /// The view of `memory`, which it borrows for `'a`: the host makes no other reference to it meanwhile. A test's
    /// own buffer only (Sprint 292; reviewer-3's Low 1 at 278): the host's views come from the mapping
    /// (`guest::memory_view`) or its pointer (`from_raw_parts`), so none rests on a slice a running vCPU may write; the
    /// type's doc test holds it so (a doc test here would vanish with the item outside tests).
    #[cfg(test)]
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

    /// Copies `out.len()` bytes from `offset` into `out`, by relaxed atomic loads: a byte a guest writes meanwhile reads
    /// as its old or new value.
    pub fn read(&self, offset: u64, out: &mut [u8]) {
        let from = self.at(offset, out.len());
        let (n, mut i) = (out.len(), 0);
        // SAFETY (each load): in bounds by `at`; a `u64` load only where `from + i` is 8-byte aligned; the bytes live
        // for `'a`, and the host reaches them only by atomics while threads share the view
        while i < n && !(from as usize + i).is_multiple_of(8) {
            out[i] = unsafe { AtomicU8::from_ptr(from.add(i)) }.load(Relaxed);
            i += 1;
        }
        while i + 8 <= n {
            let word = unsafe { AtomicU64::from_ptr(from.add(i).cast()) }.load(Relaxed);
            out[i..i + 8].copy_from_slice(&word.to_ne_bytes());
            i += 8;
        }
        while i < n {
            out[i] = unsafe { AtomicU8::from_ptr(from.add(i)) }.load(Relaxed);
            i += 1;
        }
    }

    /// Copies `bytes` to `offset`, by relaxed atomic stores.
    pub fn write(&self, offset: u64, bytes: &[u8]) {
        let to = self.at(offset, bytes.len());
        let (n, mut i) = (bytes.len(), 0);
        // SAFETY (each store): as `read`'s loads
        while i < n && !(to as usize + i).is_multiple_of(8) {
            unsafe { AtomicU8::from_ptr(to.add(i)) }.store(bytes[i], Relaxed);
            i += 1;
        }
        while i + 8 <= n {
            let word = u64::from_ne_bytes(bytes[i..i + 8].try_into().unwrap());
            unsafe { AtomicU64::from_ptr(to.add(i).cast()) }.store(word, Relaxed);
            i += 8;
        }
        while i < n {
            unsafe { AtomicU8::from_ptr(to.add(i)) }.store(bytes[i], Relaxed);
            i += 1;
        }
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

    /// The pair at `offset`, 16-byte aligned, `(low, high)`, read by one `LDP`, single-copy atomic with FEAT_LSE2 as
    /// `store_pair_release`'s `STP` is, after which an acquire fence (Sprint 286): a dispatch pair moved from one way to
    /// the other is read whole, whatever the dispatcher or another writer does meanwhile.
    #[cfg(target_arch = "aarch64")]
    pub fn load_pair(&self, offset: u64) -> (u64, u64) {
        let p = self.at(offset, 16);
        assert!(
            p.cast::<u128>().is_aligned(),
            "a pair at 0x{offset:x}, not 16-byte aligned"
        );
        let (low, high): (u64, u64);
        // SAFETY: in bounds and 16-byte aligned; the pair is reached only by whole-pair accesses while threads share it
        unsafe {
            std::arch::asm!("ldp {low}, {high}, [{p}]", low = out(reg) low, high = out(reg) high, p = in(reg) p,
                options(nostack, preserves_flags, readonly));
        }
        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
        (low, high)
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

    /// Sprint 279: a copy of every length 0 to 40 at every offset 0 to 15 reads back what was written, the head, the
    /// aligned words and the tail each exercised (plant 1 stops the word loop one word early).
    #[test]
    fn copies_of_every_length_and_offset_read_back() {
        let mut buf = vec![0u64; 8];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 64) };
        let mem = ConcurrentGuestMemory::new(bytes);
        for offset in 0..16u64 {
            for len in 0..=40usize {
                let data: Vec<u8> = (0..len)
                    .map(|k| (k as u8).wrapping_mul(7).wrapping_add(offset as u8 + 1))
                    .collect();
                mem.write(0, &[0; 64]);
                mem.write(offset, &data);
                let mut back = vec![0; len];
                mem.read(offset, &mut back);
                assert_eq!(back, data, "length {len} at offset {offset}");
                let mut around = [0u8; 64];
                mem.read(0, &mut around);
                assert!(
                    around[..offset as usize]
                        .iter()
                        .chain(&around[offset as usize + len..])
                        .all(|b| *b == 0),
                    "length {len} at offset {offset} wrote outside its bytes"
                );
            }
        }
    }

    /// Sprint 279: a copy out of a buffer another thread writes meanwhile gives each byte one of the values written,
    /// never a data race: the guest's thread writing a buffer the host copies, as Windows allows.
    #[test]
    fn a_copy_racing_a_writer_reads_only_bytes_written() {
        let mut buf = vec![0x1111_1111_1111_1111u64; 8];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 64) };
        let mem = ConcurrentGuestMemory::new(bytes);
        std::thread::scope(|s| {
            s.spawn(|| {
                for k in 0..20_000u32 {
                    mem.write(3, &[if k % 2 == 0 { 0x22 } else { 0x11 }; 58]);
                }
            });
            for _ in 0..20_000 {
                let mut out = [0u8; 58];
                mem.read(3, &mut out);
                assert!(
                    out.iter().all(|b| *b == 0x11 || *b == 0x22),
                    "a byte never written: {out:?}"
                );
            }
        });
    }

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
        // Sprint 286: the pair read back whole, low then high (plant 2 swaps them)
        assert_eq!(mem.load_pair(16), (0x1111, 0x2222), "the pair load_pair read");
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
