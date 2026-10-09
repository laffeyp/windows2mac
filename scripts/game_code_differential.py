"""game_code_differential.py — w2m's decoder against Intel XED on all of a game's code: its native
PE files and the code Mono generated for it on Windows (Sprint 033).

Positions:
  native  — Capstone recursive traversal (x86ref.walk) of each PE file;
  runtime — each region of a tools/jit_capture capture (methods and code buffers, duplicates by
            code_start removed), decoded linearly from its first byte. Where w2m's decoder refuses
            a position, XED decides: if XED decodes it, the position is a gap and the pass
            continues at XED's length; if XED refuses it too, the region's code has ended (data,
            padding, or a table) and the pass stops there.

Outcome per position: agree (length and iclass equal XED's), length_differs, iclass_differs,
ours_gap (XED decodes, w2m's decoder has no row), xed_refused (native only: Capstone decoded a
position XED refuses).

Decoder exit criterion (d) (WORKING_AGREEMENT, re-pointed at this profile 2026-10-09): every
spec row the game's code uses is XED-confirmed in specs/x86_decode_confirmations.json. Each agreed
position names its row (Decoded.row); the report lists used rows without a confirmation.

--record writes a substrate run as generated_differential.py does: SESSION_INIT,
COMPARISON_COMPLETED (contract x86.decode, relation exact, reference XED), then per disagreement
class DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (length_differs -> implementation_repair; every
other class -> contract_revision), then SESSION_COMPLETED.

Usage: uv run python scripts/game_code_differential.py <native_dir> <capture.bin> --game <name>
       [--json out.json] [--record] [--runs runs] [--store store]
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import Runtime, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events
from w2m.jit_capture import cpu_features, parse as parse_capture
from w2m.pe import PeError, parse
from w2m.x86dec import DecodeError, Decoded, decode
from w2m.x86dec_tables import SPEC_VERSION
from w2m.x86ref import walk
from w2m.xedref import XedResult, decode_many

ROOT = Path(__file__).resolve().parent.parent
CONFIRMATIONS = ROOT / "specs" / "x86_decode_confirmations.json"
XED_PROFILE = "xed-v2026.08.23-0bcb6237345c"
EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()
SUFFIXES = (".exe", ".dll")


class Tally:
    def __init__(self) -> None:
        self.outcome: Counter[str] = Counter()
        self.kinds: Counter[tuple[str, str, str, str]] = Counter()
        self.example: dict[tuple[str, str, str, str], str] = {}
        self.rows: Counter[str] = Counter()

    def judge(self, raw: bytes, ours: Decoded | None, x: XedResult) -> None:
        if not x.ok:
            o, k = "xed_refused", ("xed_refused", x.error, ours.iclass if ours else "", "")
        elif ours is None:
            o, k = "ours_gap", ("ours_gap", x.iclass, "", "")
        elif ours.length != x.length:
            o, k = "length_differs", ("length_differs", x.iclass, ours.iclass, f"{x.length}/{ours.length}")
        elif ours.iclass != x.iclass:
            o, k = "iclass_differs", ("iclass_differs", x.iclass, ours.iclass, "")
        else:
            self.outcome["agree"] += 1
            if ours.row:
                self.rows[ours.row] += 1
            return
        self.outcome[o] += 1
        self.kinds[k] += 1
        self.example.setdefault(k, raw[:x.length or 15].hex())


def _ours(raw: bytes) -> Decoded | None:
    try:
        return decode(raw)
    except DecodeError:
        return None


def native(root: Path, t: Tally) -> int:
    n = 0
    for f in sorted(root.iterdir()):
        if f.suffix.lower() not in SUFFIXES:
            continue
        data = f.read_bytes()
        try:
            img = parse(data)
        except PeError as e:
            print(f"not a PE32+ x86-64 image: {f.name}: {e}")
            continue
        raws = [img.read_rva(data, i.rva, 15) for i in walk(img, data).insns.values()]
        for raw, x in zip(raws, decode_many(raws), strict=True):
            t.judge(raw, _ours(raw), x)
        n += len(raws)
        print(f"  native {f.name}: {len(raws)} positions")
    return n


def runtime_code(regions: list[bytes], t: Tally) -> tuple[int, Counter[str]]:
    """Linear pass per region; XED decides where w2m's decoder refuses."""
    n = 0
    ends: Counter[str] = Counter()
    for code in regions:
        off = 0
        batch: list[tuple[bytes, Decoded]] = []
        while off < len(code):
            raw = code[off:off + 15]
            ours = _ours(raw)
            if ours is not None:
                batch.append((raw, ours))
                off += ours.length
                continue
            (x,) = decode_many([raw])
            if not x.ok:
                ends["xed_refuses" if len(raw) == 15 else "region_end"] += 1
                break
            t.judge(raw, None, x)
            n += 1
            off += x.length
        else:
            ends["exact_end"] += 1
        for (raw, ours), x in zip(batch, decode_many([r for r, _ in batch]), strict=True):
            t.judge(raw, ours, x)
        n += len(batch)
    return n, ends


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("native_dir", type=Path)
    ap.add_argument("capture", type=Path)
    ap.add_argument("--game", required=True)
    ap.add_argument("--json", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    ap.add_argument("--store", type=Path, default=Path("store"))
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    t = Tally()
    n_native = native(a.native_dir, t)
    cap = parse_capture(a.capture.read_bytes())
    regions = cap.regions()
    n_runtime, ends = runtime_code([r.code for r in regions], t)
    conf = json.loads(CONFIRMATIONS.read_text())["by_row_sha256"]
    unconfirmed = {r: n for r, n in t.rows.items() if not conf.get(r, {}).get("confirmed")}
    total = n_native + n_runtime
    print(f"positions {total} (native {n_native}, runtime {n_runtime}); {time.monotonic() - t0:.0f} s")
    print(f"capture format {cap.format}: records {len(cap.records)}, regions {len(regions)} "
          f"({len(cap.records) - len(regions)} duplicates), code bytes {sum(len(r.code) for r in regions)}, "
          f"torn tail {cap.torn_tail_bytes}; kinds {dict(Counter(r.kind for r in regions))}")
    print(f"images {Counter(r.image for r in regions if r.kind == 'method').most_common(6)}")
    print(f"cpu features {cpu_features(cap) or 'not recorded (format 1)'}")
    print(f"region ends {dict(ends)}")
    for o in ("agree", "ours_gap", "length_differs", "iclass_differs", "xed_refused"):
        print(f"  {o:15} {t.outcome[o]}")
    print(f"criterion (d): rows used {len(t.rows)}; confirmed {len(t.rows) - len(unconfirmed)}; "
          f"unconfirmed {len(unconfirmed)}")
    for r, n in sorted(unconfirmed.items(), key=lambda x: -x[1])[:15]:
        print(f"  {n:8} {conf.get(r, {}).get('mn', '?'):12} {r}")
    print(f"disagreement classes {len(t.kinds)}")
    for k, n in t.kinds.most_common(30):
        print(f"  {n:7} {k} {t.example[k]}")
    if a.json:
        a.json.write_text(json.dumps({
            "game": a.game, "spec_version": SPEC_VERSION, "positions": total, "native": n_native,
            "runtime": n_runtime, "outcomes": dict(t.outcome), "region_ends": dict(ends),
            "capture": {"format": cap.format, "records": len(cap.records), "regions": len(regions),
                        "torn_tail_bytes": cap.torn_tail_bytes, "cpu_features": cpu_features(cap)},
            "rows_used": len(t.rows), "rows_unconfirmed": [
                {"row_sha256": r, "mn": conf.get(r, {}).get("mn"), "positions": n}
                for r, n in sorted(unconfirmed.items(), key=lambda x: -x[1])],
            "kinds": [{"outcome": k[0], "xed": k[1], "ours": k[2], "lengths": k[3], "count": n,
                       "example": t.example[k]} for k, n in t.kinds.most_common()]}, indent=1) + "\n")
    if a.record:
        record(a, t, total, len(t.rows), len(unconfirmed), time.monotonic() - t0)
    return 0


def record(a: argparse.Namespace, t: Tally, total: int, used: int, unconfirmed: int,
           elapsed: float) -> None:
    """One substrate run holding this differential's verdict and its routed discrepancies."""
    run_id = f"gamediff-{time.time_ns()}"
    ours = f"w2m-x86dec-spec-{SPEC_VERSION}"
    progs = a.store / "programs"
    progs.mkdir(parents=True, exist_ok=True)
    classes = t.kinds.most_common()

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=ours, contract_version=SPEC_VERSION,
            harness_version="0.0.1", purpose="conformance"))
        cmp_id = f"cmp-{run_id}"
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=cmp_id, contract_id="x86.decode", contract_version=SPEC_VERSION,
            relation="exact", reference_ref=XED_PROFILE, target_execution_id=run_id,
            verdict="discrepancy" if classes else "conforms",
            rule_ref=f"{a.game}: {total} positions (native traversal and Mono-generated regions); "
                     f"rows used {used}, unconfirmed {unconfirmed}"))
        for i, (k, n) in enumerate(classes, 1):
            raw = bytes.fromhex(t.example[k])
            digest = hashlib.sha256(raw).hexdigest()
            (progs / f"{digest}.bin").write_bytes(raw)
            did = f"{run_id}-d{i}"
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id="x86.decode", contract_version=SPEC_VERSION,
                mismatch_class="exact", minimized_program_sha256=f"sha256:{digest}",
                initial_state_sha256=EMPTY,
                first_divergence=f"{k[0]}: xed={k[1] or '-'} ours={k[2] or '-'}"
                                 f"{' lengths ' + k[3] if k[3] else ''}; {n} positions",
                reference_profile_id=XED_PROFILE, destination_profile_id=ours,
                comparison_id=cmp_id))
            impl = k[0] == "length_differs"
            yield events.validate(events.DISCREPANCY_ROUTED(
                discrepancy_id=did,
                repair_path="implementation_repair" if impl else "contract_revision",
                reference_conforms=impl,
                grounds=("spec row fields agree with XED; the decoder's length routine is wrong"
                         if impl else "the spec lacks the row or rule XED applies "
                                      "(Decision 2026-10-08: XED judges)")))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if classes else "completed",
            discrepancy_count=len(classes), accepted_count=0, elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(spec_version=SPEC_VERSION, reference=XED_PROFILE, game=a.game, positions=total)
        b.producer_kind("comparator", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: harness)
        b.initial("comparator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / run_id).run(topology))
    kinds = Counter(e["kind"] for e in read_record(result.record_root)
                    if not e["kind"].startswith("substrate."))
    print(f"record {result.record_root}: {dict(kinds)}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
