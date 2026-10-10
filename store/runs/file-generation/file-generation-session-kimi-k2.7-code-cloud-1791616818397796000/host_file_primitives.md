file_bridge.py — the host side of the guest runtime's file system (Sprint 042).

The guest runtime (`tools/guest_runtime/file.c`) implements the Windows file calls: dispositions,
sharing, error codes, name rules, file positions. It reaches the Mac only through the primitives
here, on the transport page (hvc #8), all under one root directory per run that stands in for the
guest's working directory:

    FILE_LOOKUP  (path, chars)          -> kind: 0 absent, 1 file, 2 directory, 3 a directory on the way is missing;
                                           arg[5] the file id (inode) for 1 and 2
    FILE_OPEN    (path, chars, create)  -> host descriptor, or -errno; create=1 is O_CREAT|O_EXCL; arg[5] the file id
    FILE_CLOSE   (fd)                   -> 0
    FILE_READ    (fd, buffer, n, offset)-> bytes read (pread)
    FILE_WRITE   (fd, buffer, n, offset)-> bytes written (pwrite)
    FILE_SIZE    (fd)                   -> size
    FILE_TRUNCATE(fd, size)             -> 0
    FILE_UNLINK  (path, chars)          -> 0, or -errno

Paths arrive as UTF-16LE as the guest normalized them. Components compare ASCII case-folded, as
NTFS compares ASCII names (its upcase table; non-ASCII folding is not needed by any recorded row
and is not done). An absolute path, a drive letter or a component that leaves the root is
Unimplemented: the run stops rather than guess.
