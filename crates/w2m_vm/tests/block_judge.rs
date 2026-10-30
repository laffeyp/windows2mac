// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The whole-block judge's files (Sprint 252; `docs/design-translation-cache-2026-10-30.md`, card 1): the stored one,
//! `store/translation-cache/whole-blocks-2026-10-30.tsv`, and the gates' batch run's, `build/batch_run/whole-blocks.tsv`,
//! which `w2m-vm --judge-blocks` writes. Each holds its totals last; the totals add up; the blocks judged are the run's
//! "compiled"; and every block that differs names a class this test knows, with its reason.

use std::fs;
use std::path::{Path, PathBuf};

const STORED: &str = "store/translation-cache/whole-blocks-2026-10-30.tsv";
/// The run the stored file came from compiled this many blocks in the VM (Sprint 250's held first stop).
const STORED_COMPILED: u64 = 274_837;
/// The classes of difference the judge may name, each with why it is not a fault of the translator. None so far: the
/// batch run's 274,837 blocks were word for word and mark for mark the C's (2026-10-30).
const CLASSES: [(&str, &str); 0] = [];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// The file's faults: its totals line, their sum, the blocks judged against `compiled`, each differing line's class.
fn problems(text: &str, compiled: u64) -> Vec<String> {
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    let Some(totals) = lines.last().and_then(|l| l.strip_prefix("# judged\t")) else {
        return vec!["no \"# judged\" line last".to_string()];
    };
    let n: Vec<u64> = totals.split('\t').filter_map(|f| f.parse().ok()).collect();
    let [judged, equal, differing] = n[..] else {
        return vec![format!("the totals line is \"# judged\t{totals}\"")];
    };
    if judged != equal + differing {
        found.push(format!(
            "{judged} judged, where {equal} equal and {differing} differing"
        ));
    }
    if judged != compiled {
        found.push(format!("{judged} judged, where the run compiled {compiled}"));
    }
    let rows: Vec<&str> = lines.iter().filter(|l| !l.starts_with('#')).copied().collect();
    if rows.len() as u64 != differing {
        found.push(format!(
            "{} differing lines, where the totals say {differing}",
            rows.len()
        ));
    }
    for row in rows {
        let class = row.rsplit('\t').next().unwrap_or("");
        if !CLASSES.iter().any(|(c, _)| *c == class) {
            found.push(format!("a block of class \"{class}\", which no reason names: {row}"));
        }
    }
    found
}

#[test]
fn the_stored_judgement_holds() {
    let text = fs::read_to_string(root().join(STORED)).unwrap();
    assert_eq!(problems(&text, STORED_COMPILED), Vec::<String>::new());
}

/// The gates' batch run's judgement, against its stop file's "compiled".
#[test]
fn the_gates_judgement_holds() {
    let dir = root().join("build/batch_run");
    let (Ok(text), Ok(stop)) = (
        fs::read_to_string(dir.join("whole-blocks.tsv")),
        fs::read_to_string(dir.join("first-stop.tsv")),
    ) else {
        let what = "no build/batch_run/whole-blocks.tsv or first-stop.tsv (scripts/gates.sh writes them)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires them");
        }
        eprintln!("{what}: the gates' judgement is not held here");
        return;
    };
    let compiled = stop
        .lines()
        .find_map(|l| l.strip_prefix("compiled\t"))
        .and_then(|n| n.parse().ok())
        .expect("the stop file's compiled line");
    assert_eq!(problems(&text, compiled), Vec::<String>::new());
}

/// Each fault the check names: totals that do not add up, a count off the run's, a line with no class.
#[test]
fn each_fault_in_a_judgement_is_named() {
    let good = "# start\tend\n# judged\t3\tequal\t3\tdiffering\t0\n";
    assert_eq!(problems(good, 3), Vec::<String>::new());
    assert_eq!(problems(good, 4), ["3 judged, where the run compiled 4"]);
    assert_eq!(
        problems("# judged\t3\tequal\t2\tdiffering\t0\n", 3),
        ["3 judged, where 2 equal and 0 differing"]
    );
    let unnamed = "# start\n1000\t1010\t2\taa\tbb\t0x1004\twords\n# judged\t3\tequal\t2\tdiffering\t1\n";
    assert_eq!(
        problems(unnamed, 3),
        ["a block of class \"words\", which no reason names: 1000\t1010\t2\taa\tbb\t0x1004\twords"]
    );
    assert_eq!(problems("# start\n", 0), ["no \"# judged\" line last"]);
}
