// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The case runner's inputs (Sprints 149, 150), shared by its test and by `case_runs`, the binary that puts its
//! verdicts on a record: the downloaded Intel results whose hashes are recorded, the frozen Python lowering's words
//! pinned by their header and committed sha256, each run's committed counts, and the VM the cases run in.

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

/// The fixture's header: `# base 0x.. exit 0x.. symbols <sha256> src <commit>`.
pub struct Header {
    pub base: u64,
    pub exit: u64,
    pub symbols: String,
    pub src: String,
}

/// The frozen Python lowering's words for every case instruction (`build/case_runner/python-words.txt`; its generator
/// is named in WORKING_AGREEMENT): code -> (ends, words), or None where the Python refused.
pub struct Fixture {
    pub header: Header,
    pub sha256: String,
    pub words: BTreeMap<Vec<u8>, Option<(bool, Vec<u32>)>>,
}

impl Fixture {
    /// Reads the fixture and checks it is the one `store/case_runner/python-words.sha256` pins. None when it is absent.
    pub fn load(root: &Path) -> Option<Result<Fixture, String>> {
        let text = fs::read_to_string(root.join("build/case_runner/python-words.txt")).ok()?;
        Some(Fixture::parse(root, &text))
    }

    fn parse(root: &Path, text: &str) -> Result<Fixture, String> {
        let pinned =
            fs::read_to_string(root.join("store/case_runner/python-words.sha256")).map_err(|e| e.to_string())?;
        let got = sha256(text.as_bytes());
        if got != pinned.trim() {
            return Err(format!("the fixture is not the one committed by its sha256 ({got})"));
        }
        let mut lines = text.lines();
        let first = lines.next().ok_or("an empty fixture")?;
        let p: Vec<&str> = first.split_whitespace().collect();
        if !(p.len() == 9 && p[0] == "#" && p[1] == "base" && p[3] == "exit" && p[5] == "symbols" && p[7] == "src") {
            return Err(format!(
                "the fixture's header is not `# base .. exit .. symbols .. src ..`: {first}"
            ));
        }
        let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| e.to_string());
        let header = Header {
            base: hex(p[2])?,
            exit: hex(p[4])?,
            symbols: p[6].to_string(),
            src: p[8].to_string(),
        };
        let words = lines
            .map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                let code = (0..p[0].len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&p[0][i..i + 2], 16).unwrap())
                    .collect();
                let words = (p[1] == "ok").then(|| {
                    (
                        p[2] == "1",
                        p[3..].iter().map(|w| u32::from_str_radix(w, 16).unwrap()).collect(),
                    )
                });
                (code, words)
            })
            .collect();
        Ok(Fixture {
            header,
            sha256: got,
            words,
        })
    }

    /// The compiler the runner takes (`cases::Compile`): the fixture's words for the layout its header names (the
    /// Python's lockstep compile, vm_backend 4611-4618), refusing any other layout.
    pub fn compile(&self, code: &[u8], _address: u64, base: u64, exit_va: u64) -> Result<Compiled, String> {
        assert_eq!(
            (base, exit_va),
            (self.header.base, self.header.exit),
            "the fixture's words were made for another layout"
        );
        match self.words.get(code) {
            Some(Some((ends, words))) => Ok(Compiled {
                words: words.clone(),
                ends: *ends,
            }),
            Some(None) => Err("the Python refused it".into()),
            None => Err("not in the fixture".into()),
        }
    }
}

/// The sha256 of a symbol table as the fixture's generator writes it: `name=0x<address>` lines, sorted by name.
pub fn symbols_sha256(symbols: &BTreeMap<String, u64>) -> String {
    let text: String = symbols.iter().map(|(n, v)| format!("{n}=0x{v:x}\n")).collect();
    sha256(text.as_bytes())
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

/// The guest the cases run in (`case_memory`, the runtime built for the host, the services, the stencil symbols),
/// mapped into a VM with one vCPU, handed to `f`. Fails when the fixture was made for another runtime symbol table.
pub fn with_case_vm<T>(
    root: &Path,
    fixture: &Fixture,
    f: impl FnOnce(&mut Guest, &Vcpu<'_>) -> T,
) -> Result<T, String> {
    let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA).map_err(|e| format!("{e:?}"))?;
    // the words call the runtime's symbols: the fixture was made with this symbol table
    let have = symbols_sha256(&image.symbols);
    if have != fixture.header.symbols {
        return Err(format!(
            "the fixture's words were made for another runtime symbol table ({have} here, {} in the fixture)",
            fixture.header.symbols
        ));
    }
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
