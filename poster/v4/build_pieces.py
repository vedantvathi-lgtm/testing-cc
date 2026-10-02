#!/usr/bin/env python3
"""Poster v4 pieces in the IndiaFOSS deck style (palette, fonts and line weights taken from the deck XML:
#0033A0 titles, #2F2F2F text, #61C4D3 subheads, #1C446A thin outlines, #E15E32 'per target', #F8F8F8 code,
#BEBECB rules; Arial (= Liberation Sans metrics), Consolas-like mono). Each piece is its own HTML page,
rendered individually to PNG at 2x; the pieces are assembled in Canva. Numbers come from numbers.json and
check_cost.json."""
import json, os, subprocess
HERE = os.path.dirname(os.path.abspath(__file__)); P = os.path.join(HERE, "pieces"); os.makedirs(P, exist_ok=True)
N = {k: v["value"] for k, v in json.load(open(os.path.join(HERE, "..", "numbers.json"))).items()}
CC = json.load(open(os.path.join(HERE, "..", "check_cost.json")))
ci = N["vello_suite_ci_across_scenes"]

CSS = """
* { margin: 0; padding: 0; box-sizing: border-box }
html, body { background: #fff; color: #2F2F2F; font-family: "Liberation Sans", Arial, sans-serif; font-size: 40px; line-height: 1.3 }
.mono, code, .codebox { font-family: "Inconsolata LGC Nerd Font Mono", Consolas, monospace }
b, .blue { color: #0033A0 } .orange { color: #E15E32 } .teal { color: #61C4D3 } .muted { color: #6B6B78 }
h2 { font-size: 66px; color: #0033A0; font-weight: bold; line-height: 1.1; margin-bottom: 26px }
h2 .n { color: #61C4D3; margin-right: 18px }
h3 { font-size: 44px; color: #61C4D3; font-weight: bold; margin: 16px 0 10px 0 }
ul { list-style: none } li { position: relative; padding-left: 34px; margin-bottom: 12px }
li:before { content: "\\2022"; position: absolute; left: 4px; color: #2F2F2F }
.codebox { background: #F8F8F8; padding: 18px 24px; font-size: 36px; line-height: 1.42; white-space: pre }
.hl { background: #FBEAA5; display: inline-block; width: 100% }
.box { border: 3px solid #1C446A; padding: 16px 20px; text-align: center; background: #fff }
.cap { font-size: 32px; color: #6B6B78; line-height: 1.3 }
#piece { display: inline-block; padding: 0 }
html { zoom: 2 }
"""

def page(body, width, extra=""):
    return f'<!doctype html><html><head><meta charset="utf-8"><style>{CSS}{extra}</style></head><body><div id="piece" style="width:{width}px">{body}</div></body></html>'

pieces = {}

# ---------- header (deck title-slide style: white, big blue title, bold name, small role line, thin rule) ----------
pieces["header"] = (3179, page(f"""
<div style="padding:56px 76px 0 76px">
 <div style="display:flex;justify-content:space-between;align-items:flex-start">
  <div style="font-size:118px;font-weight:bold;color:#0033A0;line-height:1.08;width:2500px">Static Analysis for Bounds Check Elision in the Naga WGSL Compiler</div>
  <div style="font-size:44px;color:#0033A0;text-align:right;margin-top:18px">IICT 2026<br><span style="color:#6B6B78">Paper #55</span></div>
 </div>
 <div style="margin-top:26px;font-size:54px;font-weight:bold">Bhuvan B</div>
 <div style="font-size:38px;font-weight:bold;margin-top:6px">ACM SIGPLAN-M Mentee &nbsp;|&nbsp; Open Source Contributor</div>
 <div style="height:4px;background:#BEBECB;margin-top:32px"></div>
</div>""", 3179))

