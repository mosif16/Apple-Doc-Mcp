use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use super::types::{
    CudaCategory, CudaCategoryItem, CudaExample, CudaMethod,
    CudaMethodIndex, CudaMethodKind, CudaParameter, CudaReturnField,
    CudaReturnType, CudaTechnology,
    CUDA_MEMORY_METHODS, CUDA_DEVICE_METHODS, CUDA_EXECUTION_METHODS,
    CUDA_STREAM_METHODS, CUDA_EVENT_METHODS, CUDA_ERROR_METHODS,
    CUDA_KERNEL_CONSTRUCTS, CUDA_LIBRARY_METHODS, CUDA_GPU_SPECS,
    CUDA_OPTIMIZATION_METHODS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const CUDA_DOCS_URL: &str = "https://docs.nvidia.com/cuda";
const CUDA_RUNTIME_API_URL: &str = "https://docs.nvidia.com/cuda/cuda-runtime-api";
const CUDA_PROGRAMMING_GUIDE_URL: &str = "https://docs.nvidia.com/cuda/cuda-c-programming-guide";

#[derive(Debug)]
pub struct CudaClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<String>,
    fetch_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for CudaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CudaClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("cuda");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create CUDA cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::hours(1)),
            fetch_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Get available technologies (CUDA categories)
    #[instrument(name = "cuda_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<CudaTechnology>> {
        let runtime_api = CudaTechnology {
            identifier: "cuda:runtime".to_string(),
            title: "CUDA Runtime API".to_string(),
            description: format!(
                "CUDA Runtime API - {} functions for memory management, device control, streams, and events",
                CUDA_MEMORY_METHODS.len() + CUDA_DEVICE_METHODS.len() + CUDA_EXECUTION_METHODS.len()
                + CUDA_STREAM_METHODS.len() + CUDA_EVENT_METHODS.len() + CUDA_ERROR_METHODS.len()
            ),
            url: CUDA_RUNTIME_API_URL.to_string(),
            item_count: CUDA_MEMORY_METHODS.len() + CUDA_DEVICE_METHODS.len() + CUDA_EXECUTION_METHODS.len()
                + CUDA_STREAM_METHODS.len() + CUDA_EVENT_METHODS.len() + CUDA_ERROR_METHODS.len(),
        };

        let kernel_programming = CudaTechnology {
            identifier: "cuda:kernels".to_string(),
            title: "Kernel Programming".to_string(),
            description: format!(
                "CUDA Kernel Programming - {} constructs for __global__, __shared__, __device__, synchronization, atomics, and warp operations",
                CUDA_KERNEL_CONSTRUCTS.len()
            ),
            url: CUDA_PROGRAMMING_GUIDE_URL.to_string(),
            item_count: CUDA_KERNEL_CONSTRUCTS.len(),
        };

        let libraries = CudaTechnology {
            identifier: "cuda:libraries".to_string(),
            title: "CUDA Libraries".to_string(),
            description: format!(
                "CUDA Libraries - {} functions from cuBLAS, cuDNN, cuFFT, cuRAND, and NCCL",
                CUDA_LIBRARY_METHODS.len()
            ),
            url: format!("{}/libraries", CUDA_DOCS_URL),
            item_count: CUDA_LIBRARY_METHODS.len(),
        };

        let gpu_specs = CudaTechnology {
            identifier: "cuda:gpu".to_string(),
            title: "GPU Specifications".to_string(),
            description: format!(
                "GPU Specifications - {} items covering RTX 3070, RTX 4090, compute capabilities, and hardware features",
                CUDA_GPU_SPECS.len()
            ),
            url: "https://developer.nvidia.com/cuda-gpus".to_string(),
            item_count: CUDA_GPU_SPECS.len(),
        };

        let optimization = CudaTechnology {
            identifier: "cuda:optimization".to_string(),
            title: "Optimization Techniques".to_string(),
            description: format!(
                "Optimization Techniques - {} best practices for memory coalescing, occupancy, warp efficiency, and performance tuning",
                CUDA_OPTIMIZATION_METHODS.len()
            ),
            url: "https://docs.nvidia.com/cuda/cuda-c-best-practices-guide".to_string(),
            item_count: CUDA_OPTIMIZATION_METHODS.len(),
        };

        Ok(vec![runtime_api, kernel_programming, libraries, gpu_specs, optimization])
    }

    /// Get a category of methods
    #[instrument(name = "cuda_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<CudaCategory> {
        let (methods, title, description): (Vec<&CudaMethodIndex>, &str, &str) = match identifier {
            "cuda:runtime" | "runtime" | "api" => {
                let methods: Vec<&CudaMethodIndex> = CUDA_MEMORY_METHODS.iter()
                    .chain(CUDA_DEVICE_METHODS.iter())
                    .chain(CUDA_EXECUTION_METHODS.iter())
                    .chain(CUDA_STREAM_METHODS.iter())
                    .chain(CUDA_EVENT_METHODS.iter())
                    .chain(CUDA_ERROR_METHODS.iter())
                    .collect();
                (methods, "CUDA Runtime API", "High-level API for GPU memory management, device control, kernel execution, streams, events, and error handling")
            }
            "cuda:memory" | "memory" => (
                CUDA_MEMORY_METHODS.iter().collect(),
                "Memory Management",
                "Functions for allocating, freeing, and copying GPU memory",
            ),
            "cuda:device" | "device" => (
                CUDA_DEVICE_METHODS.iter().collect(),
                "Device Management",
                "Functions for device enumeration, selection, and property queries",
            ),
            "cuda:execution" | "execution" | "launch" => (
                CUDA_EXECUTION_METHODS.iter().collect(),
                "Kernel Execution",
                "Functions for launching kernels and managing execution configuration",
            ),
            "cuda:stream" | "stream" | "streams" => (
                CUDA_STREAM_METHODS.iter().collect(),
                "Stream Management",
                "Functions for creating and managing asynchronous streams",
            ),
            "cuda:event" | "event" | "events" => (
                CUDA_EVENT_METHODS.iter().collect(),
                "Event Management",
                "Functions for GPU timing and synchronization",
            ),
            "cuda:error" | "error" | "errors" => (
                CUDA_ERROR_METHODS.iter().collect(),
                "Error Handling",
                "Functions for CUDA error checking and reporting",
            ),
            "cuda:kernels" | "kernels" | "kernel" => (
                CUDA_KERNEL_CONSTRUCTS.iter().collect(),
                "Kernel Programming",
                "Kernel constructs, memory qualifiers, thread indexing, and synchronization primitives",
            ),
            "cuda:libraries" | "libraries" | "libs" => (
                CUDA_LIBRARY_METHODS.iter().collect(),
                "CUDA Libraries",
                "cuBLAS, cuDNN, cuFFT, cuRAND, and NCCL functions",
            ),
            "cuda:cublas" | "cublas" | "blas" => (
                CUDA_LIBRARY_METHODS.iter().filter(|m| m.category == "cublas").collect(),
                "cuBLAS",
                "CUDA Basic Linear Algebra Subroutines for matrix operations",
            ),
            "cuda:cudnn" | "cudnn" | "dnn" => (
                CUDA_LIBRARY_METHODS.iter().filter(|m| m.category == "cudnn").collect(),
                "cuDNN",
                "CUDA Deep Neural Network library for convolutions, activations, and more",
            ),
            "cuda:cufft" | "cufft" | "fft" => (
                CUDA_LIBRARY_METHODS.iter().filter(|m| m.category == "cufft").collect(),
                "cuFFT",
                "CUDA Fast Fourier Transform library",
            ),
            "cuda:curand" | "curand" | "random" => (
                CUDA_LIBRARY_METHODS.iter().filter(|m| m.category == "curand").collect(),
                "cuRAND",
                "CUDA Random Number Generation library",
            ),
            "cuda:nccl" | "nccl" => (
                CUDA_LIBRARY_METHODS.iter().filter(|m| m.category == "nccl").collect(),
                "NCCL",
                "NVIDIA Collective Communications Library for multi-GPU operations",
            ),
            "cuda:gpu" | "gpu" | "specs" | "rtx" => (
                CUDA_GPU_SPECS.iter().collect(),
                "GPU Specifications",
                "RTX 3070 and RTX 4090 specifications, compute capabilities, and hardware features",
            ),
            "cuda:optimization" | "optimization" | "optimize" | "performance" => (
                CUDA_OPTIMIZATION_METHODS.iter().collect(),
                "Optimization Techniques",
                "Best practices for memory coalescing, occupancy, warp efficiency, and performance",
            ),
            _ => anyhow::bail!("Unknown CUDA category: {identifier}"),
        };

        let items = methods
            .iter()
            .map(|m| CudaCategoryItem {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind: m.kind,
                url: self.get_method_url(m),
            })
            .collect();

        Ok(CudaCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
        })
    }

    /// Get URL for a method
    fn get_method_url(&self, method: &CudaMethodIndex) -> String {
        match method.kind {
            CudaMethodKind::RuntimeApi => {
                format!("{}/group__CUDART__{}.html", CUDA_RUNTIME_API_URL, method.category.to_uppercase())
            }
            CudaMethodKind::DriverApi => {
                format!("{}/group__CUDA__{}.html", CUDA_DOCS_URL, method.category.to_uppercase())
            }
            CudaMethodKind::KernelConstruct => {
                format!("{}/index.html#programming-model", CUDA_PROGRAMMING_GUIDE_URL)
            }
            CudaMethodKind::Library => {
                match method.category {
                    "cublas" => "https://docs.nvidia.com/cuda/cublas/index.html".to_string(),
                    "cudnn" => "https://docs.nvidia.com/deeplearning/cudnn/index.html".to_string(),
                    "cufft" => "https://docs.nvidia.com/cuda/cufft/index.html".to_string(),
                    "curand" => "https://docs.nvidia.com/cuda/curand/index.html".to_string(),
                    "nccl" => "https://docs.nvidia.com/deeplearning/nccl/index.html".to_string(),
                    _ => format!("{}/libraries", CUDA_DOCS_URL),
                }
            }
            CudaMethodKind::GpuSpec => {
                "https://developer.nvidia.com/cuda-gpus".to_string()
            }
            CudaMethodKind::Optimization => {
                "https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html".to_string()
            }
        }
    }

    /// Get all methods as a flat list for searching
    fn all_methods() -> impl Iterator<Item = &'static CudaMethodIndex> {
        CUDA_MEMORY_METHODS.iter()
            .chain(CUDA_DEVICE_METHODS.iter())
            .chain(CUDA_EXECUTION_METHODS.iter())
            .chain(CUDA_STREAM_METHODS.iter())
            .chain(CUDA_EVENT_METHODS.iter())
            .chain(CUDA_ERROR_METHODS.iter())
            .chain(CUDA_KERNEL_CONSTRUCTS.iter())
            .chain(CUDA_LIBRARY_METHODS.iter())
            .chain(CUDA_GPU_SPECS.iter())
            .chain(CUDA_OPTIMIZATION_METHODS.iter())
    }

    /// Build detailed method documentation
    fn build_method_doc(&self, index_entry: &CudaMethodIndex) -> CudaMethod {
        let examples = self.generate_examples(index_entry);
        let parameters = self.infer_parameters(index_entry);

        CudaMethod {
            name: index_entry.name.to_string(),
            description: index_entry.description.to_string(),
            kind: index_entry.kind,
            url: self.get_method_url(index_entry),
            parameters,
            returns: self.infer_return_type(index_entry),
            examples,
        }
    }

    /// Generate example code for a method
    fn generate_examples(&self, method: &CudaMethodIndex) -> Vec<CudaExample> {
        let mut examples = Vec::new();

        match method.name {
            // Memory Management Examples
            "cudaMalloc" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"float *d_array;
size_t size = N * sizeof(float);

// Allocate device memory
cudaError_t err = cudaMalloc((void**)&d_array, size);
if (err != cudaSuccess) {
    printf("cudaMalloc failed: %s\n", cudaGetErrorString(err));
    return;
}

// Use the memory...

// Free when done
cudaFree(d_array);"#.to_string(),
                    description: Some("Basic device memory allocation".to_string()),
                });
            }
            "cudaMemcpy" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"float *h_data = (float*)malloc(N * sizeof(float));
