# Windows-to-macOS Binary Porting System
## Research Direction v0.1

Date: October 3, 2026

## 1. Goal and working premise

Build a system that obtains the Windows distribution of a Steam game from the Mac command line, prepares it automatically, and launches it as a native Mac application. The original Windows binaries are the input. Developer source code, debug symbols, and LLVM bitcode are optional information, never required inputs.

The working premise is that complete behavioral specifications can be turned into correct implementations by the project's own generation and automated verification system. The research therefore concerns the contracts, representations, composition rules, and optimization objectives that define the system. Compiler, runtime, graphics backend, test infrastructure, and distribution integration may all be built independently.

The user-facing operation is one command. Installation, compilation, persistent state, dependency management, and session services are internal responsibilities. Wine, Rosetta, and D3DMetal are not architectural dependencies of this proposal.

Illustrative interface for the proposed system:

```text
w2m steam run <appid>
w2m steam install <appid>
w2m run /path/to/Game.exe
```

These are proposed commands, not commands provided by an existing implementation.

## 2. Central architecture: specialize the execution contract

The system is a specification-driven binary porting compiler.

Its input is a Windows application together with a versioned definition of the machine and system behavior that application can observe. Its output is a native implementation specialized for the application and the destination Mac.

```text
Windows distribution
  + Windows execution contract
  + Apple hardware/macOS contract
  + declared external-service interfaces
                         |
                         v
           application-specializing compiler
                         |
                         v
native CPU code + native GPU code + required runtime services
```

There are two linked compiler systems. The specification compiler produces executable reference models, test generators, trace checkers, and obligations for valid transformations. The application compiler consumes these contracts and produces native code, resource plans, and a runtime specialized to each application.

The shared contract connects them. A compiler optimization and a runtime replacement are accepted against the same definition of behavior.

The general operation is partial evaluation: fix the application, destination, and stable installation facts; compile away work that no longer depends on live inputs; retain efficient implementations for the remaining operations. This is the proposed organizing principle, not a claim that an existing project already implements the full design.

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

## 4. The input and output are application packages

For Steam, the input unit is the selected application distribution: executable images, libraries, assets, launch configuration, dependencies, and the service interactions required by the application. PE files already encode machine types, sections, imports, relocations, and other loading information; the frontend starts there rather than requiring source reconstruction.[^pe]

Command-line acquisition is an established separate capability. DepotDownloader documents macOS installation and a Windows platform selector. Its relevance is that acquisition can be specified and tested independently of execution.[^depot]

Steam integration includes a launch/session contract. Valve documents that Steamworks initialization depends on the Steam client, application identity, user context, and account license state.[^steam]

The proposed system therefore owns authenticated acquisition, dependency resolution, preparation, launch context, and the bridge to legitimate Steam services. External authentication and service decisions remain real external inputs; the translator preserves their protocol rather than manufacturing successful results.

The output unit is a native application bundle. It can contain native executable modules, helpers, assets, Metal archives, state-reconstruction metadata, and the runtime components the application requires. Original PE bytes may be retained where the application contract includes inspection of its images.

A single statically linked file is not the governing requirement. The governing requirement is that the user performs no compatibility setup and manages no separate Windows installation.

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

## 6. Put work at the earliest valid stage

The staging rule is: perform an operation as soon as all of its required inputs are available and stable.

Installation-time work includes translating shipped CPU code, resolving known imports, preparing known graphics pipelines, creating stable path mappings, selecting target implementations, and specializing runtime code to the application.

Update-time work invalidates and regenerates artifacts affected by changed binaries, assets, contracts, compiler versions, or destination capabilities.

Execution-time work includes input, mutable state, synchronization, network responses, and any code or pipeline definitions supplied during execution. Complete knowledge of an operation's semantics does not make its future inputs constant.