# ---------- key finding (deck slide 2 style: thin outlined statement box) ----------
pieces["finding"] = (3027, page(f"""
<div style="display:flex;gap:60px;align-items:center">
 <div style="flex:1;border:4px solid #1C446A;padding:40px 50px">
  <div style="font-size:78px;font-weight:bold;line-height:1.16">Proving array indices in bounds removes <span class="blue">102 of the 186</span> bounds clamps in vello’s GPU shaders.</div>
  <div style="font-size:58px;line-height:1.3;margin-top:26px">On an Intel iGPU that wins back the full cost of the checks: <b class="orange">+{N['vello_intel_suite_pct']:.1f}%</b>.
   On NVIDIA the checks cost only {CC['realworld:nvidia:vello']['all_checks_pct']:.1f}% in total, so the gain is small: <b>+{N['vello_nvidia_suite_pct']:.1f}%</b>.</div>
  <div class="cap" style="margin-top:22px">+{N['vello_intel_suite_pct']:.2f}% / +{N['vello_nvidia_suite_pct']:.2f}%: pre-registered suite geomeans over {ci['intel']['n_scenes']} vello scenes each,
   95% CI +{ci['intel']['ci_lo_pct']:.2f} to +{ci['intel']['ci_hi_pct']:.2f}% and +{ci['nvidia']['ci_lo_pct']:.2f} to +{ci['nvidia']['ci_hi_pct']:.2f}%. Total check cost (checks on vs. off) is exploratory.</div>
 </div>
 <div style="width:420px;text-align:center">
  <div id="qrslot" style="width:420px;height:420px;background:#fff"></div>
  <div style="font-size:34px;margin-top:14px;color:#1C446A">code, data and the source of every number</div>
 </div>
</div>""", 3027))

# ---------- hero (deck slide 8 style: teal subheads, grey code boxes, yellow highlight) ----------
hero_code = """<span class="muted">@workgroup_size(256)</span>
<span class="muted">var&lt;workgroup&gt; s: array&lt;u32, 256&gt;;</span>

for (var stride = 128u; stride &gt; 0u;
     stride = stride &gt;&gt; 1u) {
  if (lid &lt; stride) {
<span class="hl">    s[lid] = s[lid] + s[lid + stride];</span>
  }
  workgroupBarrier();
}"""
spv = "\n".join(f'<span style="text-decoration:line-through;color:#E15E32">UMin {x} %uint_255</span>' for x in ["%lid", "%lid", "%lid_plus_stride", "%lid"])
pieces["hero"] = (3027, page(f"""
<h2>How it works: one proof, and a tree reduction loses all four clamps</h2>
<div style="display:grid;grid-template-columns:1080px 1fr 900px;gap:60px">
 <div><h3>The loop (reduce_tree)</h3><div class="codebox">{hero_code}</div></div>
 <div><h3>What the analysis derives</h3>
  <ol style="list-style:none;font-size:40px;line-height:1.35">
   <li style="padding-left:56px"><span class="teal" style="position:absolute;left:0;font-weight:bold">1</span><code>lid</code> ∈ [0, 255]: the workgroup size</li>
   <li style="padding-left:56px"><span class="teal" style="position:absolute;left:0;font-weight:bold">2</span><code>stride</code> ∈ [1, 128]: its only update is <code>&gt;&gt; 1</code></li>
   <li style="padding-left:56px"><span class="teal" style="position:absolute;left:0;font-weight:bold">3</span><code>lid &lt; stride</code> ⇒ <code>lid</code> ∈ [0, 127]</li>
   <li style="padding-left:56px"><span class="teal" style="position:absolute;left:0;font-weight:bold">4</span><code>lid + stride</code> ∈ [1, 255] fits <code>s</code>, <b>so no clamp is needed</b></li>
  </ol></div>
 <div><h3>Emitted SPIR-V (Restrict)</h3><div class="codebox" style="font-size:34px">{spv}</div>
  <div style="font-size:40px;margin-top:18px"><b>4 → 0 clamps.</b> The binary is byte-identical (sha256) to the build with all checks off.</div></div>
</div>
<div class="cap" style="margin-top:20px">Shown with wgpu’s Vulkan settings; SPIR-V ids renamed for reading. Artifact: runs-v4, wgpu fork 19ffb4e.</div>
""", 3027, "li:before{content:none}"))

