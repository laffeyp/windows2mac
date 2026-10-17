# Recentering, 2026-10-16: knowable first

**History from 2026-10-16:** superseded by `docs/plan.md`, the one live direction document (full review `docs/review-2026-10-16-full.md`, item 6; BLACKBOARD Decisions, 2026-10-16 user).

Amends `docs/recentering-2026-10-14.md` (not edited) on the user's direction of 2026-10-16 (BLACKBOARD Decisions):
w2m runs any game, at speed, and everything a file, header or specification states is entered before a run looks
for it. The frontier finds only the residue.

## What went wrong

The frontier was meant to find what nothing written down tells. It became the way w2m learns everything:

- **Services.** Hail to the Rainbow's 7 x86-64 PE files declare 633 imports from Windows DLLs in their import and
  delay-import tables. w2m serves 238. Each of the 338 services in `tools/guest_runtime/` was added when a run stopped
  on it, one stop per frontier run, while the list sat in the files.
- **The compiler inside the VM.** Its stencil table was inferred from one batch run of one game: 21,182 forms, and
  89-94% of instructions from outside that run have no stencil. The rules in `src/w2m/rules` already define every
  instruction the lifter takes.
- **Instruction coverage.** The lifter's classes grew as runs reached instructions. The game's code is on disk, and
  x64 images list every function's start in their exception directory (`.pdata`), so the instructions a game holds
  can be read without running it.
- **Image formats.** BioShock 2 Remastered, the other game on disk, is 16 PE files, all 32-bit (machine 0x14C); w2m
  reads PE32+ only. Nothing had looked. (Hail to the Rainbow's depot also holds 122 images marked 0x14C beside its 7
  x86-64 ones; the census of Sprint 085 shows all 122 are .NET assemblies, IL that mono runs, not native 32-bit code.
  This note corrects the first version of this document, which counted them as 32-bit images.)

## What is knowable, and how each enters

1. **The service surface.** Every PE in every depot on disk: imports and delay imports by DLL and name; the
   functions it looks up by name (`GetProcAddress` strings in its data, and the held Windows captures'
   `.dynamic.tsv`). Grouped by DLL and by the header that declares each (Windows SDK and mingw-w64 headers give the
   prototypes), and scheduled as a graph: families of related functions built and judged together, not one stop at a
   time.
2. **The instruction surface.** Every x86-64 image's functions from `.pdata`, exports and entry points, decoded by
   recursive descent: the instruction classes and forms each game holds, against the lifter's rules and the decoder's
   spec rows. What no rule lifts is the lifter's work list; what the model steps run (x87, SHLD/SHRD, RCR, the flag and
   string forms) is the compiler's.
3. **The compiler.** Generated from the rules for every lifted operation (recentering step 6 as written), not inferred
   from a run; the table inferred from the batch run stays only as a judge.
4. **Image formats and architectures.** The census counts machines and subsystems across depots; 32-bit x86 is in
   scope (the user's "any game").
5. **The frontier.** It starts from a snapshot taken after the knowable is in (step 10 of 2026-10-14), and a stop on
   something any of 1-4 would have shown is a defect in the census, not a finding.

## The order from here

1. **The static census** (the algorithm): a script over every depot that writes the service surface, the instruction
   surface and the image formats as committed data, with what w2m covers today beside each. Re-run when a depot is
   added; its output plans the next sprints.
2. **The compiler generated from the rules,** judged by the record's table and the class case sets.
3. **The census's instruction work list:** model steps lowered natively, unlifted classes lifted.
4. **The census's service work list,** family by family, in the guest runtime.
5. **32-bit images.**
6. Then the 2026-10-14 steps that remain: the kernel inside the VM (host requests), several vCPUs, the ahead-of-time
   cache, the frontier as a targeted check, graphics alongside.
