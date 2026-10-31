# every frame: its ticks, its start, and its calls by 16 MiB range
$1 == "CALL" {
  t0 = $4 + 0; if (base == "") { base = t0; wraps = 0; last = t0 }
  if (t0 < last - 2147483648) wraps += 4294967296; last = t0; t = t0 + wraps
  ticks = $3 + 0; end = t + ticks
  if (n == 0) { fstart = t; fsum = 0; delete rc }
  if ((end - fstart) * hz >= freq) { out(); fstart = t; fsum = 0; delete rc }
  fsum += ticks; r = substr($2, 1, length($2) - 6); rc[r] += ticks; n++
}
function out(   r, s) { s = ""; for (r in rc) s = s sprintf(" 0x%s000000 %.2f", r, rc[r] * 1000 / freq); printf "%.3f %.3f%s\n", fsum * 1000 / freq, (fstart - base) / freq, s }
END { out() }
