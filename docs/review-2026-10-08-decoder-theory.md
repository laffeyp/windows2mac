# Decoder approach against published work — 2026-10-08

The project builds an x86-64 decoder two ways (Capstone as the existing decoder; w2m's own, generated from `specs/x86_decode.json`) and treats disagreements as the work queue (user decision 2026-10-08). This note checks that approach against published work and names what changes.

## What the literature says

**Differential testing of x86 decoders is established practice, and every decoder tested has bugs.** Paleari, Martignoni, Fresi Roglia, and Bruschi, "N-version disassembly: differential testing of x86 disassemblers" (ISSTA 2010), compared disassemblers against each other and against the CPU itself as the ideal decoder; they found bugs in all eight popular x86 disassemblers tested. Woodruff, Carroll, and Peters, "Differential Analysis of x86-64 Instruction Decoders" (Trail of Bits, LangSec 2021; tool: mishegos), ran hundreds of millions of tests per hour across Capstone, libopcodes, udis86, XED, and Zydis and found hundreds of errors. Their blog report states that Capstone "reports both false positives and false negatives with some regularity" and recommends XED or Zydis.

**Independently built versions do not fail independently.** Knight and Leveson, "An Experimental Evaluation of the Assumption of Independence in Multiversion Programming" (IEEE TSE 12(1), 1986): 27 versions written from one specification, one million tests; simultaneous failures far exceeded the number independence predicts. Agreement between two versions is weaker evidence than it looks.

**A real CPU is the ground truth for length and validity.** Paleari et al. delegate to the CPU; Domas's sandsifter determines instruction length on hardware with a page-fault trick and found undocumented instructions on every major vendor's processors and bugs in disassemblers.

**Real binaries are not enough input.** Both mishegos (structured generation over a model of prefixes, ModRM, SIB) and Paleari et al. generate encodings instead of relying on compiler output. Compiler output exercises a small, regular part of the encoding space.

**Mnemonic agreement is not semantic agreement.** Kim et al., "Testing Intermediate Representations for Binary Analysis" (ASE 2017, MeanDiff), differentially tested the *semantics* lifters assign to instructions and found 24 unknown semantic bugs in three state-of-the-art lifters. Dasgupta, Park, Kasampalis, Adve, and Roşu, "A Complete Formal Semantics of x86-64 User-Level Instruction Set Architecture" (PLDI 2019), formalize 3,155 instruction variants (774 mnemonics), executable, tested against more than 7,000 instruction-level tests, and found bugs in Intel's manual itself.

**There is an authoritative machine-readable x86 encoding specification.** Intel's XED (github.com/intelxed/xed, Apache-2.0) is generated from datafiles that describe the whole instruction set; it is Intel's own decoder.

## What this says about the current work

1. **Capstone is the wrong primary reference.** The literature names it as the least reliable of the common decoders. Fix: make XED the primary reference (Intel's own tables), keep Capstone and LLVM as additional voters, and add Zydis if a fourth is needed.
2. **Our decoder is not independent of the references.** Its spec was written by an agent whose knowledge of x86 encoding comes from the same public material the other decoders were built from, and its mnemonics were spelled to match Capstone. Agreement on 52,785 instructions is therefore weak evidence (Knight and Leveson). Fix: adjudicate disagreements by authority, not vote — CPU first, then XED/Intel SDM text, then the rest.
3. **The CPU oracle is already available.** The `windows-2025` runner executes on real x86-64 hardware. Fix: a length/validity oracle probe on the runner for every byte sequence the decoders dispute (sandsifter/Paleari method, restricted to disputed sequences).
4. **The corpus finds priorities; generation finds bugs.** Python's binaries and the probes rank what to fix first. Fix: for each spec row, generate encodings across prefixes, REX bits, ModRM/SIB forms, and immediate widths (mishegos-style) and run the full differential on them.
5. **Length and mnemonic are the first oracle, not the last.** The translator needs operands and semantics. Fix: compare operands next (registers, memory components, immediate, operand size) against XED; then compare instruction semantics by execution — the same instruction run on the runner's CPU and under w2m's lifted semantics, with Dasgupta et al.'s K semantics as the published reference model (v0.2 §5's explicit machine-semantics layer).

## Against v0.2

- §11.1 requires evidence categories. The spec's 430 entries cite SDM tables but carry no quotes and were not checked against the text; by the project's own rule they are hypotheses until a reference or the CPU confirms them. The differential is that confirmation; the spec should record which source confirmed each row.
- §11.3 separates implementation repair from contract revision. A decoder disagreement is one or the other: a wrong row in `specs/x86_decode.json` is a contract revision; a wrong rule in `src/w2m/x86dec.py` is an implementation repair. The worklist should label each item, so a fix to the decoder routine never quietly redefines what the spec says.

## Sources

- R. Paleari, L. Martignoni, G. Fresi Roglia, D. Bruschi. N-version disassembly: differential testing of x86 disassemblers. ISSTA 2010. https://air.unimi.it/handle/2434/152687
- W. Woodruff, N. Carroll, S. Peters. Differential Analysis of x86-64 Instruction Decoders. LangSec 2021, EasyChair preprint 5095. https://easychair.org/publications/preprint/1LHr ; blog: https://blog.trailofbits.com/2019/10/31/destroying-x86_64-instruction-decoders-with-differential-fuzzing/
- J. C. Knight, N. G. Leveson. An Experimental Evaluation of the Assumption of Independence in Multiversion Programming. IEEE TSE 12(1), 1986. https://www.kth.se/social/files/564df871f2765419e306178d/KnightLeveson.pdf
- C. Domas. sandsifter: the x86 processor fuzzer. https://github.com/xoreaxeaxeax/sandsifter
- S. Kim et al. Testing Intermediate Representations for Binary Analysis. ASE 2017. https://softsec.kaist.ac.kr/~soomink/paper/ase17main-mainp491-p.pdf
- S. Dasgupta, D. Park, T. Kasampalis, V. S. Adve, G. Roşu. A Complete Formal Semantics of x86-64 User-Level Instruction Set Architecture. PLDI 2019. https://fsl.cs.illinois.edu/publications/dasgupta-park-kasampalis-adve-rosu-2019-pldi.pdf
- Intel XED. https://github.com/intelxed/xed
