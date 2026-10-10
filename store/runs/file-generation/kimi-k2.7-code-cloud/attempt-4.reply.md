Looking at the failure, all file operations are returning error 5 (ERROR_ACCESS_DENIED) and `exists: false`. This suggests the host file primitives aren't working as expected. The issue is likely in how `w2m_host` is being called.

The problem: `w2m_host(op, a0, a1, a2, a3, &second)` takes a pointer to `second`, but the host reads `arg[5]` from the transport page. Looking at runtime.h, `w2m_host` is declared as `u64 w2m_host(u64 op, u64 a0, u64 a1, u64 a2, u64 a3, u64 *second);` — the `second` pointer is where the host writes the second return value. But I'm passing `(u64)path` where `path` is a local array on the guest runtime stack. The host needs to read the path from guest memory, but `path` is in the runtime's own stack, not in the guest address space? Actually wait — the runtime is compiled into the VM and runs in host context, but its stack is... hmm.

Actually, the bigger issue: the host transport probably expects `arg[0]` to be a guest pointer, but `path` is a runtime stack buffer. The runtime and guest share the same address space? Let me re-read.

The runtime is "freestanding C compiled for AArch64 and run inside the managed VM". Guest pointers can be dereferenced directly. The runtime is linked into the VM. So runtime memory and guest memory are in the same address space. The host reads the transport page, and `arg[0]` would be a physical/guest address. So passing `(u64)path` should work.

But all lookups return kind=0 with error 5. Wait, the host primitive `FILE_LOOKUP (path, chars)` — I passed `chars=0`. Maybe `chars` needs to be the character count, not 0? Let me re-read: "FILE_LOOKUP (path, chars) -> kind". I was passing 0 for chars. That would explain why the host can't read the path properly.

Actually, looking more carefully: I should pass the length of the path string in characters/bytes. The host probably needs `chars` to know how many bytes to read from `path`. With `chars=0`, it returns kind=0 (absent) or errors.

Wait, but the error code is 5, not 2 or 3. Let me think... If chars=0, the host might interpret an empty path and return error. Or maybe the host returns -errno in `result` and I'm not handling it.

Let me reconsider the host interface. `w2m_host` returns the result. For FILE_LOOKUP, the result is `kind`. The second return value (file id) goes through `arg[5]`. If chars=0, the host can't look up anything, so it might return kind=0 (absent) and I set error based on that.

Actually for `disp.create_new.absent`, kind would be 0, must_create=1, must_not_exist=1, so we'd proceed to create. Then FILE_OPEN with create_arg=1 returns -errno. What errno? If path is empty/invalid, maybe -EACCES (-13), which maps to ERROR_ACCESS_DENIED (5). That matches!

So the root cause is passing `chars=0` to FILE_LOOKUP and FILE_OPEN. I need to pass the actual byte length of the normalized path.

Also for FILE_UNLINK, same issue.

Let me also check FILE_OPEN: I passed `(u64)path, 0, create_arg, 0`. The second argument should be the character count.

Let me fix all `w2m_host` calls to pass the string length where needed.

Also, I need to make sure `normalize_path` returns the length or that I compute it. I'll compute with a simple loop.

Another issue: for `FILE_OPEN`, when `create_arg=0` and file doesn't exist, the host returns -errno. But I already checked existence with LOOKUP, so this should be fine.

Let me also reconsider the `chars` parameter. The paths are UTF-16LE as the guest normalized them, but my normalized path is ASCII bytes. The host primitive says "Paths arrive as UTF-16LE as the guest normalized them." Hmm, but I'm normalizing to ASCII host path. The host side says "Components compare ASCII case-folded". So the host expects an ASCII path? Or does it expect UTF-16LE?

Wait, re-reading: "Paths arrive as UTF-16LE as the guest normalized them. Components compare ASCII case-folded, as NTFS compares ASCII names". This is a bit ambiguous. But since I'm passing a byte string, `chars` might mean bytes. The host probably reads `chars` bytes from `path`. So I should pass the byte length of my ASCII path.

Actually, maybe the host expects UTF-16LE and `chars` is the number of UTF-16 code units. But the contract says non-ASCII folding is not needed, and the probe uses ASCII paths. The host side implementation in file_bridge.py would convert. But since I'm already normalizing to ASCII, I need to know what the host expects.

Given that my previous implementation passed 0 and got error 5, and the host needs a length, I'll pass the byte length of the ASCII path. If the host expects UTF-16LE length, it would be wrong, but let's see — the probe paths are ASCII, so byte length equals UTF-16LE code unit count. So it works either way for ASCII paths.

Wait, but my normalized path has '/' separators and uppercase ASCII. If the host expects UTF-16LE, it would see bytes 0x2f 0x00 etc. That's wrong. Hmm.

Let me re-read the primitive description more carefully: "Paths arrive as UTF-16LE as the guest normalized them." This suggests the guest should pass UTF-16LE. But then "Components compare ASCII case-folded" — the host does the case folding on UTF-16LE? That seems odd.

Actually, I think the host side takes whatever bytes and interprets them. The description might mean that originally paths come as UTF-16LE from Windows, but the runtime can normalize them. The host probably just reads `chars` bytes from the pointer and treats them as the path string (possibly UTF-8 or ASCII).

Given the previous failure mode, the most important fix is passing the correct length. Let me pass byte length and see.

Actually, I realize I should also check: when I call `w2m_host(W2M_FILE_OPEN, (u64)path, 0, create_arg, 0, &second)`, the third argument `a2` is `create`. The signature is `w2m_host(op, a0, a1, a2, a3, &second)`. For FILE_OPEN, args are `(path, chars, create)`. So a0=path, a1=chars, a2=create. Yes.

For FILE_LOOKUP, args are `(path, chars)`. a0=path, a1=chars.

For FILE_READ, args are `(fd, buffer, n, offset)`. a0=fd, a1=buffer, a2=n, a3=offset.

For FILE_WRITE, same.

For FILE_SIZE, `(fd)`. a0=fd.

For FILE_TRUNCATE, `(fd, size)`. a0=fd, a1=size.

For FILE_UNLINK, `(path, chars)`. a0=path, a1=chars.

For FILE_CLOSE, `(fd)`. a0=fd.

OK so I need to pass path length for LOOKUP, OPEN, UNLINK.

Let me also think about other potential issues:

1. `normalize_path` strips trailing dots and spaces. The contract says Windows opens 'name.' and 'name ' as 'name'. Good.

2. For `name.dot.no_backup_semantics`, path "." — I special-cased it. But after stripping, "." becomes empty? Wait, my normalization: for ".", start=0, pos=1, end after stripping dots/spaces = 0. So it skips the component. Then out_len=0, returns ERROR_PATH_NOT_FOUND. But the contract expects ERROR_ACCESS_DENIED (5). So I need to special-case "." before the empty check. Actually I did special-case it, but only if path[0]=='.' && path[1]==0. But `path` is the output, and at that point I haven't written "." to it yet. Let me trace: for input ".", the loop skips leading separators, start=0, pos=1, end=0 after stripping, continues, out_len=0, then returns ERROR_PATH_NOT_FOUND. So my special case `if (path[0] == '.' && path[1] == 0)` never triggers because we return earlier. I need to detect "." component before stripping.

3. For empty path, contract expects ERROR_PATH_NOT_FOUND (3). Good.

4. For missing dir, contract expects ERROR_PATH_NOT_FOUND (3). My host returns kind=3, I set ERROR_PATH_NOT_FOUND. Good.

5. For `name.other_absent` (B), contract expects ERROR_FILE_NOT_FOUND (2). Good.

6. For sharing, I need to make sure the table matches. My logic seems right but let me double-check with the contract rows.

7. For delete with open handle no share delete, contract expects ret=0, err=32, exists=true. My logic: delete_blocked checks if any open handle to id doesn't share delete. If blocked, return ERROR_SHARING_VIOLATION (32), ret=0. Good. But the file still exists. Good.

8. For delete with open handle share delete, contract expects ret=1, err=57005 (sentinel, meaning last error untouched). My logic: if not blocked, unlink, ret=1, set_error(0). But contract says err=57005, meaning DeleteFileW doesn't set last error on success. Wait, the probe does SetLastError(SENTINEL) before DeleteFileW, then GetLastError after. If DeleteFileW succeeds and doesn't touch last error, it reads 57005. So I should NOT set_error(0) on success for DeleteFileW. Same for other successful calls? Let me check the contract.

Looking at `del.closed`: ret=1, err=57005. So DeleteFileW on success leaves last error as sentinel.

`disp.create_new.absent`: valid=true, err=0. So CreateFileW on success sets last error to 0.

`disp.create_always.existing`: valid=true, err=183. Sets to ERROR_ALREADY_EXISTS.

`disp.open_always.existing`: valid=true, err=183.

`io.write`: ret=1, err=57005. WriteFile on success doesn't touch last error.

`io.seek.current_0`: ret=1, err=57005. SetFilePointerEx on success doesn't touch last error.

`close.twice`: ret=0, err=6. CloseHandle on invalid handle sets ERROR_INVALID_HANDLE.

So the pattern is: on success, some APIs set last error to 0, some leave it untouched. Specifically:
- CreateFileW: sets 0 or ERROR_ALREADY_EXISTS
- CloseHandle: sets 0 on success
- DeleteFileW: leaves untouched on success
- ReadFile: leaves untouched on success
- WriteFile: leaves untouched on success
- SetFilePointerEx: leaves untouched on success

Wait, but `del.closed` err=57005 means DeleteFileW success leaves untouched. But `disp.create_new.absent` err=0 means CreateFileW success sets 0. Let me verify with other rows.

`disp.open_existing.existing`: err=0. CreateFileW sets 0.
`disp.truncate_existing.existing`: err=0. CreateFileW sets 0.
`share.r.r.r.r`: valid=true, err=0. CreateFileW sets 0.

So CreateFileW always sets last error (0 or 183). CloseHandle sets 0 on success. DeleteFileW, ReadFile, WriteFile, SetFilePointerEx leave untouched on success.

But wait, `io.read_at_eof`: ret=1, err=57005. ReadFile at EOF success leaves untouched.

`io.write_at_100`: ret=1, err=57005. WriteFile success leaves untouched.

OK so I need to adjust:
- k32_DeleteFileW: on success, do NOT call set_error(0)
- k32_ReadFile: on success, do NOT call set_error(0)
- w2m_file_write: on success, do NOT call set_error(0)
- k32_SetFilePointerEx: on success, do NOT call set_error(0)

But on failure, all set appropriate error.

For CreateFileW and CloseHandle, set 0 on success.

Actually wait, let me re-check CloseHandle. `close.twice`: first CloseHandle succeeds (ret=1? not logged), then second CloseHandle ret=0 err=6. So CloseHandle on success sets 0, on failure sets ERROR_INVALID_HANDLE. The probe doesn't log the first close's err. But we should set 0 on success.

Now, another issue: the probe's `make` function uses CREATE_ALWAYS. My CreateFileW for CREATE_ALWAYS on existing file truncates and sets ERROR_ALREADY_EXISTS. Good.

For CREATE_ALWAYS on absent file, creates and sets 0. Good.

For OPEN_ALWAYS on existing file, sets ERROR_ALREADY_EXISTS. Good.

For TRUNCATE_EXISTING on existing file, truncates and sets 0. Good.

For TRUNCATE_EXISTING on absent file, returns ERROR_FILE_NOT_FOUND. Good.

For CREATE_NEW on existing, returns ERROR_FILE_EXISTS (80). Good.

For CREATE_NEW on absent, creates and sets 0. Good.

Now, the handle table: I need to make sure handle 0x74 is skipped. My make_handle does that.

But wait, if idx+1 == 0x74, I return idx+2. But then the next handle would be idx+1 again? No, because alloc_entry returns the next free slot. If slot i gets handle i+2 (because i+1=0x74), then slot i+1 would get handle i+2 which is already used? Wait no, handle values are derived from index, not stored. So if index 0x73 gets handle 0x75, and index 0x74 gets handle 0x75? No: make_handle(0x73) returns 0x75. make_handle(0x74) returns... 0x74 == 0x74, so skip to 0x76? Wait my code: `u64 h = idx + 1; if (h == 0x74) return idx + 2;`. For idx=0x74, h=0x75, not 0x74, so returns 0x75. For idx=0x73, h=0x74, returns 0x75. Conflict!

Bug: both index 0x73 and 0x74 would map to handle 0x75. I need a better scheme. Maybe just skip handle 0x74 and ensure no collision. Simplest: handle = idx + 1; if handle >= 0x74, handle++. But then handle_index must reverse: if h > 0x74, idx = h - 2; else idx = h - 1. And h==0x74 is invalid.

Let me fix:
```c
static u64 make_handle(unsigned idx)
{
    u64 h = idx + 1;
    if (h >= 0x74) h++;
    return h;
}

static unsigned handle_index(u64 h)
{
    if (h == 0 || h == 0x74) return (unsigned)-1;
    if (h > MAX_FILES + 1) {
        if (h == MAX_FILES + 2) return MAX_FILES - 1; /* because we skipped 0x74 */
        return (unsigned)-1;
    }
    if (h > 0x74) return (unsigned)(h - 2);
    return (unsigned)(h - 1);
}
```

Hmm, this is getting messy. Let me think simpler. MAX_FILES=64. Handles 1..65, but skip 0x74=116. Since 116 > 65, we don't actually hit 0x74 with 64 entries. Wait, 0x74 = 116 decimal. With MAX_FILES=64, max handle is 65. So we never generate 0x74 anyway! The skip code is unnecessary if max handles < 116. But the problem says "Handle values must not be 0x74". Since we only use 1..64, we're fine. I can remove the skip logic entirely. But to be safe, I'll keep a simple version.

Actually, the issue says handle values must not be 0x74, but with 64 handles we won't reach it. So I can just use handle = idx + 1. But maybe the host checks for 0x74 specifically. Let me just use idx+1 and not worry.

Wait, but the previous code had the skip bug. Let me simplify to handle = idx + 1, with MAX_FILES=64, so handles 1-64, none equal 0x74.

Now, another issue: the host might return file descriptors starting from 0 or small numbers. My handle table uses fd to check if entry is free (fd==0). But host fd could be 0? Unlikely, but possible if stdin. Actually file descriptors from open() are usually >= 3. But to be safe, I should use a separate `used` flag, not rely on fd==0. Let me add `int used` to file_entry.

Actually, the host returns "host descriptor" which is probably a Python file object index or actual fd. Could be 0? Let me use a `used` flag.

Let me also think about the path normalization for ".". I need to detect the "." component before stripping. If a component is exactly "." (after separators), it should be skipped (as Windows does). But if the entire path is ".", then we need special handling. Actually for `name.dot.no_backup_semantics`, the input is "." and flags don't include FILE_FLAG_BACKUP_SEMANTICS. Windows returns ERROR_ACCESS_DENIED. So I should detect "." early and return ERROR_ACCESS_DENIED if no backup semantics.

For "..", I currently unimplement if it leaves root. But the probe doesn't use "..". Fine.

For trailing dots/spaces: I strip them per component. But I need to be careful: "name." should open as "name". My code does that.

For empty path: returns ERROR_PATH_NOT_FOUND. Good.

For missing dir: host returns kind=3. Good.

Now, the `chars` parameter. Let me compute `path_len` as the byte length of the normalized ASCII path. I'll pass that.

But wait, the host might expect the number of UTF-16 characters in the original path, not bytes. Since I'm normalizing to ASCII, the byte length is the same as the character length of the normalized path. But the host might read `chars` bytes. Let me assume bytes.

Actually, re-reading file_bridge.py: "FILE_LOOKUP (path, chars) -> kind". It says "chars" not "bytes". But in Python, it might read `chars` bytes. Given the previous code passed 0, passing any reasonable length should help.

Let me also check if I need to pass the path as UTF-16LE. The runtime.h says paths arrive as UTF-16LE as the guest normalized them. But I'm normalizing to ASCII bytes. Hmm. Maybe the host expects UTF-16LE and my ASCII bytes are wrong.

Wait, but the host side says "Components compare ASCII case-folded, as NTFS compares ASCII names". This implies the host does the case folding, so it should receive the original UTF-16LE path. But then why would the runtime normalize?

Actually, I think the runtime should pass the UTF-16LE path to the host, and the host does case folding. But the runtime also needs to handle trailing dot/space stripping and ".." validation. Hmm.

But the contract says the runtime implements "name rules". The host just provides primitives. So maybe the runtime passes the normalized ASCII path to the host, and the host uses it directly.

Given the previous failure was error 5 (access denied) due to chars=0, let me first fix the length issue and see. If paths need to be UTF-16LE, I'd need to pass the original UTF-16 path (with normalization applied differently).

Actually, let me reconsider. The host primitive `FILE_LOOKUP (path, chars)` — if `path` is a pointer and `chars` is the number of UTF-16 code units, then I should pass the original UTF-16LE string (or a normalized version in UTF-16LE). But I'm converting to ASCII bytes. That would be wrong.

But the contract says "Components compare ASCII case-folded". If the host receives UTF-16LE, it would compare each code unit case-folded. For ASCII, that's the same as byte comparison after uppercasing. So if I pass UTF-16LE with uppercased ASCII and stripped trailing dots/spaces, it should work.

Hmm, but the host description says "Paths arrive as UTF-16LE as the guest normalized them." So the guest (runtime) normalizes and passes UTF-16LE. I should pass UTF-16LE.

But then what about the `chars` parameter? It would be the number of UTF-16 code units.

Wait, but my previous code passed ASCII bytes with chars=0. The host couldn't read it properly. If I switch to UTF-16LE, I need to build a UTF-16LE normalized path.

Actually, maybe the host just reads `chars` bytes and treats them as a byte string. The description "UTF-16LE" might be misleading or refer to the original Windows form. Let me look at the failure again: error 5 with exists=false. If the host couldn't parse the path at all, it might return kind=0 (absent), and then CREATE_NEW would try to create with chars=0, getting EACCES.

I think the safest is to pass the normalized path as a byte string with its byte length. The host file_bridge.py probably uses `os.path.join(root, path.decode('utf-8'))` or similar. If I pass UTF-16LE bytes, it would decode incorrectly. If I pass ASCII bytes, it would decode correctly.

Let me assume ASCII bytes with byte length.

Actually, you know what, let me just pass the byte length and keep ASCII. That's the most likely fix.

Now, another issue: for `FILE_OPEN` with create=1 (O_CREAT|O_EXCL), if the file exists, host returns -EEXIST. But I already check existence with LOOKUP, so I shouldn't call OPEN with create=1 if exists. My code does that correctly.

For `FILE_OPEN` with create=0 on existing file, host returns fd. Good.

For `FILE_OPEN` with create=0 on absent file, host returns -ENOENT. But I check existence first and return ERROR_FILE_NOT_FOUND. Good.

Wait, but for OPEN_ALWAYS on absent, I call LOOKUP (kind=0), then must_create=1, create_arg=1, OPEN. Good.

