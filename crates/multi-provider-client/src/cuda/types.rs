use serde::{Deserialize, Serialize};

// ============================================================================
// CUDA DOCUMENTATION PROVIDER
// ============================================================================
//
// NVIDIA CUDA (Compute Unified Device Architecture) is a parallel computing
// platform and programming model developed by NVIDIA for general computing
// on GPUs. This provider focuses on CUDA kernel development and optimization
// with specific coverage for RTX 3070 and RTX 4090 GPUs.
//
// Key Features:
// - CUDA Runtime API: High-level API for memory management and kernel execution
// - CUDA Driver API: Low-level API for fine-grained control
// - Kernel Programming: __global__, __device__, __shared__ memory
// - Libraries: cuBLAS, cuDNN, NCCL, cuFFT, cuRAND
// - Optimization: Memory coalescing, occupancy, warp-level primitives
//
// GPU Specifications Covered:
// - RTX 3070 (GA104, Compute Capability 8.6, 5888 CUDA cores, 8GB GDDR6)
// - RTX 4090 (AD102, Compute Capability 8.9, 16384 CUDA cores, 24GB GDDR6X)
//
// Documentation Sources:
// - https://docs.nvidia.com/cuda/cuda-runtime-api/
// - https://docs.nvidia.com/cuda/cuda-c-programming-guide/
// - https://developer.nvidia.com/cuda-toolkit
//
// ============================================================================

/// CUDA technology representation (API categories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of CUDA documentation (Runtime API, Kernels, Libraries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<CudaCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: CudaMethodKind,
    pub url: String,
}

/// Kind of CUDA documentation item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CudaMethodKind {
    /// Runtime API function (cudaMalloc, cudaMemcpy, etc.)
    RuntimeApi,
    /// Driver API function (cuModuleLoad, cuLaunchKernel, etc.)
    DriverApi,
    /// Kernel programming construct (__global__, __shared__, etc.)
    KernelConstruct,
    /// CUDA library function (cuBLAS, cuDNN, etc.)
    Library,
    /// GPU specification or hardware feature
    GpuSpec,
    /// Optimization technique or best practice
    Optimization,
}

impl std::fmt::Display for CudaMethodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuntimeApi => write!(f, "Runtime API"),
            Self::DriverApi => write!(f, "Driver API"),
            Self::KernelConstruct => write!(f, "Kernel Construct"),
            Self::Library => write!(f, "Library"),
            Self::GpuSpec => write!(f, "GPU Specification"),
            Self::Optimization => write!(f, "Optimization"),
        }
    }
}

