# Reference archive status

Access date: 2026-08-27 (Asia/Kolkata). Hashes are in `SHA256SUMS`.

## Verified downloads

- WGSL, WebGPU, SPIR-V, and Vulkan specifications: HTTP 200.
- ABCD PDF, Würthinger 2009 PDF, Readyset, Rust Performance Book, GSOHC landing page, and wgpu issue #6345: HTTP 200.
- Go `prove.go` and `test/prove.go` HTML snapshots: HTTP 200; sparse Go checkout at commit `026391c1b9eacdf10a734f2bdf1f99e63500ab4c`.
- LLVM sparse checkout at commit `39e3e92e4cce60990594ef36d8b0427c4ea0ad16`.

## Link caveats

- The old LLVM ScalarEvolution documentation URL returned HTTP 404; use the archived source checkout and current LLVM documentation navigation.
- The LLVM RFC endpoint returned HTTP 429 during this audit; it is listed but not treated as a downloaded source.
- The historical Go `loopbce.go` URL returned HTTP 404; current source is in the sparse checkout and should be located by symbol.
- The 2007 JKU page failed TLS verification in this environment; the 2009 author PDF is archived.
- `references/raw/wgpu.git` is a shallow upstream checkout; it is reference-only and not project evidence.

## Addendum — 2026-09-13 audit (verified this session)

- Go sparse checkout extended: `loopbce.go` has MOVED upstream — it now lives at
  `src/cmd/compile/internal/ssacompile/loopbce.go` (not `internal/ssa/`). The
  literature map's `src/cmd/compile/internal/ssa/loopbce.go` path is stale.
  Checkout updated from `026391c1b9ea` to `9834516` (master, forced update 2026-09-13).
  `test/prove.go` (2,967 lines) and `test/loopbce.go` (509 lines) oracles added to checkout.
- LLVM checkout contains `llvm/lib/Transforms/Scalar/InductiveRangeCheckElimination.cpp`
  plus `llvm/test/Transforms/IRCE/*.ll` (verified present). SCEV at
  `llvm/lib/Analysis/ScalarEvolution.cpp` verified present.
- New clone: `references/raw/webgpu-samples` (shallow, github.com/webgpu/webgpu-samples,
  75 .wgsl files) — restores the external corpus used by `corpus_hit_rate` RESULTS.md,
  which previously lived only in /tmp and was lost.
- STILL MISSING (recorded, not silently substituted):
  - Würthinger 2007 PPPJ PDF: TLS failure to ssw.jku.at (both 2026-08-27 and 2026-09-13).
    A 2009-version fallback exists locally. Do not cite W07 page contents from memory.
  - Cousot & Cousot POPL 1977 PDF: no verified direct URL on record; a guessed URL
    returned an HTML error page and was discarded. Obtain via a verified source
    (ACM DL / di.ens.fr listing) before citing beyond bibliographic metadata.
  - LLVM IRCE RFC thread: rate-limited (HTTP 429 both attempts). Substantive content
    is covered by the archived IRCE source + tests; the RFC link remains citation-only.
- SHA256 of newly pinned key files recorded in `SHA256SUMS.new` (merge into SHA256SUMS
  after review).

## Addendum — 2026-09-20 acquisition sweep (all three gaps closed; verified this session)

All routes, HTTP statuses and hash records follow the methodology in
`IICT_CONTEXT/09-RESEARCH-METHODOLOGY.md`. Extraction records in
`references/extract/<slug>/EXTRACTION.md`.

- **Würthinger 2007 (W07) — ACQUIRED (T1, author-hosted).**
  `https://www.christianwimmer.at/Publications/Wuerthinger07a/Wuerthinger07a.pdf`
  (co-author Christian Wimmer's own site; plain HTTP only — HTTPS resets from this
  environment). HTTP 200, 250,226 bytes, PDF 1.4, 9 pages. First-page text verified:
  title + three authors + JKU Linz affiliation. SHA256 `53befa0c…1dd3e6` (full hash in
  `SHA256SUMS.new`). Archived: `references/raw/papers/Wuerthinger07a.pdf`.
  Failed routes this session (recorded): scispace mirror HTTP 202 bot-challenge
  (0 bytes); ssw.jku.at HTTPS HTTP 000 (TLS, 3rd failure); ACM DL HTTP 403
  Cloudflare; Semantic Scholar API `openAccessPdf.status: CLOSED`; Wayback CDX for
  `Wuerthinger07/Wuerthinger07.pdf` shows landing-page captures only (text/html),
  no archived PDF at that path.
- **Cousot & Cousot POPL 1977 — ACQUIRED (T1, author-hosted).**
  `https://www.di.ens.fr/~cousot/publications.www/CousotCousot-POPL-77-ACM-p238--252-1977.pdf`
  (link discovered inside the author's canonical landing page
  `di.ens.fr/~cousot/COUSOTpapers/POPL77.shtml`, HTTP 200). PDF 1.1, 16 pages,
  941,105 bytes, "Acrobat Distiller 3.0". **Scanned images, no text layer** —
  extracted via OCR (`references/extract/cousot-popl77/`). SHA256 `5d2956ee…a417238`.
  Archived: `references/raw/papers/CousotCousot-POPL-77-ACM-p238--252-1977.pdf`.
- **LLVM IRCE RFC thread (Phabricator D6693) — ACQUIRED.**
  `https://reviews.llvm.org/D6693` served HTTP 200 this session (previously 429).
  841,430 bytes; `<title>` verified: "⚙ D6693 New pass: inductive range check
  elimination"; 83,320 text chars extracted (full review discussion incl. the
  patch's file list). SHA256 `e74aa427…f5672398`. Archived:
  `references/raw/issues/llvm-irce-rfc-D6693.html`. Note: reviews.llvm.org is the
  official frozen archive of the discontinued Phabricator instance (stated in page
  header), so this is T1 for the RFC thread.
- Also archived: `references/raw/issues/christianwimmer-at-pubs-index.html`
  (HTTP-page evidence of the W07 source, 30,803 bytes).
- Extraction set created 2026-09-20 (`references/extract/`): abcd (114,973 chars,
  13 pp), wuerthinger07 (67,160 chars, 9 pp), wuerthinger09 (90,284 chars, 22 pp),
  gsohc (22,700), readyset-bounds-checks (19,213), rust-perf-book-bounds-checks
  (1,095 — chapter genuinely short), wgpu-6345 (4,885), llvm-irce-rfc-d6693
  (87,158), cousot-popl77 (OCR, in progress at write time).
