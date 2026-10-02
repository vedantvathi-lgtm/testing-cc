#!/usr/bin/env python3
"""IICT reference extraction: archived PDF/HTML -> page-marked text.

Usage:
  python3 references/extract_pdf.py <input.pdf|input.html> <slug> [--ocr]

Output (per the research methodology, IICT_CONTEXT/09-RESEARCH-METHODOLOGY.md):
  references/extract/<slug>/<slug>.txt      page-marked text ("=== [Page N] ===")
  references/extract/<slug>/pages/*.png     (only with --ocr) rasterized pages
  references/extract/<slug>/EXTRACTION.md   provenance record (source file, tool, date, hash)

Rule: text is extracted verbatim. No summarization, no cleanup beyond
whitespace normalization inside lines. Page markers must match the source
page numbers so quotes in research notes can cite page N.
"""

import hashlib
import html as html_mod
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent  # workspace root
ARCHIVE = ROOT / "references" / "raw"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pdf_page_count(pdf: Path) -> int:
    out = subprocess.run(["pdfinfo", str(pdf)], capture_output=True, text=True).stdout
    m = re.search(r"^Pages:\s+(\d+)", out, re.M)
    return int(m.group(1)) if m else 0


def extract_pdf_textlayer(pdf: Path) -> list[str]:
    """Return per-page text using pdftotext (form-feed separated)."""
    out = subprocess.run(
        ["pdftotext", "-layout", str(pdf), "-"],
        capture_output=True, text=True,
    ).stdout
    pages = out.split("\f")
    if pages and pages[-1].strip() == "":
        pages.pop()
    return pages


def extract_pdf_ocr(pdf: Path, pages_dir: Path) -> list[str]:
    """Rasterize each page and OCR it. Slow; verbatim-ish; errors recorded."""
    n = pdf_page_count(pdf)
    if n == 0:
        return []
    pages_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["pdftoppm", "-r", "200", "-png", str(pdf), str(pages_dir / "page")],
        check=True,
    )
    texts = []
    for i in range(1, n + 1):
        img = pages_dir / f"page-{i:02d}.png"
        if not img.exists():
            # pdftoppm numbering can widen with page count
            img = pages_dir / f"page-{i}.png"
        if not img.exists():
            texts.append(f"[OCR ERROR: raster for page {i} missing]")
            continue
        import os
        env = dict(os.environ)
        # eng.traineddata is missing from the system tessdata; user-space copy
        # (see IICT_CONTEXT/09-RESEARCH-METHODOLOGY.md operational cautions)
        env.setdefault("TESSDATA_PREFIX", str(Path.home() / ".tessdata"))
        r = subprocess.run(
            ["tesseract", str(img), "-", "--psm", "3"],
            capture_output=True, text=True, env=env,
        )
        texts.append(r.stdout if r.returncode == 0 else f"[OCR ERROR page {i}: exit {r.returncode}]")
    return texts


def extract_html(h: Path) -> str:
    raw = h.read_text(errors="replace")
    txt = re.sub(r"<script.*?</script>", " ", raw, flags=re.S | re.I)
    txt = re.sub(r"<style.*?</style>", " ", txt, flags=re.S | re.I)
    txt = re.sub(r"<[^>]+>", " ", txt)
    txt = html_mod.unescape(txt)
    txt = re.sub(r"[ \t]+", " ", txt)
    txt = re.sub(r"\n\s*\n+", "\n\n", txt)
    return txt


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    src = Path(sys.argv[1]).resolve()
    slug = sys.argv[2]
    ocr = "--ocr" in sys.argv[3:]

    if not src.exists():
        print(f"ERROR: source not found: {src}")
        return 1

    out_dir = ROOT / "references" / "extract" / slug
    out_dir.mkdir(parents=True, exist_ok=True)

    if src.suffix.lower() == ".pdf":
        n = pdf_page_count(src)
        if ocr:
            pages = extract_pdf_ocr(src, out_dir / "pages")
            tool = "pdftoppm -r 200 + tesseract --psm 3"
        else:
            pages = extract_pdf_textlayer(src)
            tool = "pdftotext -layout"
        body = []
        for i, t in enumerate(pages, 1):
            body.append(f"=== [Page {i}] ===\n{t.rstrip()}\n")
        text = "\n".join(body)
        kind = "pdf"
    else:
        text = extract_html(src)
        n = 0
        tool = "python3 html->text (scripts stripped, tags stripped, entities unescaped)"
        kind = "html"

    out_txt = out_dir / f"{slug}.txt"
    out_txt.write_text(text, encoding="utf-8")

    note = f"""# Extraction record — {slug}

- Source: `{src.relative_to(ROOT)}` ({kind}, {src.stat().st_size} bytes)
- SHA256: `{sha256(src)}`
- Tool: {tool}
- Pages: {n if kind == 'pdf' else 'n/a (html)'}
- Text chars: {len(text)}
- Extracted (UTC): {datetime.now(timezone.utc).isoformat(timespec='seconds')}
- Output: `{out_txt.relative_to(ROOT)}`
- Rule: verbatim text only; quotes in research notes cite `{{slug}}.txt` page markers.
"""
    (out_dir / "EXTRACTION.md").write_text(note, encoding="utf-8")
    print(f"OK {slug}: {len(text)} chars, {n or '-'} pages -> {out_txt}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
