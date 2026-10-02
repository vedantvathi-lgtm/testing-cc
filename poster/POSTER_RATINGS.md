# Poster ratings: 35 LLVM Dev Meeting posters vs. `poster_A0.pdf` (IICT 2026 #55)

Date: 2026-10-01. Analyst session: poster-design rating subagent.
Inputs: `references/posters/llvm-devmtg/*/*.pdf` (MANIFEST.json), `poster/poster_A0.pdf`,
`poster/build_poster.py` (CSS), `IndiaFOSS-llvm-talk(1).pptx`.
Scratch renders: `/tmp/claude-1000/-home-bb-Projects-Dave-Sonar-WGPU--master/2d0bb526-b354-4893-b8ca-9b04d2002a5e/scratchpad/rating/`.

## 0. Evidence and method (AGENTS.md labels)

- **VERIFIED (this session):** every poster was rendered with `pdftoppm -scale-to 1100` and viewed. The user's poster and
  12 candidates (Lee, Rocha, CuSan, VAST, PoTATo, Preto, Fry, Mitenkov, Porpodas, Pichler, Lomuller, Albano) were also
  viewed at 1900 px. Crops at 60 dpi were viewed for the user's poster (5 regions) and for CuSan's hero.
- **VERIFIED:** page size, word count (`pdftotext`), fonts (`pdffonts`), and text heights (`pdftotext -bbox`)
  for all 36 PDFs. The scripts are `meta.sh`, `fs.py` and `lines.py` in the scratch dir.
- **Calibration (VERIFIED):** the word-bbox height is about 1.12 × font size. The user's CSS says body 25 pt and
  title 84 pt (`build_poster.py:42,50`). The measured bbox modes are 28 and 93.9, which gives ratios of 1.12 and 1.118.
  Every "est. pt" below is bbox ÷ 1.12, normalised to A0 by the long side. These are estimates, not exact font sizes.
- **Geometry percentages** (band heights, column splits) were read off the 1900 px renders by eye. Treat them as ±1 %.
- **DEVIATION:** I expected poppler to render every poster faithfully. It did not. Poppler drops Fry's (CIRCT ISS)
  gradient title band ("Singular matrix in shading pattern fill"), so the title is invisible. Evidence: `p24-1100-1.png`
  vs `fry_gs.png`. Consequence: Fry was rated from the Ghostscript render, where the title is present.
- **DEVIATION:** Mitenkov's poster is one raster image (9934×14044, `pdfimages -list`), so it has no text layer.
  Its word count of 0 is an artefact and its type sizes are judged by eye only.
  Fioravanti's PDF has 2 pages, and only page 1 was rated.
- **Scores are design judgements (opinion), not measurements.** The rubric is 1–10 on each criterion, and Overall is
  the mean of the seven. Where a poster shows no results at all, "Results" is scored 1–2, which pulls down a few
  good-looking WIP posters (VAST, PoTATo, Preto, Fry, Mitenkov). The note on each of those says so.

Rubric keys: **M** = 5-second message, **F** = reading flow, **H** = hierarchy and boundaries, **S** = show-don't-tell,
**D** = density and legibility at 1–2 m, **R** = results presentation, **A** = aesthetic cohesion.

## 1. Ranking table (all 35 + the user's poster)

