// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The skeleton's record and its golden check (Sprint 158; link 6 of `docs/design-host-compile-path-2026-10-24.md`).
//!
//! The record is `record_translations`' (`src/w2m/vm_backend.py` 5570-5612): one session, purpose `compilation`:
//! SESSION_INIT; per block translated, in compile order, CODE_TARGET_DISCOVERED (discovery `static` for the entry, else
//! `runtime_transfer`; stage `execution`) and TRANSLATION_PUBLISHED (the guest range, code version 0, the decoder spec's
//! version, artifact `vm-code-cache:<code address>`); SESSION_COMPLETED. `w2m-vm` writes it as substrate's line
//! protocol.
//!
//! The golden check reads two records' w2m events (every frame but substrate's own) and holds them equal in order,
//! kind for kind and field for field, but for three fields each implementation sets for itself: `elapsed_seconds` (a
//! time), `harness_version` (the Python's 0.0.1, the crate's own version), and `vocab_version` (the frozen Python
//! writes 0.9, the Rust vocabulary is 0.10).

use std::path::Path;
use std::time::Duration;

use events::{
    CodeTargetDiscovered, Discovery, FiniteF64, HarnessU64, Id, SessionCompleted, SessionInit, SessionOutcome,
    SessionPurpose, Sha256, Stage, TranslationPublished, VOCAB_VERSION, Value, Version, harness_line,
};

use crate::ledger::frames;

/// The profile both hosts' skeleton records name.
pub const PROFILE: &str = "w2m-vm-skeleton";
/// The fields the golden check does not compare, and why: see the module's documentation.
pub const EXEMPT: [&str; 3] = ["elapsed_seconds", "harness_version", "vocab_version"];
const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");

fn u(n: u64) -> HarnessU64 {
    HarnessU64::new(n).expect("a guest address below 2^53")
}

/// `record_translations`' events, as substrate lines: the guest image's sha256 (64 hex digits), its entry, each block
/// (start, end, code address) in compile order, and the session's time.
pub fn translation_lines(image_sha256: &str, entry: u64, blocks: &[(u64, u64, u64)], elapsed: Duration) -> Vec<String> {
    let image = Sha256::new(format!("sha256:{image_sha256}")).expect("64 lowercase hex digits");
    let spec = Version::new(x86_decoder::SPEC_VERSION).expect("the decoder spec's major.minor.patch");
    let mut lines = vec![harness_line(&SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new(PROFILE).expect("a constant, not empty"),
        contract_version: spec.clone(),
        harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version"),
        purpose: SessionPurpose::Compilation,
    })];
    for &(start, end, code) in blocks {
        lines.push(harness_line(&CodeTargetDiscovered {
            image_sha256: image.clone(),
            guest_address: u(start),
            discovery: if start == entry {
                Discovery::Static
            } else {
                Discovery::RuntimeTransfer
            },
            stage: Stage::Execution,
        }));
        lines.push(harness_line(&TranslationPublished {
            translation_id: Id::new(format!("tr-{}-{start:x}", &image_sha256[..12])).expect("not empty"),
            image_sha256: image.clone(),
            guest_range_start: u(start),
            guest_range_end: u(end),
            code_version: u(0),
            stage: Stage::Execution,
            contract_version: spec.clone(),
            artifact_id: Id::new(format!("vm-code-cache:{code:x}")).expect("not empty"),
        }));
    }
    let zero = u(0);
    lines.push(harness_line(&SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        // `round(time.monotonic() - t0, 3)`
        elapsed_seconds: FiniteF64::new((elapsed.as_secs_f64() * 1000.0).round() / 1000.0)
            .expect("a measured duration is finite"),
    }));
    lines
}