For CREATE_ALWAYS on existing, I call LOOKUP (kind=1), must_create=1, must_not_exist=0, create_arg=0, OPEN, then truncate. Good.

Now, the sharing check: I check against existing open handles to the same file id. But what about the case where the file doesn't exist yet and multiple CREATE_NEW calls? The first creates it, the second would see kind=1 and must_not_exist=1, returning ERROR_FILE_EXISTS. That's correct per Windows.

But the sharing table tests all use OPEN_EXISTING on an existing file. Good.

Now, the delete_blocked function: it checks if any open handle to id doesn't share delete. But after a file is deleted (unlinked) while handles are open, subsequent opens should fail with ERROR_FILE_NOT_FOUND. My code: after unlink, the file id is still associated with open handles. If a new CreateFileW calls LOOKUP, the host returns kind=0 (absent) because the file is unlinked. So we return ERROR_FILE_NOT_FOUND. Good.

But wait, the open handles still have the old file id. If we close them, that's fine. The host fd is still valid until closed.

For `del.pending.create_new`: after deleting with share delete, the probe calls CREATE_NEW on the same path. Windows returns valid=true, err=0 (creates a new file). My code: LOOKUP returns kind=0 (absent), CREATE_NEW with must_create=1, must_not_exist=1, create_arg=1, OPEN. Host creates new file. Good.

For `del.pending.after_close`: after closing the original handle, the new file exists with size 0. My code: the new file was created, so it exists. Good.

Now, let me also check `io.after_close`: size should be 101. The probe writes 11 bytes, then seeks to 100, writes 1 byte. So size = 101. My code: FILE_WRITE at offset 100 writes 1 byte, extending the file. Host should handle that. Good.

`io.read_gap`: seek to 9, read 4 bytes. File is "hello world" (11 bytes). At position 9, bytes are 'l','d', then 2 bytes beyond EOF are zero. So read 4 bytes: 'l','d',0,0 = 0x6c640000. My code: FILE_READ at offset 9 for 4 bytes. Host pread returns bytes read, zero-filling beyond EOF? Actually pread doesn't zero-fill; it returns fewer bytes. But the contract says 4 bytes read with data 6c640000. So the host must zero-fill the gap. The file_bridge.py says "bytes read (pread)". Standard pread doesn't zero-fill. But the contract expects zero-filled. So the host implementation must zero-fill, or the runtime must. Since the runtime just calls FILE_READ, the host must do it. I'll assume the host does.

Actually, the contract quote says "A write operation increases the size of the file to the file pointer position plus the size of the buffer written, which results in the intervening bytes being zero initialized." So the host must zero-initialize the gap. Good.

Now, let me also handle the `name.dot.no_backup_semantics` case properly. I need to detect "." as the entire path and return ERROR_ACCESS_DENIED if no backup semantics.

Let me rewrite normalize_path to:
1. Read UTF-16LE into buf.
2. If empty, return ERROR_PATH_NOT_FOUND.
3. Split into components.
4. If the only component is "." and no backup semantics, return ERROR_ACCESS_DENIED.
5. Otherwise skip "." components.
6. For "..", pop or unimplement.
7. Strip trailing dots/spaces.
8. Case-fold.
9. Build path.

Actually, for ".", Windows with backup semantics opens the directory. Without, access denied. The probe doesn't test with backup semantics, so I just need access denied without.

For the path ".", after splitting, the component is ".". I should detect: if there's exactly one component which is ".", and no backup semantics, return ERROR_ACCESS_DENIED.

But what about "a/./b"? The "." component should be skipped. So I need to detect "." only when it's the entire effective path.

Let me restructure.

Also, I need to handle the case where the path normalizes to empty (e.g., input "." or "./" or ""). For empty input, ERROR_PATH_NOT_FOUND. For "." without backup semantics, ERROR_ACCESS_DENIED. For "." with backup semantics, unimplemented. For "./" same as ".".

OK let me write the code.

One more thing: the `chars` parameter. I'll compute the byte length of the final ASCII path. But I also need to consider: does the host want the length including null terminator? Probably not. I'll pass without null.

Let me also add a `used` flag to file_entry.

Now, let me think about the `w2m_host` call signature. It takes `u64 *second`. The host writes the second return value to `*second`. I'm passing `&second` where `second` is a local u64. Good.

But wait, the transport page has `arg[6]` and `result`. The `w2m_host` function probably fills the transport page, does hvc #8, and returns `result`. The `second` pointer is for `arg[5]`. Good.

Now, let me also consider: the runtime is compiled with `-mgeneral-regs-only`, so no SIMD. Fine.

Let me also make sure I don't use any C library functions. I use no headers but runtime.h. I define my own types. Good.

Let me write the corrected file.c.

Actually, one more subtlety: for `k32_CreateFileW`, the contract expects `err=0` for success. But for `CreateFileW` with `OPEN_ALWAYS` or `CREATE_ALWAYS` on existing file, err=183. My code sets that. Good.

For `CreateFileW` success on new file, err=0. Good.

For `CreateFileW` failure, err is set appropriately. Good.

For `k32_CloseHandle` success, err=0. Good.

For `k32_DeleteFileW` success, err should be untouched (57005). So I should NOT call set_error(0). But what about failure? Set error. Good.

For `k32_ReadFile` success, err untouched. Good.

For `w2m_file_write` success, err untouched. Good.

For `k32_SetFilePointerEx` success, err untouched. Good.

Wait, but for `io.seek.begin_minus_1`, failure sets ERROR_NEGATIVE_SEEK (131). The probe's sentinel for new_pointer is -7. My code sets it to -7 on failure. Good.

For `io.seek.current_after_failure`, after the failed seek, position should still be 11 (unchanged). My code doesn't update f->position on failure. Good.

Now, for `k32_SetFilePointerEx` with invalid handle, I set new_pointer to -1. But the probe doesn't test that. Fine.

For `k32_ReadFile` and `w2m_file_write` with invalid handle, I set bytes_read/written to 0. Good.

Now, the `bytes_read` parameter might be NULL? The probe always passes non-NULL. But I check. Good.

For `w2m_file_write`, the comment says runtime.c already set `*(u32 *)written = 0` and checked non-NULL. So I don't need to set to 0, but it's safe.

Now, let me also think about `k32_CreateFileW` with `security_attributes` non-NULL. The probe passes NULL. If non-NULL, I should unimplement. I currently ignore it. The problem says "Call w2m_unimplemented for any argument form you do not support". Security attributes non-NULL is unsupported. Let me add a check.

Also `template` non-NULL is unsupported. I check that.

Flags other than FILE_ATTRIBUTE_NORMAL and FILE_FLAG_BACKUP_SEMANTICS? The probe uses FILE_ATTRIBUTE_NORMAL and sometimes FILE_FLAG_BACKUP_SEMANTICS. If other flags, unimplement. Let me add a check.

Actually, the contract says "flags other than FILE_ATTRIBUTE_NORMAL, security attributes, templates, overlapped I/O and absolute paths are outside this scenario." So I can unimplement on unsupported flags.

Let me check flags: FILE_ATTRIBUTE_NORMAL = 0x80. FILE_FLAG_BACKUP_SEMANTICS = 0x02000000. The probe uses FILE_ATTRIBUTE_NORMAL in most cases, and FILE_FLAG_BACKUP_SEMANTICS for the directory test. So I should allow these two. If other bits set, unimplement.

But wait, FILE_ATTRIBUTE_NORMAL is 0x80, and FILE_FLAG_BACKUP_SEMANTICS is 0x02000000. These don't overlap. So allowed mask = 0x02000080. If (flags & ~allowed_mask) != 0, unimplement.

Actually, Windows allows FILE_ATTRIBUTE_NORMAL to be combined with flags. The probe uses either FILE_ATTRIBUTE_NORMAL or FILE_FLAG_BACKUP_SEMANTICS | FILE_ATTRIBUTE_NORMAL? Let me check: `named("name.dot.no_backup_semantics", L".", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);` — no backup semantics. Others use FILE_ATTRIBUTE_NORMAL. The directory test uses FILE_ATTRIBUTE_NORMAL. So flags are either 0x80 or 0x80 | 0x02000000. Allowed mask 0x02000080.

OK.

Now, for `name.dot.no_backup_semantics`, flags=FILE_ATTRIBUTE_NORMAL (no backup semantics), path=".", should return ERROR_ACCESS_DENIED. Good.

Now, let me write the code.

One more thing: the `normalize_path` function currently returns a u32 error code and also calls set_error. But for DeleteFileW, on path error, I return logged(0) without setting error? Wait, normalize_path sets error. Then I return. Good. But for DeleteFileW success, I shouldn't set error 0. For failure paths, normalize_path sets error. Good.

But wait, for CreateFileW, normalize_path might set ERROR_PATH_NOT_FOUND. Then I return INVALID_HANDLE_VALUE. Good.

Now, the path length: I'll compute `path_len` with a loop.

Let me also reconsider whether to pass UTF-16LE or ASCII. I'll go with ASCII bytes for now. If that doesn't work, I'd need to switch. But given the previous error was chars=0, fixing that is the priority.

Actually, I just realized: the host might be reading the path from guest memory, but my `path` buffer is on the runtime stack. The runtime stack might not be accessible to the host? But the runtime is in the VM, so its memory should be accessible. The problem says "Guest pointers can be dereferenced directly" — this applies to guest code, but the runtime is also in the same address space. The host reads the transport page and then accesses `arg[0]` which is a pointer. If that pointer is in runtime stack, the host should be able to access it.

Hmm, but maybe the host only has access to guest memory, not runtime memory? The runtime is linked into the VM, so it's part of the same address space. I think it's accessible.

OK let me write the code.

Wait, I should also check: for `FILE_OPEN`, when create=0 and file exists, the host opens with what flags? It should open with the access we need (read/write). The host primitive doesn't take access mode. It probably opens with O_RDWR or similar. But the runtime tracks access and enforces it. For read-only handle, the runtime rejects writes. For write-only, rejects reads. The host fd might be opened RW, but we enforce access in the runtime. That's fine.

But for sharing, the host doesn't know about sharing. We enforce it in the runtime by checking existing handles. Good.

Now, one issue: when we have multiple handles to the same file, they all share the same host fd? No, each FILE_OPEN call returns a new fd. Good.

But wait, the host might open the file with O_EXCL or something. For create=1, it's O_CREAT|O_EXCL. For create=0, it's just open. Good.

OK, let me write the corrected file.c now.

Actually, I want to double-check the sharing logic with a few contract rows:

`share.r.0.r.0`: first handle read access, no share. Second handle read access, no share. Conflict because share1=0 doesn't allow acc2=READ. err=32. first=true. Good.

`share.r.0.w.0`: first read, no share. second write, no share. share1 doesn't allow write. err=32. Good.

`share.r.r.r.r`: first read, share read. second read, share read. share1 allows read, share2 allows read. No conflict. err=0. Good.

`share.r.r.r.w`: first read, share read. second read, share write. share1 allows read. share2 allows... acc1=read, share2 has write but not read. So share2 doesn't allow acc1=read. Conflict. err=32. Good.

`share.r.w.w.r`: first read, share write. second write, share read. share1 has write but not read. acc2=write, share1 allows write. share2 has read but not write. acc1=read, share2 doesn't allow write? Wait acc1=read, share2=read allows read. So no conflict? But contract says err=32. Let me re-check.

`share.r.w.w.r`: first access=r, share=w. second access=w, share=r.
- share1=w: does it allow acc2=w? share1 has FILE_SHARE_WRITE, yes.
- share2=r: does it allow acc1=r? share2 has FILE_SHARE_READ, yes.
No conflict. But contract says valid=false, err=32.

Hmm, that's a problem. Let me re-read the contract row:
```
"s": "share.r.w.w.r",
"first": true,
"valid": false,
"err": 32
```

Wait, maybe I misread the access/share mapping. The format is `share.<first access>.<first share>.<second access>.<second share>`. So:
- first: access=r, share=w
- second: access=w, share=r

My analysis says no conflict. But Windows says conflict. Why?

Maybe Windows sharing rule is different. Let me think. Windows sharing: when opening a file, the requested sharing mode must be compatible with all existing opens. Specifically, the new open's sharing mode must allow the access of all existing opens, and the existing opens' sharing modes must allow the new open's access.

For first open (r, w): it requests write sharing but not read sharing. This means it allows others to write but not read. Wait, that's odd. If I open a file for read and share write, I'm saying "others can write while I read". But I'm not sharing read, so others can't read.

Second open (w, r): it requests read sharing but not write sharing. It wants to write. Existing first open shares write, so second can write. Existing first open doesn't share read, but second doesn't need read. Second shares read, which allows first's read access. So no conflict?

But Windows says conflict. Maybe I'm misunderstanding the share mode semantics.

Actually, in Windows, `dwShareMode` specifies what access OTHER processes can request. So if I open with share=write, I'm allowing others to write. If I open with share=read, I'm allowing others to read.

For first (access=r, share=w): I want to read, and I allow others to write.
For second (access=w, share=r): I want to write, and I allow others to read.

Can second write while first reads? First allowed write sharing, so yes.
Can first read while second writes? Second allowed read sharing, but that means others can read, not that first can read while second writes. Hmm.

Actually, the sharing compatibility is symmetric. Let me look at the Microsoft table.

The MS quote: "the system compares the requested access and sharing modes to those specified when the file was opened. If you specify an access or sharing mode that conflicts with the modes specified in the previous call, CreateFile fails."

The actual rule: For each existing open, the new open's desired access must be allowed by the existing open's share mode, AND the new open's share mode must allow the existing open's desired access.

For first (r,w) and second (w,r):
- Existing first share=w: allows write. New second access=w: allowed.
- New second share=r: allows read. Existing first access=r: allowed.
No conflict.

But contract says conflict. So maybe my interpretation of the labels is wrong, or the contract row is different.

Wait, let me re-read the probe code:
```c
static const DWORD ACCESS[] = {GENERIC_READ, GENERIC_WRITE, GENERIC_READ | GENERIC_WRITE};
static const char *const ACCESS_NAME[] = {"r", "w", "rw"};
static const DWORD SHARE[] = {0, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE};
static const char *const SHARE_NAME[] = {"0", "r", "w", "rw"};
```

And the row format: `share.<a1>.<s1>.<a2>.<s2>`. So `share.r.w.w.r` means a1=r, s1=w, a2=w, s2=r. My interpretation is correct.

But the contract says this should fail. Let me look at more rows to understand the rule.

`share.r.w.w.w`: first r,w; second w,w. Contract: valid=false, err=32.
`share.r.w.w.rw`: first r,w; second w,rw. Contract: valid=true, err=0.

Interesting! With second share=rw, it succeeds. With second share=w or r, it fails.

So the rule seems to be: the second open's share mode must include BOTH read and write sharing if the first open has any access? No, that doesn't fit `share.r.r.r.r` which succeeds with second share=r.

Let me look at all rows where first is (r,w):

`share.r.w.r.*`: all fail.
`share.r.w.w.r`: fail.
`share.r.w.w.w`: fail.
`share.r.w.w.rw`: success.
`share.r.w.rw.*`: all fail.

So when first share=w (only write sharing), the only successful second open is (w, rw). That is, second access=w and second share=rw.

Hmm, what if the rule is about the COMBINATION of access and share? Maybe Windows checks that the new open's share mode is a superset of the existing opens' share modes? Or that the intersection of share modes allows both accesses?

Let me think differently. Maybe the rule is: the new open's share mode must allow the existing open's access, AND the existing open's share mode must allow the new open's access. But also, the new open's access must be allowed by the combination?

Wait, for first (r,w) and second (w,r):
- Existing share=w allows new access=w: OK.
- New share=r allows existing access=r: OK.
But fails.

For first (r,w) and second (w,rw):
- Existing share=w allows new access=w: OK.
- New share=rw allows existing access=r: OK.
Succeeds.

The difference is new share=r vs rw. With rw, it succeeds. So maybe the rule also requires that the new open's share mode allows the new open's access? That doesn't make sense.

Actually, maybe the rule is: the new open's share mode must be compatible with the existing open's share mode such that their combined share modes allow both accesses. For first (r,w) and second (w,r): combined share = w & r = 0. This allows neither read nor write. But existing access=r and new access=w both need to be allowed? Not sure.

For first (r,w) and second (w,rw): combined share = w & rw = w. Allows write. Existing access=r needs read sharing, but combined only has write. Hmm.

Let me look at the actual Windows sharing table. I recall that Windows uses this rule:
- A new open succeeds if, for every existing open:
  - (new_access & ~existing_share) == 0, i.e., existing share allows new access
  - (existing_access & ~new_share) == 0, i.e., new share allows existing access

But that gives the wrong answer for (r,w)/(w,r). Let me check the contract again for that specific row.

Actually, maybe I misread the row. Let me search the contract for "share.r.w.w.r":

```
   {
    "s": "share.r.w.w.r",
    "first": true,
    "valid": false,
    "err": 32
   },
```

Yes, it fails. So my rule is wrong.

Let me look for a pattern. Maybe the rule is:
- existing_share must allow new_access
- new_share must allow existing_access
- AND new_share must allow new_access? No, that would mean you must share the access you're using, which is not how it works.

Wait, maybe the rule is about the file being opened in a way that the combination of all opens is consistent. Let me think of it as: each open specifies what sharing it allows. The system must ensure that no open is doing something that another open has forbidden.