/// Detailed method documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaMethod {
    pub name: String,
    pub description: String,
    pub kind: CudaMethodKind,
    pub url: String,
    pub parameters: Vec<CudaParameter>,
    pub returns: Option<CudaReturnType>,
    pub examples: Vec<CudaExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaReturnType {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<CudaReturnField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaReturnField {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaExample {
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Static method index entry (pre-defined for all CUDA items)
#[derive(Debug, Clone)]
pub struct CudaMethodIndex {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: CudaMethodKind,
    pub category: &'static str,
}

// ============================================================================
// CUDA RUNTIME API - MEMORY MANAGEMENT
// ============================================================================

pub const CUDA_MEMORY_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaMalloc", description: "Allocates memory on the device (GPU). Returns a pointer to the allocated device memory. Essential for any CUDA program that processes data on the GPU.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaFree", description: "Frees memory on the device that was previously allocated with cudaMalloc. Always pair with cudaMalloc to prevent memory leaks.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemcpy", description: "Copies data between host and device memory. Supports cudaMemcpyHostToDevice, cudaMemcpyDeviceToHost, cudaMemcpyDeviceToDevice, and cudaMemcpyHostToHost.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemcpyAsync", description: "Asynchronous memory copy between host and device. Requires pinned host memory for true async behavior. Essential for overlapping computation with data transfer.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMallocManaged", description: "Allocates Unified Memory accessible from both CPU and GPU. Automatic page migration between CPU and GPU memory. Simplifies programming but may have performance implications.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMallocHost", description: "Allocates page-locked (pinned) host memory. Enables faster host-device transfers and is required for truly asynchronous memcpy operations.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaFreeHost", description: "Frees page-locked host memory allocated by cudaMallocHost. Must be used instead of free() for pinned memory.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemset", description: "Initializes device memory to a specified value. Sets each byte to the value, useful for zeroing arrays.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemsetAsync", description: "Asynchronous version of cudaMemset. Can be overlapped with kernel execution when using streams.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMalloc3D", description: "Allocates 3D memory on the device with proper pitch alignment. Returns a cudaPitchedPtr for optimal memory access patterns.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMallocPitch", description: "Allocates pitched 2D memory on the device. Ensures proper memory alignment for 2D array access, improving memory coalescing.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemcpy2D", description: "Copies 2D memory regions between host and device. Handles pitch differences between source and destination.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
    CudaMethodIndex { name: "cudaMemGetInfo", description: "Returns available and total device memory. Useful for checking available GPU memory before large allocations.", kind: CudaMethodKind::RuntimeApi, category: "memory" },
];

// ============================================================================
// CUDA RUNTIME API - DEVICE MANAGEMENT
// ============================================================================

pub const CUDA_DEVICE_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaGetDeviceCount", description: "Returns the number of CUDA-capable devices in the system. First call in multi-GPU applications.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaSetDevice", description: "Sets the current device for CUDA operations. All subsequent CUDA calls operate on this device.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaGetDevice", description: "Returns the current device for the calling host thread.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaGetDeviceProperties", description: "Returns properties of a CUDA device including compute capability, memory size, multiprocessor count, clock rate, and maximum thread dimensions.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaDeviceSynchronize", description: "Blocks until all preceding commands in all streams have completed. Essential for timing and ensuring computation is complete before reading results.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaDeviceReset", description: "Destroys all allocations and resets all state on the current device. Clean up before program exit.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaDeviceGetAttribute", description: "Returns information about the device. Can query specific attributes like max threads per block, shared memory size, etc.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaDeviceCanAccessPeer", description: "Checks if peer-to-peer access between two devices is possible. Required for multi-GPU direct memory access.", kind: CudaMethodKind::RuntimeApi, category: "device" },
    CudaMethodIndex { name: "cudaDeviceEnablePeerAccess", description: "Enables peer-to-peer memory access between devices. Allows direct GPU-to-GPU memory transfers.", kind: CudaMethodKind::RuntimeApi, category: "device" },
];

// ============================================================================
// CUDA RUNTIME API - KERNEL EXECUTION
// ============================================================================

pub const CUDA_EXECUTION_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaLaunchKernel", description: "Launches a CUDA kernel function on the device. Specifies grid dimensions, block dimensions, shared memory size, and stream.", kind: CudaMethodKind::RuntimeApi, category: "execution" },
    CudaMethodIndex { name: "cudaFuncSetCacheConfig", description: "Sets the preferred cache configuration for a device function. Can prefer L1 cache or shared memory.", kind: CudaMethodKind::RuntimeApi, category: "execution" },
    CudaMethodIndex { name: "cudaFuncGetAttributes", description: "Returns attributes of a device function including registers per thread, shared memory per block, and constant memory.", kind: CudaMethodKind::RuntimeApi, category: "execution" },
    CudaMethodIndex { name: "cudaOccupancyMaxPotentialBlockSize", description: "Calculates optimal block size for a kernel based on occupancy. Key for performance optimization.", kind: CudaMethodKind::RuntimeApi, category: "execution" },
    CudaMethodIndex { name: "cudaOccupancyMaxActiveBlocksPerMultiprocessor", description: "Returns max active blocks per SM for a given kernel and block size. Used to analyze occupancy.", kind: CudaMethodKind::RuntimeApi, category: "execution" },
    CudaMethodIndex { name: "cudaLaunchCooperativeKernel", description: "Launches a cooperative kernel that can synchronize across the entire grid using grid.sync().", kind: CudaMethodKind::RuntimeApi, category: "execution" },
];

// ============================================================================
// CUDA RUNTIME API - STREAM MANAGEMENT
// ============================================================================

