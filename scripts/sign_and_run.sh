#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The cargo runner on this Mac (.cargo/config.toml, Sprint 132): signs a binary ad hoc with the Hypervisor
# entitlement, as src/w2m/vm_backend.py signs vm_monitor, then runs it with its arguments. hv_vm_create refuses an
# unsigned process (HV_DENIED; WORKING_AGREEMENT, the Hypervisor.framework bridge mapping).
set -eu
binary=$1
shift
codesign -s - -f --entitlements "$(dirname "$0")/../tools/vm_monitor/entitlements.plist" "$binary" 2> /dev/null
exec "$binary" "$@"