W = 1440
# ---------- left column ----------
pieces["s1"] = (W, page(f"""
<h2><span class="n">1</span>Today, wgpu clamps array indices</h2>
<div>WGSL forbids out-of-bounds memory access. On Vulkan, Metal and D3D12, wgpu rewrites <code>s[i]</code> as <code class="orange">s[min(i, 255)]</code>.</div>
<table style="width:100%;border-collapse:collapse;margin:22px 0;font-size:36px">
 <tr style="color:#0033A0;font-weight:bold;border-bottom:3px solid #1C446A"><td style="padding:8px 6px"></td><td>arrays &amp; vectors</td><td>storage buffers</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px;font-weight:bold">Vulkan</td><td>clamp</td><td>hardware (robustBufferAccess2)</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px;font-weight:bold">Metal</td><td>clamp</td><td>clamp</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px;font-weight:bold">D3D12</td><td>clamp</td><td>clamp</td></tr>
 <tr><td style="padding:8px 6px;font-weight:bold">GLES</td><td>none</td><td>none</td></tr>
</table>
<div><b>A browser must keep these checks.</b> Native apps can turn them off (vello does).</div>""", W))

def arrow(w=46): return f'<div style="width:{w}px;text-align:center;color:#1C446A;font-size:40px">→</div>'
pieces["s2"] = (W, page(f"""
<h2><span class="n">2</span>How a fact is proved</h2>
<div style="display:flex;align-items:center">
 <div class="box" style="width:250px;font-size:34px">WGSL</div>{arrow()}
 <div class="box" style="width:250px;font-size:34px">Naga IR</div>{arrow()}
 <div class="box" style="width:420px;font-size:34px;background:#F8F8F8"><b>validator:<br>interval analysis</b></div>{arrow()}
 <div class="box" style="width:330px;font-size:34px">SPIR-V / MSL / HLSL writers</div>
</div>
<div style="display:flex;margin-top:10px;font-size:30px;font-weight:bold">
 <div style="width:1062px;border-top:4px solid #0033A0;padding-top:6px;color:#0033A0;text-align:center">shared by every target: one proof per access</div>
 <div style="width:46px"></div>
 <div style="width:330px;border-top:4px solid #E15E32;padding-top:6px;color:#E15E32;text-align:center">per target</div>
</div>
<div style="margin-top:26px">Every integer carries an <b>interval</b>. Branches narrow it; a loop variable matches a known <b>recurrence</b> or becomes unknown. A proved access loses its clamp.</div>
<div style="display:grid;grid-template-columns:1fr 1fr;gap:16px 36px;margin-top:6px">
 <div><h3>Branch guard</h3><div class="codebox" style="font-size:31px">var i = data[g.x];
if (i &lt; 8u) {{ … lut[i] … }}</div><div style="font-size:36px;margin-top:8px">i ∈ [0, 7] in the branch</div></div>
 <div><h3>Bit mask</h3><div class="codebox" style="font-size:31px">lut[data[g.x] &amp; 7u]</div><div style="font-size:36px;margin-top:8px">x &amp; 7 ∈ [0, 7]</div></div>
 <div><h3>Loop recurrence</h3><div class="codebox" style="font-size:31px">for (var k = 128u; k &gt; 0u;
     k = k &gt;&gt; 1u)</div><div style="font-size:36px;margin-top:8px">k ∈ [1, 128] in the body</div></div>
 <div><h3>Relation a ≥ b</h3><div class="codebox" style="font-size:31px">if (lid &gt;= off) {{
  t = s[lid - off]; }}</div><div style="font-size:36px;margin-top:8px">lid − off ∈ [0, 254]</div></div>
</div>
<div style="border:3px solid #E15E32;background:#FFF7F2;padding:16px 22px;margin-top:26px;font-size:36px"><b class="orange">Not proved, so the clamp stays:</b>
 runtime-sized arrays, bounds like <code>i &lt; arrayLength(&amp;a)</code>, facts across calls, guards on <code>let</code> values.</div>""", W))

