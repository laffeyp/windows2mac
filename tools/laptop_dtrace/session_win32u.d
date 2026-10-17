/*
 * session_win32u.d — a running game's win32u calls (win32k's system-call stubs, which the syscall provider does not
 * list) and its calls into the three ntdll stubs the syscall provider has no probe for, for a whole play session, with
 * its frames every 15 s (Sprint 093; E2's judge). Run as: dtrace -q -s session_win32u.d -p <pid>
 * (scripts/laptop_dtrace.py session). The facts are frame_calls.d's (Sprint 092, measured 2026-10-16).
 * Every 15 s: B <ns>, F <NtGdiDdDDIPresent calls in those 15 s>, then the session's counts so far, not cleared:
 * W <export> <count> | N <ntdll stub> <count>.
 */
#pragma D option quiet
#pragma D option aggsize=16m

pid$target:win32u::entry { @w32[probefunc] = count(); }
pid$target:win32u:NtGdiDdDDIPresent:entry { @frames = count(); }
pid$target:ntdll:NtAlertMultipleThreadByThreadId:entry, pid$target:ntdll:NtAlertThreadByThreadIdEx:entry,
pid$target:ntdll:NtSetEventEx:entry
{ @ntdll[probefunc] = count(); }

profile:::tick-15sec
{
    printf("B\t%d\n", timestamp);
    printa("F\t%@d\n", @frames);
    clear(@frames);
    printa("W\t%s\t%@d\n", @w32);
    printa("N\t%s\t%@d\n", @ntdll);
}
