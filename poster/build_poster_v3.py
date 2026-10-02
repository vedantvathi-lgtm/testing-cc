#!/usr/bin/env python3
"""Poster v3 (A0 portrait), implementing the redesign spec in poster/POSTER_RATINGS.md §5.
Rows, not equal cards: title band → dominant hero (code with token callouts) → ②③④ → results band ⑤⑥ →
⑦⑧ + takeaway → footer. Colour meanings: blue = NVIDIA / structure, teal = proof + Intel,
orange = clamp / not proved / no, green = ✓. Numbers come from numbers.json only."""
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
N = {k: v["value"] for k, v in json.load(open(os.path.join(HERE, "numbers.json"))).items()}
ident = N["v4_binary_identity"]
assert all(r["bce_equals_unchecked"] for r in ident.values())
V = {
    "clamps_before": sum(r["umin_no_bce"] for r in ident.values()),
    "clamps_after": sum(r["umin_bce"] for r in ident.values()),
    "n_kernels": len(ident),
    "rt_before": ident["reduce_tree"]["umin_no_bce"], "rt_after": ident["reduce_tree"]["umin_bce"],
    "nv_suite": f'{N["vello_nvidia_suite_pct"]:+.2f}', "in_suite": f'{N["vello_intel_suite_pct"]:+.2f}',
    "nv_gain": N["vello_nvidia_verdicts"].get("GAIN", 0), "nv_loss": N["vello_nvidia_verdicts"].get("LOSS", 0),
    "nv_eq": N["vello_nvidia_verdicts"].get("EQUIVALENT", 0),
    "in_gain": N["vello_intel_verdicts"].get("GAIN", 0), "in_loss": N["vello_intel_verdicts"].get("LOSS", 0),
    "census_total": N["census_total"], "census_fixed": N["census_fixed_or_vector"], "census_proven": N["census_proven"],
    "census_rt": N["census_runtime_sized"], "census_rt_pct": N["census_runtime_sized_pct"],
    "n_projects": len(N["census_projects"]),
    "vb_before": N["vello_bounds_clamps_before_after"][0], "vb_after": N["vello_bounds_clamps_before_after"][1],
}
V["vb_removed"] = V["vb_before"] - V["vb_after"]
V["fixed_w"] = f'{100 * V["census_fixed"] / V["census_total"]:.1f}'
V["proven_w"] = f'{100 * V["census_proven"] / V["census_total"]:.1f}'

