# Full review, 2026-10-16: what to cut, what to keep

Received from the user in chat on 2026-10-16 with the instruction "apply as next move, supersedes all direction this is
the roadmap now" (BLACKBOARD Decisions, 2026-10-16 user). The text below is the paste as it arrived: the paste cut
words and lines in several places (for example "milestoode runs at 590 millioninstructions"). It is kept unrepaired;
`docs/plan.md` carries the roadmap built from its legible parts and names each gap it could not read.

---

Full review, 2026-10-16: what to cut, what to keep

w2m has not drawn a frame. In the ten days since 2026-10-07 it has made 93 sprints. It meets milestoode runs at 590 millioninstructions a second inside the vCPU. Since Sprint 070, though, no sprint has made the game run fasearer. Sprints 071 to 090rebuilt the compiler three times: a stencil table, then the same compiler in  C, then symbolic executio093 counted things.Sprints 091 and 092 copied Windows and traced it. The traces were the best    work of the week, and the
                                                                              The number that decides elaptop's play: 314 NTsystem calls and 18 win32k calls a frame at 113 frames a second, about 37,600 a second. The three commo(114 a frame),NtWaitForSingleObject (100) and NtReleaseSemaphore (64), and all three are  eduler work. Today w2mhost, at about 14 µs acrossing. At that rate those calls alone would take 0.53 s of every second al speed; inside the VM So the in-VM scheduler,on several vCPUs, is the speed gate, and the recentering's step 5 has waitedce 2026-10-14.

                                                                            The compiler's equal-w77, every compiler in Cmust emit the same arm64 words as the Python compile_insn. That made     porting safe. It also er's code: no registerallocation across instructions, no flag liveness across blocks, nothing  faster than Python's oady judge behavior against an Intel Xeon (456 of 458 accepted), and the 68 families judge calls.    Keep those two judges  passes can change thewords.                                                                   Symbolic stencil gener). Sprint 071 measured the real set: 1,287 distinct encoder sequences cover every compiled          instruction, and the cSprint 090's symbolic runmade 363,546 stencils from 1,000 instructions in 441 s, and extrapolates to tens of millions ovness for forms no gameshowed.

   Sprint 083's judged run sent 46 blocks a billion instructions to the host
   for want of a stencil.nd cached, so it is aone-time cost, not a slow path, and the "any game" rule is not broken by
   it. Keep the captured k, and stop the symbolicline.
3. Static reach as an ordalking 3,786 images withCapstone to find the system calls the imports might reach gives an upper
   bound, inflated by evel. E1 and E3 already namethe calls real processes make: 73 kinds at start-up, plus the frame's set.
   Run 093 once to size twork from the traces. The predicted defect classes (forwarders, ordinal-only exports, the API set
   schema) are worth fixioader needs them anyway.
4. The windows-function census (Sprints 085 to 088). It counts Windows
   exports and w2m's cove26-10-16 decision,Microsoft's DLLs serve every export, so that coverage figure no longer
   measures anything. Thend 087) stays, because the compiler still needs it.
5. Bookkeeping as commitsce 2026-10-14 areclosures, sprint tails, KIT_DIARY entries and record stamps. Fold each
   closure into the sprinCI on it.
6. Five live direction documents.
   - The five: docs/plan.ce.md,recentering-2026-10-14.md, recentering-2026-10-16-knowable-first.md and solution-2026-10-16.
   - What is stale: plan.md's rows 068 to 071 still list the JIT region and
     milestone A as futur says milestone C is"under 16.7 ms".
   - The fix: one documen others become history.
7. The organism and research-paper line, as work. Of it, keep only the
   snapshot at the end ofix. That snapshot cutseach measured change's 300 s rerun, which makes it a speed tool. Model
   learning, bandits and un takes seconds.
8. A UMD of our own as the route to the first frame.
   - What E3 shows: underframe goes through thehardware-queue path, with NtGdiDdDDISubmitCommandToHwQueue 5.0 times a
     frame, SignalSynchro times andNtNotifyPresentToCompositionSurface once. That is dxgkrnl's whole
     scheduling model, pl
   - The faster route: DXMT's d3d11.dll and dxgi.dll, loaded in their place,
     reach Metal through already sorted their 200imports, and Microsoft's DLLs now serve all 200.
   - The recommendation: ; the in-house drivercomes after, if ever. This bends the 2026-10-16 decision at one DLL
     pair, the one where nto a kernel driver w2mwould have to write.

Keep

- The judges:
  - the families, whose rs;
  - the class runs against the Xeon;
  - the call stream comparuns;
  - planted faults;
  - the laptop as oracle.

  No other Windows-on-Macakes every later changesafe.
- The VM: Hypervisor.fram EL1, with the runtimeinside it.
- The compiler inside thef 87,887 blocks compiledin the guest.
- Microsoft's DLLs over aservice path kept as thegame's path, its record and its fallback until the new path passes the 68
  families and milestone
- The laptop's DTrace setup: 486 NT probes and win32u through the pid
  provider. It is the jud

Do next, in this order, ent and seconds to thefirst scene

1. Measure the whole run now. Measure Hail to the Rainbow's batch run on
   today's tree: instructnd, and fps-equivalent(the rate over 41 million). The last whole-run figure is Sprint 070's
   62.42 s warm.
2. The scheduler, waits and NtDelayExecution inside the VM, with the wake
   path waking one waiterr-vCPU queues and theGICv3 made before any vCPU. The judges are the 7 threading families and
   E3's three top calls a
3. Several vCPUs under M:N. Gate them with the litmus tests (MP, SB, LB, 2+2W
   and IRIW) and report fand 8. The ceiling risesfrom about 14 to about 50.
4. Two compiler passes, jlag liveness acrossblocks, then register allocation over a region.
5. The ahead-of-time cachch retires 76.9% of play's instructions.
6. The window on AppKit'shind the seam: the firstframe. Input follows, using exactly the call list the laptop's play runs
   made.
7. Microsoft's DLLs over the w2m kernel, in parallel and off the critical
   path.
   - First: E1's start-up list, 342 calls of 62 kinds before main, plus
     csrss's \Windows\Api
   - Then: the families, moved over one at a time.
   - The game stays on thne conforms.

Steps 2 to 5 are where thnd step 6 is where thepicture comes from. Nothing in 085 to 093 shortens either.
