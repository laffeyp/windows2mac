// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The case runner's inputs (Sprints 149, 150), shared by its test and by `case_runs` and `class_judge`, the binaries
//! that put its verdicts on a record: the downloaded Intel results whose hashes are recorded, the Rust translator that
//! compiles each case's instruction, each run's committed counts, and the VM the cases run in. From Sprint 238 no
//! input is the frozen Python lowering's words.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::{recorded_sha256, sha256};
use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};

use crate::cases::{Compiled, case_memory};
use crate::guest::{Guest, vm_compiler_symbols};
use crate::process::Start;

/// One Intel run whose result is downloaded and recorded: its id, its case file, the result bytes.
pub struct Run {
    pub id: String,
    pub set: PathBuf,
    pub result: Vec<u8>,
}

impl Run {
    /// The case set's directory name (`semantics_041`).
    pub fn set_name(&self) -> String {
        self.set
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }
}

/// The Rust translator's identity, a candidate's sha256 where it compiled the class: its sources, in name order.
pub fn translator_sha(root: &Path) -> Result<String, String> {
    let dir = root.join("crates/translator/src");
    let mut names: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    names.sort();
    let mut all = Vec::new();
    for p in names {
        all.extend(p.file_name().unwrap().to_string_lossy().as_bytes());
        all.push(0);
        all.extend(fs::read(&p).map_err(|e| e.to_string())?);
        all.push(0);
    }
    Ok(sha256(&all))
}

