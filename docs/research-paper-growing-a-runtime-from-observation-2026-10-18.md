# Growing a Runtime from Observation: Exploration, Reinforcement and Memory in a Self-Extending Compatibility Layer

Draft, 2026-10-14. Working paper on the w2m project (Windows x86-64 programs on Apple Silicon macOS).

## Abstract

A compatibility layer lets a program built for one system run on another. The usual way to build one is decades of
hand work: Wine has implemented the Windows API since 1993. w2m tries a different method, called the frontier here.
The unmodified program runs inside the new layer until it stops at a missing function or instruction. That piece is
built, checked against rows recorded on real Windows, and the run starts again. In its first week this method took one
Unity game from 13 instructions to 80.9 billion, and from 4.0 million to 442 million instructions a second. Over 3,451
calls it makes no defect against a set of Windows executions.

This paper asks how the frontier can become fast, parallel and automatic. The aim is a runtime that grows over its
guest the way a plasmodial slime mould grows over a food field. The paper takes three measured behaviours of
*Physarum polycephalum*:

- it explores on many fronts and then prunes them;
- it thickens the tubes that carry flow;
- it marks its own trail with slime it will not cross again.

For each, the paper names the computer-science problem the behaviour solves, and the algorithm with known guarantees
that solves it: active automata learning, bandit allocation, verifying traces in incremental build systems, epoch
snapshots and bisection.

The analysis gives four results.

1. The frontier is active learning of a reactive system. The Windows machine answers membership queries. Conformance
   testing on the program's own traces stands in for equivalence queries, so at best the frontier learns in the PAC
   sense.
2. Rerunning from the start makes the method's cost grow with the square of the number of stops. Snapshots that are
   checked for staleness make it linear.
3. The bound on growth is the sequential critical path of the layer's own execution. It is not the number of queries.
4. In the target scenario a person plays for 15 minutes on Windows while a Mac beside the machine learns. That scenario
   is infeasible today: w2m runs about 18 times too slowly, and it cannot yet snapshot its own state. A restricted form
   is feasible: a prior runtime, capture during play, and gap-filling after play.

The paper ends with the experiments that would test each claim.

## 1. Introduction

### 1.1 The problem

A Windows program sees Windows through two surfaces. One is the x86-64 instruction set: what each instruction does to
registers, flags and memory, including faults and memory ordering between threads. The other is the operating-system
interface: about 300 functions for this game so far, each with arguments, a return value, a last-error value and
effects on state that other calls can see. A compatibility layer must produce, at both surfaces, only observations that
Windows itself would allow (v0.2 §3).

The program never sees the layer, only the surfaces. That is the sense of the metaphor this paper borrows. The guest
is digested while it goes on believing it runs on Windows. Garfinkel et al. showed that full transparency is not
reached in practice: a determined program can always find some timing or structure that tells a virtual machine from
hardware [Garfinkel 2007]. The goal is narrower. Every observation this program actually makes should be one Windows
permits.

### 1.2 The frontier method as built

w2m runs the guest in a Hypervisor.framework virtual machine on an M5 Max. The pieces are:

- a decoder generated from a specification and judged by Intel XED;
- a lifter of 254 rule modules, one per instruction class;
- a translator to arm64;
- a guest runtime of 297 Windows functions in 8,787 lines of freestanding C.

Three judges keep it honest:

- Each instruction class is judged case by case against a named Intel Xeon: 456 of 458 classes are accepted, and FSIN
  and FCOS are pinned open in the last bit of 18 cases.
- Each of 68 function families is judged row by row against probe programs run on Windows.
- The game's own call stream is judged against a set of Windows executions: one cloud-runner run and ten runs on a
  local Windows laptop.

A check counts only once a planted fault makes it fail.

The frontier is the outer loop. It runs the game until it stops, writes down what it needs, and a sprint supplies it.
Table 1 gives the record.

**Table 1. The frontier's record (repository data, 2026-10-07 to 2026-10-14).**

| Quantity | Value | Source |
|---|---|---|
| First stop | `GetSystemTimeAsFileTime` after 13 instructions | Sprint 038 |
| Longest run to a stop | 80.9 billion instructions | frontier 1791942295792627000 |
| Peak translated-code speed | 4.0 to 442 million instructions a second | Sprint 064 |
| Rate over a whole run, quiet machine | 255.7 million a second (30 billion in 117.3 s) | recentering note, 2026-10-14 |
| Rate inside the vCPU only | about 590 million a second | same |
| Host crossing, current path | about 14 µs, against 0.72 µs for the bare exit | `scripts/measure_exit.py` |
| Game's batch log | equal to Windows' 168 lines but for timings | Sprints 065–066 |
| Call-stream comparison | 0 defects over 3,451 calls | Sprint 066 |
| Divergences reduced to real defects | 790 first divergences to 5 defects | KIT_DIARY #50 |

