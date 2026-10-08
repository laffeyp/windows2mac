# Windows-to-macOS Binary Porting System
## Research Direction v0.2

Date: October 3, 2026  
Status: research direction  
Previous version: `windows_to_macos_research_direction_v0_1.md`

This is a self-contained revision. The architecture, contracts, examples, and acceptance rules below define the proposed system. External references support platform facts and research precedents; they do not imply that an existing implementation provides the whole system.

## 1. Goal and working premise

Build a system that obtains the Windows distribution of a Steam game from the Mac command line, prepares it automatically, and launches it through a native Mac application. The original Windows binaries are the input. Developer source code, debug symbols, and LLVM bitcode are optional information, never required inputs.

The working premise is that complete behavioral specifications can be turned into correct implementations by the project's own generation and automated verification system. The compiler, runtime, graphics backend, probes, test infrastructure, and distribution integration may all be built independently. Research concerns the contracts, representations, composition rules, and optimization objectives that fully specify the result.

Establishing the execution contract is part of the project. Documentation, machine models, controlled Windows probes, and application observations feed a versioned contract-construction system. Correct generation applies to an accepted specification; the evidence for that specification remains a separate, inspectable artifact.

The user-facing operation is one command. Installation, compilation, persistent state, dependency management, session services, and backend management are internal responsibilities. Wine, Rosetta, and D3DMetal are not architectural dependencies of this proposal.

Illustrative interface for the proposed system:

```text
w2m steam run <appid>
w2m steam install <appid>
w2m run /path/to/Game.exe
```

These are proposed commands, not commands provided by an existing implementation. Normal account authentication remains part of the service interaction. The user does not configure a compatibility environment or manage a separate Windows installation.

The full goal includes acquisition and Steam integration. A selected game or reduced test profile can exercise part of the system without redefining that goal. This revision does not choose a title, graphics generation, or CPU backend as the universal starting architecture.

## 2. Central architecture: specialize the execution contract

The system is a specification-driven binary porting compiler.

Its input is a Windows application together with a versioned definition of the machine and system behavior that application can observe. Its output is an implementation specialized for the application and the destination Mac.

```text
Documentation + machine models + reference probes + application evidence
                                  |
                                  v
                   versioned execution contracts
                                  |
Windows distribution + destination profile + external-service interfaces
                                  |
                                  v
                    application-specializing compiler
                                  |
                                  v
       native CPU code + native GPU code + required runtime services
```

There are two linked compiler systems. The specification compiler produces executable reference models, test generators, trace checkers, and obligations for valid transformations. The application compiler consumes these contracts and produces native code, resource plans, and a runtime specialized to each application.

The shared contract connects them. A compiler optimization and a runtime replacement are accepted against the same definition of behavior.

The contract-construction pipeline establishes and revises those definitions from evidence. The generation-and-verification harness coordinates probes, model exploration, candidate production, comparison, and failure reduction. Contract construction, specification compilation, and application compilation can advance together against explicit contract versions.

The general operation is partial evaluation: fix the application, destination, and stable installation facts; compile away work that no longer depends on live inputs; retain efficient implementations for the remaining operations. This is the proposed organizing principle, not a claim that an existing project already implements the full design.

For example, an object operation can specialize to a known object type and sharing scope. A stable configuration lookup can become a constant under a declared validity condition. A graphics operation can become part of a precomputed resource plan. The complete observable contract remains the authority while general-purpose dispatch disappears where specialization justifies it.

## 3. Define correctness over observable execution

Correctness is a relation between executions, rather than equality between host implementation details.

The target may use different physical registers, stack frames, kernel objects, GPU layouts, and scheduling machinery. The observations exposed to the application must conform to its Windows execution contract. The distinction between internal representation and observable behavior is also central to verified compiler work such as CompCert.[^compcert]

A useful contract has the following form:

```text
Given equivalent initial application state and external inputs:

- Target observations are permitted by the source contract.
- Deterministic observations match exactly.
- Required progress, completion, and interactions are preserved.
- Nondeterministic choices stay within their declared rules.
- Address and object-identity mappings preserve application-visible relations.
```

A trace includes returns, errors, memory effects, faults, synchronization, callbacks, process events, file effects, graphics submissions, and external interactions. It includes intermediate effects that another thread or process can observe, not just a final screenshot or exit code.

Test comparison uses explicitly declared relations:

| Relation | Purpose |
|---|---|
| Exact value or bits | Defined integer results, serialized bytes, error codes, precisely specified numerical results. |
| State relation | Equivalent objects and address spaces represented differently on the host. |
| Allowed outcome set | Documented nondeterministic results and selections. |
| Partial order | Required ordering among effects without requiring one particular physical schedule. |
| Numerical envelope | Bounds permitted by the selected graphics or numerical contract. |
| Progress obligation | Completion, wakeup, or eventual effects under declared environment and scheduling assumptions. |

A reference Windows execution supplies evidence and a valid witness. The contract defines the acceptable set. Capturing one schedule does not make that schedule the only acceptable result.

### 3.1 Clocks, elapsed time, and controlled replay

