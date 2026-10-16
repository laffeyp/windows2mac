/*
 * frame_calls.d — a running game's system calls and win32u calls over a window of seconds (Sprint 092, E3;
 * docs/solution-2026-10-16.md 4.3, 4.9).
 * Run as: dtrace -q -s frame_calls.d -p <pid> <from> <seconds>   (scripts/laptop_dtrace.py frames)
 * From DTrace's own second <from> after attaching, for <seconds> seconds: each NT system call of the process
 * (syscall provider) and each win32u export it calls (pid provider; win32u's exports are win32k's system-call
 * stubs, which the syscall provider does not list: measured 2026-10-16, 486 syscall functions, none NtUser or NtGdi).
 * The syscall provider has no probe for three of ntdll's 488 stubs (NtAlertMultipleThreadByThreadId,
 * NtAlertThreadByThreadIdEx, NtSetEventEx; measured 2026-10-16 against the copied ntdll.dll); the pid provider counts
 * calls into those stubs (a syscall instruction outside ntdll would not be counted).
 * Output lines: S <call> <count> | N <ntdll stub> <count> | W <export> <count> | T <first ns> <last ns> of the window.
 */
#pragma D option quiet
#pragma D option aggsize=16m

int s;
int64_t first;

dtrace:::BEGIN /$1 == 0/ { first = timestamp; }      /* s is 0 until the first tick */
profile:::tick-1sec { s++; }
profile:::tick-1sec /s == $1 + 0/ { first = timestamp; }

syscall:::entry /pid == $target && s >= $1 && s < $1 + $2/ { @sys[probefunc] = count(); }
pid$target:win32u::entry /s >= $1 && s < $1 + $2/ { @w32[probefunc] = count(); }
pid$target:ntdll:NtAlertMultipleThreadByThreadId:entry, pid$target:ntdll:NtAlertThreadByThreadIdEx:entry,
pid$target:ntdll:NtSetEventEx:entry
/s >= $1 && s < $1 + $2/ { @ntdll[probefunc] = count(); }

profile:::tick-1sec
/s == $1 + $2/
{
    printa("S\t%s\t%@d\n", @sys);
    printa("W\t%s\t%@d\n", @w32);
    printa("N\t%s\t%@d\n", @ntdll);
    printf("T\t%d\t%d\n", first, timestamp);
    exit(0);
}
