// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The first host request two exit ledgers' guests asked differently (Sprint 258): `ledger_parting <a.tsv> <b.tsv>`
//! prints the request's index, rows, op, arguments and entry counts in both, or that neither parts from the other.
//! Exit 0 either way; a ledger that does not read is named on stderr, exit 1.

use std::process::ExitCode;

use w2m_vm::exit_ledger::{Asked, asked, first_parting};

fn read(path: &str) -> Result<Vec<Asked>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    asked(&text).map_err(|e| format!("{path}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = args.as_slice() else {
        eprintln!("usage: ledger_parting <a.tsv> <b.tsv>");
        return ExitCode::FAILURE;
    };
    let (x, y) = match (read(a), read(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("ledger_parting: {e}");
            return ExitCode::FAILURE;
        }
    };
    let words = |s: &Asked| s.words.map(|w| format!("{w:x}")).join(" ");
    match first_parting(&x, &y) {
        Some((i, p, q)) => {
            println!("request {i} of {} and {}", x.len(), y.len());
            println!("{a}: row {} words {} count {}", p.row, words(&p), p.count);
            println!("{b}: row {} words {} count {}", q.row, words(&q), q.count);
        }
        None => println!("no parting in the first {} requests", x.len().min(y.len())),
    }
    ExitCode::SUCCESS
}
