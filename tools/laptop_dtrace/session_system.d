/*
 * session_system.d — every NT system call of a game's processes, from each process's start to its end, for a whole
 * play session (Sprint 093; E2's judge). Run as: dtrace -q -s session_system.d <name> <seconds>
 * (scripts/laptop_dtrace.py session), armed before the game starts.
 * A process is the game's when its image name contains $$1 (execname is cut to 14 characters, and a new process's
 * first two calls carry pid 0: measured 2026-10-16, tools/laptop_dtrace/wait_for_process.d). The counts are printed
 * every 15 s and when a game process calls NtTerminateProcess, without clearing, so the last block is the session's
 * total if DTrace is stopped after the game ends (it does not exit by itself then: microsoft/DTrace-on-Windows #4).
 * Output blocks: B <ns>, then S <pid> <call> <count> lines. Ends by itself after $2 seconds.
 */
#pragma D option quiet
#pragma D option aggsize=16m

int ticks;

syscall:::entry /pid != 0 && index(execname, $$1) >= 0/ { @sys[pid, probefunc] = count(); }

syscall::NtTerminateProcess:entry /pid != 0 && index(execname, $$1) >= 0/
{
    printf("B\t%d\n", timestamp);
    printa("S\t%d\t%s\t%@d\n", @sys);
}

profile:::tick-15sec
{
    ticks++;
    printf("B\t%d\n", timestamp);
    printa("S\t%d\t%s\t%@d\n", @sys);
}

profile:::tick-15sec /ticks * 15 >= $2/ { exit(0); }
