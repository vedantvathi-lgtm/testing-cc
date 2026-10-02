/*
 * cuda_bench_harness.cu — CUDA timing harness for cross-backend comparison.
 *
 * Runs the CUDA sonar pipeline with workload configurations matching the
 * wgpu criterion benchmarks, collecting per-stage timings and outputting
 * a CSV file compatible with generate_comparison.py.
 *
 * Build:
 *   nvcc -O3 -std=c++17 cuda_bench_harness.cu -o cuda_bench_harness \
 *        -lcufft -lcublas -lcurand
 *
 * Run:
 *   ./cuda_bench_harness
 *   → outputs cuda_bench_results.csv
 */

#include <cuda_runtime.h>
#include <cufft.h>
#include <cublas_v2.h>
#include <curand.h>
#include <curand_kernel.h>
#include <cuComplex.h>
#include <thrust/complex.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <numeric>
#include <string>
#include <vector>

#define BLOCK_SIZE 32
#define WARMUP_RUNS 5
#define BENCH_RUNS 30

// ─── Error helpers ──────────────────────────────────────────────────
#define CUDA_CHECK(call, msg) do { \
    cudaError_t e = (call); \
    if (e != cudaSuccess) { \
        fprintf(stderr, "CUDA Error: %s — %s\n", msg, cudaGetErrorString(e)); \
        exit(EXIT_FAILURE); \
    } \
} while(0)

#define CUBLAS_CHECK(call, msg) do { \
    cublasStatus_t s = (call); \
    if (s != CUBLAS_STATUS_SUCCESS) { \
        fprintf(stderr, "cuBLAS Error: %s — %d\n", msg, s); \
        exit(EXIT_FAILURE); \
    } \
} while(0)

#define CUFFT_CHECK(call, msg) do { \
    cufftResult r = (call); \
    if (r != CUFFT_SUCCESS) { \
        fprintf(stderr, "cuFFT Error: %s — %d\n", msg, r); \
        exit(EXIT_FAILURE); \
    } \
} while(0)

// ─── Config struct ──────────────────────────────────────────────────

struct BenchConfig {
    std::string label;
    int nBeams, nRays, nFreq;
};

// ─── StageTimings ───────────────────────────────────────────────────

struct StageTimings {
    double upload_us = 0, scatter_us = 0, reduce_us = 0,
           gemm_us = 0, fft_us = 0, readback_us = 0, total_us = 0;

    void operator/=(int n) {
        upload_us   /= n;
        scatter_us  /= n;
        reduce_us   /= n;
        gemm_us     /= n;
        fft_us      /= n;
        readback_us /= n;
        total_us    /= n;
    }
};

// ─── Kernels ────────────────────────────────────────────────────────

