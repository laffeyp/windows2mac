#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The cargo runner on this Mac (.cargo/config.toml, Sprint 132): signs a binary ad hoc with the Hypervisor
# entitlement, as src/w2m/vm_backend.py signs vm_monitor, then runs it with its arguments. hv_vm_create refuses an
# unsigned process (HV_DENIED; WORKING_AGREEMENT, the Hypervisor.framework bridge mapping).
set -eu
binary=$1
shift
# Signed once, under a lock: cargo-nextest runs a test binary in many processes at once (its two listings, then a
# process a test), and `codesign -f` on a file another process is signing or running failed 11 of 20 such runs
# (Sprint 168). A binary that already holds the entitlement is run as it is.
signed() { codesign -d --entitlements - "$binary" 2> /dev/null | grep -q com.apple.security.hypervisor; }
# Checked once a build (Sprint 332): `codesign -d` took 9.78-9.96 s on a signed test binary on this Mac, idle, and the
# runner starts each test binary in a process a test. Once the binary is found or made signed, the stamp beside it
# records the binary's modification time to the nanosecond; a launch whose binary still has that time skips the check.
# A relinked or replaced binary has another time and is checked and signed again. The time is compared for equality,
# not by `-nt`: this /bin/sh, bash 3.2, compares whole seconds, and a binary relinked within the second its stamp was
# written read as not newer.
stamp="$binary.signed"
stamped() { [ -f "$stamp" ] && read -r at < "$stamp" && [ "$at" = "$(stat -f %Fm "$binary")" ]; }
if ! stamped; then
  lock="$binary.signing"
  until mkdir "$lock" 2> /dev/null; do sleep 0.05; done
  trap 'rmdir "$lock"' EXIT
  if ! stamped; then
    signed || codesign -s - -f --entitlements "$(dirname "$0")/../tools/vm_monitor/entitlements.plist" "$binary" 2> /dev/null
    stat -f %Fm "$binary" > "$stamp"
  fi
  rmdir "$lock"
  trap - EXIT
fi
exec "$binary" "$@"
