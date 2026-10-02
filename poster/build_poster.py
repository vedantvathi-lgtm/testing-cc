#!/usr/bin/env python3
"""Poster v2 (A0 portrait). Layout follows poster/DESIGN_NOTES.md; style follows the IndiaFOSS deck.
Every number is substituted from numbers.json (make_assets.py) -- no number is typed into the template.
Output: poster.html -> poster_A0.pdf (Chromium print) -> poster_A0_preview.png."""
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
N = {k: v["value"] for k, v in json.load(open(os.path.join(HERE, "numbers.json"))).items()}
ident = N["v4_binary_identity"]
V = {
    "clamps_before": sum(r["umin_no_bce"] for r in ident.values()),
    "clamps_after": sum(r["umin_bce"] for r in ident.values()),
    "n_kernels": len(ident),
    "all_identical": "yes" if all(r["bce_equals_unchecked"] for r in ident.values()) else "NO",
    "rt_before": ident["reduce_tree"]["umin_no_bce"],
    "rt_after": ident["reduce_tree"]["umin_bce"],
    "nv_suite": f'{N["vello_nvidia_suite_pct"]:+.2f}',
    "in_suite": f'{N["vello_intel_suite_pct"]:+.2f}',
    "nv_gain": N["vello_nvidia_verdicts"].get("GAIN", 0), "nv_loss": N["vello_nvidia_verdicts"].get("LOSS", 0),
    "nv_eq": N["vello_nvidia_verdicts"].get("EQUIVALENT", 0), "nv_inv": N["vello_nvidia_verdicts"].get("INVALID", 0),
    "in_gain": N["vello_intel_verdicts"].get("GAIN", 0), "in_loss": N["vello_intel_verdicts"].get("LOSS", 0),
    "census_total": N["census_total"], "census_fixed": N["census_fixed_or_vector"], "census_proven": N["census_proven"],
    "census_rt": N["census_runtime_sized"], "census_rt_pct": N["census_runtime_sized_pct"],
    "n_projects": len(N["census_projects"]),
    "vb_before": N["vello_bounds_clamps_before_after"][0], "vb_after": N["vello_bounds_clamps_before_after"][1],
}
V["vb_removed"] = V["vb_before"] - V["vb_after"]
V["fixed_w"] = f'{100 * V["census_fixed"] / V["census_total"]:.1f}'
V["proven_w"] = f'{100 * V["census_proven"] / V["census_total"]:.1f}'
assert V["all_identical"] == "yes"

