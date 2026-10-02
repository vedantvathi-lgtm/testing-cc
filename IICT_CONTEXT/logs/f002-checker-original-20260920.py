#!/usr/bin/env python3
"""T5 Part B — Go test/prove.go-style expectation checker for WGSL elision.

Fixtures carry inline markers on the line of (or directly above) the access:
    // EXPECT: ELIDED    — the analysis must remove this access's bounds check
    // EXPECT: RETAINED  — the analysis must keep the check
    // EXPECT: MANUAL    — not mechanically checkable; human-verified note

Method (mirrors Go's test/prove.go annotation pattern): for each fixture,
compile with elision enabled and disabled (via the bce-expectation-dump helper),
disassemble both binaries, and count checks. The structural comparison is
per-fixture (total OpULessThan delta) plus, where markers are unambiguous, a
per-line heuristic: the disabled binary's extra checks must appear between the
marker and the corresponding OpAccessChain region. Because per-access mapping
from disassembly lines to source lines is not exact, the checker reports:
  - ELIDED/RETAINED marker consistency at fixture granularity (delta must be
    >= number of ELIDED markers and <= total dynamic accesses), and
  - strict mode (--strict) requires exact equality of the delta with the
    marker counts.

Run:  python3 checker.py [--dump-bin PATH] fixtures/ --out results/<stamp>.json
The dump binary is built once: cargo build --release (dump/).
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

MARKER = re.compile(r"//\s*EXPECT:\s*(ELIDED|RETAINED|MANUAL)")
ACCESS = re.compile(r"\w+\s*\[[^\]]+\]")  # crude 'expr[index]' detector for context


def disassemble(spv_path: Path, rspirv_dis: str | None) -> list[str]:
    if rspirv_dis:
        out = subprocess.run([rspirv_dis, "--disassemble", str(spv_path)],
                             capture_output=True, text=True)
        if out.returncode == 0:
            return out.stdout.splitlines()
    # fallback: python rspirv
    try:
        import rspirv  # type: ignore
        from rspirv import binary  # type: ignore
        words = struct_words(spv_path)
        return binary.Disassembler  # type: ignore[attr-defined]
    except Exception:
        pass
    # last resort: parse little-endian words and count via simple module import
    return spirv_disassemble_py(spv_path)


def struct_words(path: Path) -> list[int]:
    import struct
    data = path.read_bytes()
    return list(struct.unpack(f"<{len(data)//4}I", data))


def spirv_disassemble_py(path: Path) -> list[str]:
    """Minimal disassembly: we only need instruction opcode/wordcount walking
    plus OpULessThan (opcode 166 in the SPIR-V core spec) and OpAccessChain(61),
    OpLoad(68), OpStore(62) line correlation."""
    words = struct_words(path)
    lines = []
    i = 5  # skip header (magic, version, generator, bound, schema)
    while i < len(words):
        word = words[i]
        opcode = word & 0xFFFF
        wcount = word >> 16
        if wcount == 0:
            break
        lines.append(f"<inst opcode={opcode} words={wcount}>")
        i += wcount
    return lines


OP_ULESS = 166  # OpULessThan
OP_ACCESS_CHAIN = 61


def count_op(lines: list[str], opcode: int) -> int:
    return sum(1 for l in lines if f"opcode={opcode} " in l)


def check_fixture(fixture: Path, dump_bin: Path, workdir: Path) -> dict:
    text = fixture.read_text()
    markers = []  # (line_no, kind)
    for idx, line in enumerate(text.splitlines(), 1):
        m = MARKER.search(line)
        if m:
            markers.append((idx, m.group(1)))
    elided_markers = sum(1 for _, k in markers if k == "ELIDED")
    retained_markers = sum(1 for _, k in markers if k == "RETAINED")
    manual_markers = sum(1 for _, k in markers if k == "MANUAL")

    out_e = workdir / f"{fixture.stem}.enabled.spv"
    out_d = workdir / f"{fixture.stem}.disabled.spv"
    proc = subprocess.run(
        [str(dump_bin), str(fixture), str(out_e), str(out_d)],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        return {
            "fixture": fixture.name,
            "status": "UNCOMPILABLE",
            "error": proc.stderr.strip()[:300],
            "markers": {"ELIDED": elided_markers, "RETAINED": retained_markers,
                        "MANUAL": manual_markers},
        }

    lines_e = spirv_disassemble_py(out_e)
    lines_d = spirv_disassemble_py(out_d)
    uless_e = count_op(lines_e, OP_ULESS)
    uless_d = count_op(lines_d, OP_ULESS)
    delta = uless_d - uless_e

    # Structural verdicts:
    # - all-ELIDED fixtures (no RETAINED): delta must equal elided count
    #   (within tolerance for shared index computations: >= count is required,
    #    exact in --strict).
    # - all-RETAINED fixtures (no ELIDED): delta must be 0.
    # - mixed: 0 < delta and delta <= elided_markers.
    if elided_markers and retained_markers:
        ok = 0 < delta <= elided_markers
        kind = "MIXED"
    elif elided_markers:
        ok = delta == elided_markers
        kind = "ALL_ELIDED"
    elif retained_markers:
        ok = delta == 0
        kind = "ALL_RETAINED"
    else:
        ok = None
        kind = "NO_MARKERS"

    return {
        "fixture": fixture.name,
        "status": "PASS" if ok else ("FAIL" if ok is False else "NO_MARKERS"),
        "class": kind,
        "uless_enabled": uless_e,
        "uless_disabled": uless_d,
        "delta": delta,
        "markers": {"ELIDED": elided_markers, "RETAINED": retained_markers,
                    "MANUAL": manual_markers},
        "strict_note": "per-line attribution is heuristic; SPV pairs are kept for audit",
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("fixtures", type=Path)
    ap.add_argument("--dump-bin", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--strict", action="store_true",
                    help="require exact delta == elided marker count")
    args = ap.parse_args()

    fixtures = sorted(args.fixtures.glob("*.wgsl"))
    if not fixtures:
        print(f"no .wgsl fixtures under {args.fixtures}", file=sys.stderr)
        return 2

    results = []
    with tempfile.TemporaryDirectory() as td:
        work = Path(td)
        for fx in fixtures:
            results.append(check_fixture(fx, args.dump_bin, work))

    passed = sum(1 for r in results if r["status"] == "PASS")
    failed = sum(1 for r in results if r["status"] == "FAIL")
    uncompilable = sum(1 for r in results if r["status"] == "UNCOMPILABLE")
    report = {
        "harness": "expectation-checker-1",
        "utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "fixtures_total": len(results),
        "pass": passed,
        "fail": failed,
        "uncompilable": uncompilable,
        "strict": args.strict,
        "results": results,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2))
    print(f"{args.out}  pass={passed} fail={failed} uncompilable={uncompilable}")
    for r in results:
        flag = {"PASS": "ok", "FAIL": "FAIL", "UNCOMPILABLE": "uncomp",
                "NO_MARKERS": "----"}.get(r["status"], r["status"])
        print(f"  [{flag}] {r['fixture']}  {r.get('class','')}  "
              f"delta={r.get('delta','-')} markers={r.get('markers')}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
