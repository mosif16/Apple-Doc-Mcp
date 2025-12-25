use serde::{Deserialize, Serialize};

// ============================================================================
// METAL DOCUMENTATION PROVIDER
// ============================================================================
//
// Apple Metal is a low-level, low-overhead graphics and compute API for iOS,
// macOS, tvOS, and visionOS. It provides near-direct access to the GPU,
// enabling high-performance rendering and parallel computation.
//
// Key Features:
// - MTLDevice: Interface to the GPU
// - MTLCommandQueue: Queue for submitting commands to the GPU
// - MTLRenderPipelineState: Configuration for rendering operations
// - MTLComputePipelineState: Configuration for compute operations
// - MSL (Metal Shading Language): C++-based shader language
//
// Documentation Sources:
// - https://developer.apple.com/documentation/metal
// - https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
// - https://developer.apple.com/documentation/metalperformanceshaders
//
// ============================================================================

/// Metal technology representation (API categories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of Metal documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<MetalCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: MetalItemKind,
    pub url: String,
}

/// Kind of Metal documentation item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetalItemKind {
    /// Core Metal type (MTLDevice, MTLCommandQueue, etc.)
    CoreType,
    /// Metal function/method
    Function,
    /// Render pipeline component
    RenderPipeline,
    /// Compute pipeline component
    ComputePipeline,
    /// Resource management (buffers, textures)
    Resource,
    /// Metal Shading Language construct
    ShaderLanguage,
    /// MetalPerformanceShaders operation
    MPS,
    /// Metal Performance Shaders Graph (MPSGraph)
    MPSGraph,
    /// MetalFX upscaling/effects
    MetalFX,
    /// Optimization technique or best practice
    Optimization,
    /// GPU feature or capability
    GPUFeature,
}

impl std::fmt::Display for MetalItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoreType => write!(f, "Core Type"),
            Self::Function => write!(f, "Function"),
            Self::RenderPipeline => write!(f, "Render Pipeline"),
            Self::ComputePipeline => write!(f, "Compute Pipeline"),
            Self::Resource => write!(f, "Resource"),
            Self::ShaderLanguage => write!(f, "Shader Language"),
            Self::MPS => write!(f, "Metal Performance Shaders"),
            Self::MPSGraph => write!(f, "MPS Graph"),
            Self::MetalFX => write!(f, "MetalFX"),
            Self::Optimization => write!(f, "Optimization"),
            Self::GPUFeature => write!(f, "GPU Feature"),
        }
    }
}