| Rank | # | Poster (meeting) | Size | Words | Body est. pt | M | F | H | S | D | R | A | **Overall** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 10 | Rocha – Function merging by sequence alignment (2019-04) | A0 | 1122* | ~16 code / chart-heavy | 10 | 8 | 8 | 9 | 6 | 8 | 10 | **8.4** |
| 2 | 33 | Hück – CuSan host-device race sanitizer (2025-04) | A0 | 636 | 28–39 | 9 | 9 | 9 | 9 | 7 | 7 | 8 | **8.3** |
| 3 | 07 | Lee – Safely optimizing ptr/int casts (2019-04) | 1729×2591 | 554 | ~40–45 | 8 | 8 | 8 | 9 | 8 | 6 | 7 | **7.7** |
| 4 | 27 | Pichler – Execution modes of LLVM bitcode on GraalVM (2024-04) | A0 | 598 | ~33 (refs 13) | 7 | 7 | 7 | 8 | 7 | 7 | 7 | **7.1** |
| 5 | 26 | Lauko – VAST: MLIR compiler for C/C++ (2024-04) | A0 | 993 | ~18–27 | 8 | 8 | 8 | 9 | 5 | 1 | 9 | **6.9** |
| 6 | 16 | Preto – Automatic translation C++→Rust (2023-05) | A0 | 443 | ~24 | 7 | 8 | 8 | 8 | 7 | 2 | 8 | **6.9** |
| 7 | 03 | Garzella – Multi/cross-language verification (2019-04) | landscape | 241 | ~27 | 6 | 7 | 7 | 7 | 7 | 7 | 7 | **6.9** |
| 8 | 24 | Fry – ISS from HW description with CIRCT (2024-04) | 3344×4138 | 389 | ~33 | 7 | 9 | 8 | 8 | 6 | 1 | 8 | **6.7** |
| 9 | 15 | Albano – Vector predication in the LV (2023-05) | landscape | 444 | ~34 | 7 | 7 | 7 | 8 | 5 | 5 | 7 | **6.6** |
| 9 | 18 | Lomuller – MLIR for SYCL (2023-05) | A1 | 921 | ~14–22 | 7 | 8 | 7 | 8 | 4 | 6 | 6 | **6.6** |
| 9 | 28 | Shan – Fuzzlang (2024-10) | landscape small | 150 | ~68 | 7 | 6 | 6 | 6 | 8 | 7 | 6 | **6.6** |
| 12 | 17 | Mitenkov – Embedding MLIR into Python (2023-05) | A0 (raster) | n/a | by eye ~28 | 7 | 8 | 7 | 9 | 5 | 2 | 7 | **6.4** |
| 13 | 09 | Porpodas – SuperNode-SLP (2019-04) | A0 | 896 | ~23 | 6 | 6 | 7 | 9 | 5 | 5 | 6 | **6.3** |
| 13 | 25 | Konicar/Lauko – PoTATo (2024-04) | A0 | 1137 | ~21–27 | 7 | 7 | 8 | 7 | 5 | 1 | 9 | **6.3** |
| 15 | 22 | Chang – Phase ordering with off-policy DRL (2024-04) | 2592×3456 | 282 | ~46 | 5 | 6 | 6 | 6 | 7 | 6 | 6 | **6.0** |
| 16 | 21 | He – DSL compilation for FFT (2023-10) | landscape | 687 | ~25 | 5 | 6 | 7 | 7 | 5 | 4 | 7 | **5.9** |
| 16 | 30 | Chapman – Sub-FP8 RISC-V (2025-04) | A4 page | 529 | ~31 | 5 | 6 | 7 | 7 | 5 | 4 | 7 | **5.9** |
| 16 | 35 | Ledoux – Multi-level arithmetic opts (2025-04) | A0 | 1852 | ~13 | 6 | 7 | 8 | 8 | 3 | 1 | 8 | **5.9** |
| 19 | 14 | Zhou – Facebook JITed C++ (2019-04) | A0 | 126 | ~35 | 5 | 6 | 6 | 6 | 6 | 4 | 7 | **5.7** |
| 19 | 20 | Monteiro – Rust-like lifetime annotations (2023-05) | A0 | 569 | ~27 | 5 | 6 | 7 | 7 | 5 | 2 | 8 | **5.7** |
| **19** | — | **User: Bounds-check elision in Naga (IICT #55)** | A0 | 977 | **25 (CSS)** | 6 | 6 | 5 | 6 | 5 | 6 | 6 | **5.7** |
| 22 | 04 | Halkenhäuser – OpenMP backend for Polly (2019-04) | A0 | 998 | ~23 | 5 | 5 | 6 | 6 | 5 | 6 | 6 | **5.6** |
| 22 | 23 | Chang – VLIW RISC-V vector backend (2024-04) | A2 | 224 | ~41 | 5 | 5 | 6 | 7 | 6 | 5 | 5 | **5.6** |
| 22 | 32 | Fioravanti – TicTacToe vs RL engineers (2025-04) | A0, 2 pp | 651 (p1) | ~28 | 7 | 5 | 5 | 6 | 5 | 7 | 4 | **5.6** |
| 25 | 02 | Bispo – Clava (2019-04) | A0 | 407 | ~15–18 | 4 | 5 | 6 | 6 | 4 | 4 | 6 | **5.0** |
| 26 | 08 | López-Gómez – C++ contracts in Clang (2019-04) | A0 | 1028 | ~26 | 4 | 5 | 6 | 5 | 4 | 4 | 6 | **4.9** |
| 27 | 06 | Her – Splendid GVN (2019-04) | A0 | 134 | ~47 | 4 | 5 | 5 | 6 | 6 | 3 | 4 | **4.7** |
| 27 | 29 | Wang – MLIR & PyTorch → Ascend (2024-10) | 2592×3456 | 933 | ~11–17 | 5 | 6 | 6 | 6 | 3 | 1 | 6 | **4.7** |
| 29 | 19 | Popescu – Perf impact of exploiting UB (2023-05) | A0 | 725 | ~20 | 4 | 5 | 5 | 5 | 4 | 5 | 4 | **4.6** |
| 30 | 34 | Khan – SonicMachine (2025-04) | 720×900 | 298 | ~31 | 4 | 4 | 4 | 6 | 6 | 1 | 4 | **4.1** |
| 31 | 11 | Vu – Security annotations (2019-04) | A0 | 910 | ~20 | 4 | 5 | 5 | 5 | 3 | 2 | 4 | **4.0** |
| 32 | 05 | Hameeza – LLVM Miner (2019-04) | A0 | 172 | ~64 | 3 | 4 | 4 | 4 | 6 | 3 | 3 | **3.9** |
| 32 | 31 | Erasmie-Jones – NektarIR (2025-04) | A0 | 1072 | ~25 | 4 | 5 | 5 | 5 | 3 | 1 | 4 | **3.9** |
| 34 | 13 | Yerawar – Polly in Chapel (2019-04) | A0 | 772 | ~30 | 3 | 4 | 4 | 3 | 4 | 3 | 4 | **3.6** |
| 35 | 12 | Wahlster – SPMD control flow (2019-04) | A0 | 1118 | ~16–20 | 3 | 4 | 4 | 6 | 2 | 1 | 3 | **3.3** |
| 36 | 01 | Mishra – Instruction tracing (2019-04) | A0 | 655 | ~18 | 3 | 3 | 3 | 5 | 3 | 2 | 2 | **3.0** |

\*Rocha's word count is mostly IR tokens inside figures. Its prose is under 100 words.
Body est. pt is the most common word-bbox height ÷ 1.12 (`fs.py`). For the user's poster it is the CSS value.

## 2. Per-poster notes (short justification)

- **10 Rocha (8.4).** The yellow title band is about 24 % of the height. The 156 pt all-caps title doubles as the
  illustration: a DNA strand stands for "sequence alignment". A red sticker, "up to 25 % smaller code", overlaps the
  band/body boundary. In the body, a diagonal grey band on near-black carves the poster into a "how it works" river
  (left to right, top to bottom) and an Evaluation corner. Colour carries meaning from start to end: green IR lines
  merge, red lines don't. That semantic holds from the alignment matrix to the generated code. Each result chart gets a
  red badge with its average ("2 %", "15 %", "6 % smaller"). Weak points: the charts' axis text is tiny (~10 pt) and
  the evaluation section is cramped.
- **33 CuSan (8.3).** The title block is about 25 % of the height (title ≈ 155 pt, URL ≈ 62 pt, authors ≈ 46 pt).
  The hero (29–48 %) is the bug report itself: racy CUDA source on the left, the TSan report on the right, and two
  thick blue curved arrows tying each report line to the source line that caused it. Below that, numbered
  full-width-colour header bars (1–4, ≈ 54 pt white on blue) sit on white with no card borders. Thin rules separate
  the sections. Every section pairs a code listing with a small timeline diagram ("Host Thread"). The results are one
  bar chart with numbers printed on the bars. The footer is three QR "cards" for the repo, a related tool and the paper.
- **07 Lee (7.7).** The navy band is about 13.5 %. Two columns are split by a heavy vertical rule. Section headers
  are full-column light-grey bars with centred bold black text. The left column is a miscompilation *chain*: 4 code
  boxes linked by fat blue block arrows, each arrow labelled with the pass name ("int. eq. prop.", "cast elim.",
  "constant prop."), plus red callout flags ("Wrong if integer carries provenance"). Pink highlights mark the changed
  line. A dashed red arrow traces the value flow inside the last box. Code is about 40 pt. Results are one small chart
  and a 4-line bullet list. That is honest but visually weak.
- **27 Pichler (7.1).** A white header holds a big 104 pt title and four author columns. Below are blue-outlined
  boxes in rows of 2 with unequal widths, so the content sets the width, not a rigid grid. "Our Approach: Hybrid
  Execution" is a green/blue two-zone diagram whose colours match the "Managed / Native" headings. The question-shaped
  section titles ("Why not only native execution?") are strong. The references block at 13 pt is wasted space.
- **26 VAST (6.9).** There is no band. The title (with a logo) takes about 9 %, then a three-column "VAST is a
  program-analysis-focused compiler" claim strip (10–25 %). Below is a 1/3 + 2/3 grid of two rows. Section titles are
  as large as the main title, in a condensed light face. The "Compilation" diagram is a coloured *tower* of IR layers
  with vertical Provenance and Decompilation bands crossing it, which is a true dominant visual. The "Tower of IRs"
  code has hairline connector arrows from source lines to IR lines. The palette is strict: grey-blue panels and three
  reds. A diagonal "Fork me on GitHub" ribbon sits in the corner. The body copy is ~20 pt italic, too small. There are
  no results.
- **16 Preto (6.9).** One column of 5 numbered rounded panels, full width. Each panel is bullets on the left and
  before→after code on the right, separated by big grey double chevrons, with a grey pill caption under each code
  block. A dotted green connector runs from panel 4 to panel 5. Highlights are lime-green keywords. The "2. GOAL"
  panel is inverted (grey fill, white text) to stand out. There are no results, since it is WIP.
- **03 Garzella (6.9).** Landscape. Four equal columns (Problem, Methodology, Microbenchmarks, Conclusions), then a
  full-width toolflow with brace-joined IR arrows, and a red/green language × feature result matrix. The matrix is
  readable from 3 m. It has the fewest words (241) of the strong posters.
- **24 Fry (6.7, Ghostscript render).** Gold pill headers sit on rounded white panels over a teal gradient. The
  "Algorithm" panel is the standout: 4 numbered steps placed in a *cycle* with curved arrows and red edge labels
  ("After LATENCY cycles"), with dotted call-outs holding small IR snippets. The Example panel joins two inputs with a
  brace, then a merge arrow, then the output, labelled in 40 pt bold. The top-left Overview is a justified paragraph
  wall. There are no results.
- **15 Albano (6.6).** Landscape, dark frame. A text sidebar carries the narrative and the centre shows two
  basic-block → DDG transformations with big arrows. A bold one-sentence take-home strip sits at the bottom. The diff
  panels on the right are unreadable at distance.
- **18 Lomuller (6.6).** A1, black title band. Under the title is a **3-step chevron strip** ("MLIR dialect → host-device
  representation → high-level SYCL optimizations"), which states the whole poster in one row. Below are dense boxed
  panels with small code (~14–22 pt) and a simple bar chart.
- **28 Shan (6.6).** A small landscape sheet with very large text, a sketch-style workflow figure, and 2 bar charts
  with values printed on the bars. It is clear but plain.
- **17 Mitenkov (6.4).** Two columns of numbered 1–6 headers on sky blue. The best **code annotation** in the set:
  handwritten-style teal callouts with curved arrows pointing at exact tokens ("Non-SSA form", "Mapping MLIR types to
  Python types"). A yellow "lightbulb" question pill splits sections 2 and 3. Section 4 is dense code with no
  explanation. There are no results, but there is an honest "Open questions remain" box.
- **09 Porpodas (6.3).** Diagram-first: expression trees with SLP vs SN-SLP rows and "Vectorized!" / "Not
  vectorized" vertical tags. A 4-step "We are upstreaming" ladder with a "Checked in ✓". The charts are tiny.
- **25 PoTATo (6.3).** The same system as VAST (three-goal strip, dark-red stacked pipeline, Oswald-like headers). The
  body is italic 20 pt prose in many places. There are no results.
- **22 Chang (6.0).** Clear setup and results, with a big horizontal bar chart. The headings are thin pill bars and
  the layout is generic.
- **21 He, 30 Chapman (5.9).** Tidy uniform header bars and good diagrams (He's FFT lowering pipeline with numbered
  steps), but no dominant visual, and the result charts are thumbnails.
- **35 Ledoux (5.9).** Beautiful typographic system (Biolinum, Inria-style headers) and an end-to-end compilation
  flow with step labels. But at 1852 words and ~13 pt body it is a paper on a wall, with no measured results.
- **14 Zhou (5.7).** Minimal grey, centred section titles with hairlines. It is so sparse that the diagrams look
  unfinished, and its two charts have no axes.
- **20 Monteiro (5.7).** A cohesive pill-header system on cream, but it is a code-and-bullets tutorial with lots of
  dead space and no results.
- **04 Halkenhäuser (5.6).** A LaTeX baposter: blue band, 6 bar charts with tiny unreadable axes, and figure numbers.
- **23 Chang VLIW (5.6).** A good architecture diagram, but heavy outlined boxes and an algorithm block typeset like
  a paper.
- **32 Fioravanti (5.6).** The question title hooks the reader, and the speedup table is the clearest results table
  in the set. The layout is LaTeX-default with a weak hierarchy.
- **02 Bispo (5.0).** A clean maroon header, but the body is a two-column slide dump with 15–18 pt code.
- **08 López-Gómez (4.9).** Fira Thin body over a faint watermark makes for low contrast. Prose-heavy.
- **06 Her (4.7).** Rounded grey panels and Roman numerals. The CFG diagrams are fine. Times and default chart
  styling hurt it.
- **29 Wang (4.7).** Four stages are well identified, but the code walls are at ~11 pt and there are no results.
- **19 Popescu (4.6).** Four near-identical CDF plots and a wordy flag table.
- **34 Khan (4.1).** The logo is the hero. The poster has no title-message and no results.
- **11 Vu, 12 Wahlster (4.0, 3.3).** Paper layouts (Wahlster even has an abstract) at ~16–20 pt.
- **05 Hameeza (3.9).** Huge justified text, and charts with no message.
- **31 Erasmie-Jones (3.9).** A blue-band template with a dense background paragraph.
- **13 Yerawar (3.6).** A wall of text boxes and 12 thumbnail charts.
- **01 Mishra (3.0).** Unreadable trace dumps, mixed fills and no results.

## 3. Top-8 technique catalogue (reusable, with geometry)

| # | Poster | Title zone | Columns / grid | Section header style | Flow device | Code treatment | Results device |
|---|---|---|---|---|---|---|---|
| 1 | Rocha | Yellow band 0–24 %, title ≈ 156 pt all-caps, band art = metaphor | Free-form: diagonal river band divides "method" from "evaluation" | Yellow 60–70 pt text, no boxes | The river itself, plus white arrows between IR boxes | White/green/red rounded IR boxes; colour = merge status, consistent everywhere | Red circular badge per chart with the average |
| 2 | CuSan | Blue block 2.5–24.5 %, title ≈ 155 pt, repo URL ≈ 62 pt monospace in band | Hero row full width (with 24 % sidebar), then 2 × 50 % columns | Full-width filled bar, white ≈ 54 pt, numbered "1." | Numbering + thick curved arrows inside figures | Line numbers, comment line in blue bold, curved arrows from report lines to source tokens | Bar chart with values on bars; setup in ~20 pt beneath |
| 3 | Lee | Navy band 0–13.5 %, conference tag at right | 2 × 50 %, heavy vertical rule | Grey full-width bar, centred bold black | Fat block arrows labelled with pass names, between code boxes | Pink highlight on changed line, red flag call-outs, dashed red value-flow arrow | Small bar chart + 4 bullet numbers |
| 4 | Pichler | White, title ≈ 104 pt, author columns, 0–15 % | Rows of 2 boxes, widths set by content (48/52, 27/73) | Plain 40–50 pt text inside blue-outlined boxes; question titles | Row order only | Coloured inline arrows from argument to parameter, red error message | Line charts with in-plot labels ("warm-up / peak") |
| 5 | VAST | No band, title 0–9 % + 3-column claim strip 10–25 % | 1/3 + 2/3, two rows | Huge condensed light headers, as large as the title | Vertical bands crossing the IR tower | Hairline connectors source→IR; stage banners between code chunks | — (none) |
| 6 | Preto | Header 0–18 % | Single column, 5 full-width panels | "N. Title" 50 pt grey, rounded panel | Dotted connector 4→5; double chevrons ▶▶ between code | Before → after pairs with grey pill captions | — (none) |
| 7 | Garzella | Title 0–14 %, red author band 14–20 % | 4 equal columns, then a full-width figure row | Grey header tabs | Numbered maroon method tabs; braces in the toolflow | Small side-by-side multi-language snippets | Green/red result matrix |
| 8 | Fry | Title 0–12 % on gradient | 2 cols, then full-width Algorithm, then full-width Example | Gold pill bar spanning the panel | 4 numbered steps on a **curved-arrow cycle**, edge labels in red | Dotted call-out bubbles with tiny IR + red caption | — (none) |

Honourable techniques: **Mitenkov** (hand-drawn callout arrows to tokens; a yellow question pill as a section
divider), **Lomuller** (a 3-chevron summary strip directly under the title), **PoTATo/VAST** ("Fork me on GitHub"
ribbon; one strict palette), **Porpodas** (vertical verdict tags "Vectorized!" beside each row).

Distilled rules that the top 3 all follow and the user's poster does not:
1. **The title is 150 pt+, or the band is art.** Rocha and CuSan titles are ≈ 155 pt. Lee is ≈ 108 pt in a
   1729×2591 pt sheet, which is ≈ 140 pt at A0 scale.
2. **One visual is clearly bigger than everything else**, and it is the *mechanism*, not a bullet list (CuSan race
   report, Lee's 4-box chain, Rocha's merge river).
3. **Arrows point at exact lines.** The explanation is attached to the code, never in a separate numbered list.
4. **Colour has one meaning, used everywhere.** Rocha: green = mergeable, red = not. Lee: pink = the changed line.
5. **Sections are not all the same box.** CuSan uses open sections with bars, Lee uses rules, Rocha uses no boxes at
   all. Equal-weight cards are a template look.

## 4. The user's poster (`poster/poster_A0.pdf`), rated brutally: 5.7 / 10 (joint 19th of 36)

Measured facts (VERIFIED): A0 portrait, 977 words. The CSS sizes are title 84 pt, subtitle 33 pt, h2 40 pt, body
25 pt, card code 21 pt, captions 20 pt, method box 18.5 pt, hero footnote 17 pt and footer refs 15 pt
(`build_poster.py:42–147`). 83 % of words have a bbox under 28 pt, i.e. they are ≤ 25 pt. The blue band is
168 mm (14.1 %). The hero runs 182–372 mm (16 %). Two columns of 387 mm with a 23 mm gutter start at 386 mm.
Fonts: Liberation Sans + Inconsolata (`pdffonts`).

| Criterion | Score | Why |
|---|---|---|
| M 5-second | 6 | The sticker "29→0" is the right idea, but its label "clamps across 6 GPU kernels" is 23 pt and **rotated**, so it is unreadable at 2 m. The title is 84 pt: about half of Rocha or CuSan. "Clamp" is never shown visually in the band. |
| F Flow | 6 | Numbers 1–8 work. The dotted teal connectors are hairlines in the left margin and in the 23 mm gutter (panel 4 → panel 5 runs up the gutter). They read as decoration, not direction. The hero's ①–④ badges sit at the right end of code lines, and their meaning sits in a separate middle list, so the eye has to ping-pong. |
| H Hierarchy | 5 | **9 white rounded cards of identical weight** (hero + 8 panels). Panel 3 adds 4 more nested cards, and panel 6 adds 3 badges + 2 plots + a grey box. Every h2 is the same 40 pt blue. Nothing is dominant: the hero is only 16 % of the height and its code is 26 pt, *the same size as body text*. |
| S Show-don't-tell | 6 | The hero, panel 2 diagram and panel 5/6 charts are good. Panels 1, 4 and 8 are bullets and tables. The four rule cards in panel 3 are code + one result line, but nothing points *inside* the code (no highlight of the guard, no arrow to the narrowed variable). |
| D Density | 5 | 977 words is about 1.6–2× CuSan (636) or Lee (554). The method paragraph (18.5 pt, 4 lines), the hero footnote (17 pt) and the refs (15 pt) cannot be read standing up. Body 25 pt is the floor, not the norm, in the top posters (CuSan ~28–39, Lee ~40). |
| R Results | 6 | Honest and labelled ("not resolvable", geomean, CI). But in panel 5 the value labels collide with bars ("+11.6%" sits on the orange bar under "+15.8%"). The "not resolvable" italics are in two colours and stacked in the private-array row. The legend is two floating coloured words at mid-right. The x-axis runs to 17.5 with no data past 15.8. Panel 6 stacks two dot plots with **different y-scales** (NVIDIA about −0.5…5.5, Intel about −5…35) at the same height, which visually equalises a 0.59 % and a 7.01 % effect. |
| A Cohesion | 6 | The palette is consistent, but **orange means four things**: Intel, "SPIR-V today: 4 clamps" (cost), the per-target writers (panel 2), and "no" in the table, plus the sticker. Blue means NVIDIA, the title, and "this work". The reader cannot learn a colour code. |

Specific "amateur" tells vs the top posters, with locations:
1. **Title band dead space:** about 59 mm (≈ 5 % of poster height) of empty blue between the subtitle (y ≈ 7.5 %)
   and the author line (y ≈ 12.5 %), measured on the 60-dpi crop.
2. **Hero is under-scaled.** Three equal-ish zones (code 318 mm | steps | SPIR-V 192 mm) sit in a 190 mm-tall frame
   with a 27 pt tab. The SPIR-V boxes are 21 pt. CuSan and Lee make the equivalent element 2× larger, with arrows.
3. **Hero code de-emphasis:** the first two lines (`@compute…`, `var<workgroup>…`) are rendered light grey, which
   lowers contrast on the very lines that give ① its fact.
4. **Panel 3 is cards-in-a-card:** 4 sub-cards + a dashed "Not proved" box + a caption inside one panel, with 21 pt code.
5. **Panel 4 is a 4-row text table** with 21 pt tags. It is a text panel in a poster whose strongest evidence
   (byte-identical binaries) could be a visual.
6. **The takeaway box sits at the bottom of column 1** (y ≈ 92 %), after "trust", where nobody ends reading. The
   top posters put the claim at the top (Lomuller's chevron strip) or on the result (Rocha's badges).
7. **Panel 7 funnel bars** are 10 mm high with 22 pt labels and look like progress bars. The 364 / 49.9 % line is a
   20 pt caption.
8. **Footer** refs at 15 pt with a QR of 27 mm. CuSan's QR cards are about 2× larger and labelled.

## 5. Redesign spec (implementable in HTML/CSS, A0 portrait 841 × 1189 mm)

### 5.1 Content constraints (hard)
- Use **only strings and numbers already on `poster_A0.pdf` / produced by `numbers.json`**. Do not invent anything.
  No reviewer Q&A section. Never use the word "sound" about the analysis. Keep the "Not proved: the clamp stays"
  and "Open: discard on Metal/D3D12; no mechanised proof" limits, because they are what makes the poster honest.

### 5.2 Grid
- Page margin 20 mm, so the content width is 801 mm. Use a **12-column grid**, column 58 mm, gutter 9 mm
  (12 × 58 + 11 × 9 = 795 mm, with ~3 mm slack per side).
- Vertical rhythm is a 6 mm baseline unit. Section gap 12 mm. No section shares a border with another.
- **Rows, not two long columns.** Read row by row (Z-pattern), with numbers increasing left→right then down:

| Zone | y range (mm) | % H | Content |
|---|---|---|---|
| Title band | 0–150 | 12.6 % | Title, authors, venue, sticker overlapping the bottom edge |
| ① Hero (dominant) | 162–500 | 28.4 % | "A tree reduction loses every clamp" |
| Row 2 (②③④) | 512–760 | 20.9 % | 3 columns: 4 / 4 / 4 grid columns |
| Results band (⑤⑥) | 772–1040 | 22.5 % | Full-width header bar; 2 columns 6 / 6 |
| Row 4 (⑦⑧) + takeaway | 1052–1150 | 8.2 % | Coverage funnel (5 cols), related/next (7 cols) |
| Footer | 1155–1182 | 2.3 % | Refs 18 pt + QR card |

### 5.3 Title band (0–150 mm, `#0033A0`)
- Title at **128 pt bold**, white, two lines, max width 610 mm: "Static Analysis for Bounds Check Elision / in the
  Naga WGSL Compiler". Line-height 1.0.
- Subtitle on one line at 36 pt, `#CFE6EE`, directly under the title (6 mm gap). Keep the existing sentence.
- Author/venue line at 28 pt at band bottom (12 mm inset). Delete the empty gap: the band shrinks from 168 to 150 mm
  and every element sits on the 6 mm rhythm.
- **Sticker** (Rocha technique): a 150 mm orange `#E15E32` circle at right, **straddling the band's bottom edge**
  (centre at y = 140 mm), so it ties the band to the hero. Content, unrotated: "29 → 0" at 120 pt bold, then
  "clamps" at 40 pt, then "6 GPU kernels" at 30 pt. The 23 pt rotated text goes.

### 5.4 ① Hero: the dominant visual (162–500 mm, full width, about 28 % of the height)
No card border. Use a 14 mm-tall blue header bar like CuSan's: "① In one picture: a tree reduction loses every
clamp", 48 pt white. Below it are three zones on a 5 / 3 / 4 split of the 12-col grid:

1. **WGSL code (5 cols, ~325 mm wide), 38 pt monospace, all lines full contrast** (`#2F2F2F` on `#F5F6F8`). The
   access line `s[lid] = s[lid] + s[lid + stride];` gets the yellow `#FFF0A6` highlight at 40 pt bold. The three
   index expressions (`lid`, `lid`, `lid + stride`) are each underlined in orange (= clamp).
2. **Annotations attached to code (overlaying the 3-col middle zone):** each fact ①–④ is a teal `#2E9FB0` callout
   (30 pt) with a **curved 1.6 mm teal arrow ending on the exact token**. Fact ① points at `256`, ② at
   `stride >> 1u`, ③ at `lid < stride`, and ④ at `lid + stride`. Text is copied from the current hero
   ("lid ∈ [0, 255], from the workgroup size", etc.). The ④ callout ends in a green `#2E7D32` check: "proved: no
   clamp needed". This replaces the separate numbered list (Mitenkov/CuSan technique).
3. **SPIR-V before/after (4 cols), stacked:**
   - Top: "SPIR-V today: 4 clamps". Four `UMin … %uint_255` lines at 30 pt on `#FBD9D3`, orange 1 mm border,
     **each line struck through with a 1.2 mm orange line**.
   - A fat 20 mm teal down-arrow labelled "interval analysis in the validator" (24 pt). This is Lee's labelled
     block arrow.
   - Bottom: "With elision: 0 clamps / binary byte-identical (sha256) to the build with all checks off" on white,
     with a 1.4 mm blue border, 32 pt.
- The provenance footnote ("wgpu's Vulkan settings … Artifact: runs-v4/…, wgpu fork 19ffb4e") moves to the footer
  refs. It stays in full, but not in the hero.

### 5.5 Row 2: three columns (512–760 mm), each opened by a numbered header bar
Use a CuSan-style header: a 12 mm filled bar in `#1C446A` navy, with a 44 pt white "② Title". No card border below.
Separate the columns with a 0.6 mm `#C9D2E3` vertical rule (Lee).

- **② Every index pays for a clamp (4 cols).** The visual is the existing transformation strip, enlarged:
  `s[i]` → (wgpu) `s[min(i, 255)]` → (proof: i ∈ [0, 255]) `s[i]`, at 34 pt, stacked vertically. Below it, the
  panel-2 pipeline diagram redrawn vertically: WGSL/GLSL/SPIR-V → Naga IR → **Validator + interval analysis** (blue
  fill) → safe_accesses → SPIR-V / MSL / HLSL writers. Keep two bullets at most (the WGSL promise; "A browser
  cannot" switch checks off). Cut the third.
- **③ How a fact is proved (4 cols).** One lead sentence (30 pt). Then a **2 × 2 table without inner borders**:
  each cell is the rule name (teal 30 pt bold), the code at 26 pt with the relevant token highlighted yellow, and
  the result interval with a green ✓. Below that, the "Not proved: the clamp stays" strip as an orange left-border
  bar (no dashed box), 26 pt.
- **④ Can we trust a removed check? (4 cols).** Make it a **checklist visual**: four rows, each with a 14 mm icon
  disc (green ✓ for Tests / GPU / Audit, orange "!" for Open) and the existing text at 26 pt. Put "8 of 8 … byte-identical"
  and "none changed a measured binary" in bold. Keep the "Open" row exactly as now.
- Flow: a 10 mm teal chevron `›` centred in each gutter between ②→③→④ (Preto/Lee technique). Then one
  full-width thin teal arrow from ④ down into the Results header.

### 5.6 Results band (772–1040 mm)
- A full-width 16 mm blue bar reads "Results: same binaries, measured on two GPUs" at 48 pt. If that phrasing is
  judged a new claim, use the existing headings "⑤ GPU kernels: every clamp gone" and "⑥ Real code: vello, run as
  web content" as two column bars.
- **Colour encoding for the whole band: NVIDIA = `#0033A0` blue, Intel = `#2E9FB0` teal.** Orange is **reserved
  for "clamp / not proved / no"** across the whole poster. This fixes the four-meanings problem. Re-colour the
  badges to match.
- **⑤ (6 cols): kernel chart.**
  - Horizontal grouped bars, 9 mm thick, with 4 mm between the pair and 10 mm between kernels.
  - Values are printed **at the bar end, inside the plot, never overlapping**: place each label at bar end + 2 mm,
    and if two labels would collide, put the Intel label inside its bar in white.
  - Domain 0–16 %, with gridlines at 5 / 10 / 15 only.
  - "Not resolvable" becomes a short grey `#9AA3B2` tick at 0 with "n.r." at 22 pt, plus one key line under the
    chart: "n.r. = no effect distinguishable from noise".
  - Legend is two swatches at top-left in 26 pt.
  - The caption "In all 6 kernels, 29 → 0 clamps … byte-identical (sha256)" goes at 24 pt.
- **⑥ (6 cols): vello.**
  - Three badges on top, 50 pt numbers and 22 pt labels: +7.01 % Intel (teal), +0.59 % NVIDIA (blue), and
    102/186 clamps removed (navy).
  - Under them, the two dot plots **side by side, not stacked**, each with its own clearly printed y-range, and the
    title states it: "NVIDIA RTX 4060 Laptop (y: −1…6 %)" / "Intel RPL-S iGPU (y: −5…35 %)". As an alternative,
    plot both on one shared axis so the 0.59 vs 7.01 difference is visible. Pick one; do not keep the current
    equal-height stack with silently different scales.
  - Method: cut to two lines at 22 pt: "Pre-registered (98bd514). One dot per vello scene, 95 % CI; four binaries
    interleaved, 120 rounds; 10 runs NVIDIA, 5 Intel." The caveats line ("14 of the 22 NVIDIA gains are below
    +0.5 %; 2 scenes have no usable Intel measurement") **stays**, at 22 pt, because it is part of the honesty.

### 5.7 Row 4 (1052–1150 mm)
- **⑦ Coverage (5 cols).** Make the funnel a real funnel: three centred trapezoid-ish bars (18 mm tall) whose widths
  are proportional to 729 / 342 / 118. Labels go inside at 28 pt bold, with the descriptor at 24 pt to the right.
  The 364 (49.9 %) runtime-sized line becomes an orange note at 24 pt ("needs symbolic lengths: the next step").
- **⑧ Related work and next steps (7 cols).** The existing table at 24 pt, with "This work" / "Tint (Chromium)"
  headers. ✓/✗ glyphs replace "yes/no" words, coloured green/orange. Keep "Compared by reading Tint's
  IntegerRangeAnalysis source, not measured" verbatim.
- **Takeaway:** move it out of column 1. Render it as a full-width 20 mm teal-tint strip (`#E6F4F7`, 1 mm teal
  border) **directly under the Results band**, or as the last element under ⑧, at 32 pt. Keep the current text.

### 5.8 Footer (1155–1182 mm)
- Refs at **18 pt** (raised from 15 pt). Add the hero provenance line and "Every number on this poster is
  generated from run artifacts …".
- A QR card on the right, CuSan style: 40 mm QR, plus "github.com/bhuvanb404" at 24 pt in a 1 mm navy outline.

### 5.9 Type scale (pt at A0) and word budget
| Role | Size | Weight / face |
|---|---|---|
| Title | 128 | Bold, Liberation Sans/Arial (deck face) |
| Sticker number | 120 | Bold |
| Subtitle | 36 | Regular |
| Section bar | 44–48 | Bold, white on fill |
| Rule/card titles | 30 | Bold teal |
| Body | **30 (min 28)** | Regular |
| Hero code | 38 (highlight line 40 bold) | Inconsolata |
| Panel code | 26 | Inconsolata |
| Callouts / captions / chart labels | 24–26 | Regular |
| Method / caveats | 22 | Regular |
| Footer refs | 18 | Regular, the only text under 22 pt |

- **Word budget: ≤ 650 words** (from 977). The cuts: panel 1's third bullet, the method paragraph down to 2 lines,
  the "Each example was run…" caption moved to the footer, and the hero footnote moved to the footer.

### 5.10 Palette (IndiaFOSS deck family, from the pptx: `0033A0`, `2F2F2F`, `61C4D3`, `1C446A`, `E15E32`, `2E9FB0`)
| Token | Hex | Single meaning |
|---|---|---|
| `--blue` | #0033A0 | Title band, hero bar, NVIDIA |
| `--teal` | #2E9FB0 | Proof annotations/arrows, Intel, flow chevrons |
| `--navy` | #1C446A | Section bars, neutral diagram strokes |
| `--orange` | #E15E32 | **Only** clamps / checks present / not proved / "no", plus the sticker |
| `--ok` | #2E7D32 | ✓ only |
| `--hl` | #FFF0A6 | Code-line highlight (deck uses yellow highlight the same way) |
| `--ink` | #2F2F2F | Text |
| `--bg` | #FFFFFF page, #F5F6F8 code wells | No tinted page background; sections are bars + whitespace, not cards |

Rationale: the deck the user likes (`s-01…s-12`) is white with blue Arial headings, teal subheads and grey code wells
with yellow line highlights. A white page with bar headers matches it better than grey-blue cards on `#EEF1F6`.

### 5.11 How the reading flow is drawn
1. Big numbers ①–⑧ in every section bar (48 pt white digit in a 16 mm teal disc at the bar's left).
2. Inside the hero: curved teal arrows from callout to token, and a fat teal block arrow from "4 clamps" to "0 clamps".
3. Between Row 2 columns: 10 mm teal chevrons centred in the gutters.
4. Row → row: a single 1.2 mm teal arrow from the end of ④ to the Results bar. The margin/gutter hairline dotted
   connectors (`#flow` SVG in `build_poster.py`) are deleted.

### 5.12 What to cut or demote (summary)
- The 9 equal white cards and the `#EEF1F6` page tint. Replace them with open sections under header bars.
- The hero's separate ①–④ text list. Its text moves into token callouts.
- The rotated sticker sub-label.
- The method paragraph, down to 2 lines. The hero provenance footnote moves to the footer.
- The panel 1 third bullet and the "Each example was run…" caption (to the footer).
- Orange as Intel colour.
- The 17.5 % x-axis tail. The stacked dot plots with silently different y-scales.

### 5.13 Acceptance checks for the implementer
- `pdftotext poster_A0.pdf - | wc -w` ≤ 650.
- In the `fs.py` bbox test, the fraction of words under 28 pt bbox is ≤ 25 %. The only text under 22 pt is the
  footer refs.
- Every number on the new poster greps to `numbers.json` or to the current `poster_A0.pdf` text.
- `grep -i sound` on the poster text returns nothing.
- Render at 1100 px. Title, sticker, hero code and the three badges must be legible at that size, which simulates
  about 3 m.