/// The `q` quantile of `sorted` (ascending, not empty) by nearest rank: the smallest value with at least `q` of the
/// sample at or below it, the ceil(q·n)-th (Sprint 163, step 3's `exit_ns_p50` and `exit_ns_p99`).
pub fn quantile(sorted: &[u64], q: f64) -> u64 {
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

/// One MEASUREMENT_RECORDED line of the step-3 measurements' record (Sprint 163), in a Python batch run's conventions
/// (`destination_profile_id` `w2m-vm`, `backend` `managed_vm`, `phase` `discovery`, `artifact_state` `warm`).
pub fn measurement_line(
    measurement_id: &str,
    metric: events::Metric,
    value: f64,
    unit: &str,
    application: &str,
    variant: Option<&str>,
) -> String {
    harness_line(&events::MeasurementRecorded {
        measurement_id: Id::new(measurement_id).expect("not empty"),
        metric,
        value: FiniteF64::new(value).expect("a measured number is finite"),
        unit: unit.to_string(),
        phase: events::MeasurementPhase::Discovery,
        backend: events::Backend::ManagedVm,
        application_profile_id: Id::new(application).expect("not empty"),
        artifact_state: events::ArtifactState::Warm,
        destination_profile_id: Id::new("w2m-vm").expect("not empty"),
        variant: variant.map(str::to_string),
    })
}

/// A record's w2m events, in order: (kind, payload) of every frame whose kind is not substrate's.
pub fn w2m_events(record: &Path) -> Vec<(String, Value)> {
    frames(record)
        .into_iter()
        .filter_map(|f| {
            let kind = f["kind"].as_str()?.to_string();
            (!kind.starts_with("substrate.")).then(|| (kind, f["payload"].clone()))
        })
        .collect()
}

/// The golden check: `ours` holds `stored`'s events in order, kind for kind, field for field but for `EXEMPT`; the
/// first difference named.
pub fn compare(ours: &[(String, Value)], stored: &[(String, Value)]) -> Result<(), String> {
    if ours.is_empty() {
        return Err("the record holds no w2m event".into());
    }
    for (i, (a, b)) in ours.iter().zip(stored).enumerate() {
        if a.0 != b.0 {
            return Err(format!("event {i}: {} where the stored record has {}", a.0, b.0));
        }
        let (Some(pa), Some(pb)) = (a.1.as_object(), b.1.as_object()) else {
            return Err(format!("event {i} ({}): a payload that is not an object", a.0));
        };
        let keys = |p: &serde_json::Map<String, Value>| {
            p.keys()
                .filter(|k| !EXEMPT.contains(&k.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        };
        if keys(pa) != keys(pb) {
            return Err(format!(
                "event {i} ({}): fields {:?} where the stored record has {:?}",
                a.0,
                keys(pa),
                keys(pb)
            ));
        }
        for k in keys(pa) {
            if pa[&k] != pb[&k] {
                return Err(format!(
                    "event {i} ({}): {k} is {} where the stored record has {}",
                    a.0, pa[&k], pb[&k]
                ));
            }
        }
    }
    if ours.len() != stored.len() {
        return Err(format!(
            "{} w2m events where the stored record has {}",
            ours.len(),
            stored.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(lines: &[String]) -> Vec<(String, Value)> {
        lines
            .iter()
            .map(|l| {
                let v: Value = l.parse().unwrap();
                (v["kind"].as_str().unwrap().to_string(), v["payload"].clone())
            })
            .collect()
    }

    const IMAGE: &str = "6863d5ecd3a14fb5df0f1b194135fecd9e59a153aa2564ddfc2db034181d37de";

    /// Nearest rank: of four, the median is the second, not the third an index of n·q would take.
    #[test]
    fn the_quantile_is_the_nearest_rank() {
        let sample = [10, 20, 30, 40];
        assert_eq!(quantile(&sample, 0.5), 20, "p50");
        assert_eq!(quantile(&sample, 0.99), 40, "p99");
        assert_eq!(quantile(&[7], 0.5), 7, "one value");
    }

    #[test]
    fn a_time_is_not_compared_and_a_block_is() {
        let a = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x20, 0x7f00)],
            Duration::from_millis(5),
        ));
        let b = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x20, 0x7f00)],
            Duration::from_millis(9),
        ));
        assert_eq!(compare(&a, &b), Ok(()));
        let c = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x21, 0x7f00)],
            Duration::from_millis(5),
        ));
        assert_eq!(
            compare(&a, &c),
            Err("event 2 (TRANSLATION_PUBLISHED): guest_range_end is 32 where the stored record has 33".into())
        );
        assert!(
            compare(&a[..3], &a)
                .unwrap_err()
                .contains("3 w2m events where the stored record has 4")
        );
    }
}
