# Task: write environment.c for w2m

w2m runs Windows x64 programs on macOS. It translates the program's x86-64 code and runs it inside
a virtual machine on Apple silicon. The program's kernel32 imports are served by a "guest
runtime": freestanding C compiled for AArch64 with clang --target=arm64-apple-macos14 -O2 -std=c11 -Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector -fno-exceptions -fno-unwind-tables -fno-asynchronous-unwind-tables -mgeneral-regs-only, linked into the VM. There is no
C library: no memcpy, no memset, no headers but "runtime.h" (read it). Arguments arrive per the
Microsoft x64 calling convention: the first four in the C parameters, and a fifth parameter `rsp`,
when declared, is the guest stack pointer at the call (the fifth argument at *(u64 *)(rsp + 0x28)).
Guest pointers can be dereferenced directly. Each function's return value goes to RAX.

Write `environment.c` in this directory, defining exactly these functions:

  u64 k32_GetEnvironmentStringsW(void);
  u64 k32_FreeEnvironmentStringsW(u64 block);
  u64 k32_lstrlenW(u64 string);

The process's environment block is the host's: UTF-16 entries, each ending in a NUL, then one more NUL,
at the address W2M_ENVIRONMENT_VA (an empty block is a single NUL there). runtime.h declares
  u64 w2m_heap_take(u64 size);          a process-heap block of exactly `size` bytes (HeapSize reports it),
                                        or 0 when the arena is full
  void w2m_heap_release(u64 api, u64 p); gives a block back
Probes see the process heap through GetProcessHeap and HeapSize, which use the same blocks.

The last-error code is w2m_teb()->last_error_value. Return from each k32_ function through
w2m_logged(W2M_API_<name>, arg0, return_value), which records the call and returns its third
argument. Call w2m_unimplemented(W2M_API_<name>, detail) for an argument form you do not support;
it stops the run.

The judge runs the probe (probe_environment.c, compiled by Windows's clang) in w2m over your file, once per
contract: contract-environment_set.json (environment block ['ZED=last', 'alpha=1', '=C:=C:\\w2m', 'SteamAppId=480', 'Path=C:\\Windows\\system32'], command line ending 'child-set'); contract-environment_empty.json (environment block [], command line ending 'child-empty'). Each contract's rows are what Windows printed on the runners; the probe's
output in w2m must equal them exactly, row for row and field for field. Read the probe to see how
each contract's process is started (its command line and environment block).

Tools: `build` compiles environment.c and returns the compiler's errors or "built"; `judge` runs the
probe over it and returns the rows that differ, or the item runs' verdicts once every row is equal.
Work until `judge` reports every item run accepted, then answer with one line saying so.