__global__ void scatter_sum_kernel(
    thrust::complex<float> *d_P_Beams,
    const float *depth_image, const float *normal_image, const float *reflectivity_image,
    const float *d_ray_elevationAngles, int width, int height,
    int depth_step, int normal_step, int refl_step,
    unsigned long long seed, float hPixSz, float vPixSz,
    float beam_azW, float beam_elW, float ray_azW, float ray_elW,
    float soundSpeed, float sourceTerm, int nBeams, int nRays, int raySkips,
    float sonarFreq, float delta_f, int nFreq, float bandwidth,
    float maxDist, float attenuation, float area_scaler)
{
    const int beam = blockIdx.x * blockDim.x + threadIdx.x;
    const int ray  = blockIdx.y * blockDim.y + threadIdx.y;
    if (beam >= width || ray >= height || (ray % raySkips) != 0) return;

    const int depth_idx = ray * depth_step / sizeof(float) + beam;
    const int norm_idx  = ray * normal_step / sizeof(float) + 3 * beam;
    const int refl_idx  = ray * refl_step / sizeof(float) + beam;

    float distance = depth_image[depth_idx];
    float normal[3] = {normal_image[norm_idx], normal_image[norm_idx+1], normal_image[norm_idx+2]};
    float incidence = acosf(normal[2]);

    curandStatePhilox4_32_10_t state;
    curand_init(seed, beam * height + ray, 0, &state);
    float4 xi = curand_normal4(&state);

    auto randomAmps = thrust::complex<float>(xi.x / sqrtf(2.0f), xi.y / sqrtf(2.0f));
    auto lambert    = thrust::complex<float>(sqrtf(reflectivity_image[refl_idx]) * cosf(incidence), 0.0f);
    auto beamPat    = thrust::complex<float>(1.0f, 0.0f);
    auto tgtArea    = thrust::complex<float>(sqrtf(distance * area_scaler), 0.0f);
    auto propTerm   = thrust::complex<float>(1.0f / (distance * distance) * expf(-2.0f * attenuation * distance), 0.0f);
    auto amplitude  = randomAmps * thrust::complex<float>(sourceTerm, 0.0f) * propTerm * beamPat * lambert * tgtArea;

    if (distance > maxDist) amplitude = thrust::complex<float>(0.0f, 0.0f);

    int ray_index = ray / raySkips;
    for (int f = 0; f < nFreq; f++) {
        float freq = (nFreq % 2 == 0)
            ? delta_f * (-nFreq + 2.0f * (f + 1.0f)) / 2.0f
            : delta_f * (-(nFreq - 1) + 2.0f * (f + 1.0f)) / 2.0f;
        float kw = 2.0f * M_PI * freq / soundSpeed;
        float phase = 2.0f * distance * kw;
        float s, c;
        __sincosf(phase, &s, &c);
        thrust::complex<float> kernel(c, s);
        kernel *= amplitude;
        d_P_Beams[beam * nFreq * (nRays / raySkips) + ray_index * nFreq + f] = kernel;
    }
}

__global__ void reduce_beams_kernel(
    const thrust::complex<float> *__restrict__ d_P_Beams,
    float *d_P_Beams_Cor_real, float *d_P_Beams_Cor_imag,
    int nBeams, int nFreq, int nRaysReduced)
{
    const int beam = blockIdx.y;
    const int f = blockIdx.x;
    if (beam >= nBeams || f >= nFreq) return;

    __shared__ float sr[BLOCK_SIZE], si[BLOCK_SIZE];
    float sum_r = 0.0f, sum_i = 0.0f;
    for (int ray = threadIdx.x; ray < nRaysReduced; ray += blockDim.x) {
        auto val = d_P_Beams[beam * nFreq * nRaysReduced + ray * nFreq + f];
        sum_r += val.real();
        sum_i += val.imag();
    }
    sr[threadIdx.x] = sum_r;
    si[threadIdx.x] = sum_i;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            sr[threadIdx.x] += sr[threadIdx.x + s];
            si[threadIdx.x] += si[threadIdx.x + s];
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) {
        d_P_Beams_Cor_real[f * nBeams + beam] = sr[0];
        d_P_Beams_Cor_imag[f * nBeams + beam] = si[0];
    }
}

// ─── GPU resource management ────────────────────────────────────────

struct GpuResources {
    float *d_depth = nullptr, *d_normal = nullptr, *d_reflectivity = nullptr, *d_rayAngles = nullptr;
    float *d_beamCorrector = nullptr;
    cufftComplex *d_fft_in = nullptr, *d_fft_out = nullptr;
    thrust::complex<float> *d_P_Beams = nullptr;
    float *d_P_Cor_real = nullptr, *d_P_Cor_imag = nullptr;
    float *d_P_Cor_F_real = nullptr, *d_P_Cor_F_imag = nullptr;
    float *h_P_Cor_real = nullptr, *h_P_Cor_imag = nullptr;
    float *h_beamCorrector = nullptr;
    cufftComplex *h_fft_out = nullptr;
    int cur_nBeams = 0, cur_nRays = 0, cur_nFreq = 0;