Time is an explicit input and observable effect. Each clock contract states its units, frequency, precision, wraparound, ordering properties, cross-thread relationships, and relationship to timers and waits. `QueryPerformanceCounter`, `GetTickCount`, and instruction-level counter observations receive distinct contracts.

Monotonicity is applied only where the selected clock defines it. Microsoft documents that the 32-bit `GetTickCount` result wraps to zero after approximately 49.7 days; a universal increasing-value assertion would reject that prescribed behavior.[^gettickcount]

A controlled replay can inject equivalent clock observations, input events, and service responses into the reference and target. Deterministic effects under those inputs still match exactly. A live run is checked against its clock and progress contracts rather than the timestamps of an unrelated physical run.

Trace normalization preserves the relationships that influence program behavior. It cannot erase a timeout, reorder a callback, or discard a clock comparison merely to make two traces agree. Performance measurements are recorded separately from semantic acceptance.

## 4. The input and output are application packages

### 4.1 Acquisition and launch

For Steam, the input unit is the selected application distribution: executable images, libraries, assets, launch configuration, dependencies, and the service interactions required by the application. PE files encode machine types, sections, imports, relocations, and other loading information; the frontend starts there rather than requiring source reconstruction.[^pe]

Command-line acquisition is an established separate capability. DepotDownloader documents macOS installation and a Windows platform selector. Its relevance is that acquisition can be specified and tested independently of execution.[^depot]

Steam integration includes a launch/session contract. Valve documents that Steamworks initialization depends on the Steam client, application identity, user context, and account license state.[^steam]

The proposed system owns authenticated acquisition, dependency resolution, preparation, launch context, and the bridge to legitimate Steam services. External authentication and service decisions remain real external inputs; the translator preserves their protocol rather than manufacturing successful results.

### 4.2 The versioned Steam bridge

Valve's Proton `lsteamclient` provides a concrete research precedent for Windows-to-native Steam interface adapters. Its wrapper generator explicitly handles SDK versions, ABI variants, callbacks, and data conversions.[^lsteamclient]

The proposed Mac bridge uses a versioned interface contract covering method signatures, calling conventions, structure layouts, object identity and lifetime, callback execution, asynchronous results, paths, and launch/session state. The project generates adapters and tests from those definitions.

The Windows-facing interface and the actual Mac endpoint are specified separately. The Proton implementation establishes a useful adapter pattern; availability and behavior of each Mac-side interface remain destination-contract evidence. The bridge is a first-class subsystem in the product architecture.

### 4.3 Output and execution modes

The output unit is a native Mac application bundle. It can contain native executable modules, helpers, assets, Metal archives, state-reconstruction metadata, and required runtime components. Original PE bytes may be retained where the application contract includes inspection of its images.

Two execution modes remain explicit:

| Mode | Execution environment |
|---|---|
| Native-process backend | Translated ARM64 code and generated Windows behavior run in macOS processes, with native host interfaces. |
| Managed VM backend | A native Mac launcher manages translated ARM64 code and a compact guest runtime inside a VM, with explicit host-service interfaces. |

Both modes execute translated ARM64 instructions on the destination CPU. Only the first is direct native-process execution under the macOS process environment. A managed guest does not become a macOS process merely because its CPU instructions are native.

A single statically linked file is not the governing requirement. The governing requirement is automatic preparation and launch without compatibility setup or a separately managed Windows installation. Bundle metadata and diagnostic records identify the actual backend used.

### 4.4 Protected executables, kernel behavior, and service acceptance

Protected executables and anti-cheat are explicit compatibility categories. Their required behavior is divided into three contracts: executable behavior, guest operating-system behavior, and external service acceptance.

Executable behavior includes code discovery, generated or changed code, inspection of code and process state, and time observations. Required kernel interfaces and driver interactions belong to the guest operating-system contract. Authentication, entitlement, and service-side acceptance belong to the external-service contract.

These categories are recorded per application and version. A product name alone does not establish an instruction sequence, required kernel interface, or acceptance rule. Retaining original PE bytes addresses only the observations that actually read those bytes; other observations remain separately specified.

Valve's Proton documentation illustrates the distinction: EAC and BattlEye integration includes provider- or title-specific enablement, while kernel-space anti-cheat support is described separately.[^proton-services] Existing Proton support is evidence about that implementation and integration, not a universal boundary for this architecture.

This research direction does not impose a DRM-free-only scope or exclude a protection family by name. A test profile declares the categories it covers. External decisions and credentials remain real service inputs throughout translation, testing, and launch.

## 5. CPU translation and intermediate representations

The proposed frontend lifts machine instructions into an explicit machine-semantics representation. Binary lifting to LLVM bitcode has established precedent in Remill, which covers x86 and amd64 among its input architectures.[^remill]

The project's representation should make arithmetic, flags, byte-addressed memory, atomics, faults, and system effects explicit. It should not infer source-language rules that authorize changes to the behavior of the shipped binary. LLVM's concurrency documentation is relevant because its memory operations carry semantics that differ from an unrestricted model of machine-level shared memory.[^llvm]

A useful separation is:

```text
source machine semantics
        |
        v
application and system effects
        |
        v
specialized native operations
```