The method works. It is slow for one plain reason. Every fix restarts the guest from its first instruction, on one
virtual CPU, with one person or agent in the loop.

### 1.3 The target scenario

A person plays a Windows game for 15 minutes on a Windows machine. A Mac stands beside it, receiving the machine's
traces. When the session ends, the game plays on the Mac. Section 6 measures this against the numbers above. The
answer is no for now. The bottleneck is speed, not data, and a restricted version is in reach.

### 1.4 Contributions

1. A formal account of the frontier as active learning of a reactive system, with a stated oracle and a stated
   guarantee (Section 4.1).
2. Three mechanisms from *Physarum* matched to three algorithms with known properties (Sections 3 and 4).
3. A cost model showing that snapshots turn the frontier's quadratic rerun cost into linear cost, and a rule for when
   a snapshot is still valid (Sections 4.3 and 5).
4. A budget analysis of the 15-minute scenario from measured rates (Section 6).
5. An evaluation plan in which each claim has a test that could falsify it (Section 7).

## 2. The organism

*Physarum polycephalum* is a single cell with many nuclei that forages as a network of tubes. Protoplasm flows through
the tubes in a back-and-forth "shuttle streaming", driven by rhythmic contraction of actin and myosin in the tube walls
[Alim 2013; see Section 10]. Three measured behaviours concern this paper.

**Explore, then prune.** In Tero et al.'s experiment, oat flakes stood at the sites of cities around Tokyo, and light,
which the mould avoids, marked mountains and lakes. The mould first covered the arena with a dense network. It then
cut back to the tubes linking the food. The result matched the Tokyo rail network for efficiency, fault tolerance and
cost [Tero 2010]. In an earlier experiment the mould found the shortest path through a maze between two food sources
[Nakagaki 2000].

**Flow thickens tubes.** Tero, Kobayashi and Nakagaki model the network as tubes of conductance $D_{ij}$ and length
$L_{ij}$, carrying flux $Q_{ij} = D_{ij}(p_i - p_j)/L_{ij}$ between pressures at nodes. Conductance adapts as

$$\frac{dD_{ij}}{dt} = f(|Q_{ij}|) - D_{ij},$$

so a tube that carries flow grows and an idle one decays [Tero 2007]. Bonifaci, Mehlhorn and Varma proved that with
$f(|Q|) = |Q|$ the dynamics converge to the shortest path between source and sink [Bonifaci 2012]. This is one of the
few biological search heuristics with a convergence proof.

**Slime as outside memory.** The mould leaves extracellular slime on ground it has crossed, and it avoids that ground.
In a Y-maze, 39 of 40 moulds chose the unslimed arm. When both arms were slimed they showed no preference. The
avoidance stops when no fresh ground is left [Reid 2012]. The memory sits outside the organism, in the environment
it explores.

**Exploring against exploiting.** Reid et al. later set the mould a two-armed bandit, two food patches of differing
yield, and studied how it divides its effort between them [Reid 2016].

The mould is a metaphor here, not a model. What carries over is the structure of each behaviour, written as an
algorithm. Section 3 does that.

## 3. From the mould to algorithms

| Behaviour | The problem it solves | The algorithm with known properties | The frontier's version |
|---|---|---|---|
| Many fronts, then pruning | Search under uncertainty with cheap parallel probes | Best-first and Monte Carlo tree search; bandit-based planning (UCT) [Kocsis 2006] | Fork at each stop over candidate answers; prune by conformance with Windows |
| Flow thickens tubes | Allocate a fixed budget to the paths that pay | Proportional allocation; replicator dynamics [Taylor 1978]; Tero's rule with its convergence proof [Bonifaci 2012]; UCB for exploration [Auer 2002]; Gittins indices [Gittins 1979] | Compute and optimisation effort go to the code and services the guest uses most |
| Slime marks the trail | Never repeat work; know when a record is stale | Memoisation with verifying traces and early cutoff [Mokhov 2018] | Snapshots and translations keyed by content and by the identities their prefix used |
| Covering, then the cut | Find what changed between a good state and a bad one | Bisection; delta debugging [Zeller 2002] | Bisect over snapshots in time; isolate the failure-inducing change |
| A body that goes on feeding | Parallel work over a sequential execution | Epoch-parallel execution and replay [Veeraraghavan 2011] | Epochs from w2m's own checkpoints, checked in parallel |