float *d_data;
cudaMalloc(&d_data, N * sizeof(float));

// Initialize host data
for (int i = 0; i < N; i++) h_data[i] = i;

// Copy host to device
cudaMemcpy(d_data, h_data, N * sizeof(float), cudaMemcpyHostToDevice);

// Run kernel...

// Copy results back to host
cudaMemcpy(h_data, d_data, N * sizeof(float), cudaMemcpyDeviceToHost);

cudaFree(d_data);
free(h_data);"#.to_string(),
                    description: Some("Copy data between host and device".to_string()),
                });
            }
            "cudaMallocManaged" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"float *data;

// Allocate Unified Memory - accessible from CPU or GPU
cudaMallocManaged(&data, N * sizeof(float));

// Initialize on CPU
for (int i = 0; i < N; i++) data[i] = i;

// Launch kernel - data automatically migrates to GPU
kernel<<<blocks, threads>>>(data, N);
cudaDeviceSynchronize();

// Access on CPU - data automatically migrates back
printf("Result: %f\n", data[0]);

cudaFree(data);"#.to_string(),
                    description: Some("Unified Memory simplifies memory management".to_string()),
                });
            }

            // Kernel Programming Examples
            "__global__" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Vector addition kernel
__global__ void vectorAdd(float *a, float *b, float *c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        c[idx] = a[idx] + b[idx];
    }
}