The machine layer preserves the meaning of the original executable. The effect layer exposes operations such as waiting on an object, publishing a buffer, opening a file, or changing page protection. The native layer selects an implementation for those effects on the destination.

The compiler can remove source register bookkeeping, inline known calls, simplify dispatch, and choose a native internal calling convention where the execution contract permits it. Boundary adapters preserve calling conventions for callbacks, exported functions, dynamically loaded modules, and other observable interfaces.

Optimized code must retain sufficient reconstruction information for application-visible exceptions, thread contexts, and code-address observations. Internal layout freedom is conditional on preserving those observations.

### 5.1 Code discovery, identity, and validity

The source model distinguishes shipped bytes, discovered instruction targets, and the code version valid at a given execution point. Bytes can already exist in a package while the target of a control transfer becomes known only during execution. Rev.ng's code-discovery documentation provides a concrete reference for the interaction between lifting code and recovering indirect targets.[^revng]

The execution invariant is:

```text
Every transfer into guest code resolves to an implementation valid for
that guest address, the current code state, and the execution contract,
or produces the fault prescribed by the guest contract.
```

Native artifacts carry the source ranges, mapping identity, code version, and assumptions on which they depend. Code modification, remapping, and address reuse invalidate the affected assumptions. Entering a reused guest address must not revive a stale translation.

The code-publication contract defines when changed instructions become visible to other threads and which translations remain valid while code changes. Publication, invalidation, exception reconstruction, and concurrent execution are checked together. The exact synchronization mechanism is a backend implementation choice governed by that contract.

This covers statically discovered code, shipped code reached later, newly loaded modules, generated code, and code changed during execution. Code discovery selects work for the compiler; source instruction semantics determine the meaning of that work.

## 6. Put work at the earliest valid stage

The staging rule is: perform an operation as soon as all of its required inputs are available and stable.

Installation-time work includes translating known shipped CPU code, resolving known imports, preparing known graphics pipelines, creating stable path mappings, selecting target implementations, and specializing runtime code to the application.

Update-time work invalidates and regenerates artifacts affected by changed binaries, assets, contracts, compiler versions, or destination capabilities.

Execution-time work includes input, mutable state, synchronization, network responses, unresolved code targets, and code or pipeline definitions whose required inputs become available during execution. Complete knowledge of an operation's semantics does not make its future inputs constant.

AOT and on-demand compilation consume the same machine contract and produce artifacts subject to the same acceptance rules. AOT work does not require a previous observed execution. On-demand compilation is a specified execution path for resolving outstanding inputs, with the same correctness requirements as installation-time compilation.

The generated system can itself implement a fully specified dynamic compiler. Where an application contains a code generator whose semantics are known, the project can investigate retargeting that generator to emit native code directly. Other generated Windows machine code follows the source-code validity and translation contract.

Execution observations can guide scheduling and optimization. They do not turn unobserved paths into invalid paths or redefine the application's behavior. AOT remains a principal optimization stage rather than being restricted to a cache populated only by earlier execution.

Rosetta provides a useful comparison for staging: Apple's platform-security documentation describes both ahead-of-time and just-in-time translation. AOT is therefore a component of the design, not by itself the differentiator.[^rosetta]

## 7. Native implementations of Windows system behavior

A Win32 or NT operation is defined by its effects. The backend implements those effects and can inline or specialize the implementation.

For threads, the contract includes creation, suspended state, identity, stack behavior, completion, and handle lifetime. Microsoft documents that a created thread can start suspended and that its thread object becomes signaled on termination and remains until the relevant lifetime conditions are satisfied.[^thread]

A native thread can be the execution vehicle. A generated object model preserves the Windows-visible state. Host work queues are suitable for the compiler's own background tasks; replacing a Windows thread with a queue task requires a transformation that preserves all relevant thread observations.

For synchronization, local operations should remain local where their contracts permit it. Shared-object coordination should be organized around the actual objects and processes involved. A universal serialization point is not a requirement of the design.

For storage, stable namespace decisions can be compiled. Mutable file and registry behavior remains live. Registry changes can be observed through notifications, so a registry model cannot generally be replaced by installation-time constants.[^registry]

The optimization targets are unnecessary lookup, dispatch, conversion, and coordination. Correct implementations still perform the reads, writes, waits, and notifications that the application requests.

The contract inventory follows the application's required behavior and dependency set. Import scans and execution traces help discover it. They are supplemented by machine effects, dynamic loading, callback paths, exception behavior, shared state, and cross-component interactions. Entry-point counts are a coverage view, not the unit of semantic completeness.

Windowing, input, audio, networking, process loading, and presentation receive the same state-and-effects treatment as threads and storage. Their host adapters have explicit ownership, ordering, completion, and failure rules. The VM backend additionally specifies the transport across the guest/host boundary; the native-process backend specifies its in-process and cross-process host interactions.

## 8. Page geometry is a memory-semantics problem

For a source profile with 4 KB pages and a native host using 16 KB pages, keep three concepts separate: application data layout, guest virtual page semantics, and host physical backing.

Windows page operations expose protection and fault behavior. VirtualProtect operates on affected pages, including every page touched by a range, and supports one-shot guard-page behavior.[^virtualprotect]