The table hides one asymmetry. The mould's food is plain: an oat flake is food. The frontier's "food" is a correct
answer, and correctness needs an oracle. Firmware rehosting, the nearest field, shows what happens without one.
HALucinator replaces a firmware's hardware-abstraction functions with high-level models so the firmware runs without
its chip [Clements 2020]. Systems in that line judge a model mostly by whether the firmware keeps running and covers
more code. A model can keep the firmware running and still be wrong. w2m has a stronger oracle, Windows itself, and the
design below spends it carefully.

## 4. The same problems in computer science

### 4.1 Learning an interface: the frontier as model learning

Treat Windows-with-the-CPU as a reactive system $M$. Its inputs are the guest's actions: an instruction on a state, or
a call with its arguments. Its outputs are the observations the guest gets back. Because $M$ is nondeterministic
(thread schedules, timing), it defines a set of allowed traces $T(M)$, not a function. The task is to build $M'$ such
that the traces this guest $G$ produces against $M'$ lie in $T(M)$. Only the interface $G$ reaches matters; call it
$R(G)$.

This is model learning in the sense of Angluin and its descendants [Angluin 1987; Vaandrager 2017]. Angluin's L*
learns a minimal finite automaton in polynomial time from two kinds of query:

- a membership query ("what does the system output on this input?");
- an equivalence query ("is this hypothesis right? if not, give a counterexample").

Without queries, learning from given data alone is hard. Finding a minimal automaton consistent with a sample is
NP-hard [Gold 1978]. In the limit, some classes cannot be identified from positive examples at all [Gold 1967].

w2m has both kinds of query, in imperfect forms.

- **Membership queries:** probe programs run on a Windows machine. A probe calls a function on chosen arguments and
  prints the answer. On the laptop a probe job takes seconds. The family tables are the answers on file.
- **Equivalence queries:** none exist for a system the size of Windows. w2m substitutes conformance testing. The
  guest's traces under $M'$ are compared with a set of recorded Windows traces, and the family rows are compared with
  $M'$'s answers. Conformance testing of state machines has a long theory (the W-method [Chow 1978]). Angluin showed
  that an equivalence oracle can be replaced by testing on random examples, at the price of a PAC guarantee:
  approximately correct, with high probability, on the distribution the examples come from [Angluin 1988].

That last point is the theoretical form of KIT_DIARY #47: "a probe that copies the program's inputs proves the
implementation on those inputs only." A PAC guarantee holds on the sampling distribution. Rows drawn from one game's
inputs certify $M'$ on that game's inputs. Generalising needs rows from other distributions: generated inputs, held-out
rows, a second game.

The passive part, observing the guest, decides *where* to learn. The active part, the probes, decides *what is true
there*. The 15-minute scenario of Section 6 is passive observation followed by active queries.

### 4.2 Search with forks: bandits and tree search

At a stop the frontier must choose an answer. The candidates are cheap to list:

- the family's existing contract;
- the documented failure;
- Wine's or ReactOS's behaviour for the function;
- a model's draft.

Today the frontier tries one, and a wrong choice is found only by the next divergence, after a full rerun.

Forking turns this into tree search. Each node is a snapshot at a stop, each edge a candidate answer, and the value of
a node is how far its branch conforms with the reference. Upper-confidence tree search (UCT) [Kocsis 2006] allocates
rollouts across a tree with regret bounds inherited from the UCB bandit rule [Auer 2002]. The reward signal is the
growth measure of Section 5.4: new conforming code per CPU-second.

The economics is short. Each branch costs compute. Most of this Mac's 18 cores sit idle while the frontier runs one
thread, so the shadow price of compute is near zero up to the core count. Forking up to that width costs nothing but
power. Past it, branches compete, and the allocation rule matters.

Tero's rule and UCB differ in one respect that matters here. Proportional growth with decay, as in replicator dynamics
[Taylor 1978], can lock in an early leader. UCB keeps an exploration bonus that shrinks only as a branch is tried. The
mould also keeps redundant tubes, part of the fault tolerance Tero et al. measured [Tero 2010]. Either way, a branch
pruned early is not ruled out for good.

