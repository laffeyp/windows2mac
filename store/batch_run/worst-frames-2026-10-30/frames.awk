# the probe's rule (monitor_code::compile_probe): a frame opens at a call's start and holds every call ending within
# 1/hz s of it; the first to end past it folds the frame into the most and opens the next at its own start
$1 == "CALL" {
  t0 = $4 + 0; if (base == "") { base = t0; wraps = 0; last = t0 }
  if (t0 < last - 2147483648) wraps += 4294967296; last = t0; t = t0 + wraps
  ticks = $3 + 0; end = t + ticks
  if (n == 0) { fstart = t; fsum = 0 }
  if ((end - fstart) * hz >= freq) { frames[++nf] = fsum; fs[nf] = fstart; fc[nf] = fcount; fi[nf] = finst; fstart = t; fsum = 0; fcount = 0; finst = 0; delete members }
  fsum += ticks; fcount++; finst += $5; n++
}
END {
  frames[++nf] = fsum; fs[nf] = fstart; fc[nf] = fcount; fi[nf] = finst
  best = 0; for (i = 1; i <= nf; i++) if (frames[i] > frames[best + 0] || best == 0) best = i
  printf "calls %d frames %d worst %.3f ms (%d calls, %d installs) at %.3f s\n", n, nf, frames[best] * 1000 / freq, fc[best], fi[best], (fs[best] - base) / freq
}
