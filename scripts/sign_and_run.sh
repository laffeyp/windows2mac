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
if ! signed; then
  lock="$binary.signing"
  until mkdir "$lock" 2> /dev/null; do sleep 0.05; done
  trap 'rmdir "$lock"' EXIT
  signed || codesign -s - -f --entitlements "$(dirname "$0")/../tools/vm_monitor/entitlements.plist" "$binary" 2> /dev/null
  rmdir "$lock"
  trap - EXIT
fi
exec "$binary" "$@"
