# How the LLVM Dev Meeting posters are built (35 studied, 2019–2025)

Source: `references/posters/llvm-devmtg/*/` (MANIFEST.json). Studied at full size: Lee (casts, EuroLLVM'19),
Rocha (function merging), Preto (C++→Rust), Mitenkov (MLIR→Python), Fry (CIRCT ISS), Konicar/Lauko (PoTATo),
Hück (CuSan), Ledoux (arithmetic). The rest were reviewed as contact sheets.

## What the strong ones do

1. **A title band with weight** (15–20 % of the height). There is a big title, one line of authors, and a
   colour band. Rocha adds a sticker with the headline result ("up to 25 % smaller code"). CuSan puts the repo
   URL right under the title.
2. **A hero just below the title shows the tool working before any text.** CuSan shows the racy CUDA code and
   the TSan report, with arrows between them. PoTATo has a three-card goal strip and SYCL a three-step arrow
   strip. The reader gets the point in about five seconds.
3. **Sections are numbered (1, 2, 3, …)** with the number in the header (CuSan, Mitenkov, Preto, CIRCT), so the
   reading order is explicit and never has to be guessed from the layout.
4. **The boundaries are visible.** Panels have rounded borders (Preto, CIRCT), header bars or pills in one
   accent colour (CuSan, CIRCT), and a tinted page background behind white panels (PoTATo, Mitenkov).
   Whitespace alone is not used as a boundary.
5. **Flow hints connect the sections.** Preto has a dotted connector from section 4 down to section 5. CIRCT has
   curved arrows through its numbered algorithm steps. Lee and Preto use big chevrons or block arrows between
   before and after code, and Rocha uses one flowing band.
6. **Code is the main visual and it is annotated.** Changed lines are highlighted (pink or yellow) and callouts
   in an accent colour point at tokens with arrows (CuSan, Mitenkov, Lee). Captions sit under the code in grey
   pills (Preto).
7. **Text is sparse.** Bullets run one or two lines and key phrases are in colour or bold. Nobody prints
   paragraphs at 24 pt.
8. **Results are a few charts plus badge numbers**, such as Rocha's red circles ("Average 6 % smaller"), with
   direct labels and no legend hunting.
9. **The limits are stated plainly** ("Advantages & Limitations" in CIRCT, "Open questions" in Mitenkov).
10. **The footer carries the references in small type, QR codes, and the repo link.**

## Applied to paper #55

- **Title band:** IndiaFOSS-deck blue (#0033A0) with a white title. The orange sticker gives the verified headline
  "29 → 0 clamps".
- **Hero (full width):** the reduce_tree WGSL with numbered annotations ①–④ deriving the interval, a chevron,
  then the SPIR-V before and after. It ends on "byte-identical to the checks-off binary".
- **Two columns of numbered panels:**
  1. Problem
  2. Where it plugs in
  3. How a fact is proved: four verified mini examples, plus what is not proved
  4. Trust
  5. Synthetic results
  6. vello as web content
  7. Coverage
  8. Related work and next steps
- **Reading line:** a dotted connector runs from each panel's number badge to the next, including from the
  bottom of column 1 to the top of column 2.
- **Dropped:** the reviewer Q&A framing. The answers are absorbed into panel 3 as content.
- **Every number comes from `numbers.json`**, which `make_assets.py` writes, or from `probes/` outputs
  (`probes/RESULTS.md`).
