#include <windows.h>
/* Sprint 358 (the leaf locks' timing, the heap's row): two threads on two vCPUs take and give back blocks of the
 * process heap at once, as Sprint 343's two loops drive heap.c's HeapAlloc and HeapFree from the monitor code
 * (crates/w2m_vm/tests/run_loop.rs, the_process_heap_hands_no_block_to_two_vcpus_at_once), here from a program, so
 * `run_measurements` can time the heap's lock on its run.
 *   - The main thread takes the heap's handle by GetProcessHeap, makes the two threads and waits for both.
 *   - Each thread arrives at a shared word and spins until the other has too, so their rounds run at once from the
 *     first; then ROUNDS times, for each of its SLOTS slots: the slot's block, if it holds one, has its thread's mark
 *     checked at its first and last words and is given back by HeapFree; then a block of 16 to 264 bytes is taken by
 *     HeapAlloc, marked at both ends and kept in the slot. After its rounds it gives every slot's block back.
 * A spin of a pseudo-random length before each call keeps the two threads' calls out of step. Every HeapAlloc that
 * answers a block is counted in `allocated`, every HeapFree that answers TRUE in `freed`; a NULL block, a FALSE free and
 * a mark found wrong (another thread's block handed out again, or a block written over) are counted in `wrong`. Each
 * thread counts the rounds in which it saw the other's round count move (`overlap`). The counts sit in a section of
 * their own, `.counts`, which run_measurements reads through w2m-vm's --runtime-word file. The output is "done" when
 * nothing is wrong and every block taken was given back; else "failed". Built as its neighbours:
 * x86_64-w64-mingw32-gcc -O1 -nostdlib -Wl,-e,start -o heap_blocks.exe heap_blocks.c -lkernel32 (GCC 16.2.0). */
#define ROUNDS 2000
#define SLOTS 32
#define THREADS 2
struct judged {
    volatile LONG64 allocated, freed, wrong, overlap[THREADS];
};
__attribute__((section(".counts"))) struct judged c = {0, 0, 0, {0, 0}};
static volatile LONG arrived;
static volatile LONG64 round_of[THREADS];
static HANDLE heap;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
/* A spin of 0 to 63 turns, from the thread's own linear congruential sequence (Knuth's MMIX constants). */
static void jitter(ULONG64 *seed) {
    *seed = *seed * 6364136223846793005ull + 1442695040888963407ull;
    for (volatile int k = (int)(*seed >> 58); k; k--) {}
}
/* The block at `p`, `size` bytes, checked for `mark` at both ends and given back. */
static void give_back(ULONG64 *p, SIZE_T size, ULONG64 mark) {
    if (p[0] != mark || p[size / 8 - 1] != mark) InterlockedIncrement64(&c.wrong);
    if (HeapFree(heap, 0, p)) InterlockedIncrement64(&c.freed);
    else InterlockedIncrement64(&c.wrong);
}
static DWORD WINAPI worker(LPVOID arg) {
    int me = (int)(INT_PTR)arg;
    ULONG64 seed = 0x9E3779B97F4A7C15ull * (ULONG64)(me + 1);
    ULONG64 mark = 0xB10C000000000001ull + (ULONG64)me;
    ULONG64 *block[SLOTS] = {0};
    SIZE_T size[SLOTS] = {0};
    LONG64 seen = 0;
    InterlockedIncrement(&arrived);
    while (arrived < THREADS) {}
    for (LONG64 r = 1; r <= ROUNDS; r++) {
        for (int s = 0; s < SLOTS; s++) {
            jitter(&seed);
            if (block[s]) give_back(block[s], size[s], mark);
            size[s] = 16 + 8 * (SIZE_T)((s + r) % 32);
            jitter(&seed);
            block[s] = (ULONG64 *)HeapAlloc(heap, 0, size[s]);
            if (!block[s]) {
                InterlockedIncrement64(&c.wrong);
                continue;
            }
            InterlockedIncrement64(&c.allocated);
            block[s][0] = mark;
            block[s][size[s] / 8 - 1] = mark;
        }
        round_of[me] = r;
        LONG64 other = round_of[1 - me];
        if (other != seen) {
            seen = other;
            c.overlap[me]++;
        }
    }
    for (int s = 0; s < SLOTS; s++)
        if (block[s]) give_back(block[s], size[s], mark);
    return 0;
}
void start(void) {
    heap = GetProcessHeap();
    HANDLE t[THREADS];
    for (int i = 0; i < THREADS; i++) t[i] = CreateThread(NULL, 0, worker, (LPVOID)(INT_PTR)i, 0, NULL);
    if (WaitForMultipleObjects(THREADS, t, TRUE, INFINITE) != WAIT_OBJECT_0) c.wrong++;
    if (!c.wrong && c.allocated == c.freed) line("done\n", 5);
    else line("failed\n", 7);
    ExitProcess(0);
}