Consider two contiguous guest pages:

```text
[A, A + 4096)          readable and writable
[A + 4096, A + 8192)   inaccessible
```

The contract must preserve successful accesses in the first interval and the prescribed fault behavior in the second. It must also preserve address arithmetic and any application-visible page queries. Expanding every struct or allocation to 16 KB does not establish those properties.

The proposed memory subsystem therefore maintains guest page metadata separately from host allocation. Native-process lowering can combine direct mappings for compatible regions with generated enforcement where guest protections differ. It can remove checks where their redundancy is established by the contract and compiler analysis. A host backend may remap addresses internally, but guest-visible address relationships must remain valid.

### 8.1 VM capability evidence

Apple documents configurable intermediate-physical-address granularity down to 4 KB; `hv_vm_config_set_ipa_granule` is available from macOS 26.[^hypervisor-updates][^hypervisor-granule] This capability belongs to the VM address-translation layer. Guest virtual-page behavior and host backing remain separately represented.

Apple also documents guest Linux support for changing `ACTLR.TSOEN`, selecting the x86-64 TSO memory model, and preserving the setting across thread switches.[^apple-tso] This is concrete evidence for evaluating a VM-backed implementation of the source memory contract. The cited mechanism does not make Rosetta a project dependency.

A compact custom guest still has to establish the corresponding capability on its selected framework, OS, and hardware combination. Its capability probe records activation, context-switch behavior, and conformance results. The reviewed material does not establish a public native-process TSO control; that remains an open destination-interface question rather than an asserted absence.

### 8.2 Backend comparison

The combined page-granularity and memory-ordering capabilities make the VM backend a leading candidate for evaluation. They do not by themselves select the complete execution architecture.

Both backends implement the same source memory and execution contract. The comparison includes ordering enforcement, page protection, faults, exceptions, thread state, code publication, graphics and audio communication, file access, input delivery, resource sharing, and presentation. It records launch behavior and frame-time effects as well as CPU costs.

A native-process backend is measured after permitted specialization removes redundant checks and ordering operations. A VM backend is measured with its required guest/host interfaces in place. Neither is assigned an assumed per-access or zero-crossing cost in advance.

Backend selection is a recorded decision tied to a target capability profile, conformance results, and the declared cost objective. Both remain architectural candidates in this revision. A capability observation about one development machine is not a universal host requirement.

## 9. Concurrency is specified as legal execution

The concurrency contract combines source instruction ordering, atomic operations, synchronization objects, asynchronous callbacks, exceptions, and process interactions.

Target executions must be permitted by the source binary's contract and satisfy its required progress and choice obligations. It need not reproduce the physical schedule of one Windows recording. Race behavior is described at the machine-contract level; the compiler does not gain permission to erase it merely because reconstructed source code would have different language rules.

The target lowering chooses ordinary memory operations, acquire/release operations, fences, atomic sequences, and runtime coordination according to that contract. Arm documents that acquire/release instructions impose ordering different from full barriers and can avoid some costs when their ordering is sufficient.[^arm-ordering]

Instruction synchronization and data-memory ordering must remain distinct operations. ISB is not a general substitute for a data-memory barrier.[^arm-barriers]

The automated specification system generates small concurrency programs, explores their legal outcomes in the source and target models, and checks that the translation preserves the required relation. Physical execution then checks the implementation against those models. Herdtools7 is an existing research reference for generating litmus tests, simulating weak-memory models, and executing tests on architectures including x86 and AArch64.[^herd]

Optimization becomes a constrained choice among valid lowerings. Removing a fence or moving a memory operation requires an explicit justification tied to the relevant contract.

Hardware ordering support is one backend mechanism under this contract. Compiler transformations must still preserve the required ordering and atomicity. Guest atomics, faults, code publication, and host-service synchronization retain their own obligations regardless of how ordinary memory ordering is implemented.

The source and target test records distinguish model-permitted outcomes from outcomes actually observed on hardware. A missing hardware observation does not, by itself, make an outcome forbidden. The checker evaluates the declared relation and progress conditions rather than convergence on one recorded thread schedule.

## 10. Graphics: shader semantics, command semantics, and resource layout

The graphics subsystem has three connected responsibilities. Shader translation preserves the meaning of GPU programs. Command translation preserves state, bindings, execution dependencies, and presentation behavior. Resource planning maps buffers, textures, heaps, and sharing to the destination.

Direct-to-Metal translation has public precedent: DXMT describes a Metal-based implementation of Direct3D 11 and 10 used with Wine.[^dxmt] It is a reference for the graphics translation boundary, not a standalone replacement for the whole execution environment.

The proposed pipeline is:

```text
DXBC / DXIL / available HLSL
        + graphics state and binding descriptions
                         |
                         v
             GPU semantic representation
                         |
                         v
       Metal code + resource plan + pipeline archives
```

### 10.1 Shader-tool boundaries

Apple's Metal Shader Converter provides a command-line tool and a library that accept DXIL and produce a Metal library.[^metal-converter] It is an established conversion path relevant to this research.

