# Phase 1 Data Model: branch fact states

| State (per branch edge) | Meaning | Produced when | Effect at the join after `if` |
|---|---|---|---|
| **Infeasible** (`None`) | No execution can take this edge | Refined range of the tested local is empty | Edge ignored; state = other edge |
| **Unchanged** (`Some(facts)`) | Nothing learned from the condition | Condition not a comparison; operand bound unknown or not a single constant; no tracked local; local not a 32-bit integer | Facts flow through unchanged |
| **Refined** (`Some(facts')`) | Tested local's range narrowed | Comparison of tracked local vs exact constant with non-empty result | Joined (hull) with the other edge |

Invariants:
- Infeasible is only produced from an empty intersection (never from "unknown").
- Conjunction true-edge / disjunction false-edge: refine left then right; each part may only
  narrow or prove emptiness.
- Conjunction false-edge / disjunction true-edge: Unchanged (unchanged from current code).