// Launch kernel
int blockSize = 256;
int numBlocks = (N + blockSize - 1) / blockSize;
vectorAdd<<<numBlocks, blockSize>>>(d_a, d_b, d_c, N);
cudaDeviceSynchronize();"#.to_string(),
                    description: Some("Basic kernel definition and launch".to_string()),
                });
            }
            "__shared__" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Matrix transpose using shared memory
__global__ void transposeCoalesced(float *odata, float *idata, int width, int height) {
    __shared__ float tile[TILE_DIM][TILE_DIM + 1]; // +1 to avoid bank conflicts

    int x = blockIdx.x * TILE_DIM + threadIdx.x;
    int y = blockIdx.y * TILE_DIM + threadIdx.y;

    // Coalesced read from global memory to shared memory
    if (x < width && y < height)
        tile[threadIdx.y][threadIdx.x] = idata[y * width + x];

    __syncthreads(); // Wait for all threads to load tile

    // Coalesced write from shared memory to global memory
    x = blockIdx.y * TILE_DIM + threadIdx.x;
    y = blockIdx.x * TILE_DIM + threadIdx.y;

    if (x < height && y < width)
        odata[y * height + x] = tile[threadIdx.x][threadIdx.y];
}"#.to_string(),
                    description: Some("Shared memory for coalesced memory access".to_string()),
                });
            }
            "__syncthreads" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Parallel reduction using shared memory
