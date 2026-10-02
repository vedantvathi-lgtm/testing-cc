# Contract: `FunctionInfo.safe_accesses` (proof set)

Consumers: SPIR-V writer (`back/spv/index.rs:522`), `proc::index::access_needs_check`
(`proc/index.rs:344`) used by MSL and HLSL writers.

1. An `Access` handle is in the set ⇒ on every execution reaching it, its index is within
   `[0, length)` of the indexed object. (Soundness — this feature restores it for GLSL/SPIR-V
   input.)
2. Absence from the set carries no meaning beyond "not proven"; consumers emit the configured
   policy's check.
3. The set is a pure function of the validated module and the elision flag; no dependence on
   front end beyond the IR it produced.

This feature changes no signature and no consumer; it only removes false infeasibility inside
the producer.
