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

/// The fixture's header: `# base 0x.. exit 0x.. symbols <sha256> layout <sha256> src <commit>` (layout from Sprint 152).
pub struct Header {
    pub base: u64,
    pub exit: u64,
    pub symbols: String,
    /// the sha256 of the guest layout constants the runtime is compiled against (`runtime_defines`), as `symbols`'
    /// lines: the words may embed them (a load of the process area's 0x7ffe_0000_3000)
    pub layout: String,
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
        let named = ["#", "base", "", "exit", "", "symbols", "", "layout", "", "src", ""];
        if p.len() != named.len() || named.iter().zip(&p).any(|(n, v)| !n.is_empty() && n != v) {
            return Err(format!(
                "the fixture's header is not `# base .. exit .. symbols .. layout .. src ..`: {first}"
            ));
        }
        let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| e.to_string());
        let header = Header {
            base: hex(p[2])?,
            exit: hex(p[4])?,
            symbols: p[6].to_string(),
            layout: p[8].to_string(),
            src: p[10].to_string(),
        };
        // the words are the frozen Python's at this commit of src/w2m, the last that touched it
        let at = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["log", "-1", "--format=%H", "--", "src/w2m"])
            .output()
            .map_err(|e| e.to_string())?;
        let src_now = String::from_utf8_lossy(&at.stdout).trim().to_string();
        if src_now != header.src {
            return Err(format!(
                "the fixture was made at src/w2m {}, and src/w2m is now at {src_now}",
                header.src
            ));
        }
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

/// The Rust translator for the classes its rules lift, the fixture's Python words for the rest (Sprint 151): the
/// compiler that judges link 5's groups against Windows while the others are still the Python's. Each class the Rust
/// compiled is counted, and its words are compared with the fixture's, which were made for the same layout with the
/// same flag arguments (no NZCV source, every flag live after, no NZCV out).
pub struct Hybrid<'a> {
    pub fixture: &'a Fixture,
    /// the cases' runtime image's functions the words call (CPUID's from Sprint 159; PCMPISTRI's and the
    /// approximations' from Sprint 161)
    pub runtime: translator::lower::RuntimeCalls,
    /// iclass -> cases the Rust compiled
    pub rust: std::cell::RefCell<BTreeMap<String, usize>>,
    /// a code whose Rust words differ from the Python's: (code, its class, the difference)
    pub unlike: std::cell::RefCell<Vec<(Vec<u8>, String, String)>>,
}

impl<'a> Hybrid<'a> {
    pub fn new(fixture: &'a Fixture, runtime: translator::lower::RuntimeCalls) -> Hybrid<'a> {
        Hybrid {
            fixture,
            runtime,
            rust: Default::default(),
            unlike: Default::default(),
        }
    }

    /// `cases::Compile` (the Python's lockstep compile, vm_backend 4611-4618: `lift(code + bytes(16), CODE_VA)`).
    pub fn compile(&self, code: &[u8], address: u64, base: u64, exit_va: u64) -> Result<Compiled, String> {
        let mut padded = code.to_vec();
        padded.extend([0u8; 16]);
        let Ok(ins) = translator::lift::lift(&padded, address) else {
            return self.fixture.compile(code, address, base, exit_va);
        };
        let state_va = guest_runtime::layout::PROCESS_VA + guest_runtime::layout::STATE_OFF;
        let mut a = a64_encoder::assembler::Assembler::new(base);
        let ends = match translator::lower::compile_insn(
            &mut a,
            &ins,
            exit_va,
            state_va,
            &self.runtime,
            None,
            translator::lower::ALL_FLAGS,
            false,
        ) {
            Ok(ends) => ends,
            // a lowering the Rust refuses by name as a later group (Sprint 177: the MMX ops the one lifter emits) is the
            // Python's, as a class the Rust does not lift is; any other refusal fails
            Err(translator::lower::CompileError::LaterGroup { .. }) => {
                return self.fixture.compile(code, address, base, exit_va);
            }
            Err(e) => return Err(format!("the Rust compiler: {e}")),
        };
        let words = a.finish();
        *self.rust.borrow_mut().entry(ins.iclass.clone()).or_default() += 1;
        if let Ok(python) = self.fixture.compile(code, address, base, exit_va)
            && (python.words != words || python.ends != ends)
        {
            let shown = |w: &[u32]| w.iter().map(|x| format!("{x:08x}")).collect::<Vec<_>>().join(" ");
            self.unlike.borrow_mut().push((
                code.to_vec(),
                ins.iclass.clone(),
                format!(
                    "rust [{}] {ends}, python [{}] {}",
                    shown(&words),
                    shown(&python.words),
                    python.ends
                ),
            ));
        }
        Ok(Compiled { words, ends })
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
    // and the layout constants: the fixture was made with these
    let layout: BTreeMap<String, u64> = runtime_defines().into_iter().collect();
    let have = symbols_sha256(&layout);
    if have != fixture.header.layout {
        return Err(format!(
            "the fixture's words were made for another guest layout ({have} here, {} in the fixture)",
            fixture.header.layout
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