/// The case files by their SHA-256, and each downloaded Intel result whose hash and set are recorded
/// (`build/probe-runs/<run>/result-1.bin` against `store/probe-runs/<run>/results.sha256` and `cases.sha256`).
pub fn runs(root: &Path) -> Vec<Run> {
    let mut sets: BTreeMap<String, PathBuf> = BTreeMap::new();
    for e in fs::read_dir(root.join("probes")).unwrap() {
        let p = e.unwrap().path().join("cases.bin");
        let named = p
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("semantics");
        if p.exists() && named {
            sets.insert(sha256(&fs::read(&p).unwrap()), p);
        }
    }
    let mut out = Vec::new();
    let Ok(dirs) = fs::read_dir(root.join("build/probe-runs")) else {
        return out;
    };
    for e in dirs {
        let dir = e.unwrap().path();
        let id = dir.file_name().unwrap().to_string_lossy().to_string();
        let store = root.join("store/probe-runs").join(&id);
        let (Ok(raw), Ok(sums), Ok(case_sums), Ok(cpu)) = (
            fs::read(dir.join("result-1.bin")),
            fs::read_to_string(store.join("results.sha256")),
            fs::read_to_string(store.join("cases.sha256")),
            fs::read_to_string(store.join("cpu.txt")),
        ) else {
            continue;
        };
        if !cpu.to_lowercase().contains("intel") || recorded_sha256(&sums, "result-1.bin") != Some(sha256(&raw)) {
            continue;
        }
        if let Some(set) = recorded_sha256(&case_sums, "cases.bin").and_then(|h| sets.get(&h)) {
            out.push(Run {
                id,
                set: set.clone(),
                result: raw,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The compiler the case runner takes (`cases::Compile`): the Rust translator, at the block base and miss stub the
/// runner chooses (Sprint 151; from Sprint 238 alone, the frozen Python lowering's words retired). A code it does not
/// lift or lower is a refusal. Each class it compiled is counted.
pub struct CaseCompiler {
    /// the cases' runtime image's functions the words call (CPUID's from Sprint 159; PCMPISTRI's and the
    /// approximations' from Sprint 161)
    pub runtime: translator::lower::RuntimeCalls,
    /// iclass -> cases compiled
    pub compiled: std::cell::RefCell<BTreeMap<String, usize>>,
}

impl CaseCompiler {
    pub fn new(runtime: translator::lower::RuntimeCalls) -> CaseCompiler {
        CaseCompiler {
            runtime,
            compiled: Default::default(),
        }
    }

    /// `cases::Compile` (the Python's lockstep compile, vm_backend 4611-4618: `lift(code + bytes(16), CODE_VA)`), with
    /// no NZCV source, every flag live after, no NZCV out.
    pub fn compile(&self, code: &[u8], address: u64, base: u64, exit_va: u64) -> Result<Compiled, String> {
        let mut padded = code.to_vec();
        padded.extend([0u8; 16]);
        let ins = translator::lift::lift(&padded, address).map_err(|e| format!("the Rust lifter: {e:?}"))?;
        let state_va = guest_runtime::layout::PROCESS_VA + guest_runtime::layout::STATE_OFF;
        let mut a = a64_encoder::assembler::Assembler::new(base);
        let ends = translator::lower::compile_insn(
            &mut a,
            &ins,
            exit_va,
            state_va,
            &self.runtime,
            None,
            translator::lower::ALL_FLAGS,
            false,
        )
        .map_err(|e| format!("the Rust compiler: {e}"))?;
        *self.compiled.borrow_mut().entry(ins.iclass.clone()).or_default() += 1;
        Ok(Compiled {
            words: a.finish(),
            ends,
        })
    }
}

/// One run's committed counts (`store/case_runner/expected.tsv`).
#[derive(Debug, PartialEq, Eq)]
pub struct Counts {
    pub set: String,
    pub cases: usize,
    pub judged: usize,
    pub windows_faults: usize,
    pub faults_judged: usize,
    pub model: usize,
    pub undefined_memory: usize,
}

pub fn expected(root: &Path) -> BTreeMap<String, Counts> {
    let text = fs::read_to_string(root.join("store/case_runner/expected.tsv")).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| p[i].parse::<usize>().unwrap();
            (
                p[0].to_string(),
                Counts {
                    set: p[1].to_string(),
                    cases: n(2),
                    judged: n(3),
                    windows_faults: n(4),
                    faults_judged: n(5),
                    model: n(6),
                    undefined_memory: n(7),
                },
            )
        })
        .collect()
}

/// The frozen registry's map, iclass -> rule module (`store/rules/registry.tsv`).
pub fn registry(root: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(root.join("store/rules/registry.tsv"))
        .unwrap()
        .lines()
        .map(|l| {
            let (ic, m) = l.split_once('\t').unwrap();
            (ic.to_string(), m.to_string())
        })
        .collect()
}

/// The functions of the runtime the cases run on, built for the host as `with_case_vm` builds it, that the words call
/// (`w2m_cpuid`, `w2m_pcmpistri`, `w2m_approximate`).
pub fn case_runtime_calls(root: &Path) -> Result<translator::lower::RuntimeCalls, String> {
    let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA).map_err(|e| format!("{e:?}"))?;
    if !image.symbols.contains_key("w2m_cpuid") {
        return Err("the guest runtime defines no w2m_cpuid".to_string());
    }
    Ok(translator::lower::RuntimeCalls::from_symbols(|s| {
        image.symbols.get(s).copied()
    }))
}

/// The guest the cases run in (`case_memory`, the runtime built for the host, the services, the stencil symbols),
/// mapped into a VM with one vCPU, handed to `f`.
pub fn with_case_vm<T>(root: &Path, f: impl FnOnce(&mut Guest, &Vcpu<'_>) -> T) -> Result<T, String> {
    let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA).map_err(|e| format!("{e:?}"))?;
    let texts = guest_runtime::texts(root).map_err(|e| format!("{e:?}"))?;
    let services = guest_runtime::services::services(&texts).map_err(|e| format!("{e:?}"))?;
    let table = fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).map_err(|e| e.to_string())?;
    let names = vm_compiler_symbols(&table)?;
    let loaded = case_memory();
    let mut guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("app.exe"),
        &BTreeSet::new(),
        &names,
    )
    .map_err(|e| format!("{e:?}"))?;
    let vm = Vm::new(40).map_err(|e| format!("{e:?}"))?;
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .map_err(|e| format!("{e:?}"))?;
    let vcpu = Vcpu::new(&vm).map_err(|e| format!("{e:?}"))?;
    Ok(f(&mut guest, &vcpu))
}