The project owns the shader contract, input representations, resource-binding contract, and conformance checker. Metal Shader Converter is recorded as a research reference and a candidate comparison implementation. Selecting it as a production component would be a separate dependency decision. Its existence does not collapse shader conversion, command translation, and resource planning into one subsystem.

DXBC, DXIL, and available HLSL remain distinct input paths. The referenced DXIL conversion path does not establish the project's DXBC frontend. Each path is accepted against its shader and graphics profile, regardless of which conversion implementation is selected.

### 10.2 Compilation, numerical results, and resources

Precompilation covers both shader code and the pipeline descriptions required to produce device-specific programs. Apple documents binary archives generated from Metal IR and pipeline-state descriptions to avoid runtime shader compilation.[^metal-archives]

The artifact cache is keyed by shader and pipeline inputs, compiler and contract versions, and the relevant target configuration. Definitions arriving during execution use the same specified pipeline at the stage where those inputs become available.

Graphics conformance distinguishes exact results from results with specified numerical latitude. Microsoft's Direct3D floating-point rules define precision requirements and permitted variation for several operations.[^d3d-float] A test contract should enforce those rules, rather than silently replacing them with equality to whichever physical GPU produced one reference image. A separately selected GPU-specific compatibility profile may impose a more exact target.

Metal supports heap resource aliasing; its documentation explicitly describes resources sharing an allocation.[^metal-heaps] The research target is correspondence between source and destination lifetime, aliasing, alignment, and synchronization rules.

Unified memory is an optimization resource, not a command to place every allocation in the same access mode. Apple distinguishes shared, private, and memoryless resource storage, and documents different suitable uses.[^metal-storage] Direct3D 12 also exposes UMA and cache-coherent UMA capabilities.[^d3d-uma]

The planner should remove copies whose distinct-storage effects are unobservable, retain required synchronization, and select GPU-private or tile-local storage where it best implements the contract. CPU and GPU access to shared physical storage does not require a blanket claim that all pointer values or resource layouts are identical.

A graphics reference run records the source GPU, driver, API profile, shader inputs, pipeline state, and resource initial state. Exact comparisons apply where the contract fixes the result. Permitted numerical variation is named by the relevant rule; it is not an arbitrary tolerance chosen after a mismatch. A separately selected GPU-specific profile can impose more exact behavior.

The native-process and VM backends use the same graphics behavior contract. A VM's host-side Metal connection additionally specifies resource ownership, visibility, submission order, completion, and presentation across the guest/host boundary. Shared physical storage is an optimization only when those relations remain valid.

## 11. Contract construction, executable specifications, and automatic verification

### 11.1 Establish the contract from explicit evidence

Contract construction is a first-class subsystem. It turns documentation, machine models, controlled reference probes, and application evidence into versioned behavioral definitions.

The evidence record distinguishes three categories:

| Evidence category | What it establishes |
|---|---|
| Documented requirement | A stated platform rule, with its source, version, and applicability. |
| Observed implementation behavior | A result produced by a named probe on a recorded reference profile. |
| Hypothesis | A proposed explanation or rule, with the observations or model checks needed to distinguish it from alternatives. |

An accepted contract records the rule, its applicability, its evidence, and its unresolved questions. Documentation and observation retain their identities. A successful probe is a witness to an execution; repeated observations do not silently become a universal requirement.

Undocumented behavior can be part of a source profile when the evidence establishes its role. Microsoft's PEB documentation, for example, describes an internal structure with reserved fields and a layout that can change.[^peb] The project therefore records build-specific observations separately from rules claimed across Windows versions.

A reference profile includes the Windows build, CPU identity and exposed features, relevant library versions, GPU and driver where applicable, application and probe hashes, initial state, and controlled external inputs. The resulting evidence can be replayed, compared, and traced to the contract it supports.

Native x86-64 Windows is the physical reference for native x86 instruction, ordering, and fault observations. Other environments have declared evidence roles. Microsoft documents that Windows on Arm translates x86 and x64 application instructions; it is consequently a different execution profile, not a transparent substitute for native x86 reference measurements.[^windows-arm]

The probe pipeline creates small discriminating programs, varies relevant initial states and schedules, records effects, and reduces discrepancies. Model exploration determines permitted executions under the proposed rules. Physical runs test those rules against the selected implementation. Coverage records distinguish explored model states, observed executions, and accepted contract obligations.

### 11.2 Compile the behavioral definition

Each contract is a machine-readable object with a stable identifier and version. Its fields include state, operations, preconditions, transitions, observable effects, permissible nondeterminism, ordering, faults, progress, host representation relations, and allowed optimizations. Evidence references and applicability profiles accompany the behavioral definition.

A contract compiler derives model tests, operation-sequence generators, concurrency explorations, trace normalization, differential comparisons, and regression cases. The generator and checker have separate responsibilities: generated candidates do not redefine the acceptance condition.

For example, a non-signaled auto-reset event with two pending waiters and one signal is governed by a selection rule, not a fixed winner. Microsoft specifies that one waiting thread is released and that FIFO ordering must not be assumed.[^event]

An illustrative test fragment is:

```yaml
contract: windows.event.auto_reset.single_signal
initial:
  signaled: false
  pending_waiters: [a, b]
execution:
  signal_count: 1
  resets: 0
  waiter_cancellation: false
observation:
  phase: after_signal_consumption
allowed_completed_wait_sets:
  - [a]
  - [b]
forbidden:
  - both_waits_complete_from_the_single_signal
postcondition:
  signaled_after_consumption: false
```

The surrounding contract supplies scheduler assumptions, object lifetime, error behavior, memory ordering, and observation points. The fragment is not the entire event-object specification and does not promise an arbitrary bounded wakeup time.

The same contract is checked through the reference model, the Windows probe, the generated runtime, and compiled programs using that runtime. This makes runtime composition part of verification rather than assuming that independently correct API calls automatically compose correctly.

### 11.3 Structured discrepancies and distinct repair paths

A failure produces a minimized program, initial state, relevant event trace, contract identifiers, reference and destination profiles, and mismatch classification. The report identifies the first relevant divergence and retains enough context to reproduce the failure. The generation system receives a structured discrepancy rather than an undifferentiated statement that a game failed.

Implementation repair and contract revision are distinct operations. An implementation candidate is repaired against an unchanged acceptance condition. Evidence that the definition is wrong or incomplete enters a contract-revision path, producing a separately accepted version with explicit rationale and dependency invalidation.

Both paths can be automated. Neither permits an implementation candidate to change its expected output merely to obtain a passing result. A changed contract causes affected implementations and evidence to be re-evaluated under the new version.

### 11.4 Substrate as the orchestration harness

Substrate is the proposed harness for coordinating contract discovery, generation, verification, and regression work. Its public design provides typed producers and events coordinated through an append-only run log, including writer/checker and replay-oriented workflows.[^substrate] The w2m integration described here is a proposed topology, not a claim that the compiler, Windows probes, or Metal runtime already exist in Substrate.

```text
contract evidence -> candidate definition -> model and reference checks
accepted contract -> implementation candidate -> conformance checks
failed check      -> minimized discrepancy -> implementation repair
contract mismatch -> evidence-bearing revision -> dependency revalidation
accepted artifact -> application and product checks -> recorded result
```

The harness starts and coordinates workers for Windows probes, model exploration, compilation, target execution, trace comparison, and reduction. Each worker emits typed results associated with input hashes, contract versions, and environment profiles. The record preserves why work started, which evidence it consumed, and which artifact or discrepancy it produced.

The append-only orchestration record is distinct from the guest program's execution order. Recording evidence in one log does not serialize the game or declare that concurrent guest effects have one total semantic order. Trace records retain the partial-order and observation rules from section 3.

The runtime workers own instruction execution, operating-system behavior, and graphics behavior. Substrate owns their coordination. Exact bindings to local development-kit APIs remain implementation details to be specified against the actual kit version; this document does not depend on unverified local API names.

## 12. Composition boundaries and test levels

The specifications must cover the seams between components. A CPU memory effect can become a GPU input; a page fault can become a Windows exception; a file completion can signal an event; thread exit can satisfy another thread's wait. The order and visibility of those effects are part of the contract.

A compact validation hierarchy is:

| Level | Subject |
|---|---|
| Machine | Instructions, flags, memory accesses, atomics, exceptions. |
| System | Handles, objects, loading, threads, files, registry, timing. |
| Composition | Concurrent operations, callbacks, shared mappings, CPU/GPU visibility. |
| Application | Executable graphs, assets, input, saves, networking, graphics, service sessions. |
| Product | Download, install, update, launch, close, relaunch, and removal from the command line. |

Component tests diagnose the mechanism. Application and product tests establish that the composed system fulfills the user-facing contract.

The reference model can be deliberately simple. The optimized implementation can differ radically in its internal structure. Both are checked against the same observable rules.

### 12.1 Application profiles and an end-to-end product slice

A selected game supplies a concrete distribution, dependency graph, execution paths, graphics profile, and service requirements. Its profile guides probes and contract coverage. One observed run is evidence about that profile, not the complete definition of the program.

A narrow execution experiment can qualify instructions, APIs, shaders, or a backend. A product slice includes the actual user operation: resolve the selected Steam distribution, obtain it under the user's account, prepare the application, establish its required Steam session, run it, preserve state, close, and relaunch it. Update and removal are checked under the same product lifecycle contract.

Service fixtures support controlled component tests. Product qualification also records behavior with the actual service interfaces required by the selected application. A successful local executable launch is distinguished from successful Steam acquisition and launch.

This revision selects no universal D3D11-first, VM-first, or DRM-free-only architecture. Test profiles can be narrower than the full goal, and every qualification result states that profile. The architecture retains acquisition, integration, translation, and runtime behavior as composable responsibilities from the outset.

### 12.2 Measure reference implementations without adopting them

Existing implementations can supply comparative runs, implementation ideas, and additional discrepancies. A comparison records the exact implementation version, application inputs, graphics settings, destination profile, and cold or warm artifact state. Repository activity dates are not a substitute for conformance or performance evidence.

Component comparisons explain mechanisms. Whole-application and product comparisons measure their combined effects. External implementations remain comparison subjects unless a separate dependency decision explicitly adopts one.

