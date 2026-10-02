# IICT 2026 #55 poster (A0 portrait, 841 x 1189 mm)

Built 2026-10-01. Three layers, all regenerable:

| Layer | Where | What |
|---|---|---|
| numbers + charts | `make_assets.py` → `numbers.json`, `assets/*.{png,svg}` | every printed number with its source artifact (Rule 0.6) |
| pieces (diagrams, tables, text panels) | Figma file `YBH3UvmizBumyfu3bApbL2` ("IICT 2026 #55 poster pieces"), exported at 3x → `pieces/*.png` | pipeline, policies, worked example, R4 Q&A, rule table, related work, soundness, methods |
| collation | Canva design `DAHWusBW1Jc` (3179 x 4494 px = A0 at 96 dpi); media ids in `pieces/canva_ids.txt` | three 979-px columns, 76-px margins, 45-px gutters |

`compose_preview.py` rebuilds the same layout locally from the Canva coordinates at 3x
(`poster_A0_preview.{png,pdf}`, not committed) to check positions and overlaps at print scale.

Fonts: Figma pieces use Inter / Source Code Pro (Arial is not available in Figma); charts use Liberation Sans.

Known divergence: `pieces/policies.png` has the old caption; the poster uses `policies_nocaption.png` plus a Canva
text caption (the Figma text is corrected, but the Figma Starter-plan MCP call limit blocked re-export).

Forbidden-claims scrub (P-8) run on all piece text and Canva text: no "0.0 ns", 7,568 ns, ps figures, bare 74.1%,
"sound" claim, "fuzzed", "verified on MSL/HLSL", first-campaign headline.