/// Detailed Metal API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalMethod {
    pub name: String,
    pub description: String,
    pub kind: MetalItemKind,
    pub url: String,
    pub parameters: Vec<MetalParameter>,
    pub returns: Option<MetalReturnType>,
    pub examples: Vec<MetalExample>,
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalReturnType {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<MetalReturnField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalReturnField {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalExample {
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Static method index entry
#[derive(Debug, Clone)]
pub struct MetalMethodIndex {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: MetalItemKind,
    pub category: &'static str,
}

// ============================================================================
// METAL CORE TYPES
// ============================================================================

pub const METAL_CORE_TYPES: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLDevice", description: "The interface to a GPU that you use to draw graphics or perform parallel computation. Create command queues, allocate buffers/textures, and create pipeline states through this object.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLCommandQueue", description: "A queue that organizes command buffers to be executed by a GPU. Create command buffers from this queue and commit them for GPU execution.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLCommandBuffer", description: "A container that stores encoded GPU commands. Encode render, compute, or blit commands, then commit the buffer to the command queue.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLCommandEncoder", description: "An encoder that writes GPU commands into a command buffer. Base protocol for render, compute, and blit encoders.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLRenderCommandEncoder", description: "Encodes rendering commands into a command buffer. Set pipeline states, bind resources, and issue draw calls.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLComputeCommandEncoder", description: "Encodes compute (GPGPU) commands into a command buffer. Dispatch compute kernels and manage threadgroups.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLBlitCommandEncoder", description: "Encodes memory copy and fill operations. Copy between buffers, textures, and generate mipmaps.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLLibrary", description: "A collection of compiled shader functions. Load from .metallib files or compile from source at runtime.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLFunction", description: "A single compiled shader function. Retrieve from MTLLibrary by name for use in pipeline states.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLFence", description: "A synchronization primitive for coordinating work between the CPU and GPU or between encoders.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLEvent", description: "A synchronization primitive for signaling and waiting on GPU events across command buffers.", kind: MetalItemKind::CoreType, category: "core" },
    MetalMethodIndex { name: "MTLHeap", description: "A memory pool for allocating buffers and textures. Enables efficient sub-allocation and resource aliasing.", kind: MetalItemKind::CoreType, category: "core" },
];

// ============================================================================
// METAL RESOURCE MANAGEMENT
// ============================================================================

pub const METAL_RESOURCES: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLBuffer", description: "A typeless allocation of GPU memory. Use for vertex data, uniforms, storage buffers, or indirect command arguments.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLTexture", description: "A structured collection of texture image data. Supports 1D, 2D, 3D, cube maps, and array textures.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLSamplerState", description: "Defines how to sample a texture. Configure filtering, addressing, LOD, and comparison functions.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLDepthStencilState", description: "Configures depth and stencil testing. Set depth compare functions, write masks, and stencil operations.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLResourceStateCommandEncoder", description: "Manages resource residency and sparse texture mappings for explicit resource state management.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLAccelerationStructure", description: "A data structure for ray tracing. Build from geometry to enable GPU-accelerated ray intersection queries.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLIntersectionFunctionTable", description: "A table of intersection functions for ray tracing custom geometry.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLArgumentEncoder", description: "Encodes resources into an argument buffer for bindless rendering. Reduces CPU overhead.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "MTLIndirectCommandBuffer", description: "A buffer containing GPU-generated draw or dispatch commands. Enables GPU-driven rendering.", kind: MetalItemKind::Resource, category: "resource" },
    MetalMethodIndex { name: "makeBuffer", description: "Creates a new buffer on the device. Options include storageModeShared (CPU/GPU), storageModePrivate (GPU-only), and storageModeManaged.", kind: MetalItemKind::Function, category: "resource" },
    MetalMethodIndex { name: "makeTexture", description: "Creates a new texture from a descriptor. Configure width, height, pixel format, usage, and storage mode.", kind: MetalItemKind::Function, category: "resource" },
];

// ============================================================================
// METAL RENDER PIPELINE
// ============================================================================

pub const METAL_RENDER_PIPELINE: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLRenderPipelineState", description: "An immutable object representing the complete render pipeline configuration. Create from MTLRenderPipelineDescriptor.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "MTLRenderPipelineDescriptor", description: "Configures a render pipeline. Set vertex/fragment functions, color attachments, depth/stencil formats, and blending.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "MTLRenderPassDescriptor", description: "Defines render targets for a render pass. Configure color, depth, and stencil attachments with load/store actions.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "MTLVertexDescriptor", description: "Describes the layout of vertex attributes and buffer bindings. Define attribute formats, offsets, and buffer strides.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "MTLRenderPassColorAttachmentDescriptor", description: "Configures a color attachment. Set texture, load/store actions, clear color, and resolve texture for MSAA.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "MTLRenderPassDepthAttachmentDescriptor", description: "Configures the depth attachment. Set texture, load/store actions, clear depth value.", kind: MetalItemKind::RenderPipeline, category: "render" },
    MetalMethodIndex { name: "drawPrimitives", description: "Draws primitives without indexing. Specify primitive type (triangle, line, point), vertex start, and count.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "drawIndexedPrimitives", description: "Draws indexed primitives. Provide index buffer, index count, and primitive type for efficient mesh rendering.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "drawMeshThreadgroups", description: "Draws using mesh shaders (Apple Silicon). Specify threadgroup counts for object and mesh shader stages.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setVertexBuffer", description: "Binds a buffer to a vertex shader buffer argument table slot. Essential for passing vertex data and uniforms.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setFragmentTexture", description: "Binds a texture to a fragment shader texture argument table slot.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setRenderPipelineState", description: "Sets the active render pipeline state for subsequent draw calls.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setDepthStencilState", description: "Sets the depth/stencil state for depth testing and stencil operations.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setViewport", description: "Sets the viewport transformation. Define origin, size, and depth range.", kind: MetalItemKind::Function, category: "render" },
    MetalMethodIndex { name: "setScissorRect", description: "Sets the scissor rectangle for clipping rendered fragments.", kind: MetalItemKind::Function, category: "render" },
];