pub const CUDA_STREAM_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaStreamCreate", description: "Creates a new asynchronous stream. Operations in different streams can execute concurrently.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamDestroy", description: "Destroys an asynchronous stream. Waits for all preceding commands to complete first.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamSynchronize", description: "Blocks until all commands in a stream have completed. More granular than cudaDeviceSynchronize.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamQuery", description: "Queries whether all operations in a stream have completed. Non-blocking status check.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamWaitEvent", description: "Makes a stream wait for an event. Enables cross-stream synchronization.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamCreateWithFlags", description: "Creates a stream with specific flags (cudaStreamDefault or cudaStreamNonBlocking).", kind: CudaMethodKind::RuntimeApi, category: "stream" },
    CudaMethodIndex { name: "cudaStreamCreateWithPriority", description: "Creates a stream with a specified priority. Higher priority streams may preempt lower priority ones.", kind: CudaMethodKind::RuntimeApi, category: "stream" },
];

// ============================================================================
// CUDA RUNTIME API - EVENT MANAGEMENT
// ============================================================================

pub const CUDA_EVENT_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaEventCreate", description: "Creates an event object for timing or synchronization.", kind: CudaMethodKind::RuntimeApi, category: "event" },
    CudaMethodIndex { name: "cudaEventDestroy", description: "Destroys an event object.", kind: CudaMethodKind::RuntimeApi, category: "event" },
    CudaMethodIndex { name: "cudaEventRecord", description: "Records an event in a stream. The event is recorded when all preceding operations complete.", kind: CudaMethodKind::RuntimeApi, category: "event" },
    CudaMethodIndex { name: "cudaEventSynchronize", description: "Waits for an event to complete. Blocks until the event has been recorded.", kind: CudaMethodKind::RuntimeApi, category: "event" },
    CudaMethodIndex { name: "cudaEventElapsedTime", description: "Computes elapsed time between two events in milliseconds. Primary method for GPU timing.", kind: CudaMethodKind::RuntimeApi, category: "event" },
    CudaMethodIndex { name: "cudaEventQuery", description: "Queries whether an event has been recorded. Non-blocking status check.", kind: CudaMethodKind::RuntimeApi, category: "event" },
];

// ============================================================================
// CUDA RUNTIME API - ERROR HANDLING
// ============================================================================

pub const CUDA_ERROR_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "cudaGetLastError", description: "Returns the last error from a CUDA runtime call. Resets the error to cudaSuccess after returning.", kind: CudaMethodKind::RuntimeApi, category: "error" },
    CudaMethodIndex { name: "cudaPeekAtLastError", description: "Returns the last error without resetting it. Useful for checking errors without affecting state.", kind: CudaMethodKind::RuntimeApi, category: "error" },
    CudaMethodIndex { name: "cudaGetErrorName", description: "Returns the name of a CUDA error code as a string.", kind: CudaMethodKind::RuntimeApi, category: "error" },
    CudaMethodIndex { name: "cudaGetErrorString", description: "Returns a description of a CUDA error code as a string.", kind: CudaMethodKind::RuntimeApi, category: "error" },
];

// ============================================================================
// CUDA KERNEL PROGRAMMING CONSTRUCTS
// ============================================================================

