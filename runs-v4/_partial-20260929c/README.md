# Quarantined: campaign of 2026-09-29 ~11:49Z on wgpu 19ffb4e

Launched when the machine was idle. At 17:19:46 IST (11:49:46Z) an unrelated GPU compute job
(spectrometry_public python, PID 21175, 100% GPU) started. 4 runs completed; the rest were refused by
the pre-run noise gate (environment-blocked.txt). Measured-window vs job-start check:
empty, matmul_tiled, reduce_tree ended 20.2 s / 9.8 s / 0.3 s before the job started;
**vello_binning overlapped the job entirely** (the pre-run gate could not detect a job starting
mid-run). Harness now re-checks GPU compute apps after timing (`quiet_during_run`).
None of these runs are used in any summary; the campaign is re-run in full.
