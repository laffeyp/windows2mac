# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""pmc_images.py — the PmcInterrupt rows of xperf's `-a dumper` text of a W2mPmcSample trace (pmc_sample.wprp)
reduced to samples by second and image (recentering step 3: the PE images' share of the game's instructions against
Mono's run-time code).

Usage: python pmc_images.py <pmc.txt> <out dir> <image name part>

Each row is one interrupt after 1,000,000 retired instructions: PmcInterrupt, TimeStamp (µs), Process Name ( PID),
ThreadID, PrgrmCtr, CPU, Image!Function, ProfileSource, Reserved (the dumper's header, read on the laptop 2026-10-14).
The dumper names the image from the trace's image loads; code in no image (Mono's compiled methods) shows as
"Unknown!Unknown" (174 of 3,000 samples in a 3-second probe trace on the laptop, 2026-10-14). images.tsv: second, image, samples, for the processes whose name holds the part.
"""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path


def main(argv: list[str]) -> int:
    table, out, part = Path(argv[0]), Path(argv[1]), argv[2].lower()
    samples: Counter[tuple[int, str]] = Counter()
    with table.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            p = [x.strip() for x in line.split(",")]
            if len(p) < 7 or p[0] != "PmcInterrupt" or not p[1].isdigit() or part not in p[2].lower():
                continue
            image = p[6].split("!", 1)[0] if "!" in p[6] else p[6]
            samples[(int(p[1]) // 1_000_000, image or "(no image)")] += 1
    out.mkdir(parents=True, exist_ok=True)
    with (out / "images.tsv").open("w") as w:
        w.write("second\timage\tsamples\n")
        for (s, image), n in sorted(samples.items()):
            w.write(f"{s}\t{image}\t{n}\n")
    total = Counter()
    for (_, image), n in samples.items():
        total[image] += n
    print(f"{sum(total.values())} samples; " + ", ".join(f"{i} {n}" for i, n in total.most_common(8)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
