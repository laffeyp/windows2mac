// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Hypervisor.framework from Rust (Sprint 132): a `Vm` the process owns, a `Vcpu` that lives and dies on the thread
//! that created it, and `GuestMemory`, anonymous memory of the process mapped into the VM.
//!
//! Each `extern` declaration is copied from the macOS 26.5 SDK's headers (`Hypervisor.framework/Headers/hv_vm.h`,
//! `hv_vm_config.h`, `hv_vcpu.h`, `hv_vcpu_types.h`; `usr/include/arm64/hv/hv_kern_types.h`), as the bridge mapping in
//! WORKING_AGREEMENT requires; none is written from memory. The rules the types keep, from the same headers:
//! - `hv_vm_create` "Creates a VM instance for the current process", so one `Vm` exists at a time;
//! - `hv_vm_destroy` "Requires all vCPUs be destroyed", so a `Vcpu` borrows its `Vm`;
//! - `hv_vcpu_create` "Creates a vCPU instance for the current thread", and every other vCPU call "Must be called by
//!   the owning thread", so a `Vcpu` is neither `Send` nor `Sync`;
//! - `hv_vcpus_exit` may be called from any thread, so a `VcpuHandle` is both.
//!
//! On any other target the crate is empty: CI runs on Linux, and the VM's tests run in the gates on this Mac.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

mod ffi {
    use std::ffi::c_void;

    /// `hv_return_t`: a `mach_error_t`, an `int` (`hv_kern_types.h`).
    pub type HvReturn = i32;
    pub type HvVmConfig = *mut c_void;
    pub type HvVcpu = u64;

    /// `hv_vcpu_exit_exception_t` (`hv_vcpu_types.h`).
    #[repr(C)]
    pub struct HvVcpuExitException {
        pub syndrome: u64,
        pub virtual_address: u64,
        pub physical_address: u64,
    }

    /// `hv_vcpu_exit_t` (`hv_vcpu_types.h`).
    #[repr(C)]
    pub struct HvVcpuExit {
        pub reason: u32,
        pub exception: HvVcpuExitException,
    }

    #[link(name = "Hypervisor", kind = "framework")]
    unsafe extern "C" {
        pub fn hv_vm_config_create() -> HvVmConfig;
        pub fn hv_vm_config_set_ipa_size(config: HvVmConfig, ipa_bit_length: u32) -> HvReturn;
        pub fn hv_vm_config_set_ipa_granule(config: HvVmConfig, granule: u32) -> HvReturn;
        pub fn hv_vm_create(config: HvVmConfig) -> HvReturn;
        pub fn hv_vm_destroy() -> HvReturn;
        pub fn hv_vm_map(addr: *mut c_void, ipa: u64, size: usize, flags: u64) -> HvReturn;
        pub fn hv_vm_unmap(ipa: u64, size: usize) -> HvReturn;
        pub fn hv_vcpu_create(vcpu: *mut HvVcpu, exit: *mut *mut HvVcpuExit, config: *mut c_void) -> HvReturn;
        pub fn hv_vcpu_destroy(vcpu: HvVcpu) -> HvReturn;
        pub fn hv_vcpu_run(vcpu: HvVcpu) -> HvReturn;
        pub fn hv_vcpus_exit(vcpus: *mut HvVcpu, vcpu_count: u32) -> HvReturn;
        pub fn hv_vcpu_get_reg(vcpu: HvVcpu, reg: u32, value: *mut u64) -> HvReturn;
        pub fn hv_vcpu_set_reg(vcpu: HvVcpu, reg: u32, value: u64) -> HvReturn;
        pub fn hv_vcpu_get_sys_reg(vcpu: HvVcpu, reg: u16, value: *mut u64) -> HvReturn;
        pub fn hv_vcpu_set_sys_reg(vcpu: HvVcpu, reg: u16, value: u64) -> HvReturn;
        pub fn hv_vcpu_set_vtimer_mask(vcpu: HvVcpu, vtimer_is_masked: bool) -> HvReturn;
        pub fn hv_vcpu_get_simd_fp_reg(vcpu: HvVcpu, reg: u32, value: *mut c_void) -> HvReturn;
        /// `hv_return_t hv_vcpu_set_simd_fp_reg(hv_vcpu_t vcpu, hv_simd_fp_reg_t reg, hv_simd_fp_uchar16_t value)`.
        /// The value is `ext_vector_type(16) uint8_t` by value, a 128-bit short vector, which AAPCS64 passes in v0;
        /// stable Rust takes no SIMD type in an `extern "C"` signature (`simd_ffi`), so `Vcpu::set_simd` calls it from
        /// assembly and the declaration only names the symbol.
        pub fn hv_vcpu_set_simd_fp_reg();
    }