The generated system can itself implement a fully specified dynamic compiler. Where the application contains a code generator whose semantics are known, the project can investigate retargeting that generator to emit native code directly. Where the input operation is execution of newly supplied Windows machine code, that operation remains explicitly represented in the contract.

This is a complete translation design with deliberate staging. It does not require heuristic fallback behavior.

Rosetta provides a useful comparison for staging: Apple's platform-security documentation describes both ahead-of-time and just-in-time translation. AOT is therefore a component of the design, not by itself the differentiator.[^rosetta]

## 7. Native implementations of Windows system behavior

A Win32 or NT operation is defined by its effects. The backend implements those effects and can inline or specialize the implementation.

For threads, the contract includes creation, suspended state, identity, stack behavior, completion, and handle lifetime. Microsoft documents that a created thread can start suspended and that its thread object becomes signaled on termination and remains until the relevant lifetime conditions are satisfied.[^thread]

A native thread can be the execution vehicle. A generated object model preserves the Windows-visible state. Host work queues are suitable for the compiler's own background tasks; replacing a Windows thread with a queue task requires a transformation that preserves all relevant thread observations.

For synchronization, local operations should remain local where their contracts permit it. Shared-object coordination should be organized around the actual objects and processes involved. A universal serialization point is not a requirement of the design.

For storage, stable namespace decisions can be compiled. Mutable file and registry behavior remains live. Registry changes can be observed through notifications, so a registry model cannot generally be replaced by installation-time constants.[^registry]

The optimization targets are unnecessary lookup, dispatch, conversion, and coordination. Correct implementations still perform the reads, writes, waits, and notifications that the application requests.

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

A VM-backed implementation is another backend for the same contract. Apple documents configurable intermediate-physical-address granularity down to 4 KB; the setter is available from macOS 26.[^hypervisor-updates][^hypervisor-granule]

The granule capability belongs to the VM address-translation layer. It does not by itself translate x86 instructions or implement Windows object semantics. In a VM backend, translated ARM64 execution and a compact guest runtime would still need explicit interfaces to host graphics, audio, files, and presentation.

The architecture can compare native-process and VM-backed memory implementations using one conformance suite. Neither backend is selected merely from the page-size ratio.

## 9. Concurrency is specified as legal execution

The concurrency contract combines source instruction ordering, atomic operations, synchronization objects, asynchronous callbacks, exceptions, and process interactions.

It must preserve the source binary's permitted executions and required progress. It need not reproduce the physical schedule of one Windows recording. Race behavior is described at the machine-contract level; the compiler does not gain permission to erase it merely because reconstructed source code would have different language rules.

The target lowering chooses ordinary memory operations, acquire/release operations, fences, atomic sequences, and runtime coordination according to that contract. Arm documents that acquire/release instructions impose ordering different from full barriers and can avoid some costs when their ordering is sufficient.[^arm-ordering]

Instruction synchronization and data-memory ordering must remain distinct operations. ISB is not a general substitute for a data-memory barrier.[^arm-barriers]

The automated specification system generates small concurrency programs, explores their legal outcomes in the source and target models, and checks that the translation preserves the required relation. Physical execution then checks the implementation against those models. Herdtools7 is an existing research reference for generating litmus tests, simulating weak-memory models, and executing tests on architectures including x86 and AArch64.[^herd]

Optimization becomes a constrained choice among valid lowerings. Removing a fence or moving a memory operation requires an explicit justification tied to the relevant contract.

## 10. Graphics: shader semantics, command semantics, and resource layout

The graphics subsystem has three connected responsibilities. Shader translation preserves the meaning of GPU programs. Command translation preserves state, bindings, execution dependencies, and presentation behavior. Resource planning maps buffers, textures, heaps, and sharing to the destination.

Direct-to-Metal translation has public precedent: DXMT describes a Metal-based implementation of Direct3D 11 and 10.[^dxmt] The project can study that boundary while defining its own complete graphics contract.

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

