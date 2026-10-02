#!/usr/bin/env python3
"""Index legacy and schema-v3 benchmark artifacts without rewriting evidence."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEGACY_DIRS = [ROOT / "bounds-check-bench", ROOT / "matmul-paired-bench"]


def load(path):
    try:
        return json.loads(path.read_text()), None
    except Exception as error:
        return None, str(error)


def legacy_rows():
    for directory in LEGACY_DIRS:
        for path in sorted(directory.glob("*.json")):
            data, error = load(path)
            if error:
                yield path, "unparseable", error[:60]
                continue
            if not isinstance(data, dict) or "measurements" not in data:
                continue
            gaps = []
            software = data.get("software", {})
            if not software.get("naga_git_rev") and not data.get("sources"):
                gaps.append("no-source-rev")
            if software.get("wgpu_version") in ("0.1.0", None, ""):
                gaps.append("junk-wgpu-version")
            measurements = data["measurements"]
            if not any("ci_95_median" in item for item in measurements):
                gaps.append("no-ci")
            rounds = max((item.get("n_samples", 0) for item in measurements), default=0)
            if rounds < 200:
                gaps.append(f"low-n={rounds}")
            yield path, f"legacy-{data.get('schema_version', 'missing')}", ",".join(gaps)


def v3_rows():
    for path in sorted((ROOT / "runs").glob("*/*.json")):
        data, error = load(path)
        if error:
            yield path, "unparseable", error[:60]
            continue
        schema = data.get("schema_version")
        if schema != "3.0.0":
            yield path, f"schema-{schema or 'missing'}", "not-v3"
            continue
        status = data.get("status", "missing")
        gaps = []
        sources = data.get("sources", [])
        if not sources or any(not item.get("revision") for item in sources):
            gaps.append("no-source-rev")
        if any("dirty" not in item or not item.get("diff_sha256") for item in sources):
            gaps.append("incomplete-dirty-state")
        if not data.get("static_artifacts"):
            gaps.append("no-static-artifacts")
        if status == "completed":
            if not data.get("correctness", {}).get("passed"):
                gaps.append("correctness-failed")
            if data.get("headline_rounds", 0) < 200:
                gaps.append(f"low-n={data.get('headline_rounds', 0)}")
            if not data.get("negative_control_includes_zero"):
                gaps.append("negative-control-failed")
            if not data.get("paired_samples"):
                gaps.append("no-raw-pairs")
            if not data.get("valid_for_claims"):
                gaps.append("not-valid-for-claims")
        elif status == "environment_blocked":
            if not data.get("environment_error"):
                gaps.append("missing-environment-error")
        else:
            gaps.append(f"status={status}")
        yield path, f"v3-{status}", ",".join(gaps)


rows = [*legacy_rows(), *v3_rows()]
print("| File | Class | Gaps or state |")
print("|---|---|---|")
for path, classification, gaps in rows:
    print(f"| `{path.relative_to(ROOT)}` | {classification} | {gaps or 'clean'} |")

legacy = sum(classification.startswith("legacy-") for _, classification, _ in rows)
blocked = sum(classification == "v3-environment_blocked" for _, classification, _ in rows)
completed = sum(classification == "v3-completed" for _, classification, _ in rows)
claimable = sum(
    classification == "v3-completed" and not gaps
    for _, classification, gaps in rows
)
print()
print(
    f"{legacy} legacy result files; {completed} completed v3 runs; "
    f"{blocked} environment-blocked v3 runs; {claimable} claimable runs."
)
