# Bounds-check-elision reference archive

This directory is the source archive for the implementation and paper audit.
Each reference should be saved with its URL, access date, version/commit, and
the precise lesson used. Web pages and specifications are normative or
implementation references; analogies must not be presented as shader results.

## Reference manifest

| ID | Source | URL | Use |
|---|---|---|---|
| WGSL | WebGPU Shading Language specification | https://www.w3.org/TR/WGSL/ | Normative integer, array, builtin, and control-flow semantics. |
| WEBGPU | WebGPU specification | https://www.w3.org/TR/webgpu/ | Runtime/device semantics and portability limits. |
| SPIRV | SPIR-V specification | https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html | Backend instruction and structured-control-flow semantics. |
| VULKAN | Vulkan specification | https://registry.khronos.org/vulkan/specs/1.3-extensions/html/ | Device timestamp and memory-model constraints. |
| WGPU6345 | wgpu bounds-check discussion | https://github.com/gfx-rs/wgpu/issues/6345 | Upstream motivation and constraints. |
| ABCD | Bodík, Gupta, Sarkar, “ABCD”, PLDI 2000 | https://bernsteinbear.com/assets/img/abcd.pdf | Demand-driven inequality/range reasoning. |
| W07 | Würthinger et al., PPPJ 2007 | https://ssw.jku.at/Research/Papers/Wuerthinger07/ | SSA BCE, hoisting, and evaluation design. |
| W09 | Würthinger et al., 2009 | http://www.christianwimmer.at/Publications/Wuerthinger09a/Wuerthinger09a.pdf | Integer overflow and realistic performance limits. |
| COUSOT | Cousot & Cousot, 1977 | https://www.di.ens.fr/~cousot/COUSOTpapers/POPL77.shtml | Abstract interpretation foundations. |
| SCEV | LLVM ScalarEvolution | https://llvm.org/docs/ScalarEvolution.html | AddRec recurrence and range APIs. |
| IRCE | LLVM IRCE RFC | https://discourse.llvm.org/t/rfc-inductive-range-check-elimination-via-iteration-space-splitting/34549 | Affine range checks and loop splitting. |
| GO_PROVE | Go prove pass | https://github.com/golang/go/blob/master/src/cmd/compile/internal/ssa/prove.go | Dominator facts and proof propagation. |
| GO_LOOPBCE | Go loop BCE | https://github.com/golang/go/blob/master/src/cmd/compile/internal/ssa/loopbce.go | True induction-variable recognition. |
| GO_TEST | Go BCE expectations | https://github.com/golang/go/blob/master/test/prove.go | Inline positive/negative regression style. |
| READYSET | Readyset bounds-check measurement | https://readyset.io/blog/bounds-checks | Counterevidence against assuming timing wins. |
| RUSTPERF | Rust Performance Book | https://nnethercote.github.io/perf-book/bounds-checks.html | Practical safe idioms and measurement framing. |
| GSOHC | GSOHC ECOOP 2025 | https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2025.21 | Closest modern GPU/shader context and artifact practice. |

## Archive status

The manifest is created before downloading so every future copy has a stable
identifier. Downloaded documents must be checked for HTTP status, content type,
hash, and license before being cited. Dead links are recorded, not silently
replaced with an unverified mirror.