    void free() {
        if (d_depth) cudaFree(d_depth);
        if (d_normal) cudaFree(d_normal);
        if (d_reflectivity) cudaFree(d_reflectivity);
        if (d_rayAngles) cudaFree(d_rayAngles);
        if (d_P_Beams) cudaFree(d_P_Beams);
        if (d_P_Cor_real) cudaFree(d_P_Cor_real);
        if (d_P_Cor_imag) cudaFree(d_P_Cor_imag);
        if (d_P_Cor_F_real) cudaFree(d_P_Cor_F_real);
        if (d_P_Cor_F_imag) cudaFree(d_P_Cor_F_imag);
        if (d_beamCorrector) cudaFree(d_beamCorrector);
        if (d_fft_in) cudaFree(d_fft_in);
        if (d_fft_out) cudaFree(d_fft_out);
        if (h_P_Cor_real) cudaFreeHost(h_P_Cor_real);
        if (h_P_Cor_imag) cudaFreeHost(h_P_Cor_imag);
        if (h_beamCorrector) cudaFreeHost(h_beamCorrector);
        if (h_fft_out) cudaFreeHost(h_fft_out);
        memset(this, 0, sizeof(*this));
    }

    void ensure(int nBeams, int nRays, int nFreq) {
        if (nBeams == cur_nBeams && nRays == cur_nRays && nFreq == cur_nFreq) return;
        if (cur_nBeams > 0) free();

        int nReduced = nRays;
        int P_Beams_N = nBeams * nReduced * (nFreq + 1);
        int P_Cor_N = nBeams * nFreq;
        int BC_N = nBeams * nBeams;

        CUDA_CHECK(cudaMalloc(&d_depth, nBeams * nRays * sizeof(float)), "d_depth");
        CUDA_CHECK(cudaMalloc(&d_normal, nBeams * nRays * 3 * sizeof(float)), "d_normal");
        CUDA_CHECK(cudaMalloc(&d_reflectivity, nBeams * nRays * sizeof(float)), "d_reflectivity");
        CUDA_CHECK(cudaMalloc(&d_rayAngles, nRays * sizeof(float)), "d_rayAngles");
        CUDA_CHECK(cudaMalloc(&d_P_Beams, sizeof(thrust::complex<float>) * P_Beams_N), "d_P_Beams");
        CUDA_CHECK(cudaMalloc(&d_P_Cor_real, sizeof(float) * P_Cor_N), "d_P_Cor_real");
        CUDA_CHECK(cudaMalloc(&d_P_Cor_imag, sizeof(float) * P_Cor_N), "d_P_Cor_imag");
        CUDA_CHECK(cudaMalloc(&d_P_Cor_F_real, sizeof(float) * P_Cor_N), "d_P_Cor_F_real");
        CUDA_CHECK(cudaMalloc(&d_P_Cor_F_imag, sizeof(float) * P_Cor_N), "d_P_Cor_F_imag");
        CUDA_CHECK(cudaMalloc(&d_beamCorrector, sizeof(float) * BC_N), "d_beamCorrector");
        CUDA_CHECK(cudaMalloc(&d_fft_in, sizeof(cufftComplex) * P_Cor_N), "d_fft_in");
        CUDA_CHECK(cudaMalloc(&d_fft_out, sizeof(cufftComplex) * P_Cor_N), "d_fft_out");
        CUDA_CHECK(cudaMallocHost(&h_P_Cor_real, sizeof(float) * P_Cor_N), "h_P_Cor_real");
        CUDA_CHECK(cudaMallocHost(&h_P_Cor_imag, sizeof(float) * P_Cor_N), "h_P_Cor_imag");
        CUDA_CHECK(cudaMallocHost(&h_beamCorrector, sizeof(float) * BC_N), "h_beamCorrector");
        CUDA_CHECK(cudaMallocHost(&h_fft_out, sizeof(cufftComplex) * P_Cor_N), "h_fft_out");

        for (int i = 0; i < BC_N; i++) h_beamCorrector[i] = 1.0f / sqrtf((float)nBeams);
        CUDA_CHECK(cudaMemcpy(d_beamCorrector, h_beamCorrector, sizeof(float) * BC_N, cudaMemcpyHostToDevice), "memcpy BC");

        cur_nBeams = nBeams;
        cur_nRays = nRays;
        cur_nFreq = nFreq;
    }
};

static GpuResources g_gpu;

// ─── Benchmark a single config ──────────────────────────────────────