    unsafe extern "C" {
        /// `os_release` (`os/object.h`): the VM configuration is an OS object, released when no longer used
        /// (`hv_vm_config.h`).
        pub fn os_release(object: *mut c_void);
        /// `sys/mman.h`.
        pub fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
        pub fn munmap(addr: *mut c_void, len: usize) -> i32;
    }

    /// `sys/mman.h`.
    pub const PROT_READ: i32 = 0x01;
    pub const PROT_WRITE: i32 = 0x02;
    pub const MAP_PRIVATE: i32 = 0x0002;
    pub const MAP_ANON: i32 = 0x1000;
    /// `hv_vm_config.h`: `HV_IPA_GRANULE_4KB`, the first value of the `hv_ipa_granule` enum.
    pub const HV_IPA_GRANULE_4KB: u32 = 0;
}

/// An error a Hypervisor.framework call returned, with the call's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HvError {
    pub call: &'static str,
    pub code: i32,
}

impl HvError {
    /// `hv_kern_types.h`'s name for the code, when it has one.
    pub fn name(&self) -> &'static str {
        match self.code as u32 {
            0xfae9_4001 => "HV_ERROR",
            0xfae9_4002 => "HV_BUSY",
            0xfae9_4003 => "HV_BAD_ARGUMENT",
            0xfae9_4004 => "HV_ILLEGAL_GUEST_STATE",
            0xfae9_4005 => "HV_NO_RESOURCES",
            0xfae9_4006 => "HV_NO_DEVICE",
            0xfae9_4007 => "HV_DENIED",
            0xfae9_4008 => "HV_EXISTS",
            0xfae9_400f => "HV_UNSUPPORTED",
            _ => "unknown",
        }
    }
}

impl fmt::Display for HvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({:#x})", self.call, self.name(), self.code as u32)
    }
}

impl std::error::Error for HvError {}

fn check(call: &'static str, code: ffi::HvReturn) -> Result<(), HvError> {
    if code == 0 { Ok(()) } else { Err(HvError { call, code }) }
}

/// Guest memory permissions for `Vm::map` (`hv_kern_types.h`).
pub mod memory {
    pub const READ: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const EXEC: u64 = 1 << 2;
}

/// `hv_reg_t` (`hv_vcpu_types.h`): X0 to X30 are 0 to 30.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    X(u8),
    Pc,
    Fpcr,
    Fpsr,
    Cpsr,
}

impl Reg {
    fn id(self) -> u32 {
        match self {
            Reg::X(n) => {
                assert!(n <= 30, "x{n} is not a register");
                u32::from(n)
            }
            Reg::Pc => 31,
            Reg::Fpcr => 32,
            Reg::Fpsr => 33,
            Reg::Cpsr => 34,
        }
    }
}

/// `hv_sys_reg_t` ids (`hv_vcpu_types.h`), those the host sets or reads.
pub mod sys_reg {
    pub const MPIDR_EL1: u16 = 0xc005;
    pub const SCTLR_EL1: u16 = 0xc080;
    pub const ACTLR_EL1: u16 = 0xc081;
    pub const CPACR_EL1: u16 = 0xc082;
    pub const TTBR0_EL1: u16 = 0xc100;
    pub const TTBR1_EL1: u16 = 0xc101;
    pub const TCR_EL1: u16 = 0xc102;
    pub const SPSR_EL1: u16 = 0xc200;
    pub const ELR_EL1: u16 = 0xc201;
    pub const SP_EL0: u16 = 0xc208;
    pub const ESR_EL1: u16 = 0xc290;
    pub const FAR_EL1: u16 = 0xc300;
    pub const MAIR_EL1: u16 = 0xc510;
    pub const VBAR_EL1: u16 = 0xc600;
    pub const TPIDR_EL1: u16 = 0xc684;
    pub const TPIDR_EL0: u16 = 0xde82;
    pub const SP_EL1: u16 = 0xe208;
}

/// Why `hv_vcpu_run` returned (`hv_exit_reason_t`, `hv_vcpu_types.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// "asynchronous exit requested explicitly by hv_vcpus_exit() call"
    Canceled,
    /// "synchronous exception to a higher EL triggered by the guest"; `syndrome` is ESR_EL2's.
    Exception {
        syndrome: u64,
        virtual_address: u64,
        physical_address: u64,
    },
    /// The VTimer became pending; the framework masks it.
    VtimerActivated,
    Unknown,
}

impl Exit {
    /// The exception class (ESR bits 31:26), for an exception.
    pub fn exception_class(&self) -> Option<u8> {
        match self {
            Exit::Exception { syndrome, .. } => Some(((syndrome >> 26) & 0x3f) as u8),
            _ => None,
        }
    }

    /// The immediate of an `hvc` (ESR's ISS bits 15:0 when the class is 0x16).
    pub fn hvc_immediate(&self) -> Option<u16> {
        match (self.exception_class(), self) {
            (Some(0x16), Exit::Exception { syndrome, .. }) => Some((syndrome & 0xffff) as u16),
            _ => None,
        }
    }
}