// ============================================================================
// METAL COMPUTE PIPELINE
// ============================================================================

pub const METAL_COMPUTE_PIPELINE: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLComputePipelineState", description: "An immutable object representing a compute pipeline configuration. Create from a kernel function.", kind: MetalItemKind::ComputePipeline, category: "compute" },
    MetalMethodIndex { name: "MTLComputePipelineDescriptor", description: "Configures a compute pipeline. Set the compute function, threadgroup memory, and buffer mutability.", kind: MetalItemKind::ComputePipeline, category: "compute" },
    MetalMethodIndex { name: "dispatchThreadgroups", description: "Dispatches compute work. Specify the number of threadgroups and threads per threadgroup.", kind: MetalItemKind::Function, category: "compute" },
    MetalMethodIndex { name: "dispatchThreads", description: "Dispatches compute work specifying total thread count. Metal calculates optimal threadgroup distribution.", kind: MetalItemKind::Function, category: "compute" },
    MetalMethodIndex { name: "setComputePipelineState", description: "Sets the active compute pipeline state for subsequent dispatches.", kind: MetalItemKind::Function, category: "compute" },
    MetalMethodIndex { name: "setBuffer", description: "Binds a buffer to a compute kernel buffer argument table slot.", kind: MetalItemKind::Function, category: "compute" },
    MetalMethodIndex { name: "setTexture", description: "Binds a texture to a compute kernel texture argument table slot.", kind: MetalItemKind::Function, category: "compute" },
    MetalMethodIndex { name: "threadExecutionWidth", description: "The number of threads that execute in lockstep (SIMD width). 32 on Apple Silicon.", kind: MetalItemKind::GPUFeature, category: "compute" },
    MetalMethodIndex { name: "maxTotalThreadsPerThreadgroup", description: "Maximum threads per threadgroup. Typically 1024 on Apple Silicon.", kind: MetalItemKind::GPUFeature, category: "compute" },
    MetalMethodIndex { name: "simdgroup_size", description: "SIMD group size in MSL (32 on Apple Silicon). Use for efficient parallel reductions.", kind: MetalItemKind::ShaderLanguage, category: "compute" },
];

// ============================================================================
// METAL SHADING LANGUAGE (MSL)
// ============================================================================

