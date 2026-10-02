#!/usr/bin/env python3
"""T5 Part B — Go test/prove.go-style expectation checker for WGSL bounds-check elision.

Fixtures carry inline markers on the line of the access they describe:
    // EXPECT: ELIDED    — every runtime access on this line must be proven in bounds
    // EXPECT: RETAINED  — no runtime access on this line may be proven in bounds
    // EXPECT: MANUAL    — reported only; never pass/fail

Verdicts come from the compiler's own proof set (`FunctionInfo.safe_accesses`),
per access, via `bce-expectation-dump --proofs`, which prints one JSON object
per runtime `Access` with its source line. A marker on a line that has no
runtime access is an ERROR, so a fixture cannot pass vacuously.

As a secondary structural check, each fixture also reports the emitted
`OpULessThan` count with elision enabled vs disabled (opcode 176 in the SPIR-V
core grammar; spirv-0.4.0+sdk-1.4.341.0 autogen_spirv.rs:3772).

Run:  python3 checker.py fixtures/ --dump-bin dump/target/release/bce-expectation-dump \
          --out results/expect-<stamp>.json
Exit status: 0 only if every marked access matches and nothing errored.
"""

import argparse
import json
import re
import struct
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

HARNESS_VERSION = "expectation-checker-2"
MARKER = re.compile(r"//\s*EXPECT:\s*(ELIDED|RETAINED|MANUAL)")
OP_U_LESS_THAN = 176


def count_opcode(spv_path: Path, opcode: int) -> int:
    data = spv_path.read_bytes()
    words = struct.unpack(f"<{len(data) // 4}I", data)
    count, i = 0, 5  # skip the 5-word module header
    while i < len(words):
        word_count = words[i] >> 16
        if word_count == 0:
            break
        count += (words[i] & 0xFFFF) == opcode
        i += word_count
    return count


def run_dump(dump_bin: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([str(dump_bin), *args], capture_output=True, text=True)


def check_fixture(fixture: Path, dump_bin: Path, workdir: Path) -> dict:
    markers = {}
    for line_no, line in enumerate(fixture.read_text().splitlines(), 1):
        match = MARKER.search(line)
        if match:
            markers[line_no] = match.group(1)

    proofs = run_dump(dump_bin, "--proofs", str(fixture))
    if proofs.returncode != 0:
        return {"fixture": fixture.name, "status": "UNCOMPILABLE",
                "error": proofs.stderr.strip()[:300]}
    by_line: dict[int, list[bool]] = {}
    for record in map(json.loads, proofs.stdout.splitlines()):
        by_line.setdefault(record["line"], []).append(record["safe"])

    verdicts = []
    for line_no, expect in sorted(markers.items()):
        safe = by_line.get(line_no, [])
        if not safe:
            verdict = "ERROR"  # marker without a runtime access on its line
        elif expect == "MANUAL":
            verdict = "MANUAL"
        elif expect == "ELIDED":
            verdict = "PASS" if all(safe) else "FAIL"
        else:
            verdict = "PASS" if not any(safe) else "FAIL"
        verdicts.append({"line": line_no, "expect": expect, "proven": safe,
                         "verdict": verdict})

    enabled, disabled = workdir / "enabled.spv", workdir / "disabled.spv"
    structural = run_dump(dump_bin, str(fixture), str(enabled), str(disabled))
    uless = None
    if structural.returncode == 0:
        uless = {"enabled": count_opcode(enabled, OP_U_LESS_THAN),
                 "disabled": count_opcode(disabled, OP_U_LESS_THAN)}

    judged = [v["verdict"] for v in verdicts if v["verdict"] != "MANUAL"]
    if not verdicts:
        status = "NO_MARKERS"
    elif "ERROR" in judged:
        status = "ERROR"
    elif "FAIL" in judged:
        status = "FAIL"
    else:
        status = "PASS"
    return {"fixture": fixture.name, "status": status, "verdicts": verdicts,
            "unmarked_runtime_accesses": sum(len(v) for k, v in by_line.items()
                                             if k not in markers),
            "op_u_less_than": uless}


def compiler_revision(dump_bin: Path) -> str:
    repo = dump_bin.resolve().parents[5] / "wgpu"  # <root>/bounds-check-testing-framework/expectation-checker/dump/target/release/<bin>
    head = subprocess.run(["git", "-C", str(repo), "rev-parse", "--short", "HEAD"],
                          capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(["git", "-C", str(repo), "status", "--porcelain", "--", "naga/src"],
                           capture_output=True, text=True).stdout.strip()
    branch = subprocess.run(["git", "-C", str(repo), "branch", "--show-current"],
                            capture_output=True, text=True).stdout.strip()
    return f"{branch}@{head}{' +uncommitted naga/src' if dirty else ''}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("fixtures", type=Path)
    parser.add_argument("--dump-bin", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    fixtures = sorted(args.fixtures.glob("*.wgsl"))
    if not fixtures:
        print(f"no .wgsl fixtures under {args.fixtures}", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory() as tmp:
        results = [check_fixture(f, args.dump_bin, Path(tmp)) for f in fixtures]

    counts = {s: sum(r["status"] == s for r in results)
              for s in ("PASS", "FAIL", "ERROR", "UNCOMPILABLE", "NO_MARKERS")}
    report = {"harness": HARNESS_VERSION,
              "utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
              "compiler": compiler_revision(args.dump_bin),
              "fixtures_dir": str(args.fixtures), "fixtures_total": len(results),
              **{k.lower(): v for k, v in counts.items()}, "results": results}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2))

    print(f"{args.out}  " + "  ".join(f"{k.lower()}={v}" for k, v in counts.items()))
    print(f"compiler: {report['compiler']}")
    for r in results:
        print(f"  [{r['status']}] {r['fixture']}  OpULessThan={r.get('op_u_less_than')}")
        for v in r.get("verdicts", []):
            print(f"      line {v['line']:3d} expect {v['expect']:8s} proven={v['proven']} -> {v['verdict']}")
    bad = counts["FAIL"] + counts["ERROR"] + counts["UNCOMPILABLE"]
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
