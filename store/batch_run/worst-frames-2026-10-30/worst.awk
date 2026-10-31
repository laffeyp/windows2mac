# the worst frame's calls by the top of their address (16 MiB ranges), with their ticks
$1 == "CALL" {
  t0 = $4 + 0; if (base == "") { base = t0; wraps = 0; last = t0 }
  if (t0 < last - 2147483648) wraps += 4294967296; last = t0; t = t0 + wraps
  ticks = $3 + 0; end = t + ticks
  if (n == 0) { fstart = t; fsum = 0; m = 0 }
  if ((end - fstart) * hz >= freq) { if (fsum > bsum) { bsum = fsum; bm = m; for (k = 1; k <= m; k++) { ba[k] = ma[k]; bt[k] = mt[k] } } fstart = t; fsum = 0; m = 0 }
  fsum += ticks; ma[++m] = $2; mt[m] = ticks; n++
}
END {
  if (fsum > bsum) { bsum = fsum; bm = m; for (k = 1; k <= m; k++) { ba[k] = ma[k]; bt[k] = mt[k] } }
  for (k = 1; k <= bm; k++) { r = substr(ba[k], 1, length(ba[k]) - 6); c[r]++; s[r] += bt[k] }
  for (r in c) printf "  0x%s000000: %d calls, %.3f ms\n", r, c[r], s[r] * 1000 / freq
}
