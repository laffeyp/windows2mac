/*
 * process_start.d — every system call of one process, in order (Sprint 092, E1; docs/solution-2026-10-16.md 4.2).
 * Run as: dtrace -q -s process_start.d -c "cmd.exe /c <exe> <args>" <image name>   (scripts/laptop_dtrace.py)
 * The process is a child of the target, matched by its image name ($$1): `-c <exe>` itself lets the process load
 * kernel32 before the probes are armed (measured 2026-10-16: the trace began after the loader), so the parent is
 * the target and every probe is live before the child exists.
 * Lines, tab-separated, ordered later by timestamp:
 *   E <ns> <tid> <call> <arg0> ... <arg9>     a system call's entry, its first ten arguments in hex
 *   R <ns> <tid> <call> <result>              its return (the NTSTATUS)
 *   N <ns> <tid> <call> <root handle> <name>  the object name a call passes (OBJECT_ATTRIBUTES or a port name)
 * Reading user memory follows Microsoft's samples (samples/windows/file/filedeletetracker.d in
 * microsoft/DTrace-on-Windows): copyin the structure, then wstr2str on the copied buffer.
 */
#pragma D option quiet
#pragma D option bufsize=256m
#pragma D option strsize=1024

syscall:::entry
/execname == $$1/
{
    printf("E\t%d\t%d\t%s\t%x\t%x\t%x\t%x\t%x\t%x\t%x\t%x\t%x\t%x\n", timestamp, tid, probefunc,
           arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
}

syscall:::return
/execname == $$1/
{
    printf("R\t%d\t%d\t%s\t%x\n", timestamp, tid, probefunc, arg0);
}

/* OBJECT_ATTRIBUTES as the third argument: the Nt*Open* and Nt*Create* calls of the object manager's namespace */
syscall::NtOpenFile:entry, syscall::NtCreateFile:entry, syscall::NtOpenKey:entry, syscall::NtOpenKeyEx:entry,
syscall::NtCreateKey:entry, syscall::NtOpenSection:entry, syscall::NtCreateSection:entry,
syscall::NtOpenDirectoryObject:entry, syscall::NtOpenSymbolicLinkObject:entry, syscall::NtOpenEvent:entry,
syscall::NtCreateEvent:entry, syscall::NtOpenMutant:entry, syscall::NtCreateMutant:entry,
syscall::NtOpenSemaphore:entry, syscall::NtCreateSemaphore:entry, syscall::NtAlpcConnectPortEx:entry
/execname == $$1 && (probefunc == "NtAlpcConnectPortEx" ? arg1 : arg2) != 0/
{
    this->attr = (POBJECT_ATTRIBUTES)copyin(probefunc == "NtAlpcConnectPortEx" ? arg1 : arg2,
                                            sizeof(OBJECT_ATTRIBUTES));
    this->us = this->attr->ObjectName ?
        (PUNICODE_STRING)copyin((uintptr_t)this->attr->ObjectName, sizeof(UNICODE_STRING)) : 0;
    this->name = (this->us && this->us->Length) ?
        wstr2str((wchar_t*)copyin((uintptr_t)this->us->Buffer, this->us->Length), this->us->Length / 2) : "";
    printf("N\t%d\t%d\t%s\t%x\t%s\n", timestamp, tid, probefunc, (uintptr_t)this->attr->RootDirectory, this->name);
}

/* OBJECT_ATTRIBUTES as the first argument */
syscall::NtQueryAttributesFile:entry, syscall::NtQueryFullAttributesFile:entry
/execname == $$1 && arg0 != 0/
{
    this->attr = (POBJECT_ATTRIBUTES)copyin(arg0, sizeof(OBJECT_ATTRIBUTES));
    this->us = this->attr->ObjectName ?
        (PUNICODE_STRING)copyin((uintptr_t)this->attr->ObjectName, sizeof(UNICODE_STRING)) : 0;
    this->name = (this->us && this->us->Length) ?
        wstr2str((wchar_t*)copyin((uintptr_t)this->us->Buffer, this->us->Length), this->us->Length / 2) : "";
    printf("N\t%d\t%d\t%s\t%x\t%s\n", timestamp, tid, probefunc, (uintptr_t)this->attr->RootDirectory, this->name);
}

/* A port's name as a UNICODE_STRING, the second argument */
syscall::NtAlpcConnectPort:entry, syscall::NtConnectPort:entry, syscall::NtSecureConnectPort:entry
/execname == $$1 && arg1 != 0/
{
    this->us = (PUNICODE_STRING)copyin(arg1, sizeof(UNICODE_STRING));
    this->name = this->us->Length ?
        wstr2str((wchar_t*)copyin((uintptr_t)this->us->Buffer, this->us->Length), this->us->Length / 2) : "";
    printf("N\t%d\t%d\t%s\t0\t%s\n", timestamp, tid, probefunc, this->name);
}