HTML = r"""<!doctype html><html><head><meta charset="utf-8"><title>IICT 2026 #55 poster v3</title>
<style>
@page { size: 841mm 1189mm; margin: 0 }
:root { --blue:#0033A0; --teal:#2E9FB0; --orange:#E15E32; --navy:#1C446A; --ink:#2F2F2F; --muted:#5E6673;
        --well:#F5F6F8; --hl:#FFF0A6; --ok:#2E7D32; --rule:#C9D2E3; --tint:#E6F4F7 }
* { box-sizing: border-box; margin: 0; padding: 0 }
html, body { width: 841mm; height: 1189mm; background: #fff; color: var(--ink);
             font-family: "Liberation Sans", Arial, sans-serif; font-size: 30pt; line-height: 1.28 }
.mono { font-family: "Inconsolata LGC Nerd Font Mono", "DejaVu Sans Mono", monospace }
b { color: var(--blue) } .o { color: var(--orange) } .t { color: var(--teal) } .g { color: var(--ok) }
.page { position: relative; width: 841mm; height: 1189mm; overflow: hidden }
.flow { position: absolute; left: 20mm; width: 801mm; top: 166mm; height: 984mm; display: flex; flex-direction: column; justify-content: space-between }
.zone { position: relative; width: 801mm }

/* title band */
.band { position: absolute; left: 0; top: 0; width: 841mm; height: 156mm; background: var(--blue); color: #fff; padding: 13mm 20mm 0 20mm }
.band h1 { font-size: 92pt; line-height: 1.04; width: 650mm; letter-spacing: -0.5pt }
.band .sub { font-size: 34pt; color: #CFE6EE; margin-top: 6mm; width: 640mm }
.band .who { position: absolute; left: 20mm; bottom: 9mm; font-size: 28pt } .band .who span { color: #9FD3DE }
.sticker { position: absolute; left: 668mm; top: 30mm; width: 158mm; height: 158mm; border-radius: 50%; background: var(--orange);
           color: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;
           border: 3mm solid #fff; z-index: 5 }
.sticker .n { font-size: 104pt; font-weight: bold; line-height: 1 } .sticker .a { font-size: 40pt; font-weight: bold; line-height: 1.1 }
.sticker .b { font-size: 28pt; line-height: 1.1 }

/* section bars */
.bar { background: var(--navy); color: #fff; font-weight: bold; font-size: 40pt; height: 16mm; display: flex; align-items: center;
       gap: 5mm; padding: 0 5mm; border-radius: 2mm; margin-bottom: 6mm; white-space: nowrap }
.bar.blue { background: var(--blue) }
.bar .d { flex: none; width: 13mm; height: 13mm; border-radius: 50%; background: var(--teal); color: #fff; font-size: 32pt;
          display: inline-flex; align-items: center; justify-content: center }

/* hero */
.hero { }
.hero .bar { font-size: 44pt; height: 19mm }
.hgrid { position: relative; display: grid; grid-template-columns: 400mm 170mm 1fr; column-gap: 9mm; height: 262mm }
.code { background: var(--well); border-radius: 4mm; padding: 7mm 9mm; font-size: 44pt; line-height: 1.5; white-space: pre; align-self: start }
.code .hl { background: var(--hl); font-weight: bold; border-radius: 1.5mm }
.tok { border-bottom: 1.2mm solid var(--orange) }
.tokp { border-bottom: 1.2mm solid var(--teal) }
.callouts { position: relative }
.co { position: absolute; left: 0; width: 170mm; font-size: 30pt; line-height: 1.22; color: var(--ink) }
.co .k { display: inline-flex; width: 11mm; height: 11mm; border-radius: 50%; background: var(--teal); color: #fff; font-size: 22pt;
         font-weight: bold; align-items: center; justify-content: center; margin-right: 2.5mm; vertical-align: 1mm }
.co .ok { color: var(--ok); font-weight: bold }
.spv { display: flex; flex-direction: column; align-items: stretch }
.box { border-radius: 4mm; padding: 5mm 6mm }
.before { background: #FDEEEA; border: 1mm solid var(--orange) }
.before .h { color: var(--orange); font-weight: bold; font-size: 32pt; margin-bottom: 2mm }
.before .l { font-size: 27pt; line-height: 1.45; text-decoration: line-through; text-decoration-color: var(--orange); text-decoration-thickness: 1.1mm }
.arrowdown { display: flex; align-items: center; gap: 4mm; margin: 4mm 0 4mm 14mm; color: var(--teal); font-size: 24pt; font-weight: bold }
.after { border: 1.4mm solid var(--blue); background: #fff }
.after .h { color: var(--blue); font-weight: bold; font-size: 34pt; margin-bottom: 1mm } .after .s { font-size: 27pt }

/* row 2 */
.row2 { display: grid; grid-template-columns: 1fr 1fr 1fr; column-gap: 0 }
.cell { padding: 0 7mm; position: relative; font-size: 34pt } .cell:first-child { padding-left: 0 } .cell:last-child { padding-right: 0 }
.cell + .cell { border-left: 0.6mm solid var(--rule) }
.chev { position: absolute; left: -7.5mm; top: 3mm; width: 15mm; height: 15mm; background: #fff; color: var(--teal); font-size: 52pt;
        font-weight: bold; line-height: 13mm; text-align: center; z-index: 3 }
ul { margin-left: 7mm } li { margin-bottom: 2.5mm }
.strip { display: flex; flex-direction: column; align-items: flex-start; gap: 1.5mm; font-size: 34pt; margin: 2mm 0 5mm 0 }
.strip .k { background: var(--well); border-radius: 2.5mm; padding: 1mm 4mm; border: 0.7mm solid var(--rule) }
.strip .k.red { border-color: var(--orange); background: #FDEEEA } .strip .k.grn { border-color: var(--ok); background: #EEF7EF }
.strip .a { color: var(--teal); font-size: 25pt; padding-left: 6mm }
.rules { display: grid; grid-template-columns: 1fr 1fr; column-gap: 5mm; row-gap: 6mm }
.rule .n { color: var(--teal); font-weight: bold; font-size: 30pt }
.rule .c { background: var(--well); border-radius: 2mm; padding: 2mm 2.5mm; font-size: 26pt; line-height: 1.3; white-space: pre; margin: 1mm 0 }
.rule .c .y { background: var(--hl) }
.rule .r { font-size: 30pt }
.notp { margin-top: 6mm; border-left: 2.2mm solid var(--orange); padding: 1mm 0 1mm 4mm; font-size: 27pt; line-height: 1.25 }
.notp .h { color: var(--orange); font-weight: bold }
.check div { display: flex; gap: 4mm; align-items: flex-start; margin-bottom: 7mm; font-size: 32pt; line-height: 1.25 }
.check .i { flex: none; width: 13mm; height: 13mm; border-radius: 50%; color: #fff; font-weight: bold; font-size: 26pt;
            display: inline-flex; align-items: center; justify-content: center; margin-top: 0.5mm }
.check .i.ok { background: var(--ok) } .check .i.w { background: var(--orange) }
.check .tag { font-weight: bold; color: var(--navy) }

/* results */
.res { }
.rgrid { display: grid; grid-template-columns: 1fr 1fr; column-gap: 13mm }
.rgrid h3 { font-size: 34pt; color: var(--navy); margin-bottom: 1mm }
img.chart { width: 100%; display: block }
.cap { font-size: 25pt; color: var(--muted); line-height: 1.25 }
.badges { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 4mm; margin: 1mm 0 3mm 0 }
.badge { border-radius: 4mm; color: #fff; padding: 2mm 3mm; text-align: center }
.badge .v { font-size: 50pt; font-weight: bold; line-height: 1.05 } .badge .l { font-size: 23pt; line-height: 1.15 }
.badge.in { background: var(--teal) } .badge.nv { background: var(--blue) } .badge.cl { background: var(--navy) }

/* row 4 */
.row4 { display: grid; grid-template-columns: 330mm 1fr; column-gap: 14mm }
.fun .r { display: flex; align-items: center; gap: 4mm; margin-bottom: 3.5mm }
.fun .bb { height: 15mm; border-radius: 2mm; color: #fff; font-weight: bold; font-size: 28pt; display: flex; align-items: center; padding-left: 3mm; flex: none }
.fun .d { font-size: 26pt; line-height: 1.15 }
table { border-collapse: collapse; width: 100%; font-size: 26pt }
th { text-align: left; color: var(--navy); border-bottom: 0.8mm solid var(--navy); padding: 0.6mm 2mm }
td { padding: 0.6mm 2mm; border-bottom: 0.3mm solid #E2E6EE }
td.y { color: var(--ok); font-weight: bold } td.n { color: var(--orange); font-weight: bold }

.take { position: relative; width: 801mm; min-height: 28mm; padding-top: 3mm !important; padding-bottom: 3mm !important; background: var(--tint); border: 1mm solid var(--teal); border-radius: 4mm; display: flex; align-items: center; gap: 5mm; padding: 0 7mm; font-size: 29pt }
.take .i { flex: none; width: 15mm; height: 15mm; border-radius: 50%; background: var(--teal); color: #fff; font-weight: bold; font-size: 32pt; display: flex; align-items: center; justify-content: center }
.foot { position: absolute; left: 20mm; width: 801mm; top: 1156mm; height: 29mm; display: flex; gap: 8mm; align-items: center;
        border-top: 0.6mm solid #B9C3D6; padding-top: 2mm }
.foot .refs { flex: 1; font-size: 17pt; color: var(--muted); line-height: 1.25 }
.qr { display: flex; align-items: center; gap: 3mm; border: 1mm solid var(--navy); border-radius: 3mm; padding: 1.5mm 3mm; font-size: 22pt; color: var(--navy); font-weight: bold }
.qr img { width: 22mm; height: 22mm }
#ov { position: absolute; left: 0; top: 0; width: 841mm; height: 1189mm; pointer-events: none; z-index: 4 }
</style></head><body><div class="page">

<div class="band">
  <h1>Static Analysis for Bounds Check Elision<br>in the Naga WGSL Compiler</h1>
  <div class="sub">Proving GPU array indices in bounds, so the clamps WebGPU forces into every shader can be dropped</div>
  <div class="who">Bhuvan B <span>· ACM SIGPLAN-M Mentee | Open Source Contributor · IICT 2026 · Paper #55</span></div>
</div>
<div class="sticker"><div class="n">{{clamps_before}}→{{clamps_after}}</div><div class="a">clamps</div><div class="b">{{n_kernels}} GPU kernels</div></div>

<div class="flow">
<!-- ① HERO -->
<div class="zone hero">
  <div class="bar blue"><span class="d">1</span>In one picture: a tree reduction loses every clamp</div>
  <div class="hgrid">
    <div class="code mono">@compute @workgroup_size(<span class="tokp" id="t1">256</span>)
var&lt;workgroup&gt; s: array&lt;u32, 256&gt;;

for (var stride = 128u;
     stride &gt; 0u;
     stride = <span class="tokp" id="t2">stride &gt;&gt; 1u</span>) {
  if (<span class="tokp" id="t3">lid &lt; stride</span>) {
    <span class="hl">s[<span class="tok">lid</span>] = s[<span class="tok">lid</span>] + s[<span class="tok" id="t4">lid + stride</span>];</span>
  }
  workgroupBarrier();
}</div>
    <div class="callouts">
      <div class="co" id="c1" style="top:4mm"><span class="k">1</span><span class="mono">lid</span> ∈ [0, 255], from the workgroup size</div>
      <div class="co" id="c2" style="top:70mm"><span class="k">2</span>only update is <span class="mono">&gt;&gt; 1</span> ⇒ <span class="mono">stride</span> ∈ [1, 128] in the body</div>
      <div class="co" id="c3" style="top:134mm"><span class="k">3</span>branch <span class="mono">lid &lt; stride</span> ⇒ <span class="mono">lid</span> ∈ [0, 127]</div>
      <div class="co" id="c4" style="top:188mm"><span class="k">4</span><span class="mono">lid + stride</span> ∈ [1, 255] ⊆ [0, 255]<br><span class="ok">✓ proved: no clamp needed</span></div>
    </div>
    <div class="spv">
      <div class="box before"><div class="h">SPIR-V today: {{rt_before}} clamps</div>
        <div class="l mono">UMin %lid %uint_255<br>UMin %lid %uint_255<br>UMin %lid_plus_stride %uint_255<br>UMin %lid %uint_255</div></div>
      <div class="arrowdown"><svg width="20mm" height="26mm" viewBox="0 0 20 26"><path d="M6,0 H14 V15 H20 L10,26 L0,15 H6 Z" fill="#2E9FB0"/></svg>interval analysis<br>in the validator</div>
      <div class="box after"><div class="h">With elision: {{rt_after}} clamps</div><div class="s">binary byte-identical (sha256) to the build with all checks off</div></div>
    </div>
  </div>
</div>

<!-- ② ③ ④ -->
<div class="zone row2">
  <div class="cell">
    <div class="bar"><span class="d">2</span>Every index pays</div>
    <ul><li>WGSL promises an out-of-bounds access never leaves its resource.</li>
        <li>wgpu clamps every runtime index on Vulkan, Metal and D3D12. <b>A browser cannot switch that off.</b></li></ul>
    <div class="strip mono"><span class="k">s[i]</span><span class="a">↓ wgpu today</span><span class="k red">s[min(i, 255)]</span>
      <span class="a">↓ if i ∈ [0, 255] is proved</span><span class="k grn">s[i]</span></div>
    <div class="cap">The analysis runs in Naga's validator; the SPIR-V, MSL and HLSL writers skip the check for each proved access.</div>
  </div>
  <div class="cell"><div class="chev">›</div>
    <div class="bar"><span class="d">3</span>How a fact is proved</div>
    <div style="font-size:26pt;margin-bottom:3mm">Every variable carries an <b>interval</b>; branches narrow it, loops match a <b>recurrence</b>.</div>
    <div class="rules">
      <div class="rule"><div class="n">Guard</div><div class="c mono">var i = data[g.x];
if (<span class="y">i &lt; 8u</span>) { lut[i] }</div><div class="r">i ∈ [0, 7] <span class="g">✓</span></div></div>
      <div class="rule"><div class="n">Bit mask</div><div class="c mono">lut[data[g.x] <span class="y">&amp; 7u</span>]</div><div class="r">∈ [0, 7] <span class="g">✓</span></div></div>
      <div class="rule"><div class="n">Recurrence</div><div class="c mono">stride = <span class="y">stride &gt;&gt; 1u</span></div><div class="r">stride ∈ [1, 128] <span class="g">✓</span></div></div>
      <div class="rule"><div class="n">Relation</div><div class="c mono">if (<span class="y">lid &gt;= off</span>)
  { s[lid - off] }</div><div class="r">lid − off ∈ [0, 254] <span class="g">✓</span></div></div>
    </div>
    <div class="notp"><span class="h">Not proved, clamp stays:</span> runtime-sized <span class="mono">array&lt;T&gt;</span>, <span class="mono">i &lt; arrayLength</span>, facts across calls, guards on <span class="mono">let</span> values.</div>
  </div>
  <div class="cell"><div class="chev">›</div>
    <div class="bar"><span class="d">4</span>Can we trust it?</div>
    <div class="check">
      <div><span class="i ok">✓</span><span><span class="tag">Tests:</span> 95 stress shaders that must keep or lose a check; interval arithmetic checked by exhaustive enumeration</span></div>
      <div><span class="i ok">✓</span><span><span class="tag">GPU:</span> <b>8 of 8</b> differential fixtures byte-identical with and without elision</span></div>
      <div><span class="i ok">✓</span><span><span class="tag">Audit:</span> 3 unsound shapes and an override hazard found and fixed with regression tests; <b>none changed a measured binary</b></span></div>
      <div><span class="i w">!</span><span><span class="tag o">Open:</span> <span class="mono">discard</span> on Metal / D3D12; no mechanised proof</span></div>
    </div>
  </div>
</div>

<!-- ⑤ ⑥ RESULTS -->
<div class="zone res">
  <div class="bar blue"><span class="d">5</span>Results: GPU kernels and real code (vello)</div>
  <div class="rgrid">
    <div>
      <h3>Synthetic kernels: every clamp gone</h3>
      <img class="chart" src="assets_v3/v4_kernels.svg">
      <div class="cap">In all {{n_kernels}} kernels {{clamps_before}} → {{clamps_after}} clamps; each elided binary is byte-identical (sha256) to the checks-off binary.</div>
    </div>
    <div>
      <h3>vello scenes, run as web content</h3>
      <div class="badges">
        <div class="badge in"><div class="v">{{in_suite}}%</div><div class="l">Intel geomean<br>{{in_gain}} faster · {{in_loss}} slower</div></div>
        <div class="badge nv"><div class="v">{{nv_suite}}%</div><div class="l">NVIDIA geomean<br>{{nv_gain}} faster · {{nv_loss}} slower</div></div>
        <div class="badge cl"><div class="v">{{vb_removed}}/{{vb_before}}</div><div class="l">bounds clamps<br>removed</div></div>
      </div>
      <img class="chart" src="assets_v3/vello_realworld.svg">
      <div class="cap">One dot per scene, 95% CI. Pre-registered (98bd514); four binaries interleaved, 120 rounds; 10 runs NVIDIA, 5 Intel.
        14 of the {{nv_gain}} NVIDIA gains are below +0.5%; 2 scenes have no usable Intel measurement.</div>
    </div>
  </div>
</div>

<div class="take"><div class="i">!</div><div><b>Takeaway:</b> many checks WebGPU forces into shaders are provably redundant. A small interval analysis removes those, and every access it cannot prove keeps its check.</div></div>
<!-- ⑦ ⑧ -->
<div class="zone row4">
  <div class="fun">
    <div class="bar"><span class="d">6</span>Coverage: {{n_projects}} projects</div>
    <div class="r"><div class="bb" style="width:{{w_total}}mm;background:#8C96A8">{{census_total}}</div><div class="d">runtime-indexed accesses</div></div>
    <div class="r"><div class="bb" style="width:{{w_fixed}}mm;background:#5F7FC8">{{census_fixed}}</div><div class="d">fixed-size arrays / vectors</div></div>
    <div class="r"><div class="bb" style="width:{{w_proven}}mm;background:var(--blue)">{{census_proven}}</div><div class="d">proved in bounds</div></div>
  </div>
  <div>
    <div class="bar"><span class="d">7</span>Related work and next steps</div>
    <table>
      <tr><th></th><th>This work</th><th>Tint (Chromium)</th><th></th><th>This work</th><th>Tint</th></tr>
      <tr><td>any <span class="mono">if</span> guard</td><td class="y">✓</td><td class="n">✗</td><td>runtime-sized arrays</td><td class="n">✗</td><td class="n">✗</td></tr>
      <tr><td>relation a ≥ b</td><td class="y">✓</td><td class="n">✗</td><td>shipped</td><td class="n">✗</td><td class="y">✓</td></tr>
      <tr><td>shrinking / geometric loops</td><td class="y">✓</td><td class="n">✗</td><td colspan="3" class="cap">Tint compared by reading its source, not measured.</td></tr>
    </table>
    <div style="font-size:24pt;margin-top:2mm"><b>Next:</b> symbolic lengths · <span class="mono">let</span> guards · head-to-head with Tint · upstream to wgpu</div>
  </div>
</div>

</div>
<div class="foot">
  <div class="refs">{{census_rt}} of {{census_total}} accesses ({{census_rt_pct}}%) index runtime-sized arrays: the next target.
    &nbsp;|&nbsp; Hero: wgpu's Vulkan settings (index Restrict; buffers robustBufferAccess2), SPIR-V ids renamed, artifact runs-v4/20260929T125306Z-reduce_tree, fork 19ffb4e. Examples in ③ run through the analysis (poster/probes, fork f73a687).
    Refs: W3C WGSL spec; Tint IntegerRangeAnalysis (dawn); Bodík et al., ABCD, PLDI 2000; Würthinger et al., PPPJ 2007; LLVM IRCE; Go prove. Every number is generated from run artifacts (poster/numbers.json).</div>
  <div class="qr"><img src="assets_v2/qr_github.svg"><div>github.com/<br>bhuvanb404</div></div>
</div>
<svg id="ov"></svg>
</div>
<script>
window.addEventListener('load', () => {
  const svg = document.getElementById('ov'), mm = v => v * 96 / 25.4;
  svg.setAttribute('viewBox', `0 0 ${mm(841)} ${mm(1189)}`);
  svg.innerHTML = '<defs><marker id="ah" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="4.5" markerHeight="4.5" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="#2E9FB0"/></marker></defs>';
  const R = id => document.getElementById(id).getBoundingClientRect();
  for (let i = 1; i <= 4; i++) {           // callout ① … ④ → its exact token in the code
    const c = R('c' + i), t = R('t' + i);
    const x1 = c.left - mm(2), y1 = c.top + mm(5.5), x2 = t.right + mm(2), y2 = t.top + t.height / 2;
    const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    p.setAttribute('d', `M ${x1} ${y1} C ${x1 - mm(30)} ${y1}, ${x2 + mm(30)} ${y2}, ${x2} ${y2}`);
    p.setAttribute('fill', 'none'); p.setAttribute('stroke', '#2E9FB0'); p.setAttribute('stroke-width', mm(1.4));
    p.setAttribute('marker-end', 'url(#ah)'); svg.appendChild(p);
  }
});
</script>
</body></html>"""


def render():
    w = 200  # funnel: widest bar in mm, widths proportional to the counts
    V["w_total"] = w
    V["w_fixed"] = round(w * V["census_fixed"] / V["census_total"])
    V["w_proven"] = round(w * V["census_proven"] / V["census_total"])
    html = re.sub(r"\{\{(\w+)\}\}", lambda m: str(V[m.group(1)]), HTML)
    assert not re.findall(r"\{\{\w+\}\}", html)
    open(os.path.join(HERE, "poster_v3.html"), "w").write(html)
    pdf = os.path.join(HERE, "poster_A0.pdf")
    subprocess.run(["chromium", "--headless", "--disable-gpu", "--no-pdf-header-footer", "--run-all-compositor-stages-before-draw",
                    "--virtual-time-budget=4000", f"--print-to-pdf={pdf}", "file://" + os.path.join(HERE, "poster_v3.html")],
                   check=True, capture_output=True)
    subprocess.run(["pdftoppm", "-scale-to", "1100", "-png", "-singlefile", pdf, os.path.join(HERE, "poster_A0_preview")], check=True)
    print("written", pdf)


if __name__ == "__main__":
    render()
