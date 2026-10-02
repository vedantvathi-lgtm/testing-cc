// Builtin-seeded bounds-check elision: soundness / scope test cases.
// Companion to the IICT paper §4 (proposed optimization) and §4.3 (scope).
// This file is paired with bounds-check-elision-zero.toml, which pins
// BoundsCheckPolicy::ReadZeroSkipWrite (the wgpu-native default).
//
// IMPORTANT - read before adding cases: every "should be elided" access
// below is written as a *direct statement in main()*. The seeding rule in
// the proposed patch only fires for an Expression::FunctionArgument whose
// own function parameter carries a Binding::BuiltIn annotation - which is
// only ever true for an entry point's own parameters. Naga performs no
// inlining before validation (verified directly against
// naga::valid::validate_function / validate_entry_point - see design doc
// §2, "Tier 1"), so moving any of these accesses into a helper function
// called from main() would silently fall into Case 6 below instead of
// testing what its name claims to test.

var<workgroup> happy_exact: array<f32, 64>;
var<workgroup> happy_oversized: array<f32, 128>;
var<workgroup> stencil_tile: array<f32, 64>;
var<workgroup> wraparound_tile: array<f32, 64>;
var<workgroup> multiplicative_tile: array<f32, 128>;
var<workgroup> helper_tile: array<f32, 64>;
var<workgroup> store_target: array<f32, 64>;

// Case 6's helper. `i` is an ordinary u32 parameter here - no Binding::BuiltIn
// survives the call boundary, regardless of what range the caller's argument
// actually had.
fn touch(i: u32, v: f32) {
    helper_tile[i] = v;
}

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    // Case 1 - happy path, exact-size buffer.
    // lid in [0,63]; array length 64. Provably in bounds.
    // EXPECT: check elided -> bare OpAccessChain/OpLoad, no OpSelectionMerge.
    let a = happy_exact[lid];

    // Case 2 - happy path, oversized buffer.
    // lid in [0,63]; array length 128. Still provably in bounds.
    // EXPECT: check elided.
    let b = happy_oversized[lid];

    // Case 3 - stencil overflow. lid+1 in [1,64]; 64 is NOT < 64.
    // EXPECT: check RETAINED.
    let c = stencil_tile[lid + 1u];

    // Case 4 - wraparound, the critical soundness case.
    // For lid=0, `lid - 1u` wraps to 4294967295 in real execution.
    // u32::checked_sub must report this range as unrepresentable (None).
    // EXPECT: check RETAINED. An elided check here is a correctness bug,
    // not a missed optimization - see paper §4.3.
    let d = wraparound_tile[lid - 1u];

    // Case 5 - multiplicative index, explicitly out of scope for v1
    // (Mul is not handled by the range-propagation arm in this version).
    // EXPECT: check RETAINED.
    let e = multiplicative_tile[lid * 2u];

    // Case 6 - helper-function-forwarded builtin (documented scope limit).
    // lid is provably in [0,63] at this call site, but that fact does not
    // survive the call into `touch`, whose own parameter carries no builtin
    // binding. EXPECT: check RETAINED, even though the access is, in fact,
    // always safe - this is the accepted v1 limitation, not a bug. If this
    // case ever shows an elided check WITHOUT a corresponding change to the
    // paper's §4.3 scope section, the patch has grown undocumented scope.
    touch(lid, a + b + c + d + e);

    // Case 7 - store-side mirror of Case 1, to exercise the cheaper
    // guarded-store path under ReadZeroSkipWrite (no OpPhi even in the
    // *unoptimized* baseline - see paper §4.4's correction).
    // EXPECT: check elided.
    store_target[lid] = a;
}