pub const CUDA_KERNEL_CONSTRUCTS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "__global__", description: "Declares a kernel function that runs on the device and is called from the host. Kernel functions must return void. Launch syntax: kernel<<<gridDim, blockDim>>>(args).", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__device__", description: "Declares a function that runs on the device and is callable only from device code. Can be inlined by the compiler for performance.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__host__", description: "Declares a function that runs on the host. Can be combined with __device__ to compile for both host and device.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__shared__", description: "Declares a variable in shared memory. Shared memory is on-chip, low-latency memory shared by all threads in a block. ~100x faster than global memory.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__constant__", description: "Declares a variable in constant memory. Constant memory is cached and optimized for read-only data broadcast to all threads.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "threadIdx", description: "Built-in variable containing the thread index within its block. Components: threadIdx.x, threadIdx.y, threadIdx.z.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "blockIdx", description: "Built-in variable containing the block index within the grid. Components: blockIdx.x, blockIdx.y, blockIdx.z.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "blockDim", description: "Built-in variable containing the dimensions of the block. Components: blockDim.x, blockDim.y, blockDim.z. Max 1024 threads per block.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "gridDim", description: "Built-in variable containing the dimensions of the grid. Components: gridDim.x, gridDim.y, gridDim.z.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__syncthreads", description: "Barrier synchronization for all threads in a block. All threads must reach this point before any can proceed. Essential for shared memory access patterns.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__syncwarp", description: "Synchronizes threads within a warp. More efficient than __syncthreads when only warp-level sync is needed (Compute Capability 7.0+).", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "warpSize", description: "Built-in constant containing the warp size (32 threads on all current NVIDIA GPUs). Important for warp-level programming.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__shfl_sync", description: "Warp shuffle operation - exchanges data between threads in a warp without shared memory. Very efficient for reductions and scans.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "__ballot_sync", description: "Returns a bitmask where each bit is set if the corresponding thread in the warp has a non-zero predicate.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "atomicAdd", description: "Performs atomic addition on global or shared memory. Thread-safe but can be a bottleneck if many threads access the same location.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "atomicCAS", description: "Atomic compare-and-swap operation. Foundation for implementing other atomic operations and lock-free algorithms.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "atomicExch", description: "Atomic exchange operation. Atomically stores a value and returns the old value.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "atomicMin", description: "Atomic minimum operation. Computes the minimum of the old value and the input value atomically.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
    CudaMethodIndex { name: "atomicMax", description: "Atomic maximum operation. Computes the maximum of the old value and the input value atomically.", kind: CudaMethodKind::KernelConstruct, category: "kernel" },
];

// ============================================================================
// CUDA LIBRARIES
// ============================================================================

pub const CUDA_LIBRARY_METHODS: &[CudaMethodIndex] = &[
    // cuBLAS
    CudaMethodIndex { name: "cublasSgemm", description: "cuBLAS single-precision general matrix multiplication. C = alpha*A*B + beta*C. Highly optimized for NVIDIA GPUs.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasDgemm", description: "cuBLAS double-precision general matrix multiplication. C = alpha*A*B + beta*C.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasHgemm", description: "cuBLAS half-precision (FP16) matrix multiplication. Leverages Tensor Cores on Volta+ GPUs for massive speedups.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasGemmEx", description: "cuBLAS extended GEMM with mixed precision support. Can use INT8, FP16, BF16, TF32, FP32, FP64.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasCreate", description: "Creates a cuBLAS handle. Required before any cuBLAS operation.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasDestroy", description: "Destroys a cuBLAS handle and releases resources.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasSetStream", description: "Associates a CUDA stream with a cuBLAS handle for async execution.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasSaxpy", description: "cuBLAS single-precision y = alpha*x + y. Fundamental BLAS Level 1 operation.", kind: CudaMethodKind::Library, category: "cublas" },
    CudaMethodIndex { name: "cublasSdot", description: "cuBLAS single-precision dot product. Returns x·y.", kind: CudaMethodKind::Library, category: "cublas" },

    // cuDNN
    CudaMethodIndex { name: "cudnnConvolutionForward", description: "cuDNN convolution forward pass. Supports multiple algorithms with auto-tuning for optimal performance.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnConvolutionBackwardData", description: "cuDNN convolution backward pass for input gradients.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnConvolutionBackwardFilter", description: "cuDNN convolution backward pass for filter gradients.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnBatchNormalizationForwardTraining", description: "cuDNN batch normalization forward pass during training.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnSoftmaxForward", description: "cuDNN softmax activation forward pass.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnCreate", description: "Creates a cuDNN handle. Required before any cuDNN operation.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnDestroy", description: "Destroys a cuDNN handle and releases resources.", kind: CudaMethodKind::Library, category: "cudnn" },
    CudaMethodIndex { name: "cudnnGetConvolutionForwardAlgorithm", description: "Selects optimal convolution algorithm based on tensor sizes and available memory.", kind: CudaMethodKind::Library, category: "cudnn" },

    // cuFFT
    CudaMethodIndex { name: "cufftExecC2C", description: "cuFFT complex-to-complex FFT execution. Highly optimized for power-of-2 sizes.", kind: CudaMethodKind::Library, category: "cufft" },
    CudaMethodIndex { name: "cufftExecR2C", description: "cuFFT real-to-complex FFT execution.", kind: CudaMethodKind::Library, category: "cufft" },
    CudaMethodIndex { name: "cufftPlan1d", description: "Creates a 1D FFT plan.", kind: CudaMethodKind::Library, category: "cufft" },
    CudaMethodIndex { name: "cufftPlan2d", description: "Creates a 2D FFT plan.", kind: CudaMethodKind::Library, category: "cufft" },
    CudaMethodIndex { name: "cufftPlan3d", description: "Creates a 3D FFT plan.", kind: CudaMethodKind::Library, category: "cufft" },

    // cuRAND
    CudaMethodIndex { name: "curandCreateGenerator", description: "Creates a cuRAND random number generator. Supports multiple algorithms (XORWOW, MRG32k3a, etc.).", kind: CudaMethodKind::Library, category: "curand" },
    CudaMethodIndex { name: "curandGenerateUniform", description: "Generates uniformly distributed floats in (0, 1].", kind: CudaMethodKind::Library, category: "curand" },
    CudaMethodIndex { name: "curandGenerateNormal", description: "Generates normally distributed floats with specified mean and stddev.", kind: CudaMethodKind::Library, category: "curand" },
    CudaMethodIndex { name: "curandSetPseudoRandomGeneratorSeed", description: "Sets the seed for a pseudo-random number generator.", kind: CudaMethodKind::Library, category: "curand" },

    // NCCL
    CudaMethodIndex { name: "ncclAllReduce", description: "NCCL all-reduce collective operation across multiple GPUs. Essential for distributed training.", kind: CudaMethodKind::Library, category: "nccl" },
    CudaMethodIndex { name: "ncclBroadcast", description: "NCCL broadcast operation - sends data from one GPU to all others.", kind: CudaMethodKind::Library, category: "nccl" },
    CudaMethodIndex { name: "ncclReduce", description: "NCCL reduce operation - reduces data to a single GPU.", kind: CudaMethodKind::Library, category: "nccl" },
    CudaMethodIndex { name: "ncclCommInitAll", description: "Initializes NCCL communicators for all GPUs in a single process.", kind: CudaMethodKind::Library, category: "nccl" },
    CudaMethodIndex { name: "ncclCommInitRank", description: "Initializes a single NCCL communicator for multi-process setups.", kind: CudaMethodKind::Library, category: "nccl" },
];