pub const METAL_SHADING_LANGUAGE: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "vertex", description: "MSL function qualifier for vertex shaders. Process per-vertex data and output clip-space positions.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "fragment", description: "MSL function qualifier for fragment/pixel shaders. Compute per-pixel colors for rasterized fragments.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "kernel", description: "MSL function qualifier for compute shaders. Runs general-purpose GPU computation.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "object", description: "MSL function qualifier for object shaders in mesh shader pipeline. Generates meshlets for mesh stage.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "mesh", description: "MSL function qualifier for mesh shaders. Generates primitives from meshlet data.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "device", description: "MSL address space for device memory. Readable and writable from the GPU.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "constant", description: "MSL address space for constant/uniform data. Read-only, optimized for broadcast access.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "threadgroup", description: "MSL address space for shared memory within a threadgroup. Fast on-chip memory for cooperation.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "thread", description: "MSL address space for per-thread local variables. Stored in registers when possible.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "texture2d", description: "MSL texture type for 2D textures. Sample with sample(), read with read(), write with write().", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "sampler", description: "MSL sampler type. Defines filtering (linear, nearest) and addressing (clamp, repeat, mirror).", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "thread_position_in_grid", description: "MSL built-in for compute kernel. Global thread index across all threadgroups.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "thread_position_in_threadgroup", description: "MSL built-in for compute kernel. Local thread index within the threadgroup.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "threadgroup_position_in_grid", description: "MSL built-in for compute kernel. Index of the current threadgroup.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "threads_per_threadgroup", description: "MSL built-in containing the threadgroup dimensions.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "simd_shuffle", description: "MSL SIMD operation. Exchange data between threads in the same SIMD group without threadgroup memory.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "simd_sum", description: "MSL SIMD reduction. Sum values across all threads in the SIMD group.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "threadgroup_barrier", description: "MSL barrier for threadgroup synchronization. All threads must reach before any proceeds. Use mem_flags.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "atomic_fetch_add", description: "MSL atomic operation. Atomically adds to a value and returns the original. Use atomic_int/atomic_uint.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "simd_prefix_exclusive_sum", description: "MSL SIMD prefix sum (scan). Each thread gets the sum of all preceding threads' values.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "float4x4", description: "MSL matrix type for 4x4 matrices. Use for model-view-projection transformations.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "packed_float3", description: "MSL packed type without padding. Use in buffers to match C struct layouts.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
    MetalMethodIndex { name: "half", description: "MSL 16-bit floating point type. More efficient than float on Apple Silicon.", kind: MetalItemKind::ShaderLanguage, category: "msl" },
];

// ============================================================================
// METAL PERFORMANCE SHADERS (MPS)
// ============================================================================

pub const METAL_MPS: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MPSImageConvolution", description: "MPS convolution filter. Apply blur, sharpen, edge detection, and custom convolution kernels.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSImageGaussianBlur", description: "MPS Gaussian blur filter. Fast, separable implementation with configurable sigma.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSImageScale", description: "MPS image scaling. Bilinear or Lanczos resampling for high-quality resizing.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSImageHistogram", description: "Computes image histogram. Returns distribution of pixel values for analysis.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSImageIntegral", description: "Computes integral (summed area table) image. Used for fast box filtering.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSMatrixMultiplication", description: "MPS matrix multiplication. C = alpha*A*B + beta*C. Optimized for Apple Silicon.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSMatrixVectorMultiplication", description: "MPS matrix-vector multiplication. y = alpha*A*x + beta*y.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSTemporaryImage", description: "A texture for intermediate MPS results. Automatically managed, efficient for filter chains.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSRayIntersector", description: "MPS ray tracing intersector. Find intersections between rays and acceleration structures.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSAccelerationStructure", description: "MPS acceleration structure for ray tracing. Build from triangles or instances.", kind: MetalItemKind::MPS, category: "mps" },
    // Neural Network Layers
    MetalMethodIndex { name: "MPSCNNConvolution", description: "MPS convolutional neural network layer. Optimized 2D convolution with bias and activation.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSCNNFullyConnected", description: "MPS fully-connected (dense) neural network layer.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSCNNPooling", description: "MPS pooling layer. Max pooling, average pooling with configurable kernel size and stride.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSCNNBatchNormalization", description: "MPS batch normalization layer. Normalizes activations for faster training.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSCNNSoftMax", description: "MPS softmax activation. Converts logits to probabilities for classification.", kind: MetalItemKind::MPS, category: "mps" },
    MetalMethodIndex { name: "MPSNNGraph", description: "MPS neural network graph. Combines layers into an optimized inference pipeline.", kind: MetalItemKind::MPS, category: "mps" },
];

// ============================================================================
// MPS GRAPH (Neural Network Training & Inference)
// ============================================================================