static VM_EXISTS: AtomicBool = AtomicBool::new(false);

/// The process's VM. One at a time: `hv_vm_create` makes the VM "for the current process". Destroyed on drop, which
/// the borrow checker places after every `Vcpu` built on it.
pub struct Vm {
    _not_send: PhantomData<*const ()>,
}

impl Vm {
    /// A VM with 4 KB IPA granules (the default is 16 KB, which refuses a 4 KB map; WORKING_AGREEMENT) and an IPA of
    /// `ipa_bits` bits (`vm_monitor.c` uses 40).
    pub fn new(ipa_bits: u32) -> Result<Vm, HvError> {
        if VM_EXISTS.swap(true, Ordering::AcqRel) {
            return Err(HvError {
                call: "hv_vm_create",
                code: 0xfae9_4008_u32 as i32,
            });
        }
        let made = unsafe {
            let config = ffi::hv_vm_config_create();
            let r = check(
                "hv_vm_config_set_ipa_granule",
                ffi::hv_vm_config_set_ipa_granule(config, ffi::HV_IPA_GRANULE_4KB),
            )
            .and_then(|()| {
                check(
                    "hv_vm_config_set_ipa_size",
                    ffi::hv_vm_config_set_ipa_size(config, ipa_bits),
                )
            })
            .and_then(|()| check("hv_vm_create", ffi::hv_vm_create(config)));
            ffi::os_release(config);
            r
        };
        match made {
            Ok(()) => Ok(Vm { _not_send: PhantomData }),
            Err(e) => {
                VM_EXISTS.store(false, Ordering::Release);
                Err(e)
            }
        }
    }

    /// Maps `memory` at `ipa` with `flags` (`memory::READ` and the rest). The memory must outlive the mapping; a
    /// `GuestMemory` borrowed for the VM's life does.
    pub fn map(&self, memory: &GuestMemory, ipa: u64, flags: u64) -> Result<(), HvError> {
        check("hv_vm_map", unsafe {
            ffi::hv_vm_map(memory.ptr.cast(), ipa, memory.len, flags)
        })
    }

    pub fn unmap(&self, ipa: u64, size: usize) -> Result<(), HvError> {
        check("hv_vm_unmap", unsafe { ffi::hv_vm_unmap(ipa, size) })
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        let r = unsafe { ffi::hv_vm_destroy() };
        debug_assert_eq!(r, 0, "hv_vm_destroy");
        VM_EXISTS.store(false, Ordering::Release);
    }
}

/// Anonymous memory of the process, zeroed, to be mapped into the VM. Freed by the kernel when the process ends,
/// however it ends, so a VM leaves no file behind (the host design's requirement 2).
pub struct GuestMemory {
    ptr: *mut u8,
    len: usize,
}