__global__ void reduce(float *input, float *output, int n) {
    __shared__ float sdata[256];

    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;

    // Load to shared memory
    sdata[tid] = (i < n) ? input[i] : 0;
    __syncthreads();

    // Parallel reduction in shared memory
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s) {
            sdata[tid] += sdata[tid + s];
        }
        __syncthreads(); // MUST sync between iterations
    }

    // Write result
    if (tid == 0) output[blockIdx.x] = sdata[0];
}"#.to_string(),
                    description: Some("Synchronization in parallel reduction".to_string()),
                });
            }
            "atomicAdd" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Histogram using atomic operations
__global__ void histogram(int *data, int *hist, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        atomicAdd(&hist[data[idx]], 1);
    }
}

// Better: Use shared memory for intermediate results
__global__ void histogramShared(int *data, int *hist, int n) {
    __shared__ int shist[256];
    int tid = threadIdx.x;

    // Initialize shared histogram
    if (tid < 256) shist[tid] = 0;
    __syncthreads();

    // Accumulate in shared memory (less contention)
    int idx = blockIdx.x * blockDim.x + tid;
    if (idx < n) atomicAdd(&shist[data[idx]], 1);
    __syncthreads();

    // Write to global memory
    if (tid < 256) atomicAdd(&hist[tid], shist[tid]);
}"#.to_string(),
                    description: Some("Atomic operations for histogram".to_string()),
                });
            }

            // Stream Examples
            "cudaStreamCreate" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"cudaStream_t stream1, stream2;
cudaStreamCreate(&stream1);
cudaStreamCreate(&stream2);

// Overlap kernel execution with memory transfer
cudaMemcpyAsync(d_a1, h_a1, size, cudaMemcpyHostToDevice, stream1);
kernel<<<blocks, threads, 0, stream2>>>(d_a2, N);

cudaStreamSynchronize(stream1);
cudaStreamSynchronize(stream2);