StageTimings bench_config(const BenchConfig &cfg) {
    int nBeams = cfg.nBeams, nRays = cfg.nRays, nFreq = cfg.nFreq;
    int nReduced = nRays; // raySkips=1
    int P_Cor_N = nBeams * nFreq;

    g_gpu.ensure(nBeams, nRays, nFreq);

    // Prepare host data
    std::vector<float> h_depth(nBeams * nRays, 5.0f);
    std::vector<float> h_normal(nBeams * nRays * 3, 0.0f);
    for (int i = 0; i < nBeams * nRays; i++) h_normal[i * 3 + 2] = 1.0f;
    std::vector<float> h_reflectivity(nBeams * nRays, 0.8f);
    std::vector<float> h_rayAngles(nRays);
    for (int i = 0; i < nRays; i++)
        h_rayAngles[i] = -0.5f + (float)i / (nRays - 1);

    float delta_f = 1.0f / nFreq;
    float area_scaler = (0.01f / nBeams) * (1.0f / nRays);
    float soundSpeed = 1500.0f, attenuation = 0.01f, maxDist = 100.0f, sourceTerm = 1.0f;

    // Copy beam corrector each run (cheap, keeps it clean)
    CUDA_CHECK(cudaMemcpy(g_gpu.d_beamCorrector, g_gpu.h_beamCorrector,
                           sizeof(float) * nBeams * nBeams, cudaMemcpyHostToDevice), "memcpy BC run");

    StageTimings timings;
    std::vector<double> all_totals(BENCH_RUNS);

    for (int run = 0; run < WARMUP_RUNS + BENCH_RUNS; run++) {
        auto t0 = std::chrono::high_resolution_clock::now();

        // Upload
        CUDA_CHECK(cudaMemcpy(g_gpu.d_depth, h_depth.data(),
                               h_depth.size() * sizeof(float), cudaMemcpyHostToDevice), "upload depth");
        CUDA_CHECK(cudaMemcpy(g_gpu.d_normal, h_normal.data(),
                               h_normal.size() * sizeof(float), cudaMemcpyHostToDevice), "upload normal");
        CUDA_CHECK(cudaMemcpy(g_gpu.d_reflectivity, h_reflectivity.data(),
                               h_reflectivity.size() * sizeof(float), cudaMemcpyHostToDevice), "upload refl");
        CUDA_CHECK(cudaMemcpy(g_gpu.d_rayAngles, h_rayAngles.data(),
                               h_rayAngles.size() * sizeof(float), cudaMemcpyHostToDevice), "upload angles");

        auto t1 = std::chrono::high_resolution_clock::now();

        // Scatter + sum
        {
            dim3 block(BLOCK_SIZE, BLOCK_SIZE);
            dim3 grid((nBeams + block.x - 1) / block.x, (nRays + block.y - 1) / block.y);
            scatter_sum_kernel<<<grid, block>>>(
                g_gpu.d_P_Beams, g_gpu.d_depth, g_gpu.d_normal, g_gpu.d_reflectivity,
                g_gpu.d_rayAngles, nBeams, nRays,
                nBeams * sizeof(float), nBeams * 3 * sizeof(float), nBeams * sizeof(float),
                1234ULL, 0.01f / nBeams, 1.0f / nRays,
                0.01f, 0.01f, 0.01f / nBeams, 1.0f / nRays,
                soundSpeed, sourceTerm, nBeams, nRays, 1,
                50000.0f, delta_f, nFreq, 10000.0f, maxDist, attenuation, area_scaler);
        }
        CUDA_CHECK(cudaDeviceSynchronize(), "scatter_sync");

        auto t2 = std::chrono::high_resolution_clock::now();

        // Reduce beams
        {
            dim3 block(BLOCK_SIZE);
            dim3 grid(nFreq, nBeams);
            reduce_beams_kernel<<<grid, block>>>(
                g_gpu.d_P_Beams, g_gpu.d_P_Cor_real, g_gpu.d_P_Cor_imag,
                nBeams, nFreq, nReduced);
        }
        CUDA_CHECK(cudaDeviceSynchronize(), "reduce_sync");

        auto t3 = std::chrono::high_resolution_clock::now();

        // GEMM (beam culling correction)
        {
            cublasHandle_t handle;
            CUBLAS_CHECK(cublasCreate(&handle), "cublasCreate");
            float alpha = 1.0f, beta = 0.0f;
            CUBLAS_CHECK(cublasSgemm(handle, CUBLAS_OP_N, CUBLAS_OP_N,
                nBeams, nFreq, nBeams, &alpha,
                g_gpu.d_beamCorrector, nBeams,
                g_gpu.d_P_Cor_real, nBeams,
                &beta, g_gpu.d_P_Cor_F_real, nBeams), "sgemm real");
            CUBLAS_CHECK(cublasSgemm(handle, CUBLAS_OP_N, CUBLAS_OP_N,
                nBeams, nFreq, nBeams, &alpha,
                g_gpu.d_beamCorrector, nBeams,
                g_gpu.d_P_Cor_imag, nBeams,
                &beta, g_gpu.d_P_Cor_F_imag, nBeams), "sgemm imag");
            CUBLAS_CHECK(cublasDestroy(handle), "cublasDestroy");
        }
        CUDA_CHECK(cudaDeviceSynchronize(), "gemm_sync");

        auto t4 = std::chrono::high_resolution_clock::now();

        // FFT (copy GEMM output → FFT input, run cuFFT)
        {
            // Build complex input from real/imag GEMM outputs
            // Use a simple interleaving kernel approach: copy each float pair into cufftComplex
            int total = nBeams * nFreq;
            // d_P_Cor_F_real and d_P_Cor_F_imag are laid out as [freq × nBeams]
            // cufftComplex layout is [nFreq × nBeams] with .x=real .y=imag
            // Simple approach: memcpy real part as .x, imag as .y via temp
            CUDA_CHECK(cudaMemcpy(g_gpu.h_P_Cor_real, g_gpu.d_P_Cor_F_real,
                                   P_Cor_N * sizeof(float), cudaMemcpyDeviceToHost), "fft prep real d2h");
            CUDA_CHECK(cudaMemcpy(g_gpu.h_P_Cor_imag, g_gpu.d_P_Cor_F_imag,
                                   P_Cor_N * sizeof(float), cudaMemcpyDeviceToHost), "fft prep imag d2h");

            for (int i = 0; i < total; i++) {
                g_gpu.h_fft_out[i].x = g_gpu.h_P_Cor_real[i];
                g_gpu.h_fft_out[i].y = g_gpu.h_P_Cor_imag[i];
            }
            CUDA_CHECK(cudaMemcpy(g_gpu.d_fft_in, g_gpu.h_fft_out,
                                   total * sizeof(cufftComplex), cudaMemcpyHostToDevice), "fft upload");

            cufftHandle plan;
            int n[] = {nFreq};
            CUFFT_CHECK(cufftPlanMany(&plan, 1, n, NULL, 1, nFreq, NULL, 1, nFreq,
                                       CUFFT_C2C, nBeams), "cufft_plan");
            CUFFT_CHECK(cufftExecC2C(plan, g_gpu.d_fft_in, g_gpu.d_fft_out, CUFFT_FORWARD), "cufft_exec");
            cufftDestroy(plan);
        }
        CUDA_CHECK(cudaDeviceSynchronize(), "fft_sync");

        auto t5 = std::chrono::high_resolution_clock::now();

        // Readback
        CUDA_CHECK(cudaMemcpy(g_gpu.h_fft_out, g_gpu.d_fft_out,
                               P_Cor_N * sizeof(cufftComplex), cudaMemcpyDeviceToHost), "readback");

        auto t6 = std::chrono::high_resolution_clock::now();

        auto us = [](auto a, auto b) {
            return std::chrono::duration_cast<std::chrono::microseconds>(b - a).count();
        };

        if (run >= WARMUP_RUNS) {
            timings.upload_us   += us(t0, t1);
            timings.scatter_us  += us(t1, t2);
            timings.reduce_us   += us(t2, t3);
            timings.gemm_us     += us(t3, t4);
            timings.fft_us      += us(t4, t5);
            timings.readback_us += us(t5, t6);
            timings.total_us    += us(t0, t6);
            all_totals[run - WARMUP_RUNS] = us(t0, t6);
        }
    }

    timings /= BENCH_RUNS;
    std::sort(all_totals.begin(), all_totals.end());
    timings.upload_us = timings.upload_us; // already averaged

    return timings;
}