For first (r,w): it forbids others from reading (because share doesn't include read), but allows others to write.
For second (w,r): it forbids others from writing (because share doesn't include write), but allows others to read.

Can both coexist? First wants to read. Second forbids writing. First doesn't write, so OK. Second wants to write. First forbids reading. Second doesn't read, so OK. But first allows writing, and second writes. That's OK because first allowed it. Second allows reading, and first reads. That's OK because second allowed it.

So logically they should coexist. But Windows says no. Why?

Maybe because first opened with share=w, meaning "I will allow writes". But first is reading. If second writes, first's read might see inconsistent data. But that's the programmer's choice.

Hmm, maybe the actual Windows rule is more restrictive. Let me look at the MS documentation table.

Actually, I found a possible rule online: Windows checks both directions, but also the new open's share mode must not be more restrictive than existing? Or the existing open's share mode must be a superset of the new open's desired access, and the new open's share mode must be a superset of the existing open's desired access. That's what I had.

Wait, maybe the issue is GENERIC_READ vs FILE_READ_DATA, and GENERIC_WRITE vs FILE_WRITE_DATA. GENERIC_READ includes more than just read data. But for sharing, maybe the system checks at a lower level.

Actually, I think the issue might be that FILE_SHARE_WRITE alone is not sufficient when combined with FILE_SHARE_READ in a certain way. Let me look at the contract rows more systematically.

Let me list all first=(r,w) rows:
- share.r.w.r.0: fail
- share.r.w.r.r: fail
- share.r.w.r.w: fail
- share.r.w.r.rw: fail
- share.r.w.w.0: fail
- share.r.w.w.r: fail
- share.r.w.w.w: fail
- share.r.w.w.rw: success
- share.r.w.rw.0: fail
- share.r.w.rw.r: fail
- share.r.w.rw.w: fail
- share.r.w.rw.rw: fail

Only second=(w,rw) succeeds. Interesting.

Now first=(r,rw) rows:
- share.r.rw.r.0: fail
- share.r.rw.r.r: success
- share.r.rw.r.w: fail
- share.r.rw.r.rw: success
- share.r.rw.w.0: fail
- share.r.rw.w.r: success
- share.r.rw.w.w: fail
- share.r.rw.w.rw: success
- share.r.rw.rw.0: fail
- share.r.rw.rw.r: success
- share.r.rw.rw.w: fail
- share.r.rw.rw.rw: success

So with first share=rw, second succeeds if second share includes the access second needs? Let's see:
- second=(r,r): access=r, share=r. share1=rw allows r. share2=r allows r (existing access=r). Success.
- second=(r,w): access=r, share=w. share1=rw allows r. share2=w doesn't allow r (existing access=r). Fail.
- second=(w,r): access=w, share=r. share1=rw allows w. share2=r doesn't allow w (existing access=r). Fail.
- second=(w,rw): access=w, share=rw. share1=rw allows w. share2=rw allows w. Success.

This matches my original rule! For first share=rw, the rule works.

But for first share=w, my rule predicts second=(w,r) should succeed, but contract says fail. Why?

Maybe because first access=r and first share=w is an unusual combination: "I read but I don't allow others to read, yet I allow others to write". Windows might reject any second open that wants to write if the first doesn't also share read? No, that doesn't make sense.

Wait, maybe the issue is that first share=w means the first process is willing to share write, but since first access is read, the system considers that the file is opened for read, and write sharing is irrelevant? No.

Let me look at first=(w,r) rows:
- share.w.r.r.0: fail
- share.w.r.r.r: fail
- share.w.r.r.w: success
- share.w.r.r.rw: success
- share.w.r.w.0: fail
- share.w.r.w.r: fail
- share.w.r.w.w: fail
- share.w.r.w.rw: fail
- share.w.r.rw.0: fail
- share.w.r.rw.r: fail
- share.w.r.rw.w: fail
- share.w.r.rw.rw: fail

With first=(w,r) (write access, share read), only second=(r,w) and second=(r,rw) succeed. My rule:
- second=(r,w): share1=r allows r. share2=w doesn't allow w (existing access=w). Fail. But contract says success!

So my rule is definitely wrong.

Hmm, so the actual rule must be different. Let me think.

For first=(w,r), second=(r,w):
- First writes, shares read (allows others to read).
- Second reads, shares write (allows others to write).
Can they coexist? First writes, second reads. First allowed read. Second allowed write. So yes, they should coexist. Contract agrees.

For first=(r,w), second=(w,r):
- First reads, shares write (allows others to write).
- Second writes, shares read (allows others to read).
Can they coexist? First reads, second writes. First allowed write. Second allowed read. So yes, they should coexist. But contract says no.

Why the asymmetry? The only difference is which access came first. Maybe Windows has a bug or specific behavior where the order matters? Or maybe I'm misreading the labels.

Wait, let me re-examine `share.r.w.w.r` vs `share.w.r.r.w`. The contract says:
- share.r.w.w.r: first=r,w; second=w,r -> fail
- share.w.r.r.w: first=w,r; second=r,w -> success

These are symmetric in access/share, just order swapped. Windows treats them differently! That's strange but possible.

Maybe the rule is: when checking compatibility, the system only checks if the NEW open's access is allowed by ALL EXISTING shares, AND the NEW open's share is sufficient for ALL EXISTING accesses. But it doesn't check if existing shares are sufficient for new access? No, that would be one-directional.

Wait, let me try a one-directional rule: new access must be allowed by existing shares. Don't check new share against existing access.

For first=(r,w), second=(w,r): existing share=w allows new access=w. So success? But contract says fail.

For first=(w,r), second=(r,w): existing share=r allows new access=r. Success. Contract agrees.

But the first case would predict success, contrary to contract.

What if the rule is: existing access must be allowed by new share. Don't check existing share against new access.

For first=(r,w), second=(w,r): new share=r allows existing access=r. Success. But contract says fail.

For first=(w,r), second=(r,w): new share=w allows existing access=w. Success. Contract agrees.

Again, first case wrong.

So it's not simply one-directional.

Let me think about what makes `share.r.w.w.r` fail while `share.w.r.r.w` succeeds. The difference is the first open's access/share combination. In the failing case, first access is read and first share includes write but not read. In the succeeding case, first access is write and first share includes read but not write.

Maybe Windows has a rule like: if an existing open has access A and share S, then a new open with access B and share T is allowed only if:
- B is allowed by S
- A is allowed by T
- AND (if A includes read and S does not include read, then B must not include write)? Some extra condition.

Actually, I recall now: Windows sharing check might be implemented as:
```
if ((new_access & ~existing_share) != 0) conflict;
if ((existing_access & ~new_share) != 0) conflict;
```
This is my original rule. But it gives wrong answers for some rows.

Wait, maybe the access constants are different. GENERIC_READ = 0x80000000, GENERIC_WRITE = 0x40000000. But Windows sharing check uses FILE_READ_DATA (0x0001) and FILE_WRITE_DATA (0x0002) bits, not GENERIC_READ/WRITE. When you request GENERIC_READ, the system maps it to FILE_READ_DATA, FILE_READ_ATTRIBUTES, etc. For sharing, it checks the specific bits.

But GENERIC_READ includes FILE_READ_DATA. GENERIC_WRITE includes FILE_WRITE_DATA. So my check using GENERIC_READ/WRITE bits should be equivalent for the probe's purposes.

Hmm.

Let me look at the actual Windows CreateFile sharing behavior. I recall that the sharing check is:
- For each open handle to the file, check:
  - desired_access & ~share_mode_of_existing == 0
  - desired_access_of_existing & ~share_mode_of_new == 0

But maybe there's an additional check: the new share_mode must not conflict with the existing share_mode? Like, if existing doesn't share read, and new doesn't share write, then... no.

Wait, I think I need to look at the contract more carefully. Maybe the row labels are different than I think. Let me re-read the probe's join function and row generation.

```c
row(join("share", ACCESS_NAME[a1], SHARE_NAME[s1], ACCESS_NAME[a2], SHARE_NAME[s2]));
```

So yes, format is share.<access1>.<share1>.<access2>.<share2>.

And the ACCESS_NAME mapping: 0=r, 1=w, 2=rw. SHARE_NAME: 0=0, 1=r, 2=w, 3=rw.

So `share.r.w.w.r` = a1=0 (r), s1=2 (w), a2=1 (w), s2=1 (r). Correct.

Now, maybe the issue is that the first handle in `share.r.w.w.r` is actually not valid? The contract says "first": true. So first open succeeded. Good.

Let me look at the full sharing matrix from the contract to deduce the rule.

I'll organize by (first_access, first_share) and (second_access, second_share).

For first=(r,0): all second fail. Because first doesn't share anything.
For first=(w,0): all second fail.
For first=(rw,0): all second fail.

For first=(r,r): 
- second=(r,0): fail (share1=r allows r, share2=0 doesn't allow r)
- second=(r,r): success
- second=(r,w): fail (share2=w doesn't allow r)
- second=(r,rw): success
- second=(w,0): fail
- second=(w,r): fail (share1=r doesn't allow w)
- second=(w,w): fail
- second=(w,rw): fail
- second=(rw,...): all fail because share1=r doesn't allow w component of rw

For first=(r,w):
- second=(r,*): all fail because share1=w doesn't allow r
- second=(w,0): fail
- second=(w,r): fail (contract)
- second=(w,w): fail
- second=(w,rw): success
- second=(rw,*): all fail

For first=(r,rw):
- second=(r,0): fail
- second=(r,r): success
- second=(r,w): fail
- second=(r,rw): success
- second=(w,0): fail
- second=(w,r): success
- second=(w,w): fail
- second=(w,rw): success
- second=(rw,0): fail
- second=(rw,r): success
- second=(rw,w): fail
- second=(rw,rw): success

For first=(w,r):
- second=(r,0): fail
- second=(r,r): fail (contract)
- second=(r,w): success
- second=(r,rw): success
- second=(w,*): all fail because share1=r doesn't allow w
- second=(rw,*): all fail

For first=(w,w):
- second=(r,0): fail
- second=(r,r): fail
- second=(r,w): fail
- second=(r,rw): fail
- second=(w,0): fail
- second=(w,r): fail
- second=(w,w): success
- second=(w,rw): success
- second=(rw,*): all fail

For first=(w,rw):
- second=(r,0): fail
- second=(r,r): fail (contract)
- second=(r,w): success
- second=(r,rw): success
- second=(w,0): fail
- second=(w,r): fail
- second=(w,w): success
- second=(w,rw): success
- second=(rw,*): all fail

For first=(rw,r):
- second=(r,0): fail
- second=(r,r): fail (contract)
- second=(r,w): fail
- second=(r,rw): success
- second=(w,0): fail
- second=(w,r): fail
- second=(w,w): fail
- second=(w,rw): fail
- second=(rw,0): fail
- second=(rw,r): fail
- second=(rw,w): fail
- second=(rw,rw): success

For first=(rw,w):
- second=(r,0): fail
- second=(r,r): fail
- second=(r,w): fail
- second=(r,rw): fail
- second=(w,0): fail
- second=(w,r): fail
- second=(w,w): fail
- second=(w,rw): success
- second=(rw,0): fail
- second=(rw,r): fail
- second=(rw,w): fail
- second=(rw,rw): success

For first=(rw,rw):
- second=(r,0): fail
- second=(r,r): success
- second=(r,w): fail
- second=(r,rw): success
- second=(w,0): fail
- second=(w,r): fail
- second=(w,w): success
- second=(w,rw): success
- second=(rw,0): fail
- second=(rw,r): success
- second=(rw,w): success
- second=(rw,rw): success

OK, from this matrix, I can try to deduce the rule.

Observation: When first share doesn't include read (i.e., share=0, w, or rw? wait rw includes read), the behavior differs.

Let me define:
- R = access has read
- W = access has write
- SR = share has read
- SW = share has write

Standard rule: new allowed if (R2 => SR1) and (W2 => SW1) and (R1 => SR2) and (W1 => SW2).

But this fails for some cases. Let me see which cases it fails for:

1. first=(r,w), second=(w,r): standard says success, contract says fail.
   R1=true, W1=false, SR1=false, SW1=true
   R2=false, W2=true, SR2=true, SW2=false
   R2=>SR1: true=>false? false. Wait R2 is false, so implication is true.
   W2=>SW1: true=>true: true.
   R1=>SR2: true=>true: true.
   W1=>SW2: false=>false: true.
   Standard says success. Contract says fail.

2. first=(w,r), second=(r,r): standard says success, contract says fail.
   R1=false, W1=true, SR1=true, SW1=false
   R2=true, W2=false, SR2=true, SW2=false
   R2=>SR1: true=>true: true.
   W2=>SW1: false=>false: true.
   R1=>SR2: false=>true: true.
   W1=>SW2: true=>false: false.
   Standard says fail. Contract says fail. OK.

3. first=(r,rw), second=(w,r): standard says success, contract says success.
   R1=true, W1=false, SR1=true, SW1=true
   R2=false, W2=true, SR2=true, SW2=false
   All implications true. OK.

4. first=(rw,r), second=(r,rw): standard says success, contract says success.
   OK.

5. first=(rw,r), second=(r,r): standard says? 
   R1=true, W1=true, SR1=true, SW1=false
   R2=true, W2=false, SR2=true, SW2=false
   W1=>SW2: true=>false: false. Standard says fail. Contract says fail. OK.

6. first=(w,rw), second=(r,r): standard says success, contract says fail.
   R1=false, W1=true, SR1=true, SW1=true
   R2=true, W2=false, SR2=true, SW2=false
   All implications true. Standard says success. Contract says fail.

7. first=(rw,w), second=(w,rw): standard says success, contract says success.
   OK.

So standard rule fails for:
- first=(r,w), second=(w,r)
- first=(w,rw), second=(r,r)
- possibly others.

What's common? In both failing cases, the first open has access that doesn't include one of the share bits. first=(r,w): access=r (no write), share=w (includes write). first=(w,rw): access=w (no read), share=rw (includes read). 

And the second open requests the access that the first open doesn't have but shares.

Maybe the rule is: an open cannot share an access it doesn't have? That is, share_mode must be a subset of desired_access? Let's check:

For first=(r,w): share=w, access=r. share includes write, access doesn't. If rule is share ⊆ access, this first open would be invalid? But contract says first=true, so it succeeded. So that's not it.

Hmm.

Maybe the rule is about the second open: the second open's share must be a subset of its access? For second=(w,r): share=r, access=w. share includes read, access doesn't. If invalid, then second would fail. That matches case 1! For case 6, second=(r,r): share=r, access=r. OK, so that doesn't explain.

Wait, case 6: first=(w,rw), second=(r,r). Second share=r, access=r. Valid. But contract says fail.

Maybe the rule is about the first open? No, first succeeded.

Let me think about this differently. Maybe Windows uses a table-based approach where certain combinations are hardcoded.

Actually, I recall that Windows sharing check might consider that if you open a file for read and share write, you're essentially saying "I don't care about writes", but the system still treats it as a read open. The conflict might arise from the fact that the first open's share mode is "write only" while the second open wants to write and share read — the system sees that the second open wants to share read, but the first open is reading and didn't allow read sharing, so... wait, the second open doesn't need read sharing for itself, it offers read sharing to others.

Hmm.

Let me look at this from the perspective of what each open "reserves":
- An open with access A and share S reserves the right to do A, and promises to allow others to do S.
- For two opens to coexist, the reservations must not conflict: each open's access must be within what the other allows.

But that's the standard rule.

Maybe Windows has an additional rule: the intersection of share modes must allow both accesses. That is, (access1 | access2) ⊆ (share1 & share2).

For case 1: access1=r, access2=w, share1=w, share2=r. share1 & share2 = 0. access1 | access2 = rw. Not subset. Fail. Matches!
For case 6: access1=w, access2=r, share1=rw, share2=r. share1 & share2 = r. access1 | access2 = rw. Not subset. Fail. Matches!
For standard success case first=(r,rw), second=(w,r): access1=r, access2=w, share1=rw, share2=r. share1 & share2 = r. access1 | access2 = rw. Not subset (w not in r). But contract says success! So this rule is too strict.

Hmm.

Let me try: access of each must be allowed by the OTHER's share, AND the intersection of shares must allow the intersection of accesses? No.

Actually, let me look at the success cases where standard rule works and see if there's a pattern.

Maybe the rule is the standard rule, but with an extra condition when one of the opens has a share bit that doesn't match its access. Specifically, if first access doesn't include write but first share includes write, then second access must not include write? Let's check:

Case 1: first=(r,w), second=(w,r). first access lacks write, share has write. second access has write. So reject. Matches.
Case 6: first=(w,rw), second=(r,r). first access lacks read, share has read. second access has read. So reject. Matches.

But is this a general rule? "If an existing open shares an access it doesn't have, a new open cannot request that access." 

For first=(r,w), second=(w,rw): first access lacks write, share has write. second access has write. Reject? But contract says success!

So that's not it either.

Hmm, what's different between second=(w,r) and second=(w,rw) when first=(r,w)? The second's share. With rw, it succeeds; with r, it fails.

Maybe the rule is: if existing share has a bit that existing access doesn't have, then new share must also have that bit? For first=(r,w), the extra bit is write (share has write, access doesn't). New share must have write. second=(w,r): share=r, lacks write. Reject. second=(w,rw): share=rw, has write. Accept. Matches!

For first=(w,rw), extra bit is read. New share must have read. second=(r,r): share=r, has read. Accept? But contract says fail. Hmm.

Wait, maybe it's the opposite: new share must NOT have the extra bit? For first=(w,rw), extra bit read. second=(r,r) has read. Reject. Matches. For first=(r,w), extra bit write. second=(w,r) doesn't have write in share. Accept? But contract says reject. Contradiction.

Let me try another angle. Maybe the rule depends on which access is "heavier" or the order.

Actually, I wonder if the contract rows are correct. They claim to be observed on Windows Server 2025 and 2022. So they are correct.

Let me search online for the exact Windows sharing algorithm. I recall seeing a table like:

```
                Existing share
                0   R   W   RW
New access
R               X   O   X   O
W               X   X   O   O
RW              X   X   X   O
```

But this is only one direction (new access vs existing share). The full check is bidirectional.

Wait, maybe the contract rows can be explained if the check is:
- new_access must be allowed by existing_share
- AND existing_access must be allowed by new_share
- AND if existing_access includes write, then new_share must include write? No.

Let me try to find a logical formula that matches all rows. This is getting complex. Maybe I should just implement a lookup table based on the contract matrix. But that's 3x4 x 3x4 = 144 entries, manageable.

Actually, the probe only tests the combinations in the contract. But a lookup table would be fragile. Better to deduce the rule.

Let me look at the matrix again and try to find the rule.

Define compatibility function C(a1,s1,a2,s2).

From the data:
- C(r,0,*,*) = false
- C(w,0,*,*) = false
- C(rw,0,*,*) = false
- C(*,*,*,0) for second is often false, but not always (e.g., first=(r,r), second=(r,0) is false because share2=0 doesn't allow existing access).

Actually, the standard rule seems to work for most cases except a few. Let me list all cases where standard rule and contract disagree.

Standard rule: allowed if (a2 & ~s1) == 0 and (a1 & ~s2) == 0.

Disagreements:
1. (r,w,w,r): standard true, contract false.
2. (w,rw,r,r): standard true, contract false.
3. (w,r,r,r): standard false, contract false. OK.
4. (r,w,w,rw): standard true, contract true. OK.

Wait, are there others? Let me check (rw,r,r,rw): standard? a2=rw, ~s1=~r = ...11111110. a2 & ~s1 = w. Not 0. So standard false. Contract? first=(rw,r), second=(r,rw): success. Wait a2=r, not rw. Let me recheck.

first=(rw,r), second=(r,rw): a1=rw, s1=r, a2=r, s2=rw.
Standard: (a2 & ~s1) = r & ~r = 0. (a1 & ~s2) = rw & ~rw = 0. True. Contract true. OK.

first=(rw,r), second=(r,r): a1=rw, s1=r, a2=r, s2=r.
Standard: (a2 & ~s1)=0. (a1 & ~s2)=rw & ~r = w. Not 0. False. Contract false. OK.

first=(w,rw), second=(r,r): a1=w, s1=rw, a2=r, s2=r.
Standard: (a2 & ~s1)=r & ~rw = 0. (a1 & ~s2)=w & ~r = w. Not 0. False. Contract false. OK! Wait earlier I said standard true, but let me recalculate.

~s2 = ~r = all bits except read. a1=w. w & ~r = w (since w is not read bit). So not 0. Standard false. Contract false. OK, no disagreement!

Let me recheck case 1: first=(r,w), second=(w,r). a1=r, s1=w, a2=w, s2=r.
(a2 & ~s1) = w & ~w = 0.
(a1 & ~s2) = r & ~r = 0.
Standard true. Contract false. Disagreement.

Case 2: first=(w,rw), second=(r,r). a1=w, s1=rw, a2=r, s2=r.
(a2 & ~s1) = r & ~rw = 0.
(a1 & ~s2) = w & ~r = w. Not 0. Standard false. Contract false. OK.

So only case 1 disagrees? Let me check more carefully.

first=(r,w), second=(w,w): a1=r, s1=w, a2=w, s2=w.
(a2 & ~s1)=0. (a1 & ~s2)=r & ~w = r. Not 0. Standard false. Contract false. OK.

first=(r,w), second=(w,0): a2=w, s2=0. (a1 & ~s2)=r & ~0 = r. Not 0. False. OK.

first=(r,w), second=(w,rw): a2=w, s2=rw. (a2 & ~s1)=0. (a1 & ~s2)=r & ~rw = 0. True. Contract true. OK.

So only one disagreement: (r,w,w,r). Are there others?

first=(w,r), second=(r,w): a1=w, s1=r, a2=r, s2=w.
(a2 & ~s1)=r & ~r=0. (a1 & ~s2)=w & ~w=0. True. Contract true. OK.

first=(w,r), second=(r,r): a1=w, s1=r, a2=r, s2=r.
(a2 & ~s1)=0. (a1 & ~s2)=w & ~r=w. False. Contract false. OK.

first=(w,r), second=(r,rw): a2=r, s2=rw. (a1 & ~s2)=w & ~rw=0. True. Contract true. OK.

first=(w,rw), second=(r,w): a1=w, s1=rw, a2=r, s2=w.
(a2 & ~s1)=0. (a1 & ~s2)=w & ~w=0. True. Contract true. OK.

first=(w,rw), second=(r,rw): a2=r, s2=rw. (a1 & ~s2)=0. True. Contract true. OK.

first=(rw,r), second=(r,r): a1=rw, s1=r, a2=r, s2=r.
(a2 & ~s1)=0. (a1 & ~s2)=rw & ~r=w. False. Contract false. OK.

first=(rw,r), second=(w,rw): a2=w, s2=rw. (a2 & ~s1)=w & ~r=w. False. Contract false. OK.

first=(rw,rw), second=(r,w): a1=rw, s1=rw, a2=r, s2=w.
(a2 & ~s1)=0. (a1 & ~s2)=rw & ~w=r. False. Contract false. OK.

first=(rw,rw), second=(w,r): a2=w, s2=r. (a2 & ~s1)=0. (a1 & ~s2)=rw & ~r=w. False. Contract false. OK.

So the ONLY disagreement between standard rule and contract is (r,w,w,r). That's just one row! Let me verify by checking the contract row for `share.r.w.w.r` again.

Yes, contract says fail. Standard rule says success.

Why would Windows reject this specific combination? first=(r, share write), second=(w, share read). 

Maybe it's a Windows bug, or maybe my standard rule is missing a subtlety. Let me think about the semantics:
- First opens for read, and says "I allow others to write".
- Second opens for write, and says "I allow others to read".

The issue might be that the second open wants to write, but the first open is reading. Even though first allowed write, the second open's share read means "I allow others to read", but the first open is already reading. The system might think: if second writes while first reads, and second doesn't share write, then... no, second is the one writing, it doesn't need to share write.

Wait, maybe the issue is that second's share read conflicts with first's lack of share read? No, first's share is write, not read. First doesn't allow others to read. But second is offering read sharing. The system might check: does first's access require the sharing that second offers? First reads, so it requires read sharing from others. Second offers read sharing. So that should be OK.

Hmm, I'm stuck. But since only one row disagrees, maybe I can special-case it. But that feels hacky.

Wait, let me re-examine. Is it possible that the contract row is a typo? The evidence says it's observed on two Windows builds. So it's real.

Maybe the standard rule is correct but I need to also consider that the first open's share mode must allow the second open's share mode? No.

Actually, I just realized: maybe Windows checks sharing in a specific order and the result depends on which handle is opened first. But in the probe, the first handle is always opened first, then the second. So the order is fixed.

Let me think about the Windows implementation. I recall that the sharing check in Windows is done by comparing the new open's access against the existing opens' share modes, and the new open's share mode against the existing opens' accesses. But maybe it also checks the new open's access against the new open's share mode? Like, you can't open for write without sharing write? No, that's not a rule.

Or maybe: the system computes the "effective share" of all opens and ensures it's compatible. The effective share is the intersection of all share modes. Then any access by any open must be allowed by the effective share.

For (r,w) and (w,r): effective share = w & r = 0. No access allowed. But first reads and second writes. Both need access. So fail. Matches!
For (w,r) and (r,w): effective share = r & w = 0. Fail? But contract says success!

So that's not it.

Hmm.

Let me try: effective share must allow the union of accesses, but only considering opens that don't share that access themselves? No.

OK, I think the most practical approach is to use the standard rule and special-case the one disagreement. But let me see if there are more disagreements I missed.

Actually, let me systematically check all 144 combinations. I'll write a small mental script.

Standard rule: allowed = ((a2 & ~s1) == 0) && ((a1 & ~s2) == 0).

I'll go through each first=(a1,s1) and compare with contract.

I already did most. Let me be thorough for first=(r,w):

second access can be r, w, rw.
- second=(r,*): a2=r, ~s1=~w. r & ~w = r (since r and w are different bits). Not 0. Fail for all share. Contract: all fail. OK.
- second=(w,0): a2=w, ~s1=~w. w & ~w=0. a1=r, ~s2=~0=all. r & all=r. Not 0. Fail. Contract fail. OK.
- second=(w,r): a2=w, ~s1=~w=0. a1=r, ~s2=~r. r & ~r=0. True. Contract: fail. DISAGREEMENT.
- second=(w,w): a2=w, ~s1=0. a1=r, ~s2=~w. r & ~w=r. Not 0. Fail. Contract fail. OK.
- second=(w,rw): a2=w, ~s1=0. a1=r, ~s2=~rw. r & ~rw=0. True. Contract true. OK.
- second=(rw,0): a2=rw, ~s1=~w. rw & ~w=r. Not 0. Fail. OK.
- second=(rw,r): a2=rw, ~s1=~w. rw & ~w=r. Not 0. Fail. OK.
- second=(rw,w): a2=rw, ~s1=~w. rw & ~w=r. Not 0. Fail. OK.
- second=(rw,rw): a2=rw, ~s1=~w. rw & ~w=r. Not 0. Fail. OK.

So only (r,w,w,r) disagrees for first=(r,w).

Now first=(w,rw):

- second=(r,0): a2=r, ~s1=~rw. r & ~rw=0. a1=w, ~s2=~0=all. w & all=w. Not 0. Fail. OK.
- second=(r,r): a2=r, ~s1=0. a1=w, ~s2=~r. w & ~r=w. Not 0. Fail. OK.
- second=(r,w): a2=r, ~s1=0. a1=w, ~s2=~w. w & ~w=0. True. Contract true. OK.
- second=(r,rw): a2=r, ~s1=0. a1=w, ~s2=~rw. w & ~rw=0. True. Contract true. OK.
- second=(w,0): a2=w, ~s1=~rw. w & ~rw=0. a1=w, ~s2=all. w & all=w. Not 0. Fail. OK.
- second=(w,r): a2=w, ~s1=0. a1=w, ~s2=~r. w & ~r=w. Not 0. Fail. OK.
- second=(w,w): a2=w, ~s1=0. a1=w, ~s2=~w. w & ~w=0. True. Contract true. OK.
- second=(w,rw): a2=w, ~s1=0. a1=w, ~s2=~rw. w & ~rw=0. True. Contract true. OK.
- second=(rw,0): a2=rw, ~s1=~rw. rw & ~rw=0. a1=w, ~s2=all. w. Fail. OK.
- second=(rw,r): a2=rw, ~s1=~rw. rw & ~rw=0. a1=w, ~s2=~r. w. Fail. OK.
- second=(rw,w): a2=rw, ~s1=~rw. 0. a1=w, ~s2=~w. 0. True. Contract true. OK.
- second=(rw,rw): a2=rw, ~s1=~rw. 0. a1=w, ~s2=~rw. 0. True. Contract true. OK.

No disagreements for first=(w,rw).

Now first=(rw,r):

- second=(r,0): a2=r, ~s1=~r. r & ~r=0. a1=rw, ~s2=all. rw. Fail. OK.
- second=(r,r): a2=r, ~s1=~r=0. a1=rw, ~s2=~r. rw & ~r=w. Fail. OK.
- second=(r,w): a2=r, ~s1=0. a1=rw, ~s2=~w. rw & ~w=r. Fail. OK.
- second=(r,rw): a2=r, ~s1=0. a1=rw, ~s2=~rw. 0. True. Contract true. OK.
- second=(w,0): a2=w, ~s1=~r. w & ~r=w. Fail. OK.
- second=(w,r): a2=w, ~s1=~r. w. Fail. OK.
- second=(w,w): a2=w, ~s1=~r. w. Fail. OK.
- second=(w,rw): a2=w, ~s1=~r. w. Fail. OK.
- second=(rw,0): a2=rw, ~s1=~r. rw & ~r=w. Fail. OK.
- second=(rw,r): a2=rw, ~s1=~r. w. Fail. OK.
- second=(rw,w): a2=rw, ~s1=~r. w. Fail. OK.
- second=(rw,rw): a2=rw, ~s1=~r. w. Fail. OK.

No disagreements.

first=(rw,w):

- second=(r,0): a2=r, ~s1=~w. r & ~w=r. Fail. OK.
- second=(r,r): a2=r, ~s1=~w. r. Fail. OK.
- second=(r,w): a2=r, ~s1=~w. r. Fail. OK.
- second=(r,rw): a2=r, ~s1=~w. r. Fail. OK.
- second=(w,0): a2=w, ~s1=~w=0. a1=rw, ~s2=all. rw. Fail. OK.
- second=(w,r): a2=w, ~s1=0. a1=rw, ~s2=~r. w. Fail. OK.
- second=(w,w): a2=w, ~s1=0. a1=rw, ~s2=~w. r. Fail. OK.
- second=(w,rw): a2=w, ~s1=0. a1=rw, ~s2=~rw. 0. True. Contract true. OK.
- second=(rw,0): a2=rw, ~s1=~w. rw & ~w=r. Fail. OK.
- second=(rw,r): a2=rw, ~s1=~w. r. Fail. OK.
- second=(rw,w): a2=rw, ~s1=~w. r. Fail. OK.
- second=(rw,rw): a2=rw, ~s1=~w. r. Fail. OK.

No disagreements.

first=(rw,rw): standard allows everything where second share includes both a1 and a2. Contract matches.

first=(r,r): standard allows second=(r,r), (r,rw), and rejects others. Contract matches.

first=(w,w): standard allows second=(w,w), (w,rw). Contract matches.

first=(w,r): standard allows second=(r,w), (r,rw). Contract matches.

first=(r,rw): standard allows second where s2 includes r if a1=r (always true for second access r or rw) and s2 includes w if a1 has w (a1=r doesn't). And a2 allowed by s1=rw (always). So standard allows second=(r,r), (r,rw), (w,r), (w,rw), (rw,r), (rw,rw). Contract matches exactly! Good.

So the ONLY disagreement is (r,w,w,r). That's bizarre but manageable.

Wait, let me double-check the contract for `share.r.w.w.r` one more time. Searching... yes:
```
   {
    "s": "share.r.w.w.r",
    "first": true,
    "valid": false,
    "err": 32
   },
```

And `share.w.r.r.w`:
```
   {
    "s": "share.w.r.r.w",
    "first": true,
    "valid": true,
    "err": 0
   },
```

So Windows is asymmetric. The only explanation I can think of is that Windows has a specific check: when the existing open has read access and write share (but not read share), and the new open has write access and read share (but not write share), it fails. This might be due to a quirk in how Windows tracks sharing.

Since it's just one case, I can special-case it. But maybe there's a more general rule I'm missing.

Actually, let me think about this: maybe Windows computes the "required share" for each open as the complement of its access? No.

Or maybe Windows uses a different mapping of GENERIC_READ/WRITE to file-specific rights for sharing purposes. GENERIC_READ maps to FILE_READ_DATA | FILE_READ_ATTRIBUTES | ... and GENERIC_WRITE maps to FILE_WRITE_DATA | FILE_WRITE_ATTRIBUTES | ... For sharing, maybe it checks FILE_READ_DATA and FILE_WRITE_DATA only. But that's what I'm doing.

Hmm.

Actually, I wonder if the issue is that the first open with (r,w) is considered to have "share write" but since it opened for read, the system internally also requires read sharing? No.

OK, I'll just use the standard rule and add a special case for (a1=r, s1=w, a2=w, s2=r). It's ugly but matches the contract.

Wait, but maybe there are other asymmetries I haven't checked. Let me check (w,r,r,r): first=(w,r), second=(r,r). Standard: a2=r & ~s1=~r=0. a1=w & ~s2=~r=w. Fail. Contract fail. OK.

(w,rw,r,r): first=(w,rw), second=(r,r). Standard: a2=r & ~s1=~rw=0. a1=w & ~s2=~r=w. Fail. Contract fail. OK.

(r,w,r,r): first=(r,w), second=(r,r). Standard: a2=r & ~s1=~w=r. Fail. OK.

So only (r,w,w,r) is the odd one.

Actually, wait. Let me also check (r,w,rw,r): first=(r,w), second=(rw,r). Standard: a2=rw & ~s1=~w=r. Fail. OK.

And (r,w,rw,w): a2=rw & ~s1=~w=r. Fail. OK.

So yes, only one.

Hmm, but I want to be sure. Let me also check if the contract has any other asymmetric successes.

Looking at first=(r,w): only second=(w,rw) succeeds.
Looking at first=(w,r): second=(r,w) and (r,rw) succeed.

So (r,w) is more restrictive than (w,r). The only difference is the access/share "mismatch" direction.

OK, I'll implement standard rule plus special case.

Actually, you know what, let me reconsider whether the special case is really needed. The contract says `share.r.w.w.r` fails. If I implement standard rule, my output would have `valid=true, err=0` for that row, causing divergence. So I need the special case.

Alternatively, maybe I can find a more general rule. Let me think...

What if the rule is: the new open's share mode must allow the existing open's access, AND the existing open's share mode must allow the new open's access, AND additionally, the new open's share mode must allow the new open's access? 

For (r,w,w,r): new share=r, new access=w. r doesn't allow w. Fail. Matches!
For (w,r,r,w): new share=w, new access=r. w doesn't allow r. Fail. But contract says success. So no.

What if: existing share must allow existing access? For (r,w): existing share=w doesn't allow existing access=r. Fail? But first open succeeded, so existing share doesn't need to allow existing access.

What if: if existing access doesn't include a bit that existing share includes, then new access must not include that bit? For (r,w): existing access lacks write, existing share has write. New access has write. Fail. Matches.
For (w,rw): existing access lacks read, existing share has read. New access has read. Fail. Matches `share.w.rw.r.r` which fails.
But for (w,rw), second=(r,w) succeeds. New access=r, existing access lacks read, existing share has read. According to rule, fail. But contract says success. So rule too strict.

Hmm.

What if the rule is: if existing access lacks bit X but existing share has bit X, then new share must have bit X? For (r,w): existing lacks write, share has write. New share must have write. second=(w,r): share=r, lacks write. Fail. Matches. second=(w,rw): share=rw, has write. Success. Matches.
For (w,rw): existing lacks read, share has read. New share must have read. second=(r,r): share=r, has read. Success? But contract says fail. Hmm.

Wait, for (w,rw), second=(r,r) fails. New share=r has read. So rule would predict success, but contract says fail.

So the rule works for first access lacking write but not for first access lacking read? That doesn't make sense.

Let me check (w,rw) more carefully. first=(w,rw), second=(r,r) fails. But first=(w,rw), second=(r,w) succeeds. The difference is new share: r vs w.

For first=(w,rw), the "extra" share bit is read (existing share has read, access doesn't). New share=r has read. According to the earlier hypothesis, new share must have the extra bit, so r should be OK. But contract says fail.

What if the rule is: new share must NOT have the extra bit? For (w,rw), extra=read. New share=r has read. Fail. Matches. New share=w lacks read. Success. Matches.
For (r,w), extra=write. New share=r lacks write. Success? But contract says fail for (r,w,w,r). Contradiction.

So the rule direction depends on whether extra is read or write? That's too arbitrary.

OK, I give up on finding a clean rule. I'll use the standard rule and add a special case for (r,w,w,r). Actually, let me also check if there are other special cases by comparing standard rule to all contract rows. I believe only (r,w,w,r) differs.

Wait, I should also consider that maybe the standard rule is correct and the contract row is wrong. But the problem says the contract is the ground truth. So I must match it.

Actually, let me re-read the contract's open questions. It doesn't mention this asymmetry. Hmm.

OK, special case it is.

Now, let me also think about whether there might be other issues with my implementation.

One issue: for `k32_CreateFileW`, when the file exists and disposition is CREATE_ALWAYS, I truncate. But I should set last error to ERROR_ALREADY_EXISTS. My code does that. Good.

For OPEN_ALWAYS on existing, set ERROR_ALREADY_EXISTS. Good.

For CREATE_NEW on absent, set 0. Good.

For TRUNCATE_EXISTING on existing, set 0. Good.

Now, for `k32_DeleteFileW`, I should NOT set last error on success. The probe expects 57005 (sentinel). My previous code set 0. I need to fix.

For `k32_ReadFile` success, don't set error. Previous code set 0. Fix.

For `w2m_file_write` success, don't set error. Previous code set 0. Fix.

For `k32_SetFilePointerEx` success, don't set error. Previous code set 0. Fix.

For `k32_CloseHandle` success, set 0. The probe doesn't check but it's correct.

For `k32_CreateFileW` success, set 0 or 183. Correct.

Now, the path length issue. I need to pass the byte length of the normalized path to FILE_LOOKUP, FILE_OPEN, FILE_UNLINK.

Let me also make sure I handle the `chars` parameter correctly. I'll compute `path_len` as the length of the ASCII string (not including null).

Now, let me also reconsider the host primitive for FILE_OPEN. It says "FILE_OPEN (path, chars, create) -> host descriptor, or -errno; create=1 is O_CREAT|O_EXCL; arg[5] the file id". So for create=1, if file exists, returns -EEXIST. For create=0, opens existing. But what about creating with CREATE_ALWAYS? I use create=0 and then truncate. Good.

But for CREATE_NEW on absent, I use create=1. Good.

For OPEN_ALWAYS on absent, I use create=1. Good.

For TRUNCATE_EXISTING on existing, I use create=0 and truncate. Good.

For OPEN_EXISTING on existing, create=0. Good.

Now, what about the host's behavior when create=0 and file doesn't exist? It returns -ENOENT. But I check LOOKUP first, so I don't call OPEN in that case.

What about create=1 and path has missing directory? Host returns -ENOENT or -ENOTDIR. But I check LOOKUP first (kind=3), so I don't call OPEN.

OK.

Now, the `name.missing_dir.create_new` expects err=3 (ERROR_PATH_NOT_FOUND). My LOOKUP returns kind=3, I set ERROR_PATH_NOT_FOUND. Good.

`name.empty` expects err=3. My normalize returns ERROR_PATH_NOT_FOUND. Good.

`name.missing_dir.open_existing` expects err=3. Good.

`name.dot.no_backup_semantics` expects err=5. I need to handle ".".

`name.trailing_dot` expects valid=true, err=0. My stripping makes "name." -> "name". Good.

`name.trailing_space` expects valid=true, err=0. My stripping makes "name " -> "name". Good.

`name.case.open_existing` expects valid=true, err=0. My case-folding makes "W2M-CF-A.TXT" -> "W2M-CF-A.TXT" (already upper). It matches the actual file "w2m-cf-a.txt" because host case-folds too. Good.

`name.case.create_new` expects valid=false, err=80. My LOOKUP finds existing file (case-folded match), must_not_exist=1, returns ERROR_FILE_EXISTS. Good.

`name.other_absent` expects valid=false, err=2. LOOKUP kind=0, must_create=0, returns ERROR_FILE_NOT_FOUND. Good.

Now, `io.write_on_read_handle`: handle opened with GENERIC_READ. WriteFile should fail with ERROR_ACCESS_DENIED (5), done=0. My w2m_file_write checks access & GENERIC_WRITE. Good.

`io.read_on_write_handle`: handle opened with GENERIC_WRITE. ReadFile should fail with ERROR_ACCESS_DENIED (5), done=0. My k32_ReadFile checks access & GENERIC_READ. Good.

`close.twice`: close an already closed handle. My lookup returns NULL, set ERROR_INVALID_HANDLE (6), return 0. Good.

Now, let me also think about `k32_CloseHandle` on stdout handle (0x74). The runtime.c probably handles stdout separately. My file.c only handles file handles. If called with 0x74, lookup returns NULL, returns 0 with ERROR_INVALID_HANDLE. But runtime.c might call CloseHandle on stdout? The probe doesn't. Fine.

Now, the handle allocation. I need to make sure I don't use fd==0 as the free marker. I'll add `used` flag.

Also, when I free an entry, I set used=0.

Now, let me write the code.

One more thing: the `w2m_host` function might clobber the `second` value even when not used. I always pass `&second` and read it when needed. Good.

Now, let me also think about whether I need to pass the path length in characters or bytes. I'll pass bytes. If the host expects UTF-16 code units, it would be wrong for non-ASCII, but the probe is ASCII. For ASCII, byte length = UTF-16 code unit count. So it works.

But wait, if the host reads `chars` bytes and my path is ASCII bytes, it reads the correct string. If it reads `chars` UTF-16 code units (i.e., 2*chars bytes), it would read garbage. So I need to know which.

Given the previous code passed 0 and got error 5, and the host description says "Paths arrive as UTF-16LE as the guest normalized them", maybe `chars` is the number of UTF-16 code units, and the host reads 2*chars bytes. If I pass ASCII bytes, the host would interpret them as UTF-16LE and get wrong characters (e.g., 'a' -> U+0061, then next byte is 'b' -> U+6200 or something). So I should pass UTF-16LE.

Hmm, this is important. Let me re-read the host description: "Paths arrive as UTF-16LE as the guest normalized them. Components compare ASCII case-folded, as NTFS compares ASCII names (its upcase table; non-ASCII folding is not needed by any recorded row and is not done)."

This says paths arrive as UTF-16LE. So the runtime should pass UTF-16LE. The host then compares components ASCII case-folded.

So I should build a UTF-16LE normalized path and pass its length in UTF-16 code units.

But wait, the runtime is supposed to implement name rules (stripping trailing dots/spaces, case-folding). The host does case-folding. So maybe the runtime should pass the UTF-16LE path with trailing dot/space stripped, and the host does case-folding.

But the contract says the runtime implements "name forms". Hmm.

Actually, the contract's open questions mention: "Naming Files says only 'Do not end a file or directory name with a space or a period'; the rows show Windows opens 'name.' and 'name ' as 'name'." This is observed Windows behavior. The runtime should implement it.

So the runtime strips trailing dots/spaces, case-folds, and passes the normalized path to the host. The host then uses it directly (or case-folds again, idempotent for ASCII).

If the host expects UTF-16LE, I should pass UTF-16LE. If it expects ASCII bytes, I should pass ASCII bytes.

Given the description says "Paths arrive as UTF-16LE", I'll pass UTF-16LE. But then I need to build a UTF-16LE string.

Actually, maybe the host file_bridge.py takes the bytes and decodes them as UTF-16LE. So I should pass UTF-16LE.

Let me build a UTF-16LE normalized path. For ASCII characters, UTF-16LE is just the byte value followed by 0x00. So "ABC" -> 0x41 0x00 0x42 0x00 0x43 0x00.

The `chars` parameter would be the number of UTF-16 code units, i.e., the number of characters, which equals the byte length of my ASCII path.

Wait, if I build UTF-16LE, the byte length is 2 * char_count. If `chars` means code units, I pass char_count. If `chars` means bytes, I pass 2*char_count.

The description says "FILE_LOOKUP (path, chars)". Given paths are UTF-16LE, `chars` likely means the number of UTF-16 characters (code units). So I pass char_count, and the host reads 2*char_count bytes.

But if the host reads `chars` bytes directly, then for UTF-16LE it would read half the string. Hmm.

I think the safest is to pass the byte length of whatever I pass. If I pass UTF-16LE, byte length = 2 * char_count. If I pass ASCII, byte length = char_count.

Given the description says UTF-16LE, I'll pass UTF-16LE with byte length = 2 * char_count. But I'm not sure if `chars` means bytes or code units. Let me assume it means bytes, since the parameter is named `chars` but in a C-like host interface it might be bytes. Actually, in Python, it might be the number of bytes to read from the pointer.

Hmm. Let me think about the previous failure. With chars=0, the host couldn't read any path. If I pass the correct byte length of a UTF-16LE string, it should work. If I pass the correct code unit count but the host expects bytes, it would read too few bytes. If I pass bytes but host expects code units, it would read too many.

Given the ambiguity, I'll pass the byte length. That's the most natural for a pointer+length interface. And I'll pass UTF-16LE bytes.

Wait, but if I pass ASCII bytes with byte length, and the host expects UTF-16LE with byte length, the host would decode ASCII bytes as UTF-16LE and get garbage. So I must pass UTF-16LE.

OK, I'll build UTF-16LE and pass byte length.

Actually, you know what, let me reconsider. The host file_bridge.py is Python. It probably does something like:
```python
path_bytes = read_guest_memory(arg0, arg1)
path = path_bytes.decode('utf-16le')
```
If arg1 is bytes, it reads arg1 bytes and decodes. If arg1 is code units, it reads 2*arg1 bytes.

Given the description says "chars", it might be code units. But in Python, it's more natural to pass bytes. Hmm.

Actually, the previous code passed 0 and failed. The fix is to pass a non-zero length. Whether it's bytes or code units, passing the right value matters. For ASCII paths, byte length = code unit count. So if I pass the ASCII byte length (which equals code unit count), it works for both interpretations as long as I pass ASCII bytes.

But if I pass UTF-16LE bytes, byte length = 2 * code unit count. If host expects code units, I should pass code unit count. If host expects bytes, I should pass byte length.

Given the ambiguity, and that ASCII byte length = code unit count, I think passing ASCII bytes with the length (which is both byte length and code unit count) is the safest. The host can decode it as UTF-16LE? No, ASCII bytes decoded as UTF-16LE would be wrong.

Wait, but the host description says "Paths arrive as UTF-16LE". So the host expects UTF-16LE. If I pass ASCII bytes, the host decodes them as UTF-16LE and gets garbage (interpreting pairs of ASCII bytes as UTF-16 code units).

For example, ASCII "AB" = 0x41 0x42. Decoded as UTF-16LE: 0x4241 = U+4241, not "AB".

So I must pass UTF-16LE.

OK, I'll pass UTF-16LE and the length in bytes (2 * char_count). If the host expects code units, this will read 2*char_count bytes, which is correct for UTF-16LE. Wait, if host expects code units and I pass bytes, it will read 2*bytes = 4*char_count bytes, too much. But for the probe, the path is short, so reading extra might hit zero bytes or other data, causing garbage.

Hmm. Let me think about what `chars` means. In the context of UTF-16LE, "chars" most naturally means the number of UTF-16 code units. So I should pass char_count, and the host reads 2*char_count bytes.

But the parameter is just a u64, could be anything. The host file_bridge.py defines the interface. Without seeing it, I have to guess.

Given the previous code passed 0 and the description says "chars", I'll assume `chars` is the number of UTF-16 code units. So I pass char_count, and the host reads 2*char_count bytes.

But then I need to ensure the host reads the correct number of bytes. If it reads `chars` bytes instead, it would read half. But the description says UTF-16LE, so reading 2*chars bytes makes sense.

OK, I'll pass UTF-16LE path and char_count as `chars`.

Actually, wait. Let me reconsider one more time. The runtime.h defines the host primitives as enum values. The actual host implementation in file_bridge.py might take `arg[1]` as the number of bytes. The description "chars" might be loose terminology.

Given that the previous failure was due to passing 0, and the probe paths are ASCII, the difference between bytes and code units is a factor of 2. If I pass the wrong one, the host might still decode correctly if it reads the full string. For example, if I pass ASCII bytes with byte length, and host reads byte length bytes and decodes as UTF-16LE, it gets garbage. If I pass UTF-16LE with byte length, and host reads byte length bytes and decodes as UTF-16LE, it gets correct string. If host instead reads `chars` bytes where `chars` is code units, it gets half the string.

I think the most robust is to pass UTF-16LE and the byte length. Because:
1. The description says paths arrive as UTF-16LE.
2. A pointer+length interface typically uses byte length.
3. If the host uses code units, it would be documented as such, but "chars" is ambiguous.

But if the host uses code units, my byte length would cause it to read too much. Hmm.

Alternatively, I can pass UTF-16LE and char_count (code units). If host uses bytes, it reads too little. But for short paths, reading too little might still work if the remaining bytes are zero? No, it would truncate.

Given the previous code passed 0, any reasonable non-zero length might work if the host stops at null terminator. But the host probably uses the length.

Let me look at the failure mode again. With chars=0, all file ops returned error 5 and exists=false. This suggests the host treated the path as empty or invalid. If the host reads 0 bytes, it gets empty path. For LOOKUP, empty path -> kind=0 (absent). Then CREATE_NEW tries to create with empty path, gets EACCES -> error 5. That matches.

So the host definitely uses the length. I need to pass the correct length.

I'll go with UTF-16LE and byte length. If that doesn't work, I'd try code units. But I can only submit one answer, so I need to choose.

Actually, let me think about the host side more. file_bridge.py says "Paths arrive as UTF-16LE as the guest normalized them." In Python, to decode UTF-16LE, you need the byte string. The natural parameter is the number of bytes. So `chars` might be a misnomer for bytes. Or it might be the number of characters, and the host multiplies by 2.

Given that the runtime is C and the host is Python, and the interface is `arg[0]` pointer, `arg[1]` length, I'll assume `arg[1]` is bytes. So I pass byte length of UTF-16LE string.

OK, decision: pass UTF-16LE path and byte length.

Wait, but the runtime is supposed to normalize the path. If I pass UTF-16LE, I need to build it. Let me modify normalize_path to produce UTF-16LE in a separate buffer, or produce ASCII and then convert.

Actually, I can produce the normalized path as UTF-16LE directly during parsing. Let me have normalize_path fill a u16 buffer and return the char count. Then the caller can pass the u16 buffer pointer and byte length (char_count * 2).

But the path might need to be passed to FILE_LOOKUP/OPEN/UNLINK. I'll have the caller provide a u16 buffer.

Let me restructure normalize_path to output UTF-16LE.

Actually, I can keep the ASCII internal representation and convert at the host call site. Simpler.

Let me define:
```c
static u32 normalize_path_utf16(u64 name, u16 *out, unsigned *out_len)
```
It reads UTF-16LE input, normalizes, writes UTF-16LE output (uppercase ASCII, stripped), returns error code.

Then in CreateFileW/DeleteFileW, I pass `(u64)out, *out_len * 2` to host.

But wait, the host might expect the path to be in guest memory. The `out` buffer is in runtime stack. Is runtime stack accessible to host? I think yes.

OK, let me write this.

Actually, I realize I can also just pass the original UTF-16LE string with its length, and let the host do normalization. But the contract says the runtime implements name rules. And the host description says "as the guest normalized them". So the runtime normalizes.

Hmm, but the host also does ASCII case-folding. So maybe the runtime doesn't need to case-fold; the host does. But the runtime needs to strip trailing dots/spaces and handle "." and "..". 

Actually, the runtime can pass the original UTF-16LE path to the host, and the host does all normalization. But the contract says the runtime implements the Windows file calls including name rules. So either way, the result must match.

To be safe, I'll normalize in the runtime: strip trailing dots/spaces, case-fold to upper, handle "." and "..". Then pass UTF-16LE to host.

Now, for case-folding, the host also does it. So double case-folding is fine.

OK, let me write the code.

One more consideration: the original path is UTF-16LE. My normalization reads u16 values. For ASCII, case-folding is simple. For non-ASCII, I unimplement.

Trailing dots/spaces: I strip per component.

".." components: pop or unimplement if leaving root.

"." components: skip, but if entire path is ".", special case.

Empty path: ERROR_PATH_NOT_FOUND.

Now, let me write the code.

I'll produce a u16 output buffer. The byte length for host is `out_len * 2`.

Let me also handle the path separator in output. I'll use backslash? Or forward slash? The host says it uses one root directory and paths are relative. It probably uses the OS separator (forward slash on macOS). But the host description doesn't specify. The runtime can use either; the host just joins with root.

Actually, the host might split on '/' or '\'. Given the runtime normalizes, I'll use '/' as separator in the output. But since I'm outputting UTF-16LE, the separator is a u16 '/'. The host can handle it.

Wait, but the original Windows paths use '\'. The host might expect '\'. Hmm. The host description says "one root directory per run that stands in for the guest's working directory". It probably uses os.path.join which handles both. I'll use '/'.

Actually, to be safe, I'll use the same separator as the input? No, I'll use '/' since it's more portable.

OK, let me write the code now.

Actually, I just realized: if I pass UTF-16LE with byte length, and the host expects code units, the host will read too many bytes. But maybe the host reads exactly `chars` bytes and decodes as UTF-16LE. If `chars` is bytes and I pass byte length, correct. If `chars` is code units and I pass code units, correct.

I'll pass byte length and hope the host uses bytes. If the host uses code units, I can note that ASCII paths have byte length = code unit count only if I pass ASCII bytes, not UTF-16LE.

Hmm, this is a real dilemma. Let me think about which is more likely.

In the host file_bridge.py, the primitive definitions are:
```python
FILE_LOOKUP  (path, chars)          -> kind
FILE_OPEN    (path, chars, create)  -> host descriptor
FILE_UNLINK  (path, chars)          -> 0, or -errno
```

If the host is Python, it might do:
```python
path = read_bytes(arg0, arg1).decode('utf-16le')
```
Here arg1 is bytes. So `chars` is actually bytes.

Or it might do:
```python
path = read_bytes(arg0, arg1 * 2).decode('utf-16le')
```
Here arg1 is code units.

Given the parameter is named `chars` but the function reads memory, I think it's more likely bytes. Because reading memory by byte count is natural.

So I'll pass byte length of UTF-16LE.

OK, final decision: pass UTF-16LE path and byte length.

Now, let me write the code.

Wait, one more thing. The problem says "Handle values must not be 0x74 (standard output)." With MAX_FILES=64, handles are 1-64, so 0x74=116 is not reached. But to be safe, I'll keep a simple skip: if the computed handle would be 0x74, skip it. But with 64 entries, it won't happen. I'll just use idx+1.

Actually, I realize that if I use idx+1, and the host/runtime has stdout as 0x74, there's no conflict. So simple idx+1 is fine.

OK, writing code now.

Let me also double-check the `w2m_logged` function. It records the call and returns its third argument. So I call `return w2m_logged(api, arg0, ret);`. Good.

For `w2m_unimplemented`, it stops the run. I use it for unsupported cases. Good.

Now, the code:

```c
/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ        0x80000000u
#define GENERIC_WRITE       0x40000000u

#define FILE_SHARE_READ     0x00000001u
#define FILE_SHARE_WRITE    0x00000002u
#define FILE_SHARE_DELETE   0x00000004u

#define CREATE_NEW          1u
#define CREATE_ALWAYS       2u
#define OPEN_EXISTING       3u
#define OPEN_ALWAYS         4u
#define TRUNCATE_EXISTING   5u

#define FILE_ATTRIBUTE_NORMAL      0x00000080u
#define FILE_FLAG_BACKUP_SEMANTICS 0x02000000u

#define FILE_BEGIN          0u
#define FILE_CURRENT        1u
#define FILE_END            2u

#define INVALID_HANDLE_VALUE ((u64)(-1))

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND        2u
#define ERROR_PATH_NOT_FOUND        3u
#define ERROR_ACCESS_DENIED         5u
#define ERROR_INVALID_HANDLE        6u
#define ERROR_NEGATIVE_SEEK         131u
#define ERROR_ALREADY_EXISTS        183u
#define ERROR_MORE_DATA             234u
#define ERROR_FILE_EXISTS           80u
#define ERROR_SHARING_VIOLATION     32u
#define ERROR_INVALID_PARAMETER     87u

#define MAX_FILES 64

typedef long long s64;
typedef unsigned short u16;

struct file_entry {
    int used;
    u64 fd;
    u64 id;
    u32 access;
    u32 share;
    u64 position;
};

static struct file_entry files[MAX_FILES];

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static struct file_entry *alloc_entry(void)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (!files[i].used) {
            files[i].used = 1;
            files[i].fd = 0;
            files[i].id = 0;
            files[i].access = 0;
            files[i].share = 0;
            files[i].position = 0;
            return &files[i];
        }
    }
    return 0;
}

static void free_entry(struct file_entry *f)
{
    f->used = 0;
    f->fd = 0;
    f->id = 0;
    f->access = 0;
    f->share = 0;
    f->position = 0;
}

static unsigned handle_index(u64 h)
{
    if (h == 0 || h > MAX_FILES)
        return (unsigned)-1;
    return (unsigned)(h - 1);
}

static struct file_entry *lookup(u64 h)
{
    unsigned idx = handle_index(h);
    if (idx >= MAX_FILES)
        return 0;
    if (!files[idx].used)
        return 0;
    return &files[idx];
}

static int is_sep(u16 c)
{
    return c == '\\' || c == '/';
}

static u16 to_upper_ascii(u16 c)
{
    if (c >= 'a' && c <= 'z')
        return (u16)(c - 'a' + 'A');
    return c;
}

/* Validate and normalize a guest UTF-16LE path into a host UTF-16LE path.
 * Returns 0 on success, fills `out` with UTF-16LE code units and `out_len`
 * with the number of code units. `out` must have room for at least 512 u16.
 * Returns a Windows error code on failure.
 */
static u32 normalize_path(u64 name, u16 *out, unsigned *out_len)
{
    u16 buf[512];
    unsigned len = 0;
    unsigned i;

    if (name == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    for (i = 0; i < 511; i++) {
        u16 c = ((u16 *)name)[i];
        if (c == 0)
            break;
        buf[i] = c;
    }
    if (i == 511) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return 0;
    }
    len = i;
    if (len == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    if (is_sep(buf[0])) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return 0;
    }
    if (len >= 2 && buf[1] == ':') {
        w2m_unimplemented(W2M_API_CreateFileW, 3);
        return 0;
    }

    unsigned out_pos = 0;
    unsigned pos = 0;
    int saw_dot_only = 0;

    while (pos < len) {
        while (pos < len && is_sep(buf[pos]))
            pos++;
        if (pos >= len)
            break;

        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos]))
            pos++;

        unsigned end = pos;
        while (end > start) {
            u16 c = buf[end - 1];
            if (c == ' ' || c == '.')
                end--;
            else
                break;
        }

        if (end == start) {
            continue;
        }

        /* Check for "." and ".." before stripping? Wait, stripping already done. */
        /* Actually, "." stripped to empty, ".." stripped to empty. So we need to detect before stripping. */
        /* Let me redo: detect raw component. */
    }
```

Hmm, I need to detect "." and ".." before stripping trailing dots. Because ".." stripped would become empty. And "." stripped becomes empty.

Let me restructure: after finding start..end (raw component, before stripping), check if it's exactly "." or "..".

For ".": if it's the only component and no backup semantics, return ERROR_ACCESS_DENIED. Otherwise skip.
For "..": pop or unimplement.

Then strip trailing dots/spaces and copy.

Wait, but ".." has trailing dots? No, ".." is two dots, not trailing dots to strip. Actually, my stripping would strip the second dot, making ".". So I need to detect ".." before stripping.

For ".", my stripping would make it empty. So detect before stripping.

Let me restructure:

```c
    while (pos < len) {
        while (pos < len && is_sep(buf[pos])) pos++;
        if (pos >= len) break;
        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos])) pos++;
        unsigned end = pos; /* raw end, not stripped */

        /* Detect "." and ".." before stripping. */
        int is_dot = 1, is_dotdot = 1;
        for (i = start; i < end; i++) {
            if (buf[i] != '.') { is_dot = 0; is_dotdot = 0; break; }
        }
        if (is_dot && end - start != 1) is_dot = 0;
        if (is_dotdot && end - start != 2) is_dotdot = 0;
        if (is_dot) {
            if (out_pos == 0 && pos >= len) {
                /* entire path is "." */
                saw_dot_only = 1;
            }
            continue; /* skip "." */
        }
        if (is_dotdot) {
            if (out_pos == 0) {
                w2m_unimplemented(W2M_API_CreateFileW, 5);
                return 0;
            }
            /* pop last component */
            while (out_pos > 0 && out[out_pos - 1] != '/')
                out_pos--;
            if (out_pos > 0) out_pos--; /* remove '/' */
            continue;
        }

        /* Strip trailing spaces and dots. */
        unsigned stripped_end = end;
        while (stripped_end > start) {
            u16 c = buf[stripped_end - 1];
            if (c == ' ' || c == '.')
                stripped_end--;
            else
                break;
        }
        if (stripped_end == start) continue;

        /* Copy component, case-fold, check ASCII. */
        if (out_pos > 0) out[out_pos++] = '/';
        for (i = start; i < stripped_end; i++) {
            u16 c = buf[i];
            if (c > 0x7f) {
                w2m_unimplemented(W2M_API_CreateFileW, 4);
                return 0;
            }
            out[out_pos++] = to_upper_ascii(c);
        }
    }

    if (out_pos == 0) {
        if (saw_dot_only) {
            /* Path is "." — handled by caller based on backup semantics. */
            out[0] = '.';
            out[1] = 0;
            *out_len = 1;
            return 0;
        }
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    *out_len = out_pos;
    return 0;
}
```

Wait, but for ".", I want to return a special indicator. Maybe have the caller handle it. Let me have normalize_path return a special code or set a flag.

Actually, for the caller (CreateFileW), if the normalized path is ".", it checks backup semantics and returns ERROR_ACCESS_DENIED or unimplements.

For DeleteFileW, if path is ".", what should happen? The probe doesn't test. I'll return ERROR_ACCESS_DENIED.

Let me add a parameter `is_dot` or have normalize_path return "." as the path and the caller checks.

Actually, I can have normalize_path return 0 with out_len=1 and out="." when the path is ".". Then the caller checks if path is ".".

But for empty path (e.g., "././"), out_pos=0 and saw_dot_only might be true if all components are ".". Hmm. If path is "././", it normalizes to ".", which is a directory. Windows would treat it as current directory. But the probe only tests ".".

For simplicity, if all components are ".", I'll return path=".".

If path is empty after skipping "." and ".." and stripping, but not all ".", return ERROR_PATH_NOT_FOUND.

Hmm, tracking saw_dot_only is tricky. Let me just track: if the only non-separator, non-".." content was "." components, then path is ".".

Actually, simpler: after normalization, if out_pos==0, check if the original path (after stripping separators) is exactly "." or consists only of "." components. If yes, return ".". Otherwise ERROR_PATH_NOT_FOUND.

But for the probe, only "." is tested. I'll handle "." specifically.

Let me track `only_dot` flag: set to 1 initially, clear if we see any non-"." component or "..". If after loop out_pos==0 and only_dot, return ".".

Wait, ".." would pop and might leave out_pos=0. If path is "foo/..", out_pos=0 but only_dot should be false (because "foo" was a real component). So only_dot should be cleared when we add a real component, and set when we see ".". For "..", we pop; if it pops to empty and only_dot is still true? Hmm.

For the probe, no ".." is used. I'll simplify: if the normalized path is empty and the original path (after removing separators) is exactly ".", return ".". Otherwise ERROR_PATH_NOT_FOUND.

Actually, let me just check: after normalization, if out_pos==0, then if the input was "." (possibly with separators like "./"), return ".", else ERROR_PATH_NOT_FOUND.

To detect input was ".": check if len > 0 and the first raw component (after separators) is "." and there's nothing else.

Let me implement:

```c
    /* Check if the entire path is "." (possibly with separators). */
    int is_dot_path = 0;
    unsigned tmp = 0;
    while (tmp < len && is_sep(buf[tmp])) tmp++;
    if (tmp < len && buf[tmp] == '.') {
        unsigned dot_end = tmp + 1;
        while (dot_end < len && buf[dot_end] == '.') dot_end++;
        /* If it's exactly one dot followed by separators/end, it's "." path. */
        if (dot_end == tmp + 1) {
            unsigned rest = dot_end;
            while (rest < len && is_sep(buf[rest])) rest++;
            if (rest == len) is_dot_path = 1;
        }
    }
```

Hmm, this is getting complicated. For the probe, only exact "." is tested. I'll just check if the input string is "." (after removing leading/trailing separators).

Actually, let me handle it in the main loop with a flag. I'll track `has_real_component` (non-dot, non-dotdot). And `has_dot` (any dot component). After loop:
- if out_pos > 0: normal path.
- else if has_dot and !has_real_component: path is "."
- else: ERROR_PATH_NOT_FOUND.

But "foo/.." has has_real_component=true, out_pos=0. Should return ERROR_PATH_NOT_FOUND or unimplement? The probe doesn't test. I'll return ERROR_PATH_NOT_FOUND.

For ".", has_dot=true, has_real_component=false, out_pos=0. Return ".".

For "./.", same.

For "..", has_real_component=false (dotdot not counted), out_pos=0. We unimplemented earlier if it leaves root. If it doesn't leave root (e.g., "foo/.."), has_real_component=true. Good.

OK, let me implement with flags.

Actually, I realize I can simplify by detecting "." path before the loop. If after skipping leading separators, the first component is "." and it's the only component, then it's a dot path.

Let me just do:

```c
    /* Detect path that is only "." (with optional separators). */
    unsigned tmp = 0;
    while (tmp < len && is_sep(buf[tmp])) tmp++;
    int is_dot_path = 0;
    if (tmp < len && buf[tmp] == '.') {
        unsigned k = tmp + 1;
        while (k < len && is_sep(buf[k])) k++;
        if (k == len) {
            /* It's just "." (possibly with separators). */
            is_dot_path = 1;
        } else if (k < len && buf[k] != '.') {
            /* It's ".\something" or similar, not dot-only. */
            is_dot_path = 0;
        } else {
            /* Could be ".." or ".\." etc. Let the main loop handle. */
            is_dot_path = 0;
        }
    }
```

Hmm, this doesn't handle "./." correctly. But the probe doesn't test it.

For the probe, only exact "." is tested. So I can just check if the trimmed path is ".".

Let me define trimmed: skip leading and trailing separators. If trimmed length is 1 and the char is '.', it's dot path.

```c
    unsigned trim_start = 0;
    while (trim_start < len && is_sep(buf[trim_start])) trim_start++;
    unsigned trim_end = len;
    while (trim_end > trim_start && is_sep(buf[trim_end - 1])) trim_end--;
    int is_dot_path = (trim_end - trim_start == 1 && buf[trim_start] == '.');
```

But this would also match "." in "a/."? No, trim_start would be at 'a'. So only exact ".".

Wait, for "./.", trim_start=1 (after '/'), trim_end=3 (before trailing '/'), length=2, buf[1]='.', buf[2]='.'. Not dot path. Good.

For ".", trim_start=0, trim_end=1, length=1, buf[0]='.'. Dot path. Good.

OK, I'll use this.

Then in the main loop, I skip "." components. If the result is empty and is_dot_path, return ".".

For "..", I unimplement if it leaves root.

Now, let me also handle the case where a component becomes empty after stripping (e.g., "a." becomes "a", "a.." becomes "a", "a " becomes "a"). Good.

For "a.b.", stripping trailing dots gives "a.b". Good.

OK, I think I have a plan. Let me write the full code.

Actually, one more thing: for DeleteFileW, the API identifier is W2M_API_DeleteFileW. I need to use that in normalize_path? But normalize_path is shared. If DeleteFileW calls normalize_path and it unimplements with W2M_API_CreateFileW, that's wrong. Hmm.

I should pass the API identifier to normalize_path, or have separate unimplement calls. For simplicity, I'll have normalize_path not call w2m_unimplemented for path-related issues; instead return a special error and let the caller unimplement. But the caller knows which API.

Actually, for path too long, non-ASCII, absolute path, drive letter, ".." leaving root — these are unimplemented for both CreateFileW and DeleteFileW. The API identifier differs. I'll pass `api` to normalize_path.

Or, I can have normalize_path return an error and the caller decides to unimplement. But some cases are genuinely unimplemented (absolute paths), while others are normal errors (empty path). Let me have normalize_path return 0 for success, 1 for normal error (caller sets last error), 2 for unimplemented (caller calls w2m_unimplemented with its API).

Hmm, complicated. Simpler: pass `u64 api` to normalize_path and call w2m_unimplemented(api, detail).

For empty path, set_error and return ERROR_PATH_NOT_FOUND directly.

For path too long, non-ASCII, absolute, drive, ".." leaving root: w2m_unimplemented(api, detail).

OK, I'll add `u64 api` parameter.

Now, for CreateFileW, api=W2M_API_CreateFileW. For DeleteFileW, api=W2M_API_DeleteFileW.

For the "." path, normalize_path returns success with path=".". The caller handles.

OK, writing code.

One more thing: the `w2m_host` call for FILE_LOOKUP with UTF-16LE path. I pass `(u64)out, out_len * 2, 0, 0, &second`. Wait, signature is `w2m_host(op, a0, a1, a2, a3, &second)`. For FILE_LOOKUP, a0=path, a1=chars/bytes. So `w2m_host(W2M_FILE_LOOKUP, (u64)out, out_len * 2, 0, 0, &second)`.

If the host expects code units, I should pass `out_len`. I'll pass `out_len * 2` (bytes) and hope.

Hmm, actually, let me reconsider. The previous code passed 0 and got error 5. If I pass `out_len * 2` and the host expects code units, it reads 2*out_len*2 = 4*out_len bytes. For a path like "w2m-cf-a.txt" (12 chars), it reads 48 bytes. The buffer has 512 u16 = 1024 bytes, so it reads 48 bytes, which includes the path and some zeros. Decoding UTF-16LE with trailing zeros: UTF-16LE doesn't treat zero as terminator in decode; it would include the zeros as U+0000 characters. That might cause the path to have null characters, which could be interpreted as part of the filename or cause issues. So passing wrong length could cause problems.

If I pass `out_len` and host expects bytes, it reads 12 bytes = 6 UTF-16 code units = "w2m-cf" (truncated). Also wrong.

So I really need to know. Let me think about the host implementation again.

The host file_bridge.py likely has a class with methods like:
```python
def file_lookup(self, path_ptr, chars):
    path_bytes = self.read_mem(path_ptr, chars * 2)
    path = path_bytes.decode('utf-16le')
    ...
```

Because it says paths arrive as UTF-16LE, `chars` is likely code units, and it reads 2*chars bytes.

Or:
```python
def file_lookup(self, path_ptr, chars):
    path_bytes = self.read_mem(path_ptr, chars)
    path = path_bytes.decode('utf-16le')
```

Here `chars` is bytes.

Which is more likely? The parameter name `chars` suggests characters/code units. And since paths are UTF-16LE, reading 2*chars bytes is natural. So I'll pass code units (out_len).

But then if host expects bytes, it reads too little. Hmm.

Actually, maybe the host doesn't decode; it just uses the bytes as a path string. In that case, I should pass ASCII bytes. But the description says UTF-16LE.

OK, I'll go with code units. Because `chars` strongly suggests character count.

Wait, but if I pass code units and the host reads 2*chars bytes, then for my UTF-16LE buffer, it reads the correct byte length. Good.

If I pass bytes and host reads bytes, it reads the correct byte length. Also good.

The question is: does host read `chars` bytes or `2*chars` bytes?

Given `chars` is the parameter name and paths are UTF-16LE, I think host reads `2*chars` bytes. So I pass code units.

OK, final decision: pass `out_len` as `chars`.

But wait, the previous code passed 0. If I pass out_len=12, host reads 24 bytes. Good for UTF-16LE.

OK, I'll pass code units.

Hmm, but then if the host actually reads `chars` bytes, it reads 12 bytes of a 24-byte UTF-16LE string, getting garbage. 

I think code units is more likely. Let me go with that.

Actually, you know what, I can hedge by passing the byte length but ensuring the buffer has a null terminator at the correct position, and hope the host stops at null. But no, the host uses length.

OK, I'll pass code units. If wrong, the divergence would be widespread.

Wait, let me reconsider the previous failure. The previous code passed ASCII bytes with chars=0. If the host reads `chars` bytes and decodes as UTF-16LE, with chars=0 it gets empty. If the host reads `2*chars` bytes and decodes as UTF-16LE, with chars=0 it gets empty. Both give empty. So the failure doesn't distinguish.

But the previous code passed ASCII bytes, not UTF-16LE. If the host reads bytes and decodes as UTF-16LE, it would decode ASCII bytes incorrectly. But with chars=0, it reads nothing. So again, no distinction.

So I need to choose. I'll choose code units because `chars` suggests characters.

Actually, let me look at the primitive names and descriptions once more:
"FILE_LOOKUP  (path, chars)          -> kind: 0 absent, 1 file, 2 directory, 3 a directory on the way is missing; arg[5] the file id (inode) for 1 and 2"

It says `chars`, not `bytes`. In Windows API, string lengths for W functions are usually in characters. So `chars` = code units. Host reads 2*chars bytes.

OK, code units it is.

Now, let me write the code with this decision.

Actually, I realize I should also consider: the runtime might need to pass the path as a null-terminated UTF-16LE string and the host uses the null terminator. But the interface has `chars`, so length is explicit.

OK, code units.

Now, let me also think about the separator. I'll use '/' in the output. The host probably handles both.

Wait, if the host does case-folding and compares components, it needs to split on separators. It probably splits on both '/' and '\'. I'll use '/'.

OK, writing code now.

One more thing: for `k32_CreateFileW`, the `security_attributes` parameter. If non-NULL, unimplement. The probe passes NULL.

For `flags_and_attributes`, allow FILE_ATTRIBUTE_NORMAL and FILE_FLAG_BACKUP_SEMANTICS. If other bits, unimplement.

For `template`, if non-NULL, unimplement.

For `creation_disposition`, validate 1-5.

For `desired_access`, allow only GENERIC_READ and/or GENERIC_WRITE. The probe uses these. If other bits, unimplement.

OK.

Now, the special sharing case (r,w,w,r). I'll add:
```c
/* Windows observed quirk: first=(r, share write), second=(w, share read) conflicts. */
if (a1 == GENERIC_READ && s1 == FILE_SHARE_WRITE &&
    a2 == GENERIC_WRITE && s2 == FILE_SHARE_READ)
    return 1;
```

Wait, the function returns 1 if conflicts. So:
```c
if ((acc1 == GENERIC_READ) && (share1 == FILE_SHARE_WRITE) &&
    (acc2 == GENERIC_WRITE) && (share2 == FILE_SHARE_READ))
    return 1;
```

But this is very specific. Is there a more general rule? Let me think...

Actually, maybe the rule is: if the first open's share mode is exactly FILE_SHARE_WRITE (only write, no read) and the first access is read, then any second open with write access and share read (but not write) conflicts. Hmm.

Or maybe: if first access is read and first share is write, then second access write requires second share to include write. So second=(w,r) fails because share doesn't include write, but second=(w,rw) succeeds.

Wait, that's a clean rule! For first=(r,w): second access write requires second share include write. So:
- second=(w,r): share=r, lacks write. Fail.
- second=(w,w): share=w, has write. But standard rule also requires share2 allow a1=read, and w doesn't allow read. So fail anyway.
- second=(w,rw): share=rw, has write. Standard rule also OK. Success.

So the extra condition is: if first access is read and first share is write, then second write access requires second share write.

But does this apply symmetrically? For first=(w,r), second access read requires second share read? Let's check:
- second=(r,w): share=w, lacks read. According to rule, fail. But contract says success!

So not symmetric.

Hmm. What if the rule is: if first access is read and first share is write, then second access write requires second share write OR second access read requires second share read? No.

Actually, the asymmetry might be because write access is "stronger" than read access. When first opens for read and shares write, it's a weak share. A second open for write with share read is rejected because... I don't know.

OK, I'll just special-case (r,w,w,r). It's one row.

Wait, but maybe there are other rows where standard rule disagrees that I haven't checked. Let me do a more systematic check.

Actually, I already checked all first=(r,w) and found only (w,r) disagrees. I checked first=(w,rw) and found no disagreements. I checked first=(rw,r), (rw,w), (rw,rw). I checked first=(r,r), (w,w), (w,r), (r,rw). I believe only one disagreement.

But let me also check first=(w,r) more carefully. I said standard rule matches. Let me verify second=(r,r): standard false, contract false. OK. second=(r,w): standard true, contract true. OK. second=(r,rw): standard true, contract true. OK.

first=(r,r): second=(r,w): standard false, contract false. OK.

So only (r,w,w,r).

OK, special case.

Now, let me also think about whether the sharing check should consider delete sharing. The probe doesn't test delete sharing conflicts. I'll ignore FILE_SHARE_DELETE for sharing conflicts (only use it for delete blocking).

Actually, for sharing conflicts, FILE_SHARE_DELETE doesn't affect read/write sharing. So my function ignores it. Good.

Now, let me write the final code.

One more thing: the `w2m_host` call for FILE_SIZE returns size. I use it. Good.

For FILE_TRUNCATE, I call with fd and size 0. Good.

For FILE_READ/WRITE, I pass offset = f->position. Good.

Now, the code:

```c
/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ        0x80000000u
#define GENERIC_WRITE       0x40000000u

#define FILE_SHARE_READ     0x00000001u
#define FILE_SHARE_WRITE    0x00000002u
#define FILE_SHARE_DELETE   0x00000004u

#define CREATE_NEW          1u
#define CREATE_ALWAYS       2u
#define OPEN_EXISTING       3u
#define OPEN_ALWAYS         4u
#define TRUNCATE_EXISTING   5u

#define FILE_ATTRIBUTE_NORMAL      0x00000080u
#define FILE_FLAG_BACKUP_SEMANTICS 0x02000000u

#define FILE_BEGIN          0u
#define FILE_CURRENT        1u
#define FILE_END            2u

#define INVALID_HANDLE_VALUE ((u64)(-1))

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND        2u
#define ERROR_PATH_NOT_FOUND        3u
#define ERROR_ACCESS_DENIED         5u
#define ERROR_INVALID_HANDLE        6u
#define ERROR_NEGATIVE_SEEK         131u
#define ERROR_ALREADY_EXISTS        183u
#define ERROR_FILE_EXISTS           80u
#define ERROR_SHARING_VIOLATION     32u
#define ERROR_INVALID_PARAMETER     87u

#define MAX_FILES 64

typedef long long s64;
typedef unsigned short u16;

struct file_entry {
    int used;
    u64 fd;
    u64 id;
    u32 access;
    u32 share;
    u64 position;
};

static struct file_entry files[MAX_FILES];

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static struct file_entry *alloc_entry(void)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (!files[i].used) {
            files[i].used = 1;
            files[i].fd = 0;
            files[i].id = 0;
            files[i].access = 0;
            files[i].share = 0;
            files[i].position = 0;
            return &files[i];
        }
    }
    return 0;
}

static void free_entry(struct file_entry *f)
{
    f->used = 0;
    f->fd = 0;
    f->id = 0;
    f->access = 0;
    f->share = 0;
    f->position = 0;
}

static unsigned handle_index(u64 h)
{
    if (h == 0 || h > MAX_FILES)
        return (unsigned)-1;
    return (unsigned)(h - 1);
}

static struct file_entry *lookup(u64 h)
{
    unsigned idx = handle_index(h);
    if (idx >= MAX_FILES)
        return 0;
    if (!files[idx].used)
        return 0;
    return &files[idx];
}

static int is_sep(u16 c)
{
    return c == '\\' || c == '/';
}

static u16 to_upper_ascii(u16 c)
{
    if (c >= 'a' && c <= 'z')
        return (u16)(c - 'a' + 'A');
    return c;
}

/* Validate and normalize a guest UTF-16LE path into a host UTF-16LE path.
 * Returns 0 on success, fills `out` with UTF-16LE code units and `out_len`
 * with the number of code units. `out` must have room for at least 512 u16.
 * Returns a Windows error code on failure (and sets last error).
 * Calls w2m_unimplemented with `api` for unsupported path forms.
 */
static u32 normalize_path(u64 name, u16 *out, unsigned *out_len, u64 api)
{
    u16 buf[512];
    unsigned len = 0;
    unsigned i;

    if (name == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    for (i = 0; i < 511; i++) {
        u16 c = ((u16 *)name)[i];
        if (c == 0)
            break;
        buf[i] = c;
    }
    if (i == 511) {
        w2m_unimplemented(api, 1);
        return 0;
    }
    len = i;
    if (len == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    if (is_sep(buf[0])) {
        w2m_unimplemented(api, 2);
        return 0;
    }
    if (len >= 2 && buf[1] == ':') {
        w2m_unimplemented(api, 3);
        return 0;
    }

    /* Detect path that is only "." (with optional separators). */
    unsigned trim_start = 0;
    while (trim_start < len && is_sep(buf[trim_start])) trim_start++;
    unsigned trim_end = len;
    while (trim_end > trim_start && is_sep(buf[trim_end - 1])) trim_end--;
    int is_dot_path = (trim_end - trim_start == 1 && buf[trim_start] == '.');

    unsigned out_pos = 0;
    unsigned pos = 0;

    while (pos < len) {
        while (pos < len && is_sep(buf[pos]))
            pos++;
        if (pos >= len)
            break;

        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos]))
            pos++;
        unsigned end = pos;

        /* Check for "." and ".." before stripping trailing dots/spaces. */
        int is_dot = 0, is_dotdot = 0;
        if (end - start == 1 && buf[start] == '.') is_dot = 1;
        else if (end - start == 2 && buf[start] == '.' && buf[start + 1] == '.') is_dotdot = 1;

        if (is_dot) {
            continue; /* skip "." */
        }
        if (is_dotdot) {
            if (out_pos == 0) {
                w2m_unimplemented(api, 5);
                return 0;
            }
            while (out_pos > 0 && out[out_pos - 1] != '/')
                out_pos--;
            if (out_pos > 0)
                out_pos--;
            continue;
        }

        /* Strip trailing spaces and dots. */
        unsigned stripped_end = end;
        while (stripped_end > start) {
            u16 c = buf[stripped_end - 1];
            if (c == ' ' || c == '.')
                stripped_end--;
            else
                break;
        }
        if (stripped_end == start)
            continue;

        if (out_pos > 0)
            out[out_pos++] = '/';
        for (i = start; i < stripped_end; i++) {
            u16 c = buf[i];
            if (c > 0x7f) {
                w2m_unimplemented(api, 4);
                return 0;
            }
            out[out_pos++] = to_upper_ascii(c);
        }
    }

    if (out_pos == 0) {
        if (is_dot_path) {
            out[0] = '.';
            out_pos = 1;
        } else {
            set_error(ERROR_PATH_NOT_FOUND);
            return ERROR_PATH_NOT_FOUND;
        }
    }

    *out_len = out_pos;
    return 0;
}

static u32 errno_to_win(u64 e)
{
    switch ((int)e) {
    case -2:    return ERROR_FILE_NOT_FOUND;
    case -13:   return ERROR_ACCESS_DENIED;
    case -17:   return ERROR_FILE_EXISTS;
    case -20:   return ERROR_PATH_NOT_FOUND;
    case -21:   return ERROR_PATH_NOT_FOUND;
    case -36:   return ERROR_FILE_EXISTS;
    case -1:
    default:    return ERROR_ACCESS_DENIED;
    }
}

static int sharing_conflicts(u32 acc1, u32 share1, u32 acc2, u32 share2)
{
    /* Windows observed quirk: first=(read, share-write), second=(write, share-read) conflicts. */
    if (acc1 == GENERIC_READ && share1 == FILE_SHARE_WRITE &&
        acc2 == GENERIC_WRITE && share2 == FILE_SHARE_READ)
        return 1;

    if ((acc2 & GENERIC_READ) && !(share1 & FILE_SHARE_READ))
        return 1;
    if ((acc2 & GENERIC_WRITE) && !(share1 & FILE_SHARE_WRITE))
        return 1;
    if ((acc1 & GENERIC_READ) && !(share2 & FILE_SHARE_READ))
        return 1;
    if ((acc1 & GENERIC_WRITE) && !(share2 & FILE_SHARE_WRITE))
        return 1;

    return 0;
}

static int delete_blocked(u64 id, struct file_entry *exclude)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (&files[i] == exclude)
            continue;
        if (files[i].used && files[i].id == id) {
            if (!(files[i].share & FILE_SHARE_DELETE))
                return 1;
        }
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp)
{
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 6);
        return 0;
    }
    if (security_attributes != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 7);
        return 0;
    }
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if ((desired_access & ~(GENERIC_READ | GENERIC_WRITE)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 8);
        return 0;
    }
    if ((flags_and_attributes & ~(FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 9);
        return 0;
    }
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_CreateFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (path_len == 1 && path[0] == '.') {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 10);
        return 0;
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 3) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if (kind == 2) {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 11);
        return 0;
    }

    int exists = (kind == 1);

    int must_create = 0, must_not_exist = 0, truncate = 0;
    switch (creation_disposition) {
    case CREATE_NEW:
        must_create = 1;
        must_not_exist = 1;
        break;
    case CREATE_ALWAYS:
        must_create = 1;
        truncate = 1;
        break;
    case OPEN_EXISTING:
        break;
    case OPEN_ALWAYS:
        must_create = 1;
        break;
    case TRUNCATE_EXISTING:
        truncate = 1;
        break;
    }

    if (!exists) {
        if (!must_create) {
            set_error(ERROR_FILE_NOT_FOUND);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    } else {
        if (must_not_exist) {
            set_error(ERROR_FILE_EXISTS);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (files[i].used && files[i].id == file_id) {
            if (sharing_conflicts(files[i].access, files[i].share,
                                  (u32)desired_access, (u32)share_mode)) {
                set_error(ERROR_SHARING_VIOLATION);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
        }
    }

    u64 create_arg = (!exists && must_create) ? 1 : 0;
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, path_len, create_arg, 0, &second);
    file_id = second;

    if ((s64)fd < 0) {
        set_error(errno_to_win(fd));
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (truncate && exists) {
        w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
    }

    struct file_entry *f = alloc_entry();
    if (!f) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    f->fd = fd;
    f->id = file_id;
    f->access = (u32)desired_access;
    f->share = (u32)share_mode;
    f->position = 0;

    u64 h = (unsigned)(f - files) + 1;

    if (exists && (creation_disposition == OPEN_ALWAYS || creation_disposition == CREATE_ALWAYS)) {
        set_error(ERROR_ALREADY_EXISTS);
    } else {
        set_error(0);
    }

    return w2m_logged(W2M_API_CreateFileW, name, h);
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }

    u64 second = 0;
    w2m_host(W2M_FILE_CLOSE, f->fd, 0, 0, 0, &second);
    free_entry(f);

    set_error(0);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_DeleteFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (path_len == 1 && path[0] == '.') {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind != 1) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (delete_blocked(file_id, 0)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 ret = w2m_host(W2M_FILE_UNLINK, (u64)path, path_len, 0, 0, &second);
    if ((s64)ret < 0) {
        set_error(errno_to_win(ret));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* DeleteFileW leaves last error untouched on success. */
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }

    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(f->access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_READ, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    f->position += n;
    if (bytes_read)
        *(u32 *)bytes_read = (u32)n;

    /* ReadFile leaves last error untouched on success. */
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    u64 second = 0;
    u64 size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, &second);
    u64 base;

    switch (move_method) {
    case FILE_BEGIN:
        base = 0;
        break;
    case FILE_CURRENT:
        base = f->position;
        break;
    case FILE_END:
        base = size;
        break;
    default:
        set_error(ERROR_INVALID_PARAMETER);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 d = (s64)distance;
    s64 target = (s64)base + d;

    if (target < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-7);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    f->position = (u64)target;
    if (new_pointer)
        *(u64 *)new_pointer = (u64)target;

    /* SetFilePointerEx leaves last error untouched on success. */
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    if (!(f->access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_WRITE, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    f->position += n;
    if (written)
        *(u32 *)written = (u32)n;

    /* WriteFile leaves last error untouched on success. */
    return 1;
}
```

Wait, I need to double-check the `w2m_file_write` return. The comment says "return 1 on success, 0 on failure". And it should call w2m_logged? The function signature is `u64 w2m_file_write(...)`. It's called by runtime.c, not directly by the guest. The problem says "Call w2m_logged(W2M_API_<name>, arg0, return_value) to return from each k32_ function". w2m_file_write is not a k32_ function, so maybe it shouldn't call w2m_logged. The previous code didn't. I'll keep it not calling w2m_logged.

But should it set last error on success? The comment says runtime.c already set *(u32 *)written = 0. It doesn't mention last error. Since WriteFile leaves last error untouched on success, w2m_file_write should not set it. Good.

Now, one issue: in `k32_CreateFileW`, I set last error to 0 or 183. But for `share.r.r.r.r`, the contract says err=0. My code: exists=true, creation_disposition=OPEN_EXISTING, so set_error(0). Good.

For `share.r.r.r.rw`, creation_disposition=OPEN_EXISTING, set_error(0). Contract says err=0. Good.

For `share.r.w.w.rw`, creation_disposition=OPEN_EXISTING, set_error(0). Contract says err=0. Good.

Wait, all sharing tests use OPEN_EXISTING. So err is 0 on success, 32 on failure. Good.

Now, for `del.closed`, DeleteFileW success, I don't set error. The probe's sentinel 57005 remains. Good.

For `del.open.share_delete`, DeleteFileW success, err=57005. Good.

For `del.open.no_share_delete`, DeleteFileW fails with ERROR_SHARING_VIOLATION (32). Good.

For `del.pending.open_existing`, after delete with share delete, the file is unlinked. CreateFileW with OPEN_EXISTING: LOOKUP kind=0, must_create=0, set_error(ERROR_FILE_NOT_FOUND), return INVALID_HANDLE_VALUE. Contract says valid=false, err=2. Good.

For `del.pending.create_new`, CreateFileW with CREATE_NEW: LOOKUP kind=0, must_create=1, create_arg=1, OPEN creates new file. set_error(0). Contract says valid=true, err=0. Good.

For `del.pending.after_close`, after closing the original handle, the new file exists with size 0. The probe measures and gets exists=true, size=0, head="". Good.

Now, one issue with delete: after DeleteFileW succeeds with share delete, the file is unlinked but open handles remain. My file table still has the old id. If a new CreateFileW creates a new file with the same path, it gets a new file_id. The old handles have the old file_id. That's fine.

But what if the old handle is used for I/O after the file is unlinked? The host fd is still valid (POSIX delete). The probe doesn't test this. Fine.

Now, the `name.trailing_dot` and `name.trailing_space`: input "w2m-cf-a.txt." and "w2m-cf-a.txt ". My stripping removes trailing dot/space, giving "w2m-cf-a.txt". LOOKUP finds it. Good.

But wait, for "w2m-cf-a.txt.", the raw component is "w2m-cf-a.txt." (not "." or ".."). After stripping trailing dots, it becomes "w2m-cf-a.txt". Good.

For "w2m-cf-a.txt ", stripping trailing spaces gives "w2m-cf-a.txt". Good.

Now, the `name.case.open_existing`: input "W2M-CF-A.TXT". My to_upper_ascii leaves it as is. LOOKUP finds the file (host case-folds). Good.

`name.case.create_new`: input "W2M-CF-A.TXT". LOOKUP finds existing file. CREATE_NEW must_not_exist=1, returns ERROR_FILE_EXISTS (80). Good.

Now, the `io` scenarios. Let me trace:

`io.write`: handle opened with rw, position=0. Write "hello world" (11 bytes). FILE_WRITE at offset 0 writes 11. position=11. done=11. err untouched (57005). Good.

`io.seek.current_0`: distance=0, FILE_CURRENT, base=11, target=11. new_pointer=11. err untouched. Good.

`io.seek.end_minus_5`: base=size=11, d=-5, target=6. position=6. Good.

`io.read_8`: read 8 bytes at offset 6. File size 11, so 5 bytes available. Host returns 5 (or zero-fills to 8?). Contract says done=5, data="776f726c64" ("world"). So host returns 5. My code: n=5, position=11. Good.

`io.read_at_eof`: position=11, read 8. Host returns 0. done=0. Good.

`io.seek.begin_minus_1`: base=0, d=-1, target=-1. Error. set_error(131). new_pointer=-7 (sentinel). position unchanged (11). Good.

`io.seek.current_after_failure`: base=11, target=11. new_pointer=11. err untouched (57005). Good.

`io.seek.begin_100`: target=100. position=100. Good.

`io.write_at_100`: write "!" at offset 100. Host extends file to 101, zero-fills gap. position=101. done=1. Good.

`io.seek.begin_9`: position=9. Good.

`io.read_gap`: read 4 at offset 9. File size 101. Bytes at 9-12: original file "hello world" has 'l' at 9, 'd' at 10, then zeros at 11-12? Wait, original "hello world" is 11 bytes (0-10). At offset 9: 'l' (9), 'd' (10), then gap zeros at 11, 12. So data = 'l','d',0,0 = 0x6c640000. Host returns 4 with zero-fill. Good.

`io.after_close`: size=101, head first 8 bytes: "hello wo" = 0x68656c6c6f20776f. Good.

`io.write_on_read_handle`: handle opened with GENERIC_READ. w2m_file_write checks access & GENERIC_WRITE, fails with ERROR_ACCESS_DENIED (5), done=0. Good.

`io.read_on_write_handle`: handle opened with GENERIC_WRITE. k32_ReadFile checks access & GENERIC_READ, fails with ERROR_ACCESS_DENIED (5), done=0. Good.

`close.twice`: close the already-closed handle. lookup fails, set_error(ERROR_INVALID_HANDLE), return 0. Good.

Now, one potential issue: for `k32_ReadFile` and `w2m_file_write`, the `buffer` pointer is a guest pointer. The host reads/writes guest memory. The runtime doesn't need to do anything special. Good.

Another issue: for `k32_SetFilePointerEx`, `distance` is a signed 64-bit value in `u64` parameter. I cast to s64. Good.

Now, the `w2m_host` call for FILE_SIZE: I pass fd and read second? No, FILE_SIZE returns size in result. I use the return value. Good.

Wait, the description says "FILE_SIZE (fd) -> size". So result is size. I do `u64 size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, &second);`. Good.

For FILE_TRUNCATE, "FILE_TRUNCATE(fd, size) -> 0". I pass fd and 0. Good.

Now, let me also think about the `w2m_host` call for FILE_CLOSE. It returns 0. I ignore. Good.

Now, one more issue: the `normalize_path` function for DeleteFileW uses W2M_API_DeleteFileW for unimplemented. Good.

But wait, for CreateFileW, if normalize_path returns ERROR_PATH_NOT_FOUND (empty path), I return INVALID_HANDLE_VALUE with err set. Good.

For DeleteFileW, if normalize_path returns ERROR_PATH_NOT_FOUND, I return 0 with err set. Good.

Now, the `is_dot_path` detection. For input ".", trim_start=0, trim_end=1, is_dot_path=true. Main loop: component "." is skipped. out_pos=0. After loop, is_dot_path true, so path=".". Good.

For input "./", trim_start=1? Wait, let me trace: len=2, buf=['.','/']. trim_start: skip separators at start, none, so trim_start=0. trim_end: skip trailing separators, trim_end=1 (pointing to '.'). trim_end - trim_start = 1, buf[0]='.'. is_dot_path=true. Main loop: skip separators (pos=1, at end). out_pos=0. After loop, path=".". Good.

For input "/.", trim_start=1, trim_end=2, length=1, buf[1]='.'. is_dot_path=true. Good.

For input "./.", len=3, buf=['.','/','.']. trim_start=0, trim_end=3, length=3, not dot path. Main loop: first component ".", skip. Then separator. Then second component ".", skip. out_pos=0. is_dot_path=false. Returns ERROR_PATH_NOT_FOUND. Hmm, but "./." is current directory. The probe doesn't test. OK.

For input "..", is_dot_path=false (trim_end-trim_start=2). Main loop: component "..", out_pos=0, unimplement. Good.

OK.

Now, let me also think about whether I need to handle the BOM or other UTF-16 issues. The probe uses simple ASCII. No need.

Now, the special sharing case. I added it. But is it really needed? Let me double-check the contract one more time for `share.r.w.w.r`. Yes, fail.

But wait, I want to make sure my standard rule doesn't already handle it due to some other effect. Let me recompute with my code:

first=(r,w): acc1=GENERIC_READ, share1=FILE_SHARE_WRITE.
second=(w,r): acc2=GENERIC_WRITE, share2=FILE_SHARE_READ.

My special case: acc1==GENERIC_READ && share1==FILE_SHARE_WRITE && acc2==GENERIC_WRITE && share2==FILE_SHARE_READ -> return 1 (conflict). Good.

Then the standard checks follow, but special case returns first.

OK.

Now, one more subtlety: the sharing check uses `desired_access` and `share_mode` as u32. The parameters are u64. I cast to u32. Good.

But what about FILE_SHARE_DELETE? I don't include it in sharing conflicts. Good.

Now, let me also think about the `delete_blocked` function. It checks if any open handle to the same id doesn't share delete. But what about handles opened after the delete? The delete only happens if no blocking handles exist at the time of delete. After delete, new opens to the same path see kind=0 (file unlinked), so they fail with ERROR_FILE_NOT_FOUND. Good.

But wait, for `del.pending.open_existing`, after DeleteFileW with share delete, the file is unlinked. Then CreateFileW with OPEN_EXISTING calls LOOKUP, gets kind=0, returns ERROR_FILE_NOT_FOUND. Good.

For `del.pending.create_new`, CreateFileW with CREATE_NEW calls LOOKUP, gets kind=0, creates new file. Good.

Now, one issue: when DeleteFileW unlinks a file that has open handles with share delete, the host's FILE_UNLINK might return -EBUSY or something if the host doesn't support POSIX delete. But the contract says both observed builds did POSIX delete. The host file_bridge.py should support it. I'll assume FILE_UNLINK succeeds even with open handles (if they share delete). If the host returns error, my code maps it.

Actually, the host description says FILE_UNLINK returns 0 or -errno. It doesn't mention share delete. But the contract expects POSIX delete behavior. So the host must implement it.

OK.

Now, let me also think about whether I need to update file_id after FILE_OPEN. Yes, I read second for file_id. Good.

But what if FILE_OPEN returns a new fd for an existing file? The file_id should be the same as LOOKUP. Good.

For a new file, LOOKUP returned kind=0, file_id was 0. After OPEN, file_id is set to the new file's id. Good.

Now, for sharing check before OPEN, I use file_id from LOOKUP. For a new file, file_id=0, so no sharing conflicts (no existing handles with id=0). Good. After OPEN, I update file_id.

But wait, could there be a race? No, single-threaded.

OK.

Now, let me also think about the `w2m_host` call for FILE_OPEN when create_arg=1 and file exists. I don't call it because I check must_not_exist first. Good.

When create_arg=0 and file doesn't exist, I don't call it because I check must_create first. Good.

OK, I think the code is ready. Let me also add some comments and clean up.

One thing: I should make sure the `u16` type is defined. I added `typedef unsigned short u16;`. Good.

Another thing: the `w2m_host` function is declared in runtime.h. Good.

Now, let me also verify the `k32_CreateFileW` parameter order. The problem says:
```
u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp);
```
Yes.

`k32_ReadFile`: `u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp);` Yes.

`k32_SetFilePointerEx`: `u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method);` Yes.

`w2m_file_write`: `u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written);` Yes.

OK.

Now, one final check: the problem says "the fifth argument is at *(u64 *)(rsp + 0x28), the sixth at rsp + 0x30, the seventh at rsp + 0x38". For CreateFileW, the fifth argument is creation_disposition at rsp+0x28 (u32). Sixth is flags_and_attributes at rsp+0x30 (u32). Seventh is template at rsp+0x38 (u64). I read these correctly.

For ReadFile, fifth argument is overlapped at rsp+0x28 (u64). I read it. Good.

OK, I think the code is correct. Let me format it nicely and output.

Actually, one more thought: the `w2m_host` call for FILE_LOOKUP with `path_len` as code units. If the host expects bytes, this is wrong. But I think code units is right.

Wait, let me reconsider once more. The host description says "FILE_LOOKUP (path, chars)". In Python, if I were implementing this, I'd likely do:
```python
def file_lookup(path_ptr, chars):
    path = read_bytes(path_ptr, chars * 2).decode('utf-16le')
```
Because paths are UTF-16LE and `chars` is the character count. This reads 2*chars bytes.

So passing `path_len` (code units) is correct.

OK, final answer.

Hmm, but I want to hedge. What if the host reads `chars` bytes? Then my code fails. Is there any way to make it work for both? Not really, unless I can detect.

But given the description explicitly says UTF-16LE and uses "chars", code units is the right choice.

OK, outputting the code.

Wait, I should also double-check: for `FILE_OPEN`, I pass `path_len` as code units. Same logic.

For `FILE_UNLINK`, same.

OK.

One more thing: the `normalize_path` function writes UTF-16LE to `out`. The host reads it. But the runtime stack `out` buffer might not be aligned? u16 arrays are aligned to 2 bytes. The host reads via pointer. Should be fine.

Also, the runtime is compiled with `-mgeneral-regs-only`, so no floating point. Fine.

OK, final code.

Actually, I realize I should also handle the case where `w2m_host` returns a value that doesn't fit in s64. But fd is a host descriptor, probably small. Fine.

Also, for `FILE_READ`, the return value `n` is bytes read. It should be <= count. I add to position. Good.

For `FILE_WRITE`, return value is bytes written. Good.

OK, I'm done. Let me output the code.

Wait, one more thing: in `k32_CreateFileW`, after `normalize_path` returns 0 with path=".", I check backup semantics. But what if `path_len==1 && path[0]=='.'` and backup semantics is set? I unimplement. Good.

For DeleteFileW with path=".", I return ERROR_ACCESS_DENIED. The probe doesn't test. Reasonable.

OK, outputting.

Hmm, actually, I want to reconsider the `is_dot_path` detection once more. For input ".", trim_start=0, trim_end=1, is_dot_path=true. Good.

But what about input ".\."? len=3, buf=['.','\\','.']. trim_start=0, trim_end=3, length=3, not dot path. Main loop: first component ".", skip. separator. second component ".", skip. out_pos=0. is_dot_path=false. Returns ERROR_PATH_NOT_FOUND. But ".\." is current directory. Not tested. OK.

What about input ".\"? len=2, buf=['.','\\']. trim_start=0, trim_end=1, length=1, buf[0]='.'. is_dot_path=true. Main loop: skip separator at end. out_pos=0. After loop, path=".". Good.

OK.

Now, outputting final code.

Actually, I realize I should also add a check for `desired_access == 0`. The probe doesn't test it, but CreateFileW with 0 access might be valid in Windows (query attributes). The contract doesn't cover it. If desired_access==0, my code allows it (since 0 & ~mask == 0). Then sharing check: no read or write access, so no conflicts. The host OPEN with create=0 would open the file. But the probe doesn't test. I'll leave it.

OK, final code.

One more thing: the `w2m_unimplemented` detail codes. I used 1-11. They don't matter much, but should be distinct. Good.

OK, outputting.</think>```c
/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ        0x80000000u
#define GENERIC_WRITE       0x40000000u

#define FILE_SHARE_READ     0x00000001u
#define FILE_SHARE_WRITE    0x00000002u
#define FILE_SHARE_DELETE   0x00000004u

#define CREATE_NEW          1u
#define CREATE_ALWAYS       2u
#define OPEN_EXISTING       3u
#define OPEN_ALWAYS         4u
#define TRUNCATE_EXISTING   5u

#define FILE_ATTRIBUTE_NORMAL      0x00000080u
#define FILE_FLAG_BACKUP_SEMANTICS 0x02000000u

#define FILE_BEGIN          0u
#define FILE_CURRENT        1u
#define FILE_END            2u

#define INVALID_HANDLE_VALUE ((u64)(-1))

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND        2u
#define ERROR_PATH_NOT_FOUND        3u
#define ERROR_ACCESS_DENIED         5u
#define ERROR_INVALID_HANDLE        6u
#define ERROR_NEGATIVE_SEEK         131u
#define ERROR_ALREADY_EXISTS        183u
#define ERROR_FILE_EXISTS           80u
#define ERROR_SHARING_VIOLATION     32u
#define ERROR_INVALID_PARAMETER     87u

#define MAX_FILES 64

typedef long long s64;
typedef unsigned short u16;

struct file_entry {
    int used;
    u64 fd;
    u64 id;
    u32 access;
    u32 share;
    u64 position;
};

static struct file_entry files[MAX_FILES];

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static struct file_entry *alloc_entry(void)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (!files[i].used) {
            files[i].used = 1;
            files[i].fd = 0;
            files[i].id = 0;
            files[i].access = 0;
            files[i].share = 0;
            files[i].position = 0;
            return &files[i];
        }
    }
    return 0;
}

static void free_entry(struct file_entry *f)
{
    f->used = 0;
    f->fd = 0;
    f->id = 0;
    f->access = 0;
    f->share = 0;
    f->position = 0;
}

static unsigned handle_index(u64 h)
{
    if (h == 0 || h > MAX_FILES)
        return (unsigned)-1;
    return (unsigned)(h - 1);
}

static struct file_entry *lookup(u64 h)
{
    unsigned idx = handle_index(h);
    if (idx >= MAX_FILES)
        return 0;
    if (!files[idx].used)
        return 0;
    return &files[idx];
}

static int is_sep(u16 c)
{
    return c == '\\' || c == '/';
}

static u16 to_upper_ascii(u16 c)
{
    if (c >= 'a' && c <= 'z')
        return (u16)(c - 'a' + 'A');
    return c;
}

/* Validate and normalize a guest UTF-16LE path into a host UTF-16LE path.
 * Returns 0 on success, fills `out` with UTF-16LE code units and `out_len`
 * with the number of code units. `out` must have room for at least 512 u16.
 * Returns a Windows error code on failure (and sets last error).
 * Calls w2m_unimplemented with `api` for unsupported path forms.
 */
static u32 normalize_path(u64 name, u16 *out, unsigned *out_len, u64 api)
{
    u16 buf[512];
    unsigned len = 0;
    unsigned i;

    if (name == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    for (i = 0; i < 511; i++) {
        u16 c = ((u16 *)name)[i];
        if (c == 0)
            break;
        buf[i] = c;
    }
    if (i == 511) {
        w2m_unimplemented(api, 1);
        return 0;
    }
    len = i;
    if (len == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    if (is_sep(buf[0])) {
        w2m_unimplemented(api, 2);
        return 0;
    }
    if (len >= 2 && buf[1] == ':') {
        w2m_unimplemented(api, 3);
        return 0;
    }

    unsigned trim_start = 0;
    while (trim_start < len && is_sep(buf[trim_start]))
        trim_start++;
    unsigned trim_end = len;
    while (trim_end > trim_start && is_sep(buf[trim_end - 1]))
        trim_end--;
    int is_dot_path = (trim_end - trim_start == 1 && buf[trim_start] == '.');

    unsigned out_pos = 0;
    unsigned pos = 0;

    while (pos < len) {
        while (pos < len && is_sep(buf[pos]))
            pos++;
        if (pos >= len)
            break;

        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos]))
            pos++;
        unsigned end = pos;

        int is_dot = (end - start == 1 && buf[start] == '.');
        int is_dotdot = (end - start == 2 && buf[start] == '.' && buf[start + 1] == '.');

        if (is_dot)
            continue;
        if (is_dotdot) {
            if (out_pos == 0) {
                w2m_unimplemented(api, 5);
                return 0;
            }
            while (out_pos > 0 && out[out_pos - 1] != '/')
                out_pos--;
            if (out_pos > 0)
                out_pos--;
            continue;
        }

        unsigned stripped_end = end;
        while (stripped_end > start) {
            u16 c = buf[stripped_end - 1];
            if (c == ' ' || c == '.')
                stripped_end--;
            else
                break;
        }
        if (stripped_end == start)
            continue;

        if (out_pos > 0)
            out[out_pos++] = '/';
        for (i = start; i < stripped_end; i++) {
            u16 c = buf[i];
            if (c > 0x7f) {
                w2m_unimplemented(api, 4);
                return 0;
            }
            out[out_pos++] = to_upper_ascii(c);
        }
    }

    if (out_pos == 0) {
        if (is_dot_path) {
            out[0] = '.';
            out_pos = 1;
        } else {
            set_error(ERROR_PATH_NOT_FOUND);
            return ERROR_PATH_NOT_FOUND;
        }
    }

    *out_len = out_pos;
    return 0;
}

static u32 errno_to_win(u64 e)
{
    switch ((int)e) {
    case -2:    return ERROR_FILE_NOT_FOUND;
    case -13:   return ERROR_ACCESS_DENIED;
    case -17:   return ERROR_FILE_EXISTS;
    case -20:   return ERROR_PATH_NOT_FOUND;
    case -21:   return ERROR_PATH_NOT_FOUND;
    case -36:   return ERROR_FILE_EXISTS;
    case -1:
    default:    return ERROR_ACCESS_DENIED;
    }
}

static int sharing_conflicts(u32 acc1, u32 share1, u32 acc2, u32 share2)
{
    /* Windows observed quirk: first=(read, share-write), second=(write, share-read) conflicts. */
    if (acc1 == GENERIC_READ && share1 == FILE_SHARE_WRITE &&
        acc2 == GENERIC_WRITE && share2 == FILE_SHARE_READ)
        return 1;

    if ((acc2 & GENERIC_READ) && !(share1 & FILE_SHARE_READ))
        return 1;
    if ((acc2 & GENERIC_WRITE) && !(share1 & FILE_SHARE_WRITE))
        return 1;
    if ((acc1 & GENERIC_READ) && !(share2 & FILE_SHARE_READ))
        return 1;
    if ((acc1 & GENERIC_WRITE) && !(share2 & FILE_SHARE_WRITE))
        return 1;

    return 0;
}

static int delete_blocked(u64 id, struct file_entry *exclude)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (&files[i] == exclude)
            continue;
        if (files[i].used && files[i].id == id) {
            if (!(files[i].share & FILE_SHARE_DELETE))
                return 1;
        }
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp)
{
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 6);
        return 0;
    }
    if (security_attributes != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 7);
        return 0;
    }
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if ((desired_access & ~(GENERIC_READ | GENERIC_WRITE)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 8);
        return 0;
    }
    if ((flags_and_attributes & ~(FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 9);
        return 0;
    }
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_CreateFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (path_len == 1 && path[0] == '.') {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 10);
        return 0;
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 3) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if (kind == 2) {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 11);
        return 0;
    }

    int exists = (kind == 1);

    int must_create = 0, must_not_exist = 0, truncate = 0;
    switch (creation_disposition) {
    case CREATE_NEW:
        must_create = 1;
        must_not_exist = 1;
        break;
    case CREATE_ALWAYS:
        must_create = 1;
        truncate = 1;
        break;
    case OPEN_EXISTING:
        break;
    case OPEN_ALWAYS:
        must_create = 1;
        break;
    case TRUNCATE_EXISTING:
        truncate = 1;
        break;
    }

    if (!exists) {
        if (!must_create) {
            set_error(ERROR_FILE_NOT_FOUND);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    } else {
        if (must_not_exist) {
            set_error(ERROR_FILE_EXISTS);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (files[i].used && files[i].id == file_id) {
            if (sharing_conflicts(files[i].access, files[i].share,
                                  (u32)desired_access, (u32)share_mode)) {
                set_error(ERROR_SHARING_VIOLATION);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
        }
    }

    u64 create_arg = (!exists && must_create) ? 1 : 0;
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, path_len, create_arg, 0, &second);
    file_id = second;

    if ((s64)fd < 0) {
        set_error(errno_to_win(fd));
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (truncate && exists) {
        w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
    }

    struct file_entry *f = alloc_entry();
    if (!f) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    f->fd = fd;
    f->id = file_id;
    f->access = (u32)desired_access;
    f->share = (u32)share_mode;
    f->position = 0;

    u64 h = (unsigned)(f - files) + 1;

    if (exists && (creation_disposition == OPEN_ALWAYS || creation_disposition == CREATE_ALWAYS)) {
        set_error(ERROR_ALREADY_EXISTS);
    } else {
        set_error(0);
    }

    return w2m_logged(W2M_API_CreateFileW, name, h);
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }

    u64 second = 0;
    w2m_host(W2M_FILE_CLOSE, f->fd, 0, 0, 0, &second);
    free_entry(f);

    set_error(0);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_DeleteFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (path_len == 1 && path[0] == '.') {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind != 1) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (delete_blocked(file_id, 0)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 ret = w2m_host(W2M_FILE_UNLINK, (u64)path, path_len, 0, 0, &second);
    if ((s64)ret < 0) {
        set_error(errno_to_win(ret));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* DeleteFileW leaves last error untouched on success. */
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }

    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(f->access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_READ, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    f->position += n;
    if (bytes_read)
        *(u32 *)bytes_read = (u32)n;

    /* ReadFile leaves last error untouched on success. */
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    u64 second = 0;
    u64 size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, &second);
    u64 base;

    switch (move_method) {
    case FILE_BEGIN:
        base = 0;
        break;
    case FILE_CURRENT:
        base = f->position;
        break;
    case FILE_END:
        base = size;
        break;
    default:
        set_error(ERROR_INVALID_PARAMETER);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 d = (s64)distance;
    s64 target = (s64)base + d;

    if (target < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-7);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    f->position = (u64)target;
    if (new_pointer)
        *(u64 *)new_pointer = (u64)target;

    /* SetFilePointerEx leaves last error untouched on success. */
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    if (!(f->access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_WRITE, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    f->position += n;
    if (written)
        *(u32 *)written = (u32)n;

    /* WriteFile leaves last error untouched on success. */
    return 1;
}
```