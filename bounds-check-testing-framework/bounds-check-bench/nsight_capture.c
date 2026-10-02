/**
 * nsight_capture.c — Minimal Nsight Graphics SDK wrapper for Vulkan compute capture.
 *
 * Uses dlsym(RTLD_DEFAULT, ...) to find Nsight SDK functions that are
 * exported by libngfx-capture-interception.so (preloaded by ngfx-capture).
 *
 * Usage:
 *   1. Call nsight_initialize() once after Vulkan device creation.
 *   2. Call nsight_start_capture() before the dispatch to capture.
 *   3. Call nsight_stop_capture() after the dispatch completes.
 */

#include <dlfcn.h>
#include <stdint.h>
#include <stddef.h>

/* ── Nsight SDK parameter structs (we only need the version field) ── */

#define NGFX_MAKE_STRUCT_VERSION(ver) ver

typedef struct {
    uint32_t version;
} NsightParams_V1;
#define NsightParams_V1_VER NGFX_MAKE_STRUCT_VERSION(1)

typedef int NGFX_Result;

/* ── Function pointer typedefs ── */

typedef NGFX_Result (*StartCaptureFn)(NsightParams_V1 *);
typedef NGFX_Result (*StopCaptureFn)(NsightParams_V1 *);
typedef NGFX_Result (*InitActivityFn)(NsightParams_V1 *);
typedef NGFX_Result (*FrameBoundaryFn)(NsightParams_V1 *);

/* ── Resolved function pointers (set once by nsight_initialize) ── */

static StartCaptureFn  g_start_capture   = NULL;
static StopCaptureFn   g_stop_capture    = NULL;
static InitActivityFn  g_init_activity   = NULL;
static FrameBoundaryFn g_frame_boundary  = NULL;

/* ── Public API ── */

/**
 * Resolve Nsight SDK symbols via dlsym and call InitializeCaptureActivity.
 * Returns 0 on success, -1 if symbols aren't available (e.g. not running
 * under ngfx-capture).
 */
int nsight_initialize(void) {
    g_start_capture  = (StartCaptureFn)dlsym(RTLD_DEFAULT,
        "NGFX_GraphicsCapture_StartCaptureVulkan");
    g_stop_capture   = (StopCaptureFn)dlsym(RTLD_DEFAULT,
        "NGFX_GraphicsCapture_StopCaptureVulkan");
    g_init_activity  = (InitActivityFn)dlsym(RTLD_DEFAULT,
        "NGFX_GraphicsCapture_InitializeCaptureActivityVulkan");
    g_frame_boundary = (FrameBoundaryFn)dlsym(RTLD_DEFAULT,
        "NGFX_FrameBoundaryVulkan");

    if (!g_init_activity || !g_start_capture || !g_stop_capture)
        return -1;

    NsightParams_V1 params = { .version = 1 };
    return g_init_activity(&params);
}

/**
 * Start a programmable capture. Must be paired with nsight_stop_capture().
 * Returns 0 on success, -1 if not initialized.
 */
int nsight_start_capture(void) {
    if (!g_start_capture) return -1;
    NsightParams_V1 params = { .version = 1 };
    return g_start_capture(&params);
}

/**
 * Stop an active capture. The capture file is written when this is called.
 * Returns 0 on success, -1 if not initialized.
 */
int nsight_stop_capture(void) {
    if (!g_stop_capture) return -1;
    NsightParams_V1 params = { .version = 1 };
    return g_stop_capture(&params);
}

/**
 * Insert a frame boundary marker. Helps Nsight Graphics identify frame
 * boundaries in headless compute applications.
 * Returns 0 on success, -1 if not available.
 */
int nsight_frame_boundary(void) {
    if (!g_frame_boundary) return -1;
    NsightParams_V1 params = { .version = 1 };
    return g_frame_boundary(&params);
}

/**
 * Returns 1 if Nsight SDK functions were resolved successfully, 0 otherwise.
 */
int nsight_is_available(void) {
    return (g_start_capture != NULL && g_stop_capture != NULL) ? 1 : 0;
}