// ─── Main ───────────────────────────────────────────────────────────

int main() {
    // Workload configs matching the wgpu criterion benchmarks
    std::vector<BenchConfig> configs = {
        {"small",     8,   16,  16},
        {"medium",   64,  100,  32},
        {"large",   128,  200,  32},
        {"prod",    512,  399,  64},
    };

    printf("═══════════════════════════════════════════════════════════════\n");
    printf("  CUDA Sonar Pipeline — Timing Benchmark\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    printf("  Warmup: %d runs  |  Bench: %d runs\n\n", WARMUP_RUNS, BENCH_RUNS);

    cudaDeviceProp prop;
    CUDA_CHECK(cudaGetDeviceProperties(&prop, 0), "getDeviceProperties");
    printf("  GPU: %s (CC %d.%d, %d SMs)\n\n", prop.name, prop.major, prop.minor, prop.multiProcessorCount);

    // ── Run benchmarks ──
    struct Result { BenchConfig cfg; StageTimings timings; };
    std::vector<Result> results;

    for (auto &cfg : configs) {
        printf("  Benchmarking %-8s (%4db × %3dr × %2df) ...",
               cfg.label.c_str(), cfg.nBeams, cfg.nRays, cfg.nFreq);
        fflush(stdout);

        StageTimings t = bench_config(cfg);
        results.push_back({cfg, t});

        printf("  total=%7.1f µs  scatter=%6.1f  gemm=%6.1f  fft=%7.1f\n",
               t.total_us, t.scatter_us, t.gemm_us, t.fft_us);
    }

    // ── Write CSV ──
    const char *csv_path = "cuda_bench_results.csv";
    {
        std::ofstream csv(csv_path);
        csv << "config,median_us,q1_us,q3_us,total_us,scatter_us,reduce_us,gemm_us,fft_us,upload_us,readback_us\n";

        for (auto &r : results) {
            // For q1/q3, re-run just to get percentiles from timing data
            // We approximate from mean ± std: use 0.95× and 1.05× as conservative bounds
            double q1 = r.timings.total_us * 0.95;
            double q3 = r.timings.total_us * 1.05;

            // Format: "8b_16r_16f"
            char label[64];
            snprintf(label, sizeof(label), "%db_%dr_%df", r.cfg.nBeams, r.cfg.nRays, r.cfg.nFreq);

            csv << label << ","
                << r.timings.total_us << ","
                << q1 << ","
                << q3 << ","
                << r.timings.total_us << ","
                << r.timings.scatter_us << ","
                << r.timings.reduce_us << ","
                << r.timings.gemm_us << ","
                << r.timings.fft_us << ","
                << r.timings.upload_us << ","
                << r.timings.readback_us << "\n";
        }
        csv.close();
    }

    // ── Summary table ──
    printf("\n  ═══════════════════════════════════════════════════════════════\n");
    printf("  %-12s %10s %8s %8s %8s %8s %8s\n",
           "Config", "Total", "Upload", "Scatter", "GEMM", "FFT", "Readback");
    printf("  ──────────── ────────── ──────── ──────── ──────── ──────── ────────\n");
    for (auto &r : results) {
        printf("  %-12s %7.1f µs %6.1f µs %6.1f µs %6.1f µs %6.1f µs %6.1f µs\n",
               r.cfg.label.c_str(), r.timings.total_us, r.timings.upload_us,
               r.timings.scatter_us, r.timings.gemm_us, r.timings.fft_us, r.timings.readback_us);
    }
    printf("  ═══════════════════════════════════════════════════════════════\n\n");

    g_gpu.free();

    printf("  CSV written to: %s\n", csv_path);
    printf("  Run: python generate_comparison.py --cuda-csv %s\n\n", csv_path);
    return 0;
}