cudaStreamDestroy(stream1);
cudaStreamDestroy(stream2);"#.to_string(),
                    description: Some("Using streams for concurrency".to_string()),
                });
            }

            // Event Examples
            "cudaEventElapsedTime" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"cudaEvent_t start, stop;
cudaEventCreate(&start);
cudaEventCreate(&stop);

cudaEventRecord(start);

// Launch kernel
myKernel<<<blocks, threads>>>(data, N);

cudaEventRecord(stop);
cudaEventSynchronize(stop);

float milliseconds = 0;
cudaEventElapsedTime(&milliseconds, start, stop);
printf("Kernel time: %f ms\n", milliseconds);

cudaEventDestroy(start);
cudaEventDestroy(stop);"#.to_string(),
                    description: Some("GPU kernel timing".to_string()),
                });
            }

            // cuBLAS Examples
            "cublasSgemm" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"cublasHandle_t handle;
cublasCreate(&handle);

// C = alpha * A * B + beta * C
float alpha = 1.0f, beta = 0.0f;
int m = 1024, n = 1024, k = 1024;

// Column-major order (Fortran style)
cublasSgemm(handle, CUBLAS_OP_N, CUBLAS_OP_N,
            m, n, k,
            &alpha,
            d_A, m,    // A is m x k
            d_B, k,    // B is k x n
            &beta,
            d_C, m);   // C is m x n

cublasDestroy(handle);"#.to_string(),
                    description: Some("Matrix multiplication with cuBLAS".to_string()),
                });
            }

            // Optimization Examples
            "grid_stride_loop" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Grid-stride loop handles any size array
__global__ void vectorAdd(float *a, float *b, float *c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int stride = blockDim.x * gridDim.x;

    for (int i = idx; i < n; i += stride) {
        c[i] = a[i] + b[i];
    }
}

// Launch with fixed grid size
int blockSize = 256;
int numBlocks = 32 * numberOfSMs; // Enough to fill GPU
vectorAdd<<<numBlocks, blockSize>>>(d_a, d_b, d_c, N);"#.to_string(),
                    description: Some("Grid-stride loop for any array size".to_string()),
                });
            }
            "memory_coalescing" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// BAD: Strided access (each thread accesses non-adjacent memory)
__global__ void badAccess(float *data, int n, int stride) {
    int idx = threadIdx.x;
    float val = data[idx * stride]; // Non-coalesced!
}

// GOOD: Coalesced access (adjacent threads access adjacent memory)
__global__ void goodAccess(float *data, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    float val = data[idx]; // Coalesced - threads 0-31 access data[0-31]
}

// Structure of Arrays (SoA) is better than Array of Structures (AoS)
// BAD: AoS
struct Particle { float x, y, z, w; };
Particle *particles; // particles[i].x, particles[i+1].x are 16 bytes apart

// GOOD: SoA
float *x, *y, *z, *w; // x[i], x[i+1] are 4 bytes apart"#.to_string(),
                    description: Some("Memory coalescing patterns".to_string()),
                });
            }
            "occupancy_optimization" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Auto-tune block size for maximum occupancy
int blockSize;
int minGridSize;

cudaOccupancyMaxPotentialBlockSize(
    &minGridSize,
    &blockSize,
    myKernel,
    0,    // No dynamic shared memory
    0     // No block size limit
);

int gridSize = (N + blockSize - 1) / blockSize;
myKernel<<<gridSize, blockSize>>>(data, N);

// Check achieved occupancy
int maxActiveBlocks;
cudaOccupancyMaxActiveBlocksPerMultiprocessor(
    &maxActiveBlocks,
    myKernel,
    blockSize,
    0
);
printf("Achieved %d active blocks per SM\n", maxActiveBlocks);"#.to_string(),
                    description: Some("Auto-tune for maximum occupancy".to_string()),
                });
            }

            // GPU Specs Examples
            "RTX_3070" | "RTX_4090" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Query GPU properties at runtime
cudaDeviceProp prop;
cudaGetDeviceProperties(&prop, 0);

printf("GPU: %s\n", prop.name);
printf("Compute Capability: %d.%d\n", prop.major, prop.minor);
printf("CUDA Cores: %d\n",
       prop.multiProcessorCount * 128); // 128 cores per SM for Ampere/Ada