pub const METAL_MPSGRAPH: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MPSGraph", description: "A compute graph for building neural networks and other computations. Supports automatic differentiation.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "MPSGraphTensor", description: "Symbolic tensor in an MPSGraph. Represents data flowing through the graph.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "MPSGraphTensorData", description: "Concrete tensor data for MPSGraph execution. Wraps MTLBuffer or CPU data.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "MPSGraphExecutable", description: "A compiled, optimized MPSGraph ready for execution. Cache for repeated inference.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "matrixMultiplication", description: "MPSGraph matrix multiplication operation. Returns A @ B tensor.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "convolution2D", description: "MPSGraph 2D convolution operation. Configure weights, bias, strides, padding.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "softMax", description: "MPSGraph softmax operation for classification output.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "reLU", description: "MPSGraph ReLU activation. max(0, x) for non-linearity.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "geLU", description: "MPSGraph GELU activation. Gaussian Error Linear Unit for transformer models.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "layerNormalization", description: "MPSGraph layer normalization for transformer models.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "gradients", description: "MPSGraph automatic differentiation. Compute gradients for training.", kind: MetalItemKind::MPSGraph, category: "mpsgraph" },
    MetalMethodIndex { name: "run", description: "Execute MPSGraph with input data. Returns output tensors.", kind: MetalItemKind::Function, category: "mpsgraph" },
    MetalMethodIndex { name: "runAsync", description: "Execute MPSGraph asynchronously. Returns immediately, signals completion.", kind: MetalItemKind::Function, category: "mpsgraph" },
];

// ============================================================================
// METALFX (Upscaling and Effects)
// ============================================================================

pub const METAL_METALFX: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLFXSpatialScaler", description: "MetalFX spatial upscaler. Uses AI to upscale rendered frames without temporal data.", kind: MetalItemKind::MetalFX, category: "metalfx" },
    MetalMethodIndex { name: "MTLFXTemporalScaler", description: "MetalFX temporal upscaler. Uses motion vectors and history for higher quality upscaling.", kind: MetalItemKind::MetalFX, category: "metalfx" },
    MetalMethodIndex { name: "MTLFXSpatialScalerDescriptor", description: "Configures spatial upscaler. Set input/output texture formats and color processing.", kind: MetalItemKind::MetalFX, category: "metalfx" },
    MetalMethodIndex { name: "MTLFXTemporalScalerDescriptor", description: "Configures temporal upscaler. Set motion vector format, depth, and jitter.", kind: MetalItemKind::MetalFX, category: "metalfx" },
    MetalMethodIndex { name: "encode", description: "MetalFX encode upscaling into command buffer. Processes input to output texture.", kind: MetalItemKind::Function, category: "metalfx" },
];

// ============================================================================
// METAL RAY TRACING
// ============================================================================

pub const METAL_RAYTRACING: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "MTLAccelerationStructureDescriptor", description: "Describes an acceleration structure for ray tracing. Configure geometry or instances.", kind: MetalItemKind::Resource, category: "raytracing" },
    MetalMethodIndex { name: "MTLPrimitiveAccelerationStructureDescriptor", description: "Describes a bottom-level acceleration structure from triangles or bounding boxes.", kind: MetalItemKind::Resource, category: "raytracing" },
    MetalMethodIndex { name: "MTLInstanceAccelerationStructureDescriptor", description: "Describes a top-level acceleration structure from instances of primitive structures.", kind: MetalItemKind::Resource, category: "raytracing" },
    MetalMethodIndex { name: "intersector", description: "MSL ray tracing intersector. Query acceleration structures for ray-geometry intersections.", kind: MetalItemKind::ShaderLanguage, category: "raytracing" },
    MetalMethodIndex { name: "ray", description: "MSL ray type for ray tracing. Contains origin, direction, min/max distance.", kind: MetalItemKind::ShaderLanguage, category: "raytracing" },
    MetalMethodIndex { name: "intersection_result", description: "MSL intersection result. Contains distance, triangle ID, barycentrics, instance ID.", kind: MetalItemKind::ShaderLanguage, category: "raytracing" },
    MetalMethodIndex { name: "useResourceHeap", description: "Makes resources in a heap available to shaders during ray tracing.", kind: MetalItemKind::Function, category: "raytracing" },
];

