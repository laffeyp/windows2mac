#include <windows.h>
/* A parent and its child pass one byte back and forth over two pipes, 10,000 rounds, every read INFINITE (Sprint 365,
 * card 4f4): the parent writes round r's byte to the child's standard input and reads the child's answer, the byte plus
 * one, from the child's standard output; the child reads a byte and writes it back plus one. Each read finds its pipe
 * empty as often as not and blocks until the other process's write wakes it, so a wake lost between processes leaves
 * both reading for ever. The parent prints the rounds whose answer came back right and the child's exit code, 7 when
 * the child read and wrote all 10,000. Pipes and the child's start as `probes/processes/processes.c` makes them
 * (CreatePipe with inheritable ends, SetHandleInformation, STARTF_USESTDHANDLES, bInheritHandles TRUE). */
#define ROUNDS 10000
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static int has(const WCHAR *line, const WCHAR *word) {
    for (const WCHAR *p = line; *p; p++) {
        int k = 0;
        while (word[k] && p[k] == word[k]) k++;
        if (!word[k]) return 1;
    }
    return 0;
}
static int decimal(char *at, unsigned v) {
    char digits[10];
    int n = 0;
    do digits[n++] = (char)('0' + v % 10); while (v /= 10);
    for (int i = 0; i < n; i++) at[i] = digits[n - 1 - i];
    return n;
}
void start(void) {
    if (has(GetCommandLineW(), L" -c")) {
        HANDLE in = GetStdHandle(STD_INPUT_HANDLE), out = GetStdHandle(STD_OUTPUT_HANDLE);
        for (int r = 0; r < ROUNDS; r++) {
            unsigned char b;
            DWORD n = 0;
            if (!ReadFile(in, &b, 1, &n, NULL) || n != 1) ExitProcess(2);
            b++;
            if (!WriteFile(out, &b, 1, &n, NULL) || n != 1) ExitProcess(3);
        }
        ExitProcess(7);
    }
    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE down_read, down_write, up_read, up_write;
    if (!CreatePipe(&down_read, &down_write, &sa, 0) || !CreatePipe(&up_read, &up_write, &sa, 0)) {
        say("pipe failed\n", 12);
        ExitProcess(1);
    }
    SetHandleInformation(down_write, HANDLE_FLAG_INHERIT, 0);
    SetHandleInformation(up_read, HANDLE_FLAG_INHERIT, 0);
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = down_read;
    si.hStdOutput = up_write;
    si.hStdError = NULL;
    PROCESS_INFORMATION pi;
    if (!CreateProcessW(NULL, cmd, NULL, NULL, TRUE, NORMAL_PRIORITY_CLASS, NULL, NULL, &si, &pi)) {
        say("create failed\n", 14);
        ExitProcess(1);
    }
    CloseHandle(down_read);
    CloseHandle(up_write);
    unsigned rounds = 0;
    for (int r = 0; r < ROUNDS; r++) {
        unsigned char b = (unsigned char)r, answer = 0;
        DWORD n = 0;
        if (!WriteFile(down_write, &b, 1, &n, NULL) || n != 1) break;
        if (!ReadFile(up_read, &answer, 1, &n, NULL) || n != 1) break;
        if (answer != (unsigned char)(b + 1)) break;
        rounds++;
    }
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    char line[48] = "rounds ";
    int at = 7;
    at += decimal(line + at, rounds);
    const char child[] = ", child ";
    for (int i = 0; child[i]; i++) line[at++] = child[i];
    at += decimal(line + at, code);
    line[at++] = '\n';
    say(line, (DWORD)at);
    ExitProcess(rounds == ROUNDS && code == 7 ? 0 : 1);
}