Precompilation covers both shader code and the pipeline descriptions required to produce device-specific programs. Apple documents binary archives generated from Metal IR and pipeline-state descriptions to avoid runtime shader compilation.[^metal-archives]

The artifact cache is keyed by shader and pipeline inputs, compiler and contract versions, and the relevant target configuration. Definitions arriving during execution use the same specified pipeline at the stage where those inputs become available.

Graphics conformance distinguishes exact results from results with specified numerical latitude. Microsoft's Direct3D floating-point rules define precision requirements and permitted variation for several operations.[^d3d-float] A test contract should enforce those rules, rather than silently replacing them with equality to whichever physical GPU produced one reference image. A separately selected GPU-specific compatibility profile may impose a more exact target.

Metal supports heap resource aliasing; its documentation explicitly describes resources sharing an allocation.[^metal-heaps] The research target is correspondence between source and destination lifetime, aliasing, alignment, and synchronization rules.

Unified memory is an optimization resource, not a command to place every allocation in the same access mode. Apple distinguishes shared, private, and memoryless resource storage, and documents different suitable uses.[^metal-storage] Direct3D 12 also exposes UMA and cache-coherent UMA capabilities.[^d3d-uma]

The planner should remove copies whose distinct-storage effects are unobservable, retain required synchronization, and select GPU-private or tile-local storage where it best implements the contract. CPU and GPU access to shared physical storage does not require a blanket claim that all pointer values or resource layouts are identical.

## 11. The executable specification and automatic test generator

Each contract is a machine-readable object with a stable identifier and version. Its fields include state, operations, preconditions, transitions, observable effects, permissible nondeterminism, ordering, faults, progress, host representation relations, and allowed optimizations.

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
allowed_completed_wait_sets:
  - [a]
  - [b]
forbidden:
  - both_waits_complete_from_the_single_signal
postcondition:
  signaled_after_consumption: false
```

The surrounding contract supplies scheduler assumptions, error behavior, memory ordering, and observation points. The fragment is not presented as the entire event-object specification.

The same contract is checked through the reference model, the Windows probe, the generated runtime, and compiled programs using that runtime. This makes runtime composition part of verification rather than assuming that independently correct API calls automatically compose correctly.

A failure produces a minimized program, initial state, relevant event trace, contract identifiers, destination description, and mismatch classification. The generation system receives a structured discrepancy rather than an undifferentiated statement that a game failed.

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

## 14. Artifact identity and reproducibility

Every generated artifact records its source hashes, source-contract version, target-contract version, compiler version, specialization facts, and verification evidence. Mutable inputs are not folded into permanent code without a validity condition and invalidation rule.

A dependency graph connects source content, contracts, native modules, graphics archives, and runtime components. Updates rebuild affected artifacts while preserving application state under its storage contract.

A reference run and a target run are reproducible with respect to their declared initial state and injected external events. Nondeterministic execution is recorded as choices constrained by the model, rather than mistaken for an unexplained test failure.

## 15. Research anchors and resulting direction

The research draws on binary lifting, semantic-preserving compilation, partial evaluation, formal memory models, state-machine testing, and native GPU compilation. Remill informs instruction lifting; CompCert informs the structure of semantic preservation; Herdtools7 informs memory-model testing; Microsoft's specifications define source-visible APIs; Apple's specifications define destination operations; DXMT demonstrates a direct Metal translation boundary.

These are research references, not required dependencies or a claim that their combination already delivers this system.

The resulting project is an automatic binary-to-native porting system whose primary design asset is a versioned, executable Windows behavior contract. A per-application compiler specializes that contract into native Mac code and native services. The command line hides acquisition, preparation, service setup, and launch behind one operation. The generated application retains the behavior the game observes while using the destination machine's own execution model internally.

## Sources

Sources were consulted on October 3, 2026. Project-specific architecture and proposed tests above are design synthesis; the references support the underlying platform behavior and established research mechanisms.

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