Gittins indices [Gittins 1979] give the optimal policy for independent bandit arms with discounting. They suit the
frontier's other choice: which open stop or speed item to work on next, when each has a cost and an uncertain payoff.
The assumption of independent arms is strong here, since a fix to one family can move many stops.

### 4.3 Memory and staleness: incremental computation

The frontier's cost model is plain arithmetic. Suppose the guest needs $n$ stops, spread over a path of length $L$
instructions, and every fix reruns from the start. The total instructions executed are

$$\sum_{k=1}^{n} \ell_k \approx \frac{nL}{2}$$

for evenly spaced stops at positions $\ell_k$: quadratic in the number of stops. With a snapshot at each stop the total
is about $L$: linear.

For this game, $L$ to the first scene is 325.4 billion instructions on Windows. For $n = 100$, a number for
illustration only, rerunning costs about 16.3 trillion instructions. At 255.7 million a second that is about 17.7
hours of serial execution. A single pass takes about 21 minutes.

Snapshots raise the question build systems answer: when is a stored result still valid? Mokhov, Mitchell and Peyton
Jones classify build systems by their rebuild strategy and name the right one for this case. **Verifying traces**
record, for each result, the hashes of the inputs it read, and a result is reused while those hashes are unchanged.
**Early cutoff** reuses dependents when a rebuilt input comes out the same as before [Mokhov 2018].

For the frontier:

- a snapshot is a result;
- its inputs are the translation rules and runtime services the prefix executed;
- it stays valid while their hashes do.

w2m already computes content identities for families and classes, which decide when a judged item must run again. The
recentering note of 2026-10-14 finds those identities too wide: an edit to `vm_backend.py` reruns all 68 families and
458 classes, 10 to 15 minutes. The same narrowing serves snapshots.

Early cutoff has a cheap form here. Each call the prefix made can be replayed alone against the new implementation,
with its recorded arguments and visible state. If every answer is the same, the snapshot survives the change without
running the guest.

### 4.4 Localising a divergence: bisection and delta debugging

Today a divergence is found by its symptom: a call whose value differs from Windows'. Its cause may lie billions of
instructions earlier.

- **With periodic snapshots,** the first state that leads to divergence is found by halving, in $\log_2(L/\Delta)$
  resumes for snapshot spacing $\Delta$.
- **When the cause is a change,** for instance a commit that moved a family's hash, delta debugging isolates the
  failure-inducing part of the change [Zeller 2002].

Both exist as tools for source histories. They need snapshots before they apply to an execution.

### 4.5 Parallel work over a sequential execution

The guest's execution is sequential in the sense that matters: state at instruction $t$ depends on state at $t-1$.
Veeraraghavan et al.'s DoublePlay splits a run into epochs. A fast thread-parallel execution produces checkpoints at
epoch boundaries. Each epoch is then rerun in parallel from its checkpoint, and the two are compared
[Veeraraghavan 2011].

The frontier can use the same split. A fast pass of w2m produces checkpoints. Many slower passes then check each epoch
in parallel: the call stream against the reference, instrumented runs, branches with candidate answers. The critical
path stays the fast pass. The checking comes off it.

A tempting shortcut does not work. One could take epoch checkpoints from the Windows machine instead, since Windows
runs the guest about 20 times faster. A Windows process image holds Windows' own internal state (heap metadata, loader
lists, thread bookkeeping inside ntdll and kernel32), laid out as Windows lays it out, not as w2m's runtime does. It
cannot resume under w2m unless w2m runs Windows' own system DLLs. Epoch checkpoints must come from w2m itself.

### 4.6 Speed: tiered compilation and snapshot restore

Two standard pieces carry the speed half.

- **Tiered dynamic optimisation.** A fast first translation, then re-optimisation of the code that proves hot. This is
  the form since Dynamo [Bala 2000], and it is Tero's rule applied to code: flow thickens the tube. w2m's census
  already counts per class and per exit kind. The step is to let it promote automatically.
- **Snapshot restore.** Firecracker maps a snapshot's memory file `MAP_PRIVATE`. Pages load on first touch and writes
  go to a private copy, so restore is fast and the cost moves to the first touch of each page
  [Agache 2020; Firecracker docs]. Nyx restores whole virtual machines thousands of times a second, at a cost set by
  the pages a test dirties [Schumilo 2021]. w2m's guest memory is already host memory, so the same private mapping
  gives copy-on-write branches.

The obstacle is the state that is not in guest memory: w2m's scheduler, handles and file objects live in a Python
host process. Moving that kernel into the virtual machine is planned for speed (recentering note, step 4). It also
makes a snapshot equal to guest memory, vCPU registers and a small device state. The speed work and the parallel work
are one change.