## 13. Performance as an explicit second objective

Correctness determines which implementations are admissible. Performance selects among them.

```text
Choose an implementation minimizing the declared cost objective,
subject to preservation of the execution contract.
```

Measure compilation on the critical path, API dispatch, interprocess coordination, memory checks, ordering operations, CPU/GPU waits, copied bytes, storage modes, frame-time distributions, launch latency, and artifact invalidation. Attribute each cost to a required application effect, required destination enforcement, or removable implementation work.

The compiler can optimize across the former Windows API boundary because the application and runtime semantics are available together. For example, it can specialize an object operation whose class and sharing scope are known, eliminate a repeated immutable lookup, or combine resource operations whose observable lifetime rules permit it.

The target is no avoidable translation work on steady execution paths, not an assumption that every correct implementation has the same cost. Runtime state can remain while compatibility dispatch disappears into native operations.

Implementation language is a separate choice. Rust can be a good host-runtime language, but guest machine semantics, foreign interfaces, and GPU behavior still have their own contracts. Changing language alone is not the architectural source of equivalence or speed.

Performance evidence distinguishes installation, cold launch, warm launch, sustained execution, code or pipeline discovery, and updates. The same application behavior and quality settings are held fixed in backend comparisons. Required destination enforcement is measured after optimization rather than assumed from the source-to-host mismatch alone.

A backend decision records the cost objective and the complete application path that was measured. Hardware TSO, page granularity, direct host calls, and shared-memory access are individual inputs to that decision. Their interaction with synchronization and presentation determines the resulting execution cost.

## 14. Artifact identity and reproducibility

Every generated artifact records its source hashes, source-contract version, target-contract version, compiler version, specialization facts, and verification evidence. Mutable inputs are not folded into permanent code without a validity condition and invalidation rule.

A dependency graph connects source content, contracts, native modules, graphics archives, and runtime components. Updates rebuild affected artifacts while preserving application state under its storage contract.

A reference run and a target run are reproducible with respect to their declared initial state and injected external events. Nondeterministic execution is recorded as choices constrained by the model, rather than mistaken for an unexplained test failure.

Evidence provenance connects accepted contracts to documentation snapshots, reference profiles, probes, model checks, and discrepancy resolutions. Contract revision invalidates the affected acceptance evidence as well as generated code. A candidate cannot continue to claim conformance to a replaced definition without revalidation.

Code artifacts additionally retain the mapping and code-validity information described in section 5. Runtime code changes invalidate the relevant generated artifacts under the publication contract. Persistent cache reuse requires the same validity conditions as a fresh compilation.

The target manifest records the selected execution backend, supported capabilities, host-service interfaces, and relevant graphics compiler configuration. A recorded development-host observation is evidence for that profile only. Replaying a harness record, re-running a controlled test, and reproducing application observations are named separately in test results.

## 15. Research anchors and resulting direction

The research draws on binary lifting, semantics-preserving compilation, partial evaluation, formal memory models, state-machine testing, and native GPU compilation. Remill informs instruction lifting; rev.ng informs code discovery; CompCert informs semantic preservation; Herdtools7 informs memory-model testing. Microsoft's specifications and Windows probes establish source-visible behavior. Apple's specifications and destination probes establish available host operations.

Proton's `lsteamclient` supplies a concrete reference for versioned interface adapters. DXMT supplies a direct Metal graphics-boundary reference within a Wine-based environment. Metal Shader Converter supplies a DXIL conversion reference. Substrate supplies the proposed orchestration substrate for the project's own workers and verification records.

These roles do not imply adoption of the referenced implementations. Production dependencies, backend selection, and the first application profile are separate recorded decisions. In particular, this revision leaves the native-process versus managed-VM choice open, treats direct native-process TSO access as unresolved, and does not assume that every Windows Steam interface has already been matched to a Mac endpoint.

The resulting project is an automatic binary-to-native porting system whose primary design asset is a versioned, executable Windows behavior contract with an explicit evidence history. A per-application compiler specializes that contract into native CPU code, native GPU code, and the required services. Acquisition, preparation, service setup, and launch are presented through one command. The implementation preserves the behavior the game observes while optimizing for the destination's actual execution capabilities.

## Revision record: v0.1 to v0.2

This revision incorporates the original research direction, the attached review dated October 3, 2026, and the subsequent assessment of that review.[^v01][^review] The existing fifteen-section structure is retained so the revised contracts remain easy to compare.

**Retained:** the full one-command Steam goal, correctness from complete specifications, observable-execution conformance, explicit nondeterminism, early staging of work, per-application specialization, backend-independent contracts, performance as an optimization objective, and reproducible artifacts.

**Made explicit:** evidence-based contract construction; clock-specific comparison; reference-hardware profiles; shipped code discovered at runtime; code identity, publication, and invalidation; the versioned Steam bridge; protected-executable and service boundaries; managed-VM versus native-process execution; hardware TSO evidence; Metal Shader Converter's possible role; Substrate orchestration; and distinct implementation-repair and contract-revision paths.

**Left open rather than silently selected:** CPU backend, first game, first graphics profile, direct native-process TSO access, Mac-side Steam interface coverage, and adoption of any external shader converter. A narrowed test profile does not change the product objective.