// ============================================================================
// METAL OPTIMIZATION TECHNIQUES
// ============================================================================

pub const METAL_OPTIMIZATION: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "resource_binding_tier", description: "Metal resource binding model. Tier 1 (discrete GPU) vs Tier 2 (Apple Silicon) affects bindless performance.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "argument_buffer", description: "Group resources into a single buffer for bindless rendering. Reduces setBuffer/setTexture calls.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "indirect_rendering", description: "GPU-driven rendering with indirect command buffers. GPU generates draw calls, reducing CPU overhead.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "triple_buffering", description: "Use 3 frames in flight to hide latency. Rotate between buffers to avoid stalls.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "memoryless_attachments", description: "Render targets that exist only in tile memory. No DRAM backing, saves bandwidth.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "tile_based_deferred_rendering", description: "Apple GPU architecture. Renders to on-chip tile memory before writing to DRAM.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "store_action_dont_care", description: "Skip storing attachment contents. Use for depth/stencil when values aren't needed later.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "half_precision", description: "Use float16 (half) when full precision isn't needed. 2x throughput on Apple Silicon.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "simd_group_reduction", description: "Use SIMD operations (simd_sum, simd_prefix_sum) instead of threadgroup barriers.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "texture_compression", description: "Use ASTC or BC compressed textures. Reduces memory bandwidth and storage.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "lossless_compression", description: "Metal automatically compresses render targets. Preserve by using optimal store actions.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "parallel_render_encoder", description: "Split render pass across multiple threads. Each thread creates commands in parallel.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "function_constants", description: "Compile-time shader specialization. Eliminates branches and unused code paths.", kind: MetalItemKind::Optimization, category: "optimization" },
    MetalMethodIndex { name: "preload_textures", description: "Use useResource() to preload textures into GPU cache before sampling.", kind: MetalItemKind::Optimization, category: "optimization" },
];

// ============================================================================
// APPLE GPU FEATURES
// ============================================================================

pub const METAL_GPU_FEATURES: &[MetalMethodIndex] = &[
    MetalMethodIndex { name: "Apple_M1", description: "Apple M1 GPU: 8 cores, ~2.6 TFLOPS FP32. Unified memory, 200GB/s bandwidth. Metal 3.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "Apple_M2", description: "Apple M2 GPU: 10 cores, ~3.6 TFLOPS FP32. Ray tracing hardware, mesh shaders. Metal 3.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "Apple_M3", description: "Apple M3 GPU: 10 cores (base), hardware ray tracing, mesh shaders, dynamic caching. Metal 3.1.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "Apple_M4", description: "Apple M4 GPU: Enhanced ray tracing, mesh shaders, hardware-accelerated machine learning. Metal 3.2.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "GPU_family_apple", description: "Query GPU family for feature support. Use supportsFamily(.apple7) for M1+, .apple8 for M2+.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "mesh_shaders", description: "Apple Silicon mesh shader support. Replace vertex processing with flexible geometry pipeline.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "hardware_ray_tracing", description: "Apple Silicon hardware ray tracing. Available on M2+. Use MTLAccelerationStructure.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "unified_memory", description: "Apple Silicon unified memory. CPU and GPU share physical memory, eliminating copies.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "tile_shading", description: "Apple GPU tile shading. Access on-chip tile memory between render passes.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "raster_order_groups", description: "Apple GPU feature for programmable blending. Guarantees order between overlapping fragments.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "dynamic_libraries", description: "Metal 3 feature. Load shader functions at runtime for modular rendering.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "function_pointers", description: "Metal 3 feature. Store and call shader functions indirectly.", kind: MetalItemKind::GPUFeature, category: "gpu" },
    MetalMethodIndex { name: "sparse_textures", description: "Partially resident textures. Only load needed regions to save memory.", kind: MetalItemKind::GPUFeature, category: "gpu" },
];
