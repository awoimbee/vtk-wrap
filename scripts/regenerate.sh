#!/usr/bin/env bash
# Export generated files for inspection; UPDATE_DOCS=1 refreshes the Rust-only
# documentation snapshots as well. The generator is private build support.
#
# MODULES selects Cargo module features (default: the normal feature set).
# For snapshots, enable every vtk* module and use an installation with them all.
# VTK_WRAP_XML_DIR may point to existing XML instead of invoking WrapVTK.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
args=(--test generator)
if [[ "${UPDATE_DOCS:-0}" == 1 ]]; then
  modules="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; p=json.load(sys.stdin)["packages"][0]; print(",".join(sorted(k for k in p["features"] if k.startswith("vtk"))))')"
  args+=(--features "$modules")
elif [[ -n "${MODULES:-}" ]]; then
  args+=(--no-default-features --features "$MODULES")
fi

cargo test "${args[@]}" export_bindings -- --ignored --nocapture