This is a research-direction document. The stated contracts and acceptance rules are design requirements, not a report of completed implementation or measured performance.

## Sources

The source list carries forward the platform and research references from v0.1. Additions concerning hardware TSO, VM granularity, Steam wrapper generation, Metal Shader Converter, clocks, reference profiles, and Substrate were checked against primary sources during this revision on October 3, 2026. Project-specific architecture and tests are design synthesis. The supplied review is input to the revision, not an independent authority for every claim it contains.

[^pe]: Microsoft, "PE Format." `https://learn.microsoft.com/en-us/windows/win32/debug/pe-format`
[^depot]: SteamRE, "DepotDownloader." `https://github.com/SteamRE/DepotDownloader`
[^steam]: Valve, "steam_api.h." `https://partner.steamgames.com/doc/api/steam_api`
[^compcert]: CompCert, "The CompCert verified compiler." `https://compcert.org/doc/index.html`
[^remill]: lifting-bits, "Remill." `https://github.com/lifting-bits/remill`
[^llvm]: LLVM, "LLVM Atomic Instructions and Concurrency Guide." `https://llvm.org/docs/Atomics.html`
[^rosetta]: Apple, "Rosetta 2 on a Mac with Apple silicon." `https://support.apple.com/guide/security/rosetta-2-on-a-mac-with-apple-silicon-secebb113be1/web`
[^thread]: Microsoft, "CreateThread function." `https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createthread`
[^registry]: Microsoft, "RegNotifyChangeKeyValue function." `https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regnotifychangekeyvalue`
[^virtualprotect]: Microsoft, "VirtualProtect function." `https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualprotect`
[^hypervisor-updates]: Apple, "Hypervisor updates." `https://developer.apple.com/documentation/updates/hypervisor`
[^hypervisor-granule]: Apple, "hv_vm_config_set_ipa_granule." `https://developer.apple.com/documentation/hypervisor/hv_vm_config_set_ipa_granule%28_%3A_%3A%29`
[^arm-ordering]: Arm, "Memory Systems, Ordering, and Barriers: Load-Acquire and Store-Release instructions." `https://support.arm.com/documentation/102336/0100/Load-Acquire-and-Store-Release-instructions`
[^arm-barriers]: Arm, "DMB, DSB, and ISB." `https://support.arm.com/documentation/dui0489/h/arm-and-thumb-instructions/dmb--dsb--and-isb`
[^herd]: Herd project, "Herdtools7." `https://github.com/herd/herdtools7`
[^dxmt]: 3Shain, "DXMT." `https://github.com/3Shain/dxmt`
[^metal-archives]: Apple, "Metal binary archives." `https://developer.apple.com/documentation/metal/metal-binary-archives`
[^d3d-float]: Microsoft, "Floating-point rules (Direct3D 11)." `https://learn.microsoft.com/en-us/windows/win32/direct3d11/floating-point-rules`
[^metal-heaps]: Apple, "MTLHeap." `https://developer.apple.com/documentation/metal/mtlheap`
[^metal-storage]: Apple, "Choosing a resource storage mode for Apple GPUs." `https://developer.apple.com/documentation/metal/choosing-a-resource-storage-mode-for-apple-gpus`
[^d3d-uma]: Microsoft, "D3D12_FEATURE_DATA_ARCHITECTURE1." `https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_feature_data_architecture1`
[^event]: Microsoft, "Event Objects (Synchronization)." `https://learn.microsoft.com/en-us/windows/win32/sync/event-objects`
[^v01]: Project source, "Windows-to-macOS Binary Porting System: Research Direction v0.1," October 3, 2026. Supplied file: `windows_to_macos_research_direction_v0_1.md`.
[^review]: Project source, "w2m review: Review - Windows-to-macOS Binary Porting System, Research Direction v0.1," reviewed October 3, 2026. Supplied file: `Pasted markdown(20261004-034101).md`.
[^gettickcount]: Microsoft, "GetTickCount function." `https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount`
[^lsteamclient]: Valve, Proton `lsteamclient`, wrapper generator on the `proton_11.0` branch. `https://github.com/ValveSoftware/Proton/blob/proton_11.0/lsteamclient/gen_wrapper.py`
[^proton-services]: Valve, "Steam Hardware and Proton," anti-cheat support and implementation boundaries. `https://partner.steamgames.com/doc/steamhardware/proton`
[^revng]: rev.ng, "The code discovery process." `https://docs.rev.ng/developer-manual/code-discovery/`
[^apple-tso]: Apple, "Accelerating the performance of Rosetta," guest TSO control and context switching. `https://developer.apple.com/documentation/virtualization/accelerating-the-performance-of-rosetta`
[^metal-converter]: Apple, "Metal shader converter." `https://developer.apple.com/metal/shader-converter/`
[^peb]: Microsoft, "PEB structure." `https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-peb`
[^windows-arm]: Microsoft, "How emulation works on Arm." `https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation`
[^substrate]: Peter Laffey, Substrate, public repository README and architecture description. `https://github.com/laffeyp/substrate`