// ============================================================================
// GPU SPECIFICATIONS - RTX 3070 & RTX 4090
// ============================================================================

pub const CUDA_GPU_SPECS: &[CudaMethodIndex] = &[
    // RTX 3070 (Ampere)
    CudaMethodIndex { name: "RTX_3070", description: "NVIDIA GeForce RTX 3070 - Ampere architecture (GA104). 5888 CUDA cores, 184 Tensor Cores (3rd gen), 46 RT Cores (2nd gen). 8GB GDDR6 on 256-bit bus. Compute Capability 8.6.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_3070_compute_capability", description: "RTX 3070 Compute Capability 8.6: Supports bfloat16, tf32 tensor operations, async copy, cooperative groups 2.0, and hardware-accelerated memory barriers.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_3070_memory", description: "RTX 3070 Memory: 8GB GDDR6, 448 GB/s bandwidth, 256-bit bus. Shared memory up to 100KB per SM (configurable). L2 cache: 4MB.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_3070_sm_config", description: "RTX 3070 SM Configuration: 46 SMs, 128 CUDA cores per SM, 4 Tensor Cores per SM. Max 1536 threads per SM, 48 warps per SM.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_3070_limits", description: "RTX 3070 Limits: Max 1024 threads per block, max 48KB shared memory per block (default), 255 registers per thread, max grid size 2^31-1 x 65535 x 65535.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },

    // RTX 4090 (Ada Lovelace)
    CudaMethodIndex { name: "RTX_4090", description: "NVIDIA GeForce RTX 4090 - Ada Lovelace architecture (AD102). 16384 CUDA cores, 512 Tensor Cores (4th gen), 128 RT Cores (3rd gen). 24GB GDDR6X on 384-bit bus. Compute Capability 8.9.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_4090_compute_capability", description: "RTX 4090 Compute Capability 8.9: All CC 8.6 features plus FP8 tensor operations, Thread Block Clusters, Tensor Memory Accelerator (TMA), and improved async copy.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_4090_memory", description: "RTX 4090 Memory: 24GB GDDR6X, 1008 GB/s bandwidth, 384-bit bus. Shared memory up to 100KB per SM. L2 cache: 72MB - massive increase for cache-sensitive workloads.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_4090_sm_config", description: "RTX 4090 SM Configuration: 128 SMs, 128 CUDA cores per SM, 4 Tensor Cores per SM. Max 1536 threads per SM, 48 warps per SM. ~2.8x the SM count of RTX 3070.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_4090_limits", description: "RTX 4090 Limits: Max 1024 threads per block, max 48KB shared memory per block (default), 255 registers per thread, supports Thread Block Clusters for SM groups.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
    CudaMethodIndex { name: "RTX_4090_tensor_cores", description: "RTX 4090 4th Gen Tensor Cores: 2x FP16/BF16 throughput vs 3rd gen, FP8 support for transformer inference (up to 1.3 PFLOPS FP8), Sparsity support for 2x speedup.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },

    // Comparison
    CudaMethodIndex { name: "RTX_3070_vs_4090", description: "RTX 4090 vs RTX 3070: 2.8x CUDA cores (16384 vs 5888), 3x memory (24GB vs 8GB), 2.25x bandwidth (1008 vs 448 GB/s), 18x L2 cache (72MB vs 4MB). Expect 2-3x real-world speedup.", kind: CudaMethodKind::GpuSpec, category: "gpu_specs" },
];

// ============================================================================
// CUDA OPTIMIZATION TECHNIQUES
// ============================================================================

pub const CUDA_OPTIMIZATION_METHODS: &[CudaMethodIndex] = &[
    CudaMethodIndex { name: "memory_coalescing", description: "Memory Coalescing: Ensure adjacent threads access adjacent memory locations. Coalesced access can achieve full memory bandwidth; non-coalesced access may be 32x slower.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "shared_memory_bank_conflicts", description: "Shared Memory Bank Conflicts: Shared memory has 32 banks. Threads accessing the same bank are serialized. Use padding or access patterns that avoid conflicts.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "occupancy_optimization", description: "Occupancy Optimization: Balance registers, shared memory, and threads per block to maximize active warps. Use cudaOccupancyMaxPotentialBlockSize for auto-tuning.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "warp_divergence", description: "Warp Divergence: Avoid branching within a warp. When threads in a warp take different paths, execution is serialized. Refactor to minimize divergent paths.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "register_pressure", description: "Register Pressure: More registers per thread = fewer concurrent threads. Use -maxrregcount to limit registers and improve occupancy, at cost of register spilling.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "grid_stride_loop", description: "Grid-Stride Loop: Pattern where each thread processes multiple elements by striding through the data. Handles arbitrary data sizes and improves cache utilization.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "stream_concurrency", description: "Stream Concurrency: Use multiple streams to overlap kernel execution with memory transfers. Requires pinned host memory and careful dependency management.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "tensor_core_optimization", description: "Tensor Core Optimization: Use FP16/BF16/TF32 for matrix operations. Ensure matrix dimensions are multiples of 16 (FP16) or 8 (TF32) for optimal Tensor Core utilization.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "l2_cache_optimization", description: "L2 Cache Optimization: RTX 4090's 72MB L2 cache benefits memory-bound kernels. Use cudaAccessPolicyWindow to control cache allocation for specific data.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "async_data_transfer", description: "Async Data Transfer: Overlap computation with data movement using cudaMemcpyAsync and multiple streams. Can achieve 2x speedup for memory-bound workloads.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "kernel_fusion", description: "Kernel Fusion: Combine multiple kernels into one to reduce kernel launch overhead and intermediate memory accesses. Particularly effective for element-wise operations.", kind: CudaMethodKind::Optimization, category: "optimization" },
    CudaMethodIndex { name: "persistent_threads", description: "Persistent Threads: Launch a fixed number of thread blocks that process work from a queue. Reduces launch overhead for many small tasks.", kind: CudaMethodKind::Optimization, category: "optimization" },
];
