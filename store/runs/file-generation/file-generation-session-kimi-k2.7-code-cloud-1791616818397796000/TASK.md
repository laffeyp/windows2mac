# Task: write file.c for w2m

w2m runs Windows x64 programs on macOS. It translates the program's x86-64 code and runs it inside
a virtual machine on Apple silicon. The program's kernel32 imports are served by a "guest
runtime": freestanding C compiled for AArch64 with clang --target=arm64-apple-macos14 -O2 -std=c11 -Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector -fno-exceptions -fno-unwind-tables -fno-asynchronous-unwind-tables -mgeneral-regs-only, linked into the VM. There is no
C library: no memcpy, no memset, no headers but "runtime.h" (read it: it defines u64, u32 and u8
and nothing else). Arguments arrive per the Microsoft x64 calling convention: the first four in
the C parameters, and `rsp` is the guest stack pointer at the call, so the fifth argument is at
*(u64 *)(rsp + 0x28), the sixth at rsp + 0x30, the seventh at rsp + 0x38. Guest pointers can be
dereferenced directly. Each function's return value goes to RAX.

Write `file.c` in this directory, defining exactly these functions:

  u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp);
      creation disposition at rsp+0x28 (u32), flags and attributes at rsp+0x30 (u32), template at rsp+0x38
  u64 k32_CloseHandle(u64 handle);
  u64 k32_DeleteFileW(u64 name);
  u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp);   overlapped at rsp+0x28
  u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method);
  u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written);
      WriteFile on a non-stdout handle; runtime.c has already set *(u32 *)written = 0 and checked
      that `written` is not NULL and lpOverlapped is NULL; return 1 on success, 0 on failure

The last-error code is w2m_teb()->last_error_value. Reach the Mac only through
w2m_host(op, a0, a1, a2, a3, &second) with the W2M_FILE_* primitives (host_file_primitives.md).
Return from each k32_ function through w2m_logged(W2M_API_<name>, arg0, return_value), which
records the call and returns its third argument. Call w2m_unimplemented(W2M_API_<name>, detail)
for an argument form you do not support; it stops the run. Handle values must not be 0x74
(standard output).

The judge runs create_file.c, compiled by Windows's clang, in w2m over your file.c. Its output
must equal the rows in contract.json exactly, row for row and field for field. Those rows are
what Windows Server 2025 and 2022 printed.

Files here: runtime.h, create_file.c (the probe), contract.json, host_file_primitives.md.
Tools: `build` compiles file.c and returns the compiler's errors or "built"; `judge` runs the probe
over file.c and returns the rows that differ, or the item run's verdict once every row is equal.
Work until `judge` reports the item run accepted, then answer with one line saying so.
