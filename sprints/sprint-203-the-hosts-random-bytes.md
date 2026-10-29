# Sprint 203 — The host's random bytes

```yaml
---
id: 203
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198", the batch run's stops in row order: the stream equals the Python's rows
0-256 and stops at row 257, RANDOM 23 (bcrypt.c's BCryptGenRandom, com.c's CoCreateGuid). The Python answers it from
`random.Random(0x5EED).randbytes(n)` when the run is on the instruction clock, so two runs of one tree make the same
calls, and from `os.urandom` otherwise (vm_backend 2357, 3840-3842). Python 3.12's `randbytes(n)` is
`getrandbits(n * 8).to_bytes(n, 'little')` (random.py 284-286); its generator is CPython's MT19937
(Modules/_randommodule.c 85-89, 129-275, 282-345, 488-535, fetched 2026-10-28 from the 3.12 branch).

- **The generator:** MT19937 in Rust as CPython has it: `init_genrand`, `init_by_array` over an integer seed's 32-bit
  words from the least significant, `genrand_uint32`, and `getrandbits`' word order; `randbytes`.
- **The request:** host request RANDOM 23 writes `a0` bytes at `a1`: on the instruction clock from the process's
  generator seeded 0x5EED, else from the host's CSPRNG (`getentropy`).
- **The run:** the batch run run again to its next stop.

## prerequisites

- 202 (closed)

## context_files

- `src/w2m/vm_backend.py` 2350-2360, 3838-3842; CPython 3.12 `Lib/random.py` 284-286 and `Modules/_randommodule.c`;
  `crates/w2m_vm/src/host_request.rs`

## signal contract

### Emits

- None: the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-256 still equal; the guests as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/mt19937.rs` (new), `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/guest.rs`,
  `crates/w2m_vm/src/lib.rs`, `store/batch_run/first-stop-2026-10-28.tsv`, `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `batch_run.rs`, the generator's unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** MT19937's reference vector (`init_by_array({0x123, 0x234, 0x345, 0x456})`, first outputs
  1067595299, 955945823, 477289528) reproduced; the batch run passes row 257 and its stream equals the Python's to its
  next stop.
- **Planted faults:** (1) the tempering's 0x9d2c5680 changed: the reference test names the output; (2) the seed's
  words taken from the most significant end: a stream row or a unit test names it; (3) `getrandbits`' last word not
  shifted for a byte count not a multiple of 4: the unit test names the bytes.

## done criteria

The host's random bytes are the Python's on the instruction clock, and the batch run passes row 257.

## self-review checklist

- [x] Scope names one verifiable property: the bytes the Python's generator makes, and the stream past row 257.
- [x] context_files cite the source read.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the host's random bytes.