pieces["s3"] = (W, page("""
<h2><span class="n">3</span>Why trust a removed check?</h2>
<ul>
 <li><b>Tests:</b> 95 hand-written shaders where a check must stay or must go; interval arithmetic checked against exhaustive enumeration.</li>
 <li><b>On GPU:</b> 8 of 8 differential runs give byte-identical output with and without elision.</li>
 <li><b>Audit:</b> we found 3 unsound loop/branch shapes and fixed each with a regression test. No measured binary changed.</li>
 <li><b class="orange">Not yet covered:</b> shapes beyond these tests, and <code>discard</code> on Metal/D3D12. Next: random-program differential testing.</li>
</ul>""", W))

# ---------- right column ----------
def row(label, gpu, cost, rec, col, share):
    sx = lambda v: v / 19.5 * 640
    return f"""<div style="display:flex;align-items:center;height:96px">
 <div style="width:330px"><div style="font-size:33px;font-weight:bold">{label}</div><div style="font-size:29px;color:{col}">{gpu}</div></div>
 <div style="position:relative;width:{sx(cost) + 4:.0f}px;height:52px;border:3px solid #BEBECB;background:#fff">
  <div style="position:absolute;left:0;top:0;height:46px;width:{max(sx(min(rec, cost)), 3):.0f}px;background:{col}"></div></div>
 <div style="margin-left:18px;font-size:32px;color:{col};font-weight:bold;white-space:nowrap">{rec:.1f}% of {cost:.1f}%<span style="color:#6B6B78;font-weight:normal">{('&nbsp; (' + share + ')') if share else ''}</span></div></div>"""
NV, IN = "#0033A0", "#E15E32"
c = lambda k: CC[k]
rows = [row("vello, 32 scenes", "Intel iGPU", c("realworld:intel:vello")["all_checks_pct"], c("realworld:intel:vello")["elision_raw_pct"], IN, "100%"),
        row("vello, 34 scenes", "NVIDIA", c("realworld:nvidia:vello")["all_checks_pct"], c("realworld:nvidia:vello")["elision_raw_pct"], NV, f'{round(100 * c("realworld:nvidia:vello")["share_recovered"])}%'),
        '<div style="height:18px"></div>',
        row("synthetic scan", "NVIDIA", c("v4:nvidia:scan_shared")["all_checks_pct"], c("v4:nvidia:scan_shared")["elision_pct"], NV, "100%"),
        row("synthetic scan", "Intel iGPU", c("v4:intel:scan_shared")["all_checks_pct"], c("v4:intel:scan_shared")["elision_pct"], IN, "100%"),
        '<div style="height:18px"></div>',
        row("image blur", "Intel iGPU", c("realworld:intel:blur")["all_checks_pct"], c("realworld:intel:blur")["elision_raw_pct"], IN, f'{round(100 * c("realworld:intel:blur")["share_recovered"])}%'),
        row("image blur", "NVIDIA", c("realworld:nvidia:blur")["all_checks_pct"], c("realworld:nvidia:blur")["elision_raw_pct"], NV, "")]
pieces["s4"] = (W, page(f"""
<h2><span class="n">4</span>Check cost, and what elision wins back</h2>
<div class="cap" style="margin-bottom:14px"><b style="color:#2F2F2F">Outline</b>: slowdown from all bounds checks (checks on vs. off). <b style="color:#2F2F2F">Fill</b>: the part elision removes.</div>
{''.join(rows)}
<div style="margin-top:22px"><b>Where checks are expensive and provable, elision wins back all of their cost, on both GPUs.</b>
 On NVIDIA, vello’s checks are nearly free. Blur’s checks cost 7% on Intel but are mostly not provable (section 5).</div>
<div class="cap" style="margin-top:14px">Exploratory: raw medians, not pre-registered. Every synthetic kernel with a measurable check cost recovers 95–100% (5 on NVIDIA, 3 on Intel). Why checks cost more on Intel is not yet known.</div>""", W))