printf("Memory: %.0f GB\n", prop.totalGlobalMem / (1024.0*1024.0*1024.0));
printf("Memory Bus: %d-bit\n", prop.memoryBusWidth);
printf("L2 Cache: %d KB\n", prop.l2CacheSize / 1024);
printf("Max Threads/Block: %d\n", prop.maxThreadsPerBlock);
printf("Max Shared Memory/Block: %zu KB\n", prop.sharedMemPerBlock / 1024);
printf("Registers/Block: %d\n", prop.regsPerBlock);
printf("Warp Size: %d\n", prop.warpSize);

// RTX 3070: Compute 8.6, 46 SMs, 5888 cores, 8GB, 4MB L2
// RTX 4090: Compute 8.9, 128 SMs, 16384 cores, 24GB, 72MB L2"#.to_string(),
                    description: Some("Query GPU specifications".to_string()),
                });
            }

            "tensor_core_optimization" => {
                examples.push(CudaExample {
                    language: "cuda".to_string(),
                    code: r#"// Use Tensor Cores with cuBLAS
cublasHandle_t handle;
cublasCreate(&handle);

// Enable TF32 for single precision (RTX 30xx/40xx)
cublasSetMathMode(handle, CUBLAS_TF32_TENSOR_OP_MATH);

// Or use explicit FP16 for maximum Tensor Core performance
half *d_A_fp16, *d_B_fp16;
float *d_C;

// Matrix dimensions should be multiples of 8 (TF32) or 16 (FP16)
int m = 4096, n = 4096, k = 4096;

// FP16 GEMM with FP32 accumulation
cublasGemmEx(handle, CUBLAS_OP_N, CUBLAS_OP_N,
             m, n, k,
             &alpha,
             d_A_fp16, CUDA_R_16F, m,
             d_B_fp16, CUDA_R_16F, k,
             &beta,
             d_C, CUDA_R_32F, m,
             CUBLAS_COMPUTE_32F,
             CUBLAS_GEMM_DEFAULT_TENSOR_OP);

// RTX 3070: 184 Tensor Cores, ~20 TFLOPS FP16
// RTX 4090: 512 Tensor Cores, ~83 TFLOPS FP16, 165 TFLOPS FP8"#.to_string(),
                    description: Some("Leverage Tensor Cores for matrix ops".to_string()),
                });
            }

            _ => {
                // Generic example for other methods
                match method.kind {
                    CudaMethodKind::RuntimeApi => {
                        examples.push(CudaExample {
                            language: "cuda".to_string(),
                            code: format!(
                                "// Check CUDA Runtime API reference for {} usage\n// https://docs.nvidia.com/cuda/cuda-runtime-api/",
                                method.name
                            ),
                            description: Some(format!("Usage of {}", method.name)),
                        });
                    }
                    CudaMethodKind::KernelConstruct => {
                        examples.push(CudaExample {
                            language: "cuda".to_string(),
                            code: format!(
                                "// {} is a CUDA kernel programming construct\n// See CUDA C++ Programming Guide for details",
                                method.name
                            ),
                            description: Some(format!("CUDA kernel construct: {}", method.name)),
                        });
                    }
                    _ => {}
                }
            }
        }

        examples
    }

    /// Infer parameters for a method based on common patterns
    fn infer_parameters(&self, method: &CudaMethodIndex) -> Vec<CudaParameter> {
        match method.name {
            "cudaMalloc" => vec![
                CudaParameter {
                    name: "devPtr".to_string(),
                    param_type: "void**".to_string(),
                    required: true,
                    description: "Pointer to allocated device memory".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "size".to_string(),
                    param_type: "size_t".to_string(),
                    required: true,
                    description: "Requested allocation size in bytes".to_string(),
                    default_value: None,
                },
            ],
            "cudaMemcpy" => vec![
                CudaParameter {
                    name: "dst".to_string(),
                    param_type: "void*".to_string(),
                    required: true,
                    description: "Destination memory address".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "src".to_string(),
                    param_type: "const void*".to_string(),
                    required: true,
                    description: "Source memory address".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "count".to_string(),
                    param_type: "size_t".to_string(),
                    required: true,
                    description: "Size in bytes to copy".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "kind".to_string(),
                    param_type: "cudaMemcpyKind".to_string(),
                    required: true,
                    description: "Type of transfer: cudaMemcpyHostToDevice, cudaMemcpyDeviceToHost, cudaMemcpyDeviceToDevice, or cudaMemcpyHostToHost".to_string(),
                    default_value: None,
                },
            ],
            "cudaLaunchKernel" => vec![
                CudaParameter {
                    name: "func".to_string(),
                    param_type: "const void*".to_string(),
                    required: true,
                    description: "Device function symbol".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "gridDim".to_string(),
                    param_type: "dim3".to_string(),
                    required: true,
                    description: "Grid dimensions (number of blocks)".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "blockDim".to_string(),
                    param_type: "dim3".to_string(),
                    required: true,
                    description: "Block dimensions (threads per block, max 1024)".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "args".to_string(),
                    param_type: "void**".to_string(),
                    required: true,
                    description: "Array of pointers to kernel parameters".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "sharedMem".to_string(),
                    param_type: "size_t".to_string(),
                    required: false,
                    description: "Dynamic shared memory size per block in bytes".to_string(),
                    default_value: Some("0".to_string()),
                },
                CudaParameter {
                    name: "stream".to_string(),
                    param_type: "cudaStream_t".to_string(),
                    required: false,
                    description: "Stream for the kernel launch".to_string(),
                    default_value: Some("0 (default stream)".to_string()),
                },
            ],
            "cudaStreamCreate" => vec![
                CudaParameter {
                    name: "pStream".to_string(),
                    param_type: "cudaStream_t*".to_string(),
                    required: true,
                    description: "Pointer to new stream identifier".to_string(),
                    default_value: None,
                },
            ],
            "cudaEventElapsedTime" => vec![
                CudaParameter {
                    name: "ms".to_string(),
                    param_type: "float*".to_string(),
                    required: true,
                    description: "Time between events in milliseconds".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "start".to_string(),
                    param_type: "cudaEvent_t".to_string(),
                    required: true,
                    description: "Starting event".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "end".to_string(),
                    param_type: "cudaEvent_t".to_string(),
                    required: true,
                    description: "Ending event".to_string(),
                    default_value: None,
                },
            ],
            "cublasSgemm" => vec![
                CudaParameter {
                    name: "handle".to_string(),
                    param_type: "cublasHandle_t".to_string(),
                    required: true,
                    description: "cuBLAS library handle".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "transa".to_string(),
                    param_type: "cublasOperation_t".to_string(),
                    required: true,
                    description: "Operation on A: CUBLAS_OP_N (no transpose), CUBLAS_OP_T (transpose)".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "transb".to_string(),
                    param_type: "cublasOperation_t".to_string(),
                    required: true,
                    description: "Operation on B: CUBLAS_OP_N or CUBLAS_OP_T".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "m".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Number of rows of matrix C and A".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "n".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Number of columns of matrix C and B".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "k".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Number of columns of A and rows of B".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "alpha".to_string(),
                    param_type: "const float*".to_string(),
                    required: true,
                    description: "Scalar used for multiplication".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "A".to_string(),
                    param_type: "const float*".to_string(),
                    required: true,
                    description: "Matrix A device pointer".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "lda".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Leading dimension of A".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "B".to_string(),
                    param_type: "const float*".to_string(),
                    required: true,
                    description: "Matrix B device pointer".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "ldb".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Leading dimension of B".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "beta".to_string(),
                    param_type: "const float*".to_string(),
                    required: true,
                    description: "Scalar used for multiplication".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "C".to_string(),
                    param_type: "float*".to_string(),
                    required: true,
                    description: "Matrix C device pointer (output)".to_string(),
                    default_value: None,
                },
                CudaParameter {
                    name: "ldc".to_string(),
                    param_type: "int".to_string(),
                    required: true,
                    description: "Leading dimension of C".to_string(),
                    default_value: None,
                },
            ],
            _ => Vec::new(),
        }
    }

    /// Infer return type for a method
    fn infer_return_type(&self, method: &CudaMethodIndex) -> Option<CudaReturnType> {
        match method.kind {
            CudaMethodKind::RuntimeApi | CudaMethodKind::DriverApi => {
                Some(CudaReturnType {
                    type_name: "cudaError_t".to_string(),
                    description: "CUDA error code. Check against cudaSuccess.".to_string(),
                    fields: vec![
                        CudaReturnField {
                            name: "cudaSuccess".to_string(),
                            field_type: "0".to_string(),
                            description: "No errors".to_string(),
                        },
                        CudaReturnField {
                            name: "cudaErrorMemoryAllocation".to_string(),
                            field_type: "2".to_string(),
                            description: "Memory allocation failed".to_string(),
                        },
                        CudaReturnField {
                            name: "cudaErrorInvalidValue".to_string(),
                            field_type: "1".to_string(),
                            description: "Invalid argument value".to_string(),
                        },
                    ],
                })
            }
            CudaMethodKind::Library => {
                match method.category {
                    "cublas" => Some(CudaReturnType {
                        type_name: "cublasStatus_t".to_string(),
                        description: "cuBLAS status code".to_string(),
                        fields: vec![
                            CudaReturnField {
                                name: "CUBLAS_STATUS_SUCCESS".to_string(),
                                field_type: "0".to_string(),
                                description: "Operation completed successfully".to_string(),
                            },
                        ],
                    }),
                    "cudnn" => Some(CudaReturnType {
                        type_name: "cudnnStatus_t".to_string(),
                        description: "cuDNN status code".to_string(),
                        fields: vec![
                            CudaReturnField {
                                name: "CUDNN_STATUS_SUCCESS".to_string(),
                                field_type: "0".to_string(),
                                description: "Operation completed successfully".to_string(),
                            },
                        ],
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get a specific method by name
    #[instrument(name = "cuda_client.get_method", skip(self))]
    pub async fn get_method(&self, name: &str) -> Result<CudaMethod> {
        let index_entry = Self::all_methods()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("CUDA method not found: {name}"))?;

        Ok(self.build_method_doc(index_entry))
    }

    /// Search for methods matching a query
    #[instrument(name = "cuda_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<CudaMethod>> {
        let query_lower = query.to_lowercase();

        // Split query into keywords
        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, &CudaMethodIndex)> = Vec::new();

        // Search all methods
        for method in Self::all_methods() {
            let name_lower = method.name.to_lowercase();
            let desc_lower = method.description.to_lowercase();
            let category_lower = method.category.to_lowercase();

            let mut score = 0i32;

            for keyword in &keywords {
                // Exact name match
                if name_lower == *keyword {
                    score += 50;
                }
                // Name contains keyword
                else if name_lower.contains(keyword) {
                    score += 20;
                }
                // Category match
                if category_lower.contains(keyword) {
                    score += 10;
                }
                // Description contains keyword
                if desc_lower.contains(keyword) {
                    score += 5;
                }
            }

            // Boost for GPU-specific queries
            if (query_lower.contains("3070") || query_lower.contains("rtx 3070")) &&
               method.name.contains("3070") {
                score += 30;
            }
            if (query_lower.contains("4090") || query_lower.contains("rtx 4090")) &&
               method.name.contains("4090") {
                score += 30;
            }

            // Boost for kernel-related queries
            if query_lower.contains("kernel") && method.kind == CudaMethodKind::KernelConstruct {
                score += 15;
            }

            // Boost for memory-related queries
            if (query_lower.contains("memory") || query_lower.contains("alloc")) &&
               method.category == "memory" {
                score += 15;
            }

            // Boost for library queries
            if query_lower.contains("cublas") && method.category == "cublas" {
                score += 20;
            }
            if query_lower.contains("cudnn") && method.category == "cudnn" {
                score += 20;
            }

            if score > 0 {
                scored_results.push((score, method));
            }
        }

        // Sort by score (highest first)
        scored_results.sort_by(|a, b| b.0.cmp(&a.0));

        // Convert to CudaMethod
        let results: Vec<CudaMethod> = scored_results
            .into_iter()
            .take(20)
            .map(|(_, m)| self.build_method_doc(m))
            .collect();

        Ok(results)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = CudaClient::new();
    }

    #[test]
    fn test_all_methods_count() {
        let count = CudaClient::all_methods().count();
        assert!(count > 50, "Expected at least 50 methods, got {}", count);
    }

    #[test]
    fn test_categories() {
        let count = CUDA_MEMORY_METHODS.len()
            + CUDA_DEVICE_METHODS.len()
            + CUDA_KERNEL_CONSTRUCTS.len()
            + CUDA_LIBRARY_METHODS.len()
            + CUDA_GPU_SPECS.len()
            + CUDA_OPTIMIZATION_METHODS.len();
        assert!(count > 80, "Expected comprehensive coverage, got {}", count);
    }
}
