// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Step 4's census closed (Sprint 242; reviewer-3's direction at 240): `docs/step-4-census-2026-10-28.md` names the
//! records and tests that show each row's state, and every row not met goes somewhere. A path the census names exists,
//! in the tree or, for one a later sprint removed, in the history; a row whose status does not begin "met" is named in
//! BLACKBOARD's Deferred as "census row N".

use std::path::{Path, PathBuf};
use std::process::Command;

const CENSUS: &str = "docs/step-4-census-2026-10-28.md";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The census table's rows, from its `| # |` header to the first line that is not a row (the close-out's table after
/// it is another): (the row's id, its cells), each row eight `|`s, seven cells.
fn rows(text: &str) -> Vec<(String, Vec<String>)> {
    text.lines()
        .skip_while(|l| !l.starts_with("| # |"))
        .skip(2)
        .take_while(|l| l.starts_with("| "))
        .map(|l| {
            let cells: Vec<String> = l.split('|').map(|c| c.trim().to_string()).collect();
            assert_eq!(cells.len(), 9, "a census row of {} cells: {l:.80}", cells.len() - 2);
            (cells[1].clone(), cells[1..8].to_vec())
        })
        .collect()
}

/// The `store/`, `crates/` and `docs/` paths in backticks, less patterns (`*`, `{`, `<`).
fn paths(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(|t| t.split_whitespace().next().unwrap_or("").to_string())
        .filter(|t| ["store/", "crates/", "docs/"].iter().any(|p| t.starts_with(p)))
        .filter(|t| !t.contains(['*', '{', '<']))
        .map(|t| t.trim_end_matches([',', '.', ';', ':', ')']).to_string())
        .collect()
}

/// Whether `path` is in the tree, or was at some commit.
fn known(root: &Path, path: &str) -> bool {
    if root.join(path).exists() {
        return true;
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "-1", "--format=%H", "--", path])
        .output()
        .unwrap();
    !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

#[test]
fn every_path_the_census_names_exists_or_existed() {
    let root = root();
    let text = std::fs::read_to_string(root.join(CENSUS)).unwrap();
    let named = paths(&text);
    let missing: Vec<&String> = named.iter().filter(|p| !known(&root, p)).collect();
    assert!(
        missing.is_empty(),
        "{CENSUS} names {} path(s) never in the tree: {missing:?}",
        missing.len()
    );
    assert!(named.len() > 20, "{} paths named", named.len());
}

#[test]
fn every_row_not_met_is_carried_in_deferred() {
    let root = root();
    let census = std::fs::read_to_string(root.join(CENSUS)).unwrap();
    let blackboard = std::fs::read_to_string(root.join("BLACKBOARD.md")).unwrap();
    let start = blackboard.find("\n## Deferred").expect("a Deferred section");
    let end = start
        + blackboard[start..]
            .find("\n## Open questions")
            .expect("Open questions after Deferred");
    let deferred = &blackboard[start..end];
    let all = rows(&census);
    let open: Vec<&String> = all
        .iter()
        .filter(|(_, cells)| !cells[5].starts_with("met"))
        .map(|(id, _)| id)
        .collect();
    // "census row 1" ends at the id, so "census row 10" does not carry row 1
    let carried = |id: &str| {
        let named = format!("census row {id}");
        deferred
            .match_indices(&named)
            .any(|(at, _)| !deferred[at + named.len()..].starts_with(|c: char| c.is_ascii_alphanumeric()))
    };
    let uncarried: Vec<&&String> = open.iter().filter(|id| !carried(id)).collect();
    assert!(
        uncarried.is_empty(),
        "census rows not met and not in Deferred: {uncarried:?}"
    );
    assert_eq!(all.len(), 19, "the census's rows: 1-15, 5a and 15a-15d");
}