def bar(label, v, col, total):
    return f"""<div style="font-size:34px;margin-top:14px">{label}</div>
<div style="width:{1440 * v / total:.0f}px;height:54px;background:{col};color:#fff;font-weight:bold;font-size:34px;padding:6px 14px">{v}</div>"""
pieces["s5"] = (W, page(f"""
<h2><span class="n">5</span>Coverage in {len(N['census_projects'])} open-source projects</h2>
{bar('runtime-indexed accesses', N['census_total'], '#BEBECB', N['census_total'])}
{bar('into fixed-size arrays or vectors: provable in principle', N['census_fixed_or_vector'], '#7F9BD9', N['census_total'])}
{bar('proved in bounds today', N['census_proven'], '#0033A0', N['census_total'])}
<div style="margin-top:18px">{N['census_runtime_sized']} accesses ({N['census_runtime_sized_pct']}%) index runtime-sized arrays: the next step is symbolic lengths.</div>""", W))

pieces["s6"] = (W, page("""
<h2><span class="n">6</span>Compared with Tint (Chrome)</h2>
<table style="width:100%;border-collapse:collapse;font-size:36px">
 <tr style="color:#0033A0;font-weight:bold;border-bottom:3px solid #1C446A"><td style="padding:8px 6px"></td><td>this work</td><td>Tint</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px">facts from any <code>if</code> guard</td><td class="blue">yes</td><td class="orange">loop exits only</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px">relation a ≥ b for a − b</td><td class="blue">yes</td><td class="orange">no</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px">shrinking / geometric loops</td><td class="blue">yes</td><td class="orange">±1 steps only</td></tr>
 <tr style="border-bottom:1px solid #BEBECB"><td style="padding:8px 6px">runtime-sized arrays</td><td class="orange">no</td><td class="orange">no</td></tr>
 <tr><td style="padding:8px 6px">shipped</td><td class="orange">no (fork)</td><td class="blue">default-on</td></tr>
</table>
<div class="cap" style="margin-top:14px">From reading Tint’s IntegerRangeAnalysis source, not measured. Classic CPU work: ABCD, Go <code>prove</code>, LLVM IRCE.</div>""", W))

# ---------- footer (deck footer: thin rule, name | title, venue) ----------
pieces["footer"] = (3027, page("""
<div style="height:3px;background:#BEBECB"></div>
<div style="display:flex;justify-content:space-between;font-size:30px;color:#0033A0;margin-top:14px">
 <div>Bhuvan B &nbsp;|&nbsp; Static Analysis for Bounds Check Elision in the Naga WGSL Compiler &nbsp;|&nbsp; a fork of wgpu/Naga</div>
 <div>every number is generated from run artifacts (poster/numbers.json)</div><div>IICT 2026</div></div>""", 3027))

FF = os.path.join(HERE, ".ff-profile"); os.makedirs(FF, exist_ok=True)
sizes = {}
for name, (w, html) in pieces.items():
    hp = os.path.join(P, name + ".html"); open(hp, "w").write(html)
    png = os.path.join(P, name + ".png")
    for attempt in range(3):
        try:
            subprocess.run(["firefox", "--headless", "--no-remote", "--profile", FF, f"--window-size={2 * (w + 40)},2800",
                            "--screenshot", png, "file://" + hp], check=True, capture_output=True, timeout=60)
            break
        except subprocess.TimeoutExpired:
            print("timeout, retrying", name)
    from PIL import Image, ImageChops
    im = Image.open(png).convert("RGB")
    bg = Image.new("RGB", im.size, (255, 255, 255))
    bbox = ImageChops.difference(im, bg).getbbox()
    im = im.crop((0, 0, 2 * w, bbox[3] + 8 if bbox else im.size[1])); im.save(png)
    sizes[name] = [w, round(im.size[1] / 2)]
json.dump(sizes, open(os.path.join(P, "sizes.json"), "w"), indent=1)
print(json.dumps(sizes))
