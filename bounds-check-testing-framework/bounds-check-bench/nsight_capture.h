#ifndef NSIGHT_CAPTURE_H
#define NSIGHT_CAPTURE_H

/**
 * Resolve Nsight SDK symbols and initialize capture activity.
 * Call once after device creation. Returns 0 on success.
 */
int nsight_initialize(void);

/**
 * Start a programmable capture. Returns 0 on success.
 */
int nsight_start_capture(void);

/**
 * Stop an active capture. Capture file is written on stop. Returns 0 on success.
 */
int nsight_stop_capture(void);

/**
 * Insert a frame boundary marker. Helps Nsight identify frames in headless compute.
 */
int nsight_frame_boundary(void);

/**
 * Returns 1 if Nsight SDK is available (running under ngfx-capture).
 */
int nsight_is_available(void);

#endif /* NSIGHT_CAPTURE_H */