## 5. Design: a runtime that grows

### 5.1 Objects

- **Reference set** $W$: recorded Windows executions of the guest (call streams with arguments and returns, logs,
  frames, retired-instruction counts by thread), plus the family rows.
- **Snapshot** $s$: guest memory, registers and device state at a point, with its dependency set
  $\mathrm{deps}(s)$, the content hashes of every rule and service its prefix used.
- **Stop**: a point where $M'$ cannot answer: an unlifted instruction, an unserved function, an argument shape no row
  covers, or a divergence from $W$.
- **Candidate**: a proposed answer for a stop, with its source (contract, documented failure, another implementation,
  a model's draft).

### 5.2 The loop

```
frontier := { s0 }                        # snapshot at process start
while budget remains:
    s := select(frontier)                 # UCB over expected conforming growth (§4.2)
    run s forward on the fastest tier until a stop, or until N instructions pass
        - write a snapshot every Δ instructions, with deps
        - stream calls to the comparator against W
    at a stop σ:
        C := candidates(σ)                # from contracts, docs, implementations, drafts
        fork one branch per c in C, up to the free cores
        each branch runs until it diverges from W or passes a horizon h
        keep the branches that conform; write their answers to the contract as hypotheses
        queue membership queries (probes) that separate the surviving candidates
    on a change to M' (a fix, a lowering):
        invalidate exactly the snapshots whose deps changed (§4.3)
        early cutoff: replay the affected calls alone; keep snapshots whose answers are unchanged
    promote: compile the hottest blocks at the higher tier; move the busiest services into the VM
```

### 5.3 What prunes a branch

The pruning signal sets the rigour of the whole design, so it is ranked.

1. A value divergence from $W$: the same call on the paired thread, with the same inputs, answered differently.
   Fatal to the branch.
2. A family row that the branch's answer contradicts. Fatal.
3. A presence or count divergence on a thread whose earlier calls conform. Recorded as the guest's scheduling, not
   fatal (Decision 2026-10-13).
4. Past the end of $W$, no oracle remains. The branch survives on progress alone, as rehosting systems do, and its
   answers stay marked as hypotheses until a probe confirms them.

Rule 4 is where this design is weakest. It is the gap the 15-minute scenario must close.

### 5.4 The growth measure

The runtime needs one number for its own size. Let $B_W$ be the basic blocks the guest executes on Windows over a
stretch, and $B_{M'}$ those it executes under w2m with every call conforming. Growth is
$|B_{M'} \cap B_W| / |B_W|$, reported per image and per stretch.

- **The Windows side** can come from Time Travel Debugging on the laptop, which records every instruction a process
  executes [Microsoft TTD], or from the laptop's retired-instruction counters at coarser grain.
- **The w2m side** comes from the translator's block set.

The measure gives the loop an objective, and it gives the evaluation a dependent variable.

### 5.5 Generalisation pressure

The PAC point of Section 4.1 sets the rule. Every family gets rows drawn from at least three distributions:

- the guest's own inputs;
- generated inputs over the argument space the documentation defines;
- held-out rows the writer of the implementation never saw.

The second guest is the real test. A runtime grown on one Unity game, run unchanged on a second Unity game of the same
engine version, should need only that game's delta.

## 6. The 15-minute scenario

### 6.1 The budget

The laptop's run to the first scene retires 325.4 billion instructions in 62 s, about 5.25 billion a second. In play it
retires about 41 million a frame at 112 frames a second, about 4.6 billion a second, spread over six busy threads. The
busiest thread does 28.9% of the instructions (laptop run lap-20261014T041748433Z-73c66e6a-01).

**Table 2. The 15-minute session, measured rates and derived quantities.**

| Quantity | Value | Kind |
|---|---|---|
| Instructions Windows retires in 15 minutes of play | about $4.1 \times 10^{12}$ (900 s × 4.6 billion a second) | derived |
| w2m whole-run rate today | 255.7 million a second | measured |
| Serial time for w2m to execute the same session | about 16,200 s, 4.5 hours | derived |
| Aggregate speed-up w2m needs to keep pace in real time | about 18× | derived |
| Busiest thread's demand at 112 frames a second | about 1.33 billion a second | derived |
| w2m's single-vCPU rate inside translated code | about 590 million a second | measured |
| Frame-rate ceiling with unlimited vCPUs and no host time, today's code | about 50 frames a second | derived |

The ceiling in the last row comes from the busiest thread. Only faster translated code raises it. More vCPUs do not.

### 6.2 What can be learned during play

During the session the Mac does not need to execute the game. It needs to absorb what the Windows machine shows.

- **The interface the session reaches.** Every call with arguments and returns, from the call-trace capture w2m
  already runs on Windows (`tools/jit_capture/call_trace.h`). Each call is checked at once against the families. A new
  function or argument shape queues a probe for a second Windows machine or the cloud runners. The laptop itself is
  busy with the game.
- **The shaders and pipeline states.** Every shader the game creates goes through `CreateVertexShader`,
  `CreatePixelShader` and the like as bytecode. Captured, each can be translated to Metal before the Mac ever runs the
  game. Valve's Fossilize does the same for Vulkan pipelines in Steam's shader pre-caching, collecting pipeline state
  from play so other machines skip the compile [Fossilize].
- **The code the session executes.** The code pages of the images that run, known exactly with an instruction trace,
  approximately with page-level sampling. These can be translated ahead of time on the Mac while play goes on. The
  images are fixed PE files, so this work does not depend on w2m's own execution.
- **The inputs.** Keyboard and mouse with their times. The laptop already records them in play mode (`input.cs`).
- **The files touched and the saved state written**, for the initial-state contract.

All of this is passive learning (Section 4.1). It decides where the runtime must be right, and it gathers the
membership queries to ask. It cannot settle what the answers are where Windows' trace does not show them, which is
most internal state.

### 6.3 What cannot be learned in 15 minutes

- **The prior.** The CPU's semantics, the 297 functions and the graphics translation exist before the session. Their
  cost was paid once and is shared across games. The session can only learn the delta. The scenario is honest only if
  it is stated as "a session teaches a prior runtime the delta for one game."
- **The guest's own execution under w2m.** Section 4.5 showed that epoch checkpoints must come from w2m. The Mac must
  therefore run the game itself at least once, end to end. At today's rate the session's 15 minutes take 4.5 hours.
- **Paths the session did not take.** A PAC guarantee covers the distribution sampled. A player who opens a menu the
  session never opened meets $M'$ where it was never tested.
- **Exact replay.** Replaying recorded inputs at recorded times does not reproduce the session. Thread schedules and
  timing differ, and the game reacts to both. w2m follows the session's path only roughly. Comparison stays set-valued
  (Section 5.3, rule 3), and states are matched by the game's own markers: log lines, scene loads, frames.

### 6.4 The verdict

The scenario fails on two counts.

- **Speed:** w2m needs about an 18-fold rise in aggregate speed, and a higher single-thread rate to exceed about 50
  frames a second.
- **Snapshots:** w2m cannot snapshot its own state, so no parallel checking off the critical path is possible.

A restricted version is in reach once three changes land: the kernel inside the VM, snapshots, and fork-and-prune.
That version is: a prior runtime, capture during the session, ahead-of-time translation of executed code and shaders
during the session, then a post-session run in which the Mac executes the game once on the fast tier while forked
branches settle the stops the session revealed. The time from session's end to playable is then set by how fast w2m
runs the game once, plus the probes the session queued.

## 7. Evaluation plan

Each claim has a test that could falsify it.

| Claim | Experiment | Falsified if |
|---|---|---|
| Snapshots make the frontier's cost linear | Replay the project's history of stops with and without snapshot reuse; count instructions executed | Reuse saves less than half, because most fixes invalidate most snapshots |
| Narrow dependency sets keep most snapshots valid | For each of the last 30 fixes, compute which snapshots' deps changed | Most fixes touch a module every prefix uses (`vm_backend.py` today) |
| Forking cuts the time to settle a stop | At recorded stops, run all candidates at once against sequential trial | Wall time to the first conforming branch does not fall with width up to the free cores |
| Pruning by conformance beats pruning by progress | Plant a wrong candidate that keeps the guest running; compare the two rules | The progress rule rejects it as often as the conformance rule |
| The growth measure tracks playability | Report growth per stretch beside the first frame and frame rate | Growth rises while the game plays no further |
| A runtime grown on one game carries to a second | Run a second Unity 2019.4 game; count new stops and new families | The second game needs as many new families as the first |
| A 15-minute session supplies the delta | One recorded session: count queued probes, captured shaders, executed pages; then time the Mac to playable | Most stops after the session lie on paths the session showed but did not explain |

## 8. Limits and threats

- **One guest.** Every number in this paper comes from one game, on one Mac, with one Windows laptop and a set of
  cloud runners. The generalisation claims are untested.
- **The oracle ends where the reference ends** (Section 5.3, rule 4). Past it, the design falls back to progress, the
  weakness of rehosting.
- **Measurement noise.** The recentering note records that load on this Mac moves timings by up to a factor of two.
  Speed claims need paired runs on a quiet machine, or counts in place of seconds.
- **The metaphor's limit.** The mould searches for paths in a fixed graph. The frontier's graph changes as it learns:
  a new answer opens code that was not reachable before. Bonifaci et al.'s convergence result applies to fixed graphs.
- **Transparency.** The guest can detect the layer if it tries [Garfinkel 2007]. This guest does not appear to. Games
  with anti-tamper code will.

## 9. Related work

- **Hand-built compatibility layers.** Wine implements the Windows API by hand and by documentation, with its own
  per-DLL conformance tests. CrossOver and Proton build on Wine. Rosetta 2, FEX-Emu and box64 translate instructions
  and leave the operating system to the host or to Wine. None grows from observation of a guest.
- **Firmware rehosting.** HALucinator replaces hardware-abstraction functions with high-level models
  [Clements 2020]. Later work infers peripheral behaviour from the firmware's own accesses. These systems are the
  closest relatives of the frontier. Their oracle is mostly progress and coverage, not a reference system.
- **Specification mining and invariant inference.** Ammons, Bodík and Larus mine temporal API rules from traces
  [Ammons 2002]. Daikon infers likely invariants from executions [Ernst 2001]. Both learn passively, from observed
  executions, and both carry the limits of Section 4.1.
- **Model learning.** L* and its successors learn state machines of black-box implementations by queries
  [Angluin 1987; Vaandrager 2017]. Applied to network protocols and embedded control software, they have found bugs by
  learning and then inspecting the model. The frontier is model learning at a scale no published learner has tried.
- **Synthesis from examples.** Counterexample-guided inductive synthesis alternates between proposing a program and
  checking it against a specification, keeping the counterexamples [Solar-Lezama 2006]. The frontier's
  candidate-and-prune step has this shape, with Windows as the checker.
- **Record, replay and snapshots.** DoublePlay [Veeraraghavan 2011] for epoch-parallel execution; Nyx
  [Schumilo 2021] and Firecracker [Agache 2020] for snapshot speed.

## 10. References

- Agache, A., et al. (2020). Firecracker: lightweight virtualization for serverless applications. *NSDI 2020*.
- Alim, K., Amselem, G., Peaudecerf, F., Brenner, M. P., Pringle, A. (2013). Random network peristalsis in *Physarum
  polycephalum* organizes fluid flows across an individual. *PNAS* 110(33):13306–13311.
- Ammons, G., Bodík, R., Larus, J. R. (2002). Mining specifications. *POPL 2002*.
- Angluin, D. (1987). Learning regular sets from queries and counterexamples. *Information and Computation*
  75(2):87–106.
- Angluin, D. (1988). Queries and concept learning. *Machine Learning* 2(4):319–342.
- Auer, P., Cesa-Bianchi, N., Fischer, P. (2002). Finite-time analysis of the multiarmed bandit problem. *Machine
  Learning* 47:235–256.
- Bala, V., Duesterwald, E., Banerjia, S. (2000). Dynamo: a transparent dynamic optimization system. *PLDI 2000*.
- Bonifaci, V., Mehlhorn, K., Varma, G. (2012). Physarum can compute shortest paths. *Journal of Theoretical Biology*
  309:121–133.
- Chow, T. S. (1978). Testing software design modeled by finite-state machines. *IEEE Transactions on Software
  Engineering* SE-4(3):178–187.
- Clements, A. A., Gustafson, E., Scharnowski, T., et al. (2020). HALucinator: firmware re-hosting through
  abstraction layer emulation. *USENIX Security 2020*, 1201–1218.
  https://www.usenix.org/conference/usenixsecurity20/presentation/clements
- Ernst, M. D., Cockrell, J., Griswold, W. G., Notkin, D. (2001). Dynamically discovering likely program invariants
  to support program evolution. *IEEE Transactions on Software Engineering* 27(2):99–123.
- Firecracker documentation: snapshot support.
  https://github.com/firecracker-microvm/firecracker/blob/master/docs/snapshotting/snapshot-support.md
- Fossilize, Valve Software. https://github.com/ValveSoftware/Fossilize
- Garfinkel, T., Adams, K., Warfield, A., Franklin, J. (2007). Compatibility is not transparency: VMM detection myths
  and realities. *HotOS XI*.
- Gittins, J. C. (1979). Bandit processes and dynamic allocation indices. *Journal of the Royal Statistical Society,
  Series B* 41(2):148–177.
- Gold, E. M. (1967). Language identification in the limit. *Information and Control* 10(5):447–474.
- Gold, E. M. (1978). Complexity of automaton identification from given data. *Information and Control*
  37(3):302–320.
- Kocsis, L., Szepesvári, C. (2006). Bandit based Monte-Carlo planning. *ECML 2006*.
- Microsoft. Time Travel Debugging: TTD.exe command-line utility.
  https://learn.microsoft.com/en-us/windows-hardware/drivers/debuggercmds/time-travel-debugging-ttd-exe-command-line-util
- Mokhov, A., Mitchell, N., Peyton Jones, S. (2018). Build systems à la carte. *Proceedings of the ACM on Programming
  Languages* 2(ICFP):79.
- Nakagaki, T., Yamada, H., Tóth, Á. (2000). Maze-solving by an amoeboid organism. *Nature* 407:470.
- Reid, C. R., Latty, T., Dussutour, A., Beekman, M. (2012). Slime mold uses an externalized spatial "memory" to
  navigate in complex environments. *PNAS* 109(43):17490–17494. https://pmc.ncbi.nlm.nih.gov/articles/3491460
- Reid, C. R., MacDonald, H., Mann, R. P., Marshall, J. A. R., Latty, T., Garnier, S. (2016). Decision-making without
  a brain: how an amoeboid organism solves the two-armed bandit. *Journal of the Royal Society Interface*
  13:20160030.
- Schumilo, S., Aschermann, C., Abbasi, A., Wörner, S., Holz, T. (2021). Nyx: greybox hypervisor fuzzing using fast
  snapshots and affine types. *USENIX Security 2021*, 2597–2614.
  https://www.usenix.org/conference/usenixsecurity21/presentation/schumilo
- Solar-Lezama, A., Tancau, L., Bodík, R., Seshia, S., Saraswat, V. (2006). Combinatorial sketching for finite
  programs. *ASPLOS 2006*.
- Taylor, P. D., Jonker, L. B. (1978). Evolutionary stable strategies and game dynamics. *Mathematical Biosciences*
  40:145–156.
- Tero, A., Kobayashi, R., Nakagaki, T. (2007). A mathematical model for adaptive transport network in path finding by
  true slime mold. *Journal of Theoretical Biology* 244(4):553–564.
- Tero, A., Takagi, S., Saigusa, T., et al. (2010). Rules for biologically inspired adaptive network design.
  *Science* 327(5964):439–442. https://ir.lib.hiroshima-u.ac.jp/00029243?locale=en
- Vaandrager, F. (2017). Model learning. *Communications of the ACM* 60(2):86–95.
- Veeraraghavan, K., Lee, D., Wester, B., et al. (2011). DoublePlay: parallelizing sequential logging and replay.
  *ASPLOS 2011*.
- Zeller, A., Hildebrandt, R. (2002). Simplifying and isolating failure-inducing input. *IEEE Transactions on
  Software Engineering* 28(2):183–200.

## Appendix A. Status of each claim

| Claim | Status |
|---|---|
| Table 1 figures | Measured; repository records and commits named in the table |
| Laptop instruction counts and frame rate | Measured; laptop run lap-20261014T041748433Z-73c66e6a-01, as reported in `docs/recentering-2026-10-14.md` |
| 18× speed-up, 4.5-hour replay, 50-frame ceiling | Derived from the measured rates above; assume the play-phase rate holds over 15 minutes |
| Quadratic rerun cost | Derived under even spacing of stops; $n = 100$ is illustrative, not measured |
| *Physarum* findings | Published experiments, cited; Y-maze counts verified against the PMC text of Reid 2012 |
| Algorithms' guarantees | As stated in the cited papers; this paper proves nothing new |
| Snapshots by `MAP_PRIVATE` on Hypervisor.framework | Proposed; Firecracker's mechanism is documented for KVM, not tested on macOS here |
| Fork-and-prune, growth measure, early cutoff by call replay | Proposed; untested |
| Bibliographic details not opened during writing | Page and volume numbers for Alim 2013, Ernst 2001, Taylor 1978, Tero 2007 and Solar-Lezama 2006 come from memory and need checking before submission |
