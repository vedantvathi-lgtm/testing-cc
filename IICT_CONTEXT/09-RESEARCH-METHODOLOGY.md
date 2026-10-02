# IICT research methodology — acquisition → extraction → verification → citation

> Binding companion to `../AGENTS.md` (which remains the core protocol). This file
> defines the *how* for reference work so every agent produces identically-shaped,
> provenance-complete research artifacts. Imported from the proven og-segfault
> workflow (fact-check agent pattern), adapted to this repo's evidence vocabulary.

## 1. The five stages

```
ACQUIRE → VERIFY → EXTRACT → READ/CLAIM → CITATION-READY
```

1. **ACQUIRE** — get the artifact into `references/raw/` by URL, with the HTTP
   status recorded. Preference order (never skip a higher tier because a lower
   one is easier):
   - **T1 author/canonical host** (author's page, publisher landing, standards body)
   - **T2 institutional/organization host** (university, project site, official archive)
   - **T3 recognized open mirrors** (arXiv, CiteSeerX, author-site copies of co-authors)
   - **T4 web.archive.org** — allowed, but the record must say `SUPERSEDED-SOURCE`
     with snapshot URL + timestamp; the original URL's failure is recorded too
   - **NEVER**: unverified mirrors, sci-hub, rehosted PDFs of unknown origin,
     "looks plausible" URLs (precedent: the discarded Cousot guess of 2026-09-13)
2. **VERIFY** — the artifact is what it claims: `file` says PDF (not HTML error
   shell), `pdfinfo` page count is sane, first-page text matches the expected
   title/authors, and for HTML the `<title>` or body contains the expected
   phrase. Compute SHA256 immediately.
3. **EXTRACT** — verbatim text only:
   `python3 references/extract_pdf.py <raw-file> <slug> [--ocr]`
   produces `references/extract/<slug>/<slug>.txt` with `=== [Page N] ===`
   markers and an `EXTRACTION.md` provenance record. Scanned PDFs need `--ocr`
   (pdftoppm 200 dpi + tesseract); OCR text is marked as OCR in the record.
   HTML sources get tag-stripped text through the same tool.
4. **READ/CLAIM** — read the *extracted text page by page*. Every factual
   sentence in a research note carries a citation of the form
   `(slug.txt p.N)` or `(file.rs:L-L)` or `(run.json field)`. If a claim cannot
   be anchored, it is written as `UNVERIFIED` or not written at all.
5. **CITATION-READY** — update `references/REFERENCE_STATUS.md`,
   `CITATION_MANIFEST.md` and `references/SHA256SUMS.new` in the same session.
   Status becomes `VERIFIED-ARCHIVED`; a T4-sourced item stays marked
   `SUPERSEDED-SOURCE`.

## 2. Research notes format (`research_notes/*.md`)

Every note opens with:

```markdown
# <Topic>

Verified <date>. Every factual sentence below carries a citation to a page
of an extracted reference, a source file, or a measurement artifact. Where
verification failed, the verdict is UNVERIFIED and nothing was filled in
by inference.
```

Then a **verdict table**:

```markdown
| # | Claim | Verdict |
|---|-------|---------|
| 1 | ...   | SUPPORTED / PARTIALLY SUPPORTED / CONTRADICTED / UNVERIFIED |
```

Then per-claim sections quoting the anchor evidence. End the note with
**"Deltas licensed"**: the concrete paper-section changes the verified claims
permit (e.g. "§X may now say Y, citing slug p.N"). Notes must not contain
numbers recalled from anywhere else — only copied anchors.

## 3. Failure handling (per AGENTS.md Rule 0)

- A failed download is recorded with the HTTP status or error text in
  `REFERENCE_STATUS.md`. No substitution without T1–T4 due diligence.
- Rate limiting (HTTP 429, bot challenges) is a *temporary* condition:
  space retries out, then record if still blocked.
- Anti-bot challenge pages (Anubis/Cloudflare "Just a moment") make the
  endpoint effectively unusable for agents; try a different tier, else record.
- Wayback throttle shells (HTTP 503 HTML ~11.8 KB with internetarchive
  chrome) must be detected by content type — never saved as the artifact.
- A source that contradicts the paper's current claim is not "wrong"; it is
  a finding. Log it in `01-CHRONOLOGY.md` discrepancies and fix the claim.

## 4. Operational cautions (learned 2026-09-20)

- DBLP API: behind an anti-bot challenge (empty body + challenge HTML) —
  do not treat an empty parse as "no results".
- ACM DL: HTTP 403 Cloudflare challenge to scripted fetches.
- Semantic Scholar API: usable, JSON, states `openAccessPdf.status` (W07 = CLOSED).
- `reviews.llvm.org` (Phabricator archive): served HTTP 200 on 2026-09-20 after
  a 2026-08/09 run of 429s; retry rather than assume permanent rate-limiting.
- ssw.jku.at: TLS broken from this environment (3 failures across 3 sessions);
  plain HTTP only 301-redirects back to HTTPS. Use the co-author site
  (christianwimmer.at, plain HTTP) or Wayback.
- archive.org: throttle generously (≥20 s between calls); the
  `availability` API can 429 while `cdx/search` still works.
- Wayback raw content: use `id_` suffix (`/web/<ts>id_/<url>`); modern
  error shells masquerade as pages.
