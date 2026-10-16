/*
 * wait_for_process.d — the pid of the first process whose image name contains $$1, printed once it has run $2
 * seconds, or nothing after 1200 s (Sprint 092, E3): frame_calls.d then attaches to it with -p, after the game has
 * loaded win32u.
 * Measured on the laptop's DTrace (2026-10-16): strstr(...) != NULL is false even on a match, so the match is
 * index() >= 0; and $2 * 1000000000 overflows 32 bits (60 gives -129542144), so $2 is cast to int64_t first.
 * A new process's first two system calls carry its execname but pid 0 (hello.exe, 2026-10-16), so pid != 0.
 */
#pragma D option quiet
int64_t born;
int game;
int waited;

syscall:::entry /born == 0 && pid != 0 && index(execname, $$1) >= 0/ { born = timestamp; game = pid; }
profile:::tick-1sec /born != 0 && timestamp - born >= (int64_t)$2 * 1000000000/ { printf("%d\n", game); exit(0); }
profile:::tick-1sec { waited++; }
profile:::tick-1sec /waited >= 1200/ { exit(1); }
