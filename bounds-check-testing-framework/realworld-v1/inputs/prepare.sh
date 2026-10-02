#!/usr/bin/env bash
# Decode the webgpu-samples image used by the blur workloads (stdlib-only decoder, cross-checked
# against PIL on 2026-09-29: identical sha256 3d5b048d...).
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
python3 "$HERE/png_to_rgba.py" "$HERE/../../../references/raw/webgpu-samples/public/assets/img/Di-3d.png" "$HERE/Di-3d.rgba" > "$HERE/Di-3d.json"
cat "$HERE/Di-3d.json"