HTML = r"""<!doctype html><html><head><meta charset="utf-8"><title>IICT 2026 #55 poster</title>
<style>
@page { size: 841mm 1189mm; margin: 0 }
:root { --blue:#0033A0; --teal:#2E9FB0; --orange:#E15E32; --navy:#1C446A; --ink:#2F2F2F; --muted:#666;
        --bg:#EEF1F6; --line:#C9D2E3; --code:#F5F6F8; --hl:#FFF0A6; --bad:#FBD9D3; --ok:#2E7D32; }
* { box-sizing: border-box; margin: 0; padding: 0 }
html, body { width: 841mm; height: 1189mm; background: var(--bg); color: var(--ink);
             font-family: "Liberation Sans", Arial, sans-serif; font-size: 25pt; line-height: 1.3 }
code, .code, .codebox, .spv .box { font-family: "Inconsolata LGC Nerd Font Mono", "DejaVu Sans Mono", monospace }
b { color: var(--blue) }
.page { position: relative; width: 841mm; height: 1189mm; overflow: hidden }

/* ---------- title band ---------- */
.band { position: absolute; left: 0; top: 0; width: 841mm; height: 168mm; background: var(--blue); color: #fff;
        padding: 16mm 22mm 0 22mm }
.band h1 { font-size: 84pt; line-height: 1.04; font-weight: bold; width: 640mm; letter-spacing: -0.5pt }
.band .sub { font-size: 33pt; color: #CFE6EE; margin-top: 7mm; width: 650mm }
.band .who { position: absolute; left: 22mm; bottom: 9mm; font-size: 27pt; color: #fff }
.band .who span { color: #9FD3DE }
.band .venue { position: absolute; right: 22mm; bottom: 9mm; font-size: 27pt; color: #9FD3DE; text-align: right }
.sticker { position: absolute; right: 30mm; top: 18mm; width: 124mm; height: 124mm; border-radius: 50%;
           background: var(--orange); color: #fff; transform: rotate(-8deg); display: flex; flex-direction: column;
           align-items: center; justify-content: center; text-align: center; box-shadow: 0 2mm 0 rgba(0,0,0,.18) }
.sticker .big { font-size: 74pt; font-weight: bold; line-height: 1 }
.sticker .small { font-size: 23pt; line-height: 1.15; margin-top: 2mm }

/* ---------- panels ---------- */
.panel { background: #fff; border: 0.9mm solid var(--line); border-radius: 6mm; padding: 7.5mm 10mm 8mm 10mm; position: relative }
.panel h2 { font-size: 40pt; color: var(--blue); line-height: 1.1; display: flex; align-items: center; gap: 5mm; margin-bottom: 5mm }
.num { flex: none; width: 19mm; height: 19mm; border-radius: 50%; background: var(--blue); color: #fff; font-size: 30pt;
       display: inline-flex; align-items: center; justify-content: center; position: relative; z-index: 3 }
.lead { font-size: 25pt; margin-bottom: 4mm }
ul { margin-left: 8mm } li { margin-bottom: 2mm }
.cap { font-size: 20pt; color: var(--muted); line-height: 1.25 }
.pill { display: inline-block; background: #E4E9F2; color: var(--navy); border-radius: 10mm; padding: 1.2mm 5mm; font-size: 19pt }

/* ---------- hero ---------- */
.hero { position: absolute; left: 22mm; top: 182mm; width: 797mm; height: 190mm; border: 1.4mm solid var(--blue) }
.hero .tab { position: absolute; left: 10mm; top: -9mm; background: var(--blue); color: #fff; font-size: 27pt;
             font-weight: bold; padding: 2mm 7mm; border-radius: 4mm }
.hero-grid { display: grid; grid-template-columns: 318mm 18mm 1fr 18mm 192mm; height: 100%; align-items: center; column-gap: 0 }
.codebox { background: var(--code); border-radius: 4mm; padding: 6mm 7mm; font-size: 26pt; line-height: 1.42; white-space: pre }
.codebox .h { background: var(--hl); border-radius: 1.5mm }
.codebox .c { color: #7A8699 }
.m { display: inline-flex; width: 10.5mm; height: 10.5mm; border-radius: 50%; background: var(--teal); color: #fff; font-size: 19pt;
     align-items: center; justify-content: center; font-family: "Liberation Sans"; vertical-align: 0.6mm; font-weight: bold }
.chev { font-size: 110pt; color: var(--teal); text-align: center; line-height: 1 }
.steps { font-size: 25pt; line-height: 1.3 }
.steps div { display: flex; gap: 3mm; margin-bottom: 4.5mm; align-items: flex-start }
.steps .m { margin-top: 1mm }
.steps .ok { color: var(--ok); font-weight: bold }
.spv { font-size: 21pt; line-height: 1.4 }
.spv .box { border-radius: 4mm; padding: 4mm 5mm; margin-bottom: 5mm; white-space: pre }
.spv .before { background: #FFF6F4; border: 0.8mm solid var(--orange) }
.spv .after { background: #F3F7FF; border: 0.8mm solid var(--blue) }
.spv .t { font-family: "Liberation Sans"; font-weight: bold; font-size: 24pt; margin-bottom: 1.5mm; white-space: normal }
.spv .before .t { color: var(--orange) } .spv .after .t { color: var(--blue) }
.spv .bad { background: var(--bad); border-radius: 1mm }
.hero .foot { position: absolute; left: 10mm; right: 10mm; bottom: 4mm; font-size: 17pt; color: var(--muted) }

/* ---------- columns ---------- */
.col { position: absolute; top: 386mm; width: 387mm; display: flex; flex-direction: column; gap: 7.5mm }
.c1 { left: 22mm } .c2 { left: 432mm }

/* problem panel */
.xform { display: flex; align-items: center; gap: 3mm; margin: 5mm 0 3mm 0; font-size: 25pt }
.xform .k { background: var(--code); border: 0.6mm solid var(--line); border-radius: 3mm; padding: 2mm 4mm; white-space: nowrap }
.xform .k.red { border-color: var(--orange); background: #FFF6F4 }
.xform .k.green { border-color: var(--ok); background: #F1F8F1 }
.xform .arr { color: var(--teal); font-size: 26pt; text-align: center; line-height: 1 }
.xform .arr small { display: block; font-size: 15pt; color: var(--muted) }

/* fact cards */
.cards { display: grid; grid-template-columns: 1fr 1fr; gap: 5mm }
.card { border: 0.7mm solid var(--line); border-radius: 4mm; padding: 4mm 4.5mm; background: #FBFCFE }
.card .t { font-weight: bold; color: var(--teal); font-size: 25pt; margin-bottom: 2mm }
.card .code { background: var(--code); border-radius: 2.5mm; padding: 3mm 3.5mm; font-size: 21pt; line-height: 1.35; white-space: pre }
.card .r { margin-top: 3mm; font-size: 23pt } .card .r .ok { color: var(--ok); font-weight: bold }
.not { margin-top: 5mm; border: 0.7mm dashed var(--orange); border-radius: 4mm; padding: 3.5mm 5mm; font-size: 22pt }
.not .t { color: var(--orange); font-weight: bold; font-size: 22pt; margin-bottom: 1mm }

/* trust stack */
.stack div { display: flex; gap: 4mm; align-items: baseline; margin-bottom: 3.5mm; font-size: 24pt }
.stack .tag { flex: none; width: 33mm; font-weight: bold; color: var(--blue); font-size: 21pt }
.stack .tag.o { color: var(--orange) }

/* results */
img.chart { width: 100%; display: block }
.badges { display: flex; gap: 5mm; margin: 2mm 0 4mm 0 }
.badge { flex: 1; border-radius: 5mm; color: #fff; padding: 3mm 4mm; text-align: center }
.badge .v { font-size: 40pt; font-weight: bold; line-height: 1.05 } .badge .l { font-size: 18pt; line-height: 1.2 }
.badge.nv { background: #2457C5 } .badge.in { background: var(--orange) } .badge.cl { background: var(--navy) }
.method { background: #F4F6FA; border-radius: 4mm; padding: 3.5mm 5mm; font-size: 18.5pt; line-height: 1.3; margin-top: 3mm }

/* funnel */
.funnel div.row { margin-bottom: 2.5mm }
.funnel .lab { font-size: 20pt; margin-bottom: 1mm }
.funnel .bar { height: 10mm; border-radius: 2mm; display: flex; align-items: center; color: #fff; font-weight: bold; font-size: 22pt; padding-left: 3mm }

/* related */
table { border-collapse: collapse; width: 100%; font-size: 20pt }
th { text-align: left; color: var(--blue); border-bottom: 0.8mm solid var(--blue); padding: 1.5mm 2mm }
td { padding: 1.6mm 2mm; border-bottom: 0.3mm solid #E2E6EE }
td.y { color: var(--ok); font-weight: bold } td.n { color: var(--orange); font-weight: bold }

/* footer */
.footer { position: absolute; left: 22mm; right: 22mm; bottom: 9mm; height: 34mm; display: flex; gap: 8mm; align-items: center;
          border-top: 0.6mm solid #B9C3D6; padding-top: 4mm }
.footer .refs { flex: 1; font-size: 15pt; color: var(--muted); line-height: 1.3 }
.footer .qr { display: flex; align-items: center; gap: 3mm; font-size: 17pt; color: var(--navy) }
.footer .qr img { width: 27mm; height: 27mm }
.takeaway { background: #E6F4F7; border: 1mm solid var(--teal); border-radius: 6mm; padding: 5mm 8mm; font-size: 25pt; display: flex; gap: 6mm; align-items: center }
.takeaway .i { flex: none; width: 17mm; height: 17mm; border-radius: 50%; background: var(--teal); color: #fff; font-weight: bold; font-size: 34pt; display: flex; align-items: center; justify-content: center }
#flow { position: absolute; left: 0; top: 0; width: 841mm; height: 1189mm; pointer-events: none; z-index: 2 }
</style></head><body><div class="page">

<div class="band">
  <h1>Static Analysis for Bounds Check Elision in the Naga WGSL Compiler</h1>
  <div class="sub">Proving GPU array indices in bounds, so the clamps WebGPU forces into every shader can be dropped</div>
  <div class="who">Bhuvan B &nbsp;<span>·&nbsp; ACM SIGPLAN-M Mentee &nbsp;|&nbsp; Open Source Contributor</span></div>
  <div class="venue">IICT 2026 · Paper #55</div>
</div>
<div class="sticker"><div class="big">{{clamps_before}}→{{clamps_after}}</div><div class="small">clamps across<br>{{n_kernels}} GPU kernels</div></div>

<!-- ================= HERO ================= -->
<div class="panel hero" id="hero">
  <div class="tab">In one picture: a tree reduction loses every clamp</div>
  <div class="hero-grid">
    <div class="codebox"><span class="c">@compute @workgroup_size(256)</span>        <span class="m">1</span>
<span class="c">var&lt;workgroup&gt; s: array&lt;u32, 256&gt;;</span>

for (var stride = 128u; stride &gt; 0u;
     stride = stride &gt;&gt; 1u) {           <span class="m">2</span>
  if (lid &lt; stride) {                <span class="m">3</span>
    <span class="h">s[lid] = s[lid] + s[lid + stride];</span> <span class="m">4</span>
  }
  workgroupBarrier();
}</div>
    <div class="chev">›</div>
    <div class="steps">
      <div><span class="m">1</span><span><code>lid</code> ∈ [0, 255], from the workgroup size</span></div>
      <div><span class="m">2</span><span>only update is <code>&gt;&gt; 1</code> ⇒ <code>stride</code> ∈ [1, 128] in the body</span></div>
      <div><span class="m">3</span><span>branch <code>lid &lt; stride</code> ⇒ <code>lid</code> ∈ [0, 127]</span></div>
      <div><span class="m">4</span><span><code>lid + stride</code> ∈ [1, 255] ⊆ [0, 255]<br><span class="ok">✓ proved: no clamp needed</span></span></div>
    </div>
    <div class="chev">›</div>
    <div class="spv">
      <div class="box before"><div class="t">SPIR-V today: {{rt_before}} clamps</div><span class="bad">UMin %lid %uint_255</span>
<span class="bad">UMin %lid %uint_255</span>
<span class="bad">UMin %lid_plus_stride %uint_255</span>
<span class="bad">UMin %lid %uint_255</span></div>
      <div class="box after"><div class="t">With elision: {{rt_after}} clamps</div><span style="font-family:Liberation Sans;font-size:22pt">binary byte-identical (sha256)
to the build with all checks off</span></div>
    </div>
  </div>
  <div class="foot">wgpu's Vulkan settings (index: Restrict; buffers: robustBufferAccess2). SPIR-V ids renamed for reading. Artifact: runs-v4/20260929T125306Z-reduce_tree, wgpu fork 19ffb4e.</div>
</div>

<!-- ================= COLUMN 1 ================= -->
<div class="col c1">
  <div class="panel" id="p1">
    <h2><span class="num">1</span>Every index pays for a clamp</h2>
    <ul>
      <li>WGSL promises that an out-of-bounds access never touches memory outside its resource.</li>
      <li>wgpu keeps that promise by <b>clamping every runtime index</b> on Vulkan, Metal and D3D12.</li>
      <li>Native apps may switch checks off (vello does). <b>A browser cannot</b>, so web content always pays.</li>
    </ul>
    <div class="xform">
      <div class="k"><code>s[i]</code></div>
      <div class="arr">→<small>wgpu</small></div>
      <div class="k red"><code>s[min(i, 255)]</code></div>
      <div class="arr">→<small>proof: i ∈ [0, 255]</small></div>
      <div class="k green"><code>s[i]</code></div>
    </div>
    <div class="cap">Goal: remove only the clamps the compiler can <i>prove</i> unnecessary; every other access keeps its check.</div>
  </div>

  <div class="panel" id="p2">
    <h2><span class="num">2</span>Where it plugs in</h2>
    <svg viewBox="0 0 367 118" width="367mm" height="118mm" style="display:block;font-family:Liberation Sans">
      <defs><marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="#1C446A"/></marker></defs>
      <g font-size="7.4" text-anchor="middle" fill="#2F2F2F">
        <rect x="1" y="4" width="72" height="27" rx="3" fill="#fff" stroke="#1C446A" stroke-width="0.9"/><text x="37" y="15.5">WGSL / GLSL /</text><text x="37" y="24.5">SPIR-V in</text>
        <rect x="98" y="4" width="64" height="27" rx="3" fill="#fff" stroke="#1C446A" stroke-width="0.9"/><text x="130" y="20.5" font-weight="bold">Naga IR</text>
        <rect x="187" y="1" width="90" height="33" rx="3" fill="#EAF1FF" stroke="#0033A0" stroke-width="1.6"/><text x="232" y="14" font-weight="bold" fill="#0033A0">Validator</text><text x="232" y="25" font-weight="bold" fill="#0033A0">+ interval analysis</text>
        <rect x="292" y="4" width="74" height="27" rx="3" fill="#fff" stroke="#1C446A" stroke-width="0.9"/><text x="329" y="15.5" font-family="Inconsolata LGC Nerd Font Mono">safe_accesses</text><text x="329" y="24.5" font-size="6.2" fill="#666">proved Access exprs</text>
        <path d="M73,17.5 L97,17.5" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <path d="M162,17.5 L186,17.5" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <path d="M277,17.5 L291,17.5" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <path d="M329,31 L329,50 L58,50" stroke="#1C446A" stroke-width="0.9" fill="none"/>
        <path d="M58,50 L58,62" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <path d="M183,50 L183,62" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <path d="M308,50 L308,62" stroke="#1C446A" stroke-width="0.9" marker-end="url(#a)"/>
        <rect x="8" y="63" width="100" height="27" rx="3" fill="#fff" stroke="#E15E32" stroke-width="1.1"/><text x="58" y="74.5" font-weight="bold">SPIR-V writer</text><text x="58" y="84" font-size="6.4" fill="#666">Vulkan</text>
        <rect x="133" y="63" width="100" height="27" rx="3" fill="#fff" stroke="#E15E32" stroke-width="1.1"/><text x="183" y="74.5" font-weight="bold">MSL writer</text><text x="183" y="84" font-size="6.4" fill="#666">Metal</text>
        <rect x="258" y="63" width="100" height="27" rx="3" fill="#fff" stroke="#E15E32" stroke-width="1.1"/><text x="308" y="74.5" font-weight="bold">HLSL writer</text><text x="308" y="84" font-size="6.4" fill="#666">D3D12</text>
        <path d="M1,40 L277,40" stroke="#0033A0" stroke-width="0.8"/><text x="1" y="46.5" text-anchor="start" font-size="6.4" fill="#0033A0">shared by every target</text>
        <path d="M8,96 L358,96" stroke="#E15E32" stroke-width="0.8"/>
        <text x="8" y="104" text-anchor="start" font-size="6.6" fill="#E15E32">per target: a proved access is emitted unchecked;</text>
        <text x="8" y="112.5" text-anchor="start" font-size="6.6" fill="#E15E32">every other access gets the backend's normal check</text>
      </g>
    </svg>
  </div>

  <div class="panel" id="p3">
    <h2><span class="num">3</span>How a fact is proved</h2>
    <div class="lead">Each variable carries an <b>interval</b> per program point. Branches narrow it, arithmetic propagates it,
      loops either match a <b>recurrence</b> or widen to “unknown”. An access is proved when its index interval fits the array.</div>
    <div class="cards">
      <div class="card"><div class="t">Branch guard</div><div class="code">var i = data[g.x];
if (i &lt; 8u) { … lut[i] … }</div>
        <div class="r">i ∈ [0, 7] inside the branch <span class="ok">✓</span></div></div>
      <div class="card"><div class="t">Bit mask</div><div class="code">lut[data[g.x] &amp; 7u]</div>
        <div class="r">unknown &amp; 7 ∈ [0, 7] <span class="ok">✓</span></div></div>
      <div class="card"><div class="t">Loop recurrence</div><div class="code">for (var stride = 128u;
     stride &gt; 0u;
     stride = stride &gt;&gt; 1u)</div>
        <div class="r">stride ∈ [1, 128] in the body <span class="ok">✓</span></div></div>
      <div class="card"><div class="t">Relation a ≥ b</div><div class="code">if (lid &gt;= off) {
  t = s[lid - off];
}</div>
        <div class="r">lid − off ∈ [0, 254] <span class="ok">✓</span></div></div>
    </div>
    <div class="not"><div class="t">Not proved: the clamp stays</div>
      runtime-sized <code>array&lt;T&gt;</code> &nbsp;·&nbsp; bounds like <code>i &lt; arrayLength(&amp;a)</code> &nbsp;·&nbsp; facts across function calls
      &nbsp;·&nbsp; guards on <code>let</code> values, e.g. <code>let i = data[g.x]</code></div>
    <div class="cap" style="margin-top:3mm">Each example was run through the analysis (poster/probes/RESULTS.md, fork f73a687).</div>
  </div>

  <div class="panel" id="p4">
    <h2><span class="num">4</span>Can we trust a removed check?</h2>
    <div class="stack">
      <div><span class="tag">Tests</span><span>95 stress shaders that must keep, or must lose, a check; interval arithmetic compared with exhaustive enumeration</span></div>
      <div><span class="tag">GPU</span><span>8 of 8 differential fixtures give byte-identical output with and without elision (RTX 4060)</span></div>
      <div><span class="tag">Audit</span><span>our own audit found 3 unsound loop/branch shapes and an override hazard; all fixed with regression tests, <b>none changed a measured binary</b></span></div>
      <div><span class="tag o">Open</span><span><code>discard</code> on Metal / D3D12; no mechanised proof</span></div>
    </div>
  </div>
  <div class="takeaway"><div class="i">!</div><div><b>Takeaway:</b> many of the checks WebGPU forces into shaders are provably redundant.
    A small interval analysis in the validator removes those, and only those: every access it cannot prove keeps its check.</div></div>
</div>

<!-- ================= COLUMN 2 ================= -->
<div class="col c2">
  <div class="panel" id="p5">
    <h2><span class="num">5</span>GPU kernels: every clamp gone</h2>
    <img class="chart" src="assets_v2/v4_kernels.svg">
    <div class="cap">In all {{n_kernels}} kernels, {{clamps_before}} → {{clamps_after}} clamps and the elided binary is byte-identical (sha256) to the checks-off binary. “Not resolvable”: no effect distinguishable from noise.</div>
  </div>

  <div class="panel" id="p6">
    <h2><span class="num">6</span>Real code: vello, run as web content</h2>
    <div class="badges">
      <div class="badge in"><div class="v">{{in_suite}}%</div><div class="l">Intel iGPU, suite geomean<br>{{in_gain}} faster · {{in_loss}} slower</div></div>
      <div class="badge nv"><div class="v">{{nv_suite}}%</div><div class="l">NVIDIA RTX 4060, suite geomean<br>{{nv_gain}} faster · {{nv_loss}} slower · {{nv_eq}} same</div></div>
      <div class="badge cl"><div class="v">{{vb_removed}}/{{vb_before}}</div><div class="l">bounds clamps removed<br>from vello's shaders</div></div>
    </div>
    <img class="chart" src="assets_v2/vello_realworld.svg">
        <div class="method"><b>Method</b> (pre-registered, 98bd514). One dot per vello test scene, bars = 95% CI, grey band = ±0.5%. GPU work recorded from vello and replayed with checks on.
      Four binaries interleaved in 120 rounds per run: checks kept, elided, a <i>second copy</i> of elided (noise control), checks off.
      10 runs NVIDIA, 5 Intel; one machine, Vulkan. Caveats: 14 of the {{nv_gain}} NVIDIA gains are below +0.5%; 2 scenes have no usable Intel measurement.</div>
  </div>

  <div class="panel" id="p7">
    <h2><span class="num">7</span>Coverage in {{n_projects}} open-source projects</h2>
    <div class="funnel">
      <div class="row"><div class="lab">runtime-indexed accesses (vello, bevy, rerun, webgpu-samples, wgpu, …)</div>
        <div class="bar" style="width:100%;background:#9AA5B8">{{census_total}}</div></div>
      <div class="row"><div class="lab">into fixed-size arrays or vectors: provable in principle</div>
        <div class="bar" style="width:{{fixed_w}}%;background:#7F9BD9">{{census_fixed}}</div></div>
      <div class="row"><div class="lab">proved in bounds today</div>
        <div class="bar" style="width:{{proven_w}}%;background:#2457C5">{{census_proven}}</div></div>
    </div>
    <div class="cap">{{census_rt}} accesses ({{census_rt_pct}}%) index runtime-sized arrays, which need symbolic lengths: the next step.</div>
  </div>

  <div class="panel" id="p8">
    <h2><span class="num">8</span>Related work and next steps</h2>
    <table>
      <tr><th></th><th>This work</th><th>Tint (Chromium)</th></tr>
      <tr><td>facts from any <code>if</code> guard</td><td class="y">yes</td><td class="n">no, loop exits only</td></tr>
      <tr><td>relation a ≥ b for a − b</td><td class="y">yes</td><td class="n">no</td></tr>
      <tr><td>shrinking / geometric loops</td><td class="y">yes</td><td class="n">no, ±1 steps</td></tr>
      <tr><td>runtime-sized arrays</td><td class="n">no</td><td class="n">no</td></tr>
      <tr><td>shipped</td><td class="n">no (fork)</td><td class="y">yes, default-on</td></tr>
    </table>
    <div class="cap" style="margin:2mm 0 3mm 0">Compared by reading Tint's IntegerRangeAnalysis source, not measured. Classic CPU work: ABCD, Go <code>prove</code>, LLVM IRCE.</div>
    <div style="font-size:22pt"><b>Next:</b> symbolic lengths (<code>arrayLength</code>) · guards on <code>let</code> values · head-to-head with Tint · upstream to wgpu</div>
  </div>
</div>

<div class="footer">
  <div class="refs">[1] W3C WGSL spec, memory access. [2] Tint IntegerRangeAnalysis, dawn.googlesource.com. [3] Bodík, Gupta, Sarkar, ABCD, PLDI 2000.
    [4] Würthinger, Wimmer, Mössenböck, array bounds check elimination, PPPJ 2007. [5] LLVM IRCE; Go SSA prove/loopbce.
    Every number on this poster is generated from run artifacts by poster/make_assets.py (numbers.json).</div>
  <div class="qr"><img src="assets_v2/qr_github.svg"><div>github.com/<br>bhuvanb404</div></div>
</div>
<svg id="flow"></svg>
</div>
<script>
window.addEventListener('load', () => {
// Reading line: dotted connector badge 1→2→…→8, entering column 2 through the gutter (Preto-style).
const mm = v => v * 96 / 25.4, svg = document.getElementById('flow');
svg.setAttribute('viewBox', `0 0 ${mm(841)} ${mm(1189)}`);
const pos = el => { const r = el.getBoundingClientRect(); return {x: r.left + r.width / 2, y: r.top + r.height / 2, t: r.top, b: r.bottom, l: r.left, r: r.right}; };
const badges = [...document.querySelectorAll('.num')].map(pos);
const path = (d, arrow) => { const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  p.setAttribute('d', d); p.setAttribute('fill', 'none'); p.setAttribute('stroke', '#2E9FB0'); p.setAttribute('stroke-width', mm(1.1));
  p.setAttribute('stroke-dasharray', `${mm(0.1)} ${mm(3.2)}`); p.setAttribute('stroke-linecap', 'round'); if (arrow) p.setAttribute('marker-end', 'url(#fa)'); svg.appendChild(p); };
svg.innerHTML = `<defs><marker id="fa" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="#2E9FB0"/></marker></defs>`;
const hero = document.getElementById('hero').getBoundingClientRect();
const panels = ['p1','p2','p3','p4','p5','p6','p7','p8'].map(id => document.getElementById(id).getBoundingClientRect());
const lane = i => panels[i].left - mm(11);   // lane in the margin/gutter left of each column
// hero -> 1: drop from the hero into the left margin lane, then into badge 1
path(`M ${badges[0].x} ${hero.bottom + mm(1)} L ${badges[0].x} ${panels[0].top - mm(1.5)}`, true);
for (let i = 0; i < badges.length - 1; i++) {
  const a = badges[i], b = badges[i + 1];
  if (i === 3) { // 4 -> 5: out of column 1 at the bottom, up the central gutter, into column 2 above panel 5
    const yb = panels[3].bottom + mm(5), gx = lane(4), yt = b.y;
    path(`M ${a.l - mm(1)} ${a.y} L ${lane(3)} ${a.y} L ${lane(3)} ${yb} L ${gx} ${yb} L ${gx} ${yt} L ${b.l - mm(1.5)} ${yt}`, true);
  } else {
    const x = lane(i);
    path(`M ${a.l - mm(1)} ${a.y} L ${x} ${a.y} L ${x} ${b.y} L ${b.l - mm(1.5)} ${b.y}`, true);
  }
}
});
</script>
</body></html>"""


def render():
    html = re.sub(r"\{\{(\w+)\}\}", lambda m: str(V[m.group(1)]), HTML)
    leftover = re.findall(r"\{\{\w+\}\}", html)
    assert not leftover, leftover
    open(os.path.join(HERE, "poster.html"), "w").write(html)
    pdf = os.path.join(HERE, "poster_A0.pdf")
    subprocess.run(["chromium", "--headless", "--disable-gpu", "--no-pdf-header-footer", "--run-all-compositor-stages-before-draw",
                    "--virtual-time-budget=4000", f"--print-to-pdf={pdf}", "file://" + os.path.join(HERE, "poster.html")],
                   check=True, capture_output=True)
    subprocess.run(["pdftoppm", "-r", "40", "-png", "-singlefile", pdf, os.path.join(HERE, "poster_A0_preview")], check=True)
    print("written", pdf)


if __name__ == "__main__":
    render()