impl GuestMemory {
    /// `len` bytes, a multiple of the host's 16 KB page.
    pub fn new(len: usize) -> std::io::Result<GuestMemory> {
        assert!(
            len > 0 && len.is_multiple_of(16 * 1024),
            "guest memory is a whole number of 16 KB pages"
        );
        let p = unsafe {
            ffi::mmap(
                std::ptr::null_mut(),
                len,
                ffi::PROT_READ | ffi::PROT_WRITE,
                ffi::MAP_PRIVATE | ffi::MAP_ANON,
                -1,
                0,
            )
        };
        if p as isize == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(GuestMemory { ptr: p.cast(), len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Writes little-endian 32-bit words at `offset`: arm64 instructions.
    pub fn write_words(&mut self, offset: usize, words: &[u32]) {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        self.as_mut_slice()[offset..offset + bytes.len()].copy_from_slice(&bytes);
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        unsafe { ffi::munmap(self.ptr.cast::<c_void>(), self.len) };
    }
}

/// A vCPU of `Vm`, on the thread that created it. Not `Send`: every call but `hv_vcpus_exit` "Must be called by the
/// owning thread" (`hv_vcpu.h`).
///
/// ```compile_fail
/// let vm = hypervisor::Vm::new(40).unwrap();
/// let vcpu = hypervisor::Vcpu::new(&vm).unwrap();
/// std::thread::scope(|s| {
///     s.spawn(move || vcpu.run());
/// });
/// ```
pub struct Vcpu<'vm> {
    id: ffi::HvVcpu,
    exit: *const ffi::HvVcpuExit,
    _vm: PhantomData<&'vm Vm>,
    _not_send: PhantomData<*const ()>,
}

/// A vCPU's id, for `hv_vcpus_exit` from any thread.
#[derive(Debug, Clone, Copy)]
pub struct VcpuHandle(ffi::HvVcpu);

impl VcpuHandle {
    /// Stops the vCPU's run: `hv_vcpu_run` returns `Exit::Canceled`, or returns at once next time if it is not
    /// running (`hv_vcpu.h`).
    pub fn stop(&self) -> Result<(), HvError> {
        let mut id = self.0;
        check("hv_vcpus_exit", unsafe { ffi::hv_vcpus_exit(&mut id, 1) })
    }
}

impl<'vm> Vcpu<'vm> {
    /// A vCPU for the current thread. "Each thread can only have one vCPU associated at a time" (`hv_vcpu.h`).
    pub fn new(_vm: &'vm Vm) -> Result<Vcpu<'vm>, HvError> {
        let mut id = 0;
        let mut exit = std::ptr::null_mut();
        check("hv_vcpu_create", unsafe {
            ffi::hv_vcpu_create(&mut id, &mut exit, std::ptr::null_mut())
        })?;
        Ok(Vcpu {
            id,
            exit,
            _vm: PhantomData,
            _not_send: PhantomData,
        })
    }

    pub fn handle(&self) -> VcpuHandle {
        VcpuHandle(self.id)
    }

    /// Runs the guest until it exits.
    pub fn run(&self) -> Result<Exit, HvError> {
        check("hv_vcpu_run", unsafe { ffi::hv_vcpu_run(self.id) })?;
        let e = unsafe { &*self.exit };
        Ok(match e.reason {
            0 => Exit::Canceled,
            1 => Exit::Exception {
                syndrome: e.exception.syndrome,
                virtual_address: e.exception.virtual_address,
                physical_address: e.exception.physical_address,
            },
            2 => Exit::VtimerActivated,
            _ => Exit::Unknown,
        })
    }

    pub fn reg(&self, reg: Reg) -> Result<u64, HvError> {
        let mut v = 0;
        check("hv_vcpu_get_reg", unsafe {
            ffi::hv_vcpu_get_reg(self.id, reg.id(), &mut v)
        })?;
        Ok(v)
    }

    pub fn set_reg(&self, reg: Reg, value: u64) -> Result<(), HvError> {
        check("hv_vcpu_set_reg", unsafe {
            ffi::hv_vcpu_set_reg(self.id, reg.id(), value)
        })
    }

    pub fn sys_reg(&self, reg: u16) -> Result<u64, HvError> {
        let mut v = 0;
        check("hv_vcpu_get_sys_reg", unsafe {
            ffi::hv_vcpu_get_sys_reg(self.id, reg, &mut v)
        })?;
        Ok(v)
    }

    pub fn set_sys_reg(&self, reg: u16, value: u64) -> Result<(), HvError> {
        check("hv_vcpu_set_sys_reg", unsafe {
            ffi::hv_vcpu_set_sys_reg(self.id, reg, value)
        })
    }

    pub fn set_vtimer_mask(&self, masked: bool) -> Result<(), HvError> {
        check("hv_vcpu_set_vtimer_mask", unsafe {
            ffi::hv_vcpu_set_vtimer_mask(self.id, masked)
        })
    }

    /// SIMD and floating-point register Q`n` (hv_vcpu.h `hv_vcpu_get_simd_fp_reg`), little-endian.
    pub fn simd(&self, n: u8) -> Result<u128, HvError> {
        let mut v = [0u8; 16];
        check("hv_vcpu_get_simd_fp_reg", unsafe {
            ffi::hv_vcpu_get_simd_fp_reg(self.id, u32::from(n), v.as_mut_ptr().cast())
        })?;
        Ok(u128::from_le_bytes(v))
    }

    /// Sets Q`n` (hv_vcpu.h `hv_vcpu_set_simd_fp_reg`) from its little-endian value.
    pub fn set_simd(&self, n: u8, value: u128) -> Result<(), HvError> {
        let bytes = value.to_le_bytes();
        let ret: u64;
        // SAFETY: the call as the header declares it, by AAPCS64: x0 the vCPU, w1 the register, v0 the 16 bytes read
        // from `bytes`; the answer in w0. clobber_abi("C") gives the callee the caller-saved registers, x30 among them.
        unsafe {
            std::arch::asm!(
                "ldr q0, [{bytes}]",
                "blr {function}",
                bytes = in(reg) bytes.as_ptr(),
                function = in(reg) ffi::hv_vcpu_set_simd_fp_reg as *const (),
                inlateout("x0") self.id => ret,
                in("x1") u64::from(n),
                clobber_abi("C"),
            );
        }
        check("hv_vcpu_set_simd_fp_reg", ret as u32 as i32)
    }
}

impl Drop for Vcpu<'_> {
    fn drop(&mut self) {
        let r = unsafe { ffi::hv_vcpu_destroy(self.id) };
        debug_assert_eq!(r, 0, "hv_vcpu_destroy");
    }
}
