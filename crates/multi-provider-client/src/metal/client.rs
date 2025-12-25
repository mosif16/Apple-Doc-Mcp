#![allow(clippy::needless_raw_string_hashes)]

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::Result;
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{instrument, warn};

use super::types::{
    MetalCategory, MetalCategoryItem, MetalExample, MetalMethod,
    MetalMethodIndex, MetalItemKind, MetalParameter,
    MetalReturnType, MetalTechnology,
    METAL_CORE_TYPES, METAL_RESOURCES, METAL_RENDER_PIPELINE,
    METAL_COMPUTE_PIPELINE, METAL_SHADING_LANGUAGE, METAL_MPS,
    METAL_MPSGRAPH, METAL_METALFX, METAL_RAYTRACING, METAL_OPTIMIZATION,
    METAL_GPU_FEATURES,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const METAL_DOCS_URL: &str = "https://developer.apple.com/documentation/metal";
const METAL_MPS_URL: &str = "https://developer.apple.com/documentation/metalperformanceshaders";
const METAL_MPSGRAPH_URL: &str = "https://developer.apple.com/documentation/metalperformanceshadersgraph";
const METAL_METALFX_URL: &str = "https://developer.apple.com/documentation/metalfx";

#[derive(Debug)]
#[allow(dead_code)]
pub struct MetalClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<String>,
    fetch_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for MetalClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MetalClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("metal");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create Metal cache directory");
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

    /// Get available technologies (Metal categories)
    #[instrument(name = "metal_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<MetalTechnology>> {
        let core = MetalTechnology {
            identifier: "metal:core".to_string(),
            title: "Metal Core API".to_string(),
            description: format!(
                "Metal Core API - {} types for GPU device management, command queues, encoders, and synchronization",
                METAL_CORE_TYPES.len() + METAL_RESOURCES.len()
            ),
            url: METAL_DOCS_URL.to_string(),
            item_count: METAL_CORE_TYPES.len() + METAL_RESOURCES.len(),
        };

        let render = MetalTechnology {
            identifier: "metal:render".to_string(),
            title: "Render Pipeline".to_string(),
            description: format!(
                "Metal Render Pipeline - {} types for vertex processing, rasterization, and fragment shading",
                METAL_RENDER_PIPELINE.len()
            ),
            url: format!("{}/render_pipeline", METAL_DOCS_URL),
            item_count: METAL_RENDER_PIPELINE.len(),
        };

        let compute = MetalTechnology {
            identifier: "metal:compute".to_string(),
            title: "Compute Pipeline".to_string(),
            description: format!(
                "Metal Compute Pipeline - {} types for GPGPU computation and parallel processing",
                METAL_COMPUTE_PIPELINE.len()
            ),
            url: format!("{}/compute_pipeline", METAL_DOCS_URL),
            item_count: METAL_COMPUTE_PIPELINE.len(),
        };

        let msl = MetalTechnology {
            identifier: "metal:msl".to_string(),
            title: "Metal Shading Language".to_string(),
            description: format!(
                "Metal Shading Language (MSL) - {} constructs for vertex, fragment, compute, and mesh shaders",
                METAL_SHADING_LANGUAGE.len()
            ),
            url: "https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf".to_string(),
            item_count: METAL_SHADING_LANGUAGE.len(),
        };

        let mps = MetalTechnology {
            identifier: "metal:mps".to_string(),
            title: "Metal Performance Shaders".to_string(),
            description: format!(
                "Metal Performance Shaders - {} optimized image processing and neural network operations",
                METAL_MPS.len()
            ),
            url: METAL_MPS_URL.to_string(),
            item_count: METAL_MPS.len(),
        };

        let mpsgraph = MetalTechnology {
            identifier: "metal:mpsgraph".to_string(),
            title: "MPS Graph".to_string(),
            description: format!(
                "MPS Graph - {} operations for building and executing compute graphs with automatic differentiation",
                METAL_MPSGRAPH.len()
            ),
            url: METAL_MPSGRAPH_URL.to_string(),
            item_count: METAL_MPSGRAPH.len(),
        };

        let metalfx = MetalTechnology {
            identifier: "metal:metalfx".to_string(),
            title: "MetalFX".to_string(),
            description: format!(
                "MetalFX - {} types for AI-powered upscaling and temporal anti-aliasing",
                METAL_METALFX.len()
            ),
            url: METAL_METALFX_URL.to_string(),
            item_count: METAL_METALFX.len(),
        };

        let raytracing = MetalTechnology {
            identifier: "metal:raytracing".to_string(),
            title: "Ray Tracing".to_string(),
            description: format!(
                "Metal Ray Tracing - {} types for hardware-accelerated ray tracing on Apple Silicon",
                METAL_RAYTRACING.len()
            ),
            url: format!("{}/ray_tracing", METAL_DOCS_URL),
            item_count: METAL_RAYTRACING.len(),
        };

        let optimization = MetalTechnology {
            identifier: "metal:optimization".to_string(),
            title: "Optimization".to_string(),
            description: format!(
                "Metal Optimization - {} techniques for maximizing GPU performance on Apple Silicon",
                METAL_OPTIMIZATION.len()
            ),
            url: "https://developer.apple.com/documentation/metal/gpu_programming_best_practices".to_string(),
            item_count: METAL_OPTIMIZATION.len(),
        };

        let gpu = MetalTechnology {
            identifier: "metal:gpu".to_string(),
            title: "Apple GPU Features".to_string(),
            description: format!(
                "Apple GPU Features - {} items covering M1/M2/M3/M4 capabilities, mesh shaders, and hardware features",
                METAL_GPU_FEATURES.len()
            ),
            url: "https://developer.apple.com/metal/gpu-features/".to_string(),
            item_count: METAL_GPU_FEATURES.len(),
        };

        Ok(vec![core, render, compute, msl, mps, mpsgraph, metalfx, raytracing, optimization, gpu])
    }

    /// Get a category of methods
    #[instrument(name = "metal_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<MetalCategory> {
        let (methods, title, description): (Vec<&MetalMethodIndex>, &str, &str) = match identifier {
            "metal:core" | "core" | "device" => {
                let methods: Vec<&MetalMethodIndex> = METAL_CORE_TYPES.iter()
                    .chain(METAL_RESOURCES.iter())
                    .collect();
                (methods, "Metal Core API", "GPU device, command queues, encoders, resources, and synchronization primitives")
            }
            "metal:resources" | "resources" | "buffer" | "texture" => (
                METAL_RESOURCES.iter().collect(),
                "Metal Resources",
                "Buffers, textures, samplers, and GPU memory management",
            ),
            "metal:render" | "render" | "rendering" => (
                METAL_RENDER_PIPELINE.iter().collect(),
                "Render Pipeline",
                "Render pipeline states, descriptors, vertex processing, and draw commands",
            ),
            "metal:compute" | "compute" | "gpgpu" => (
                METAL_COMPUTE_PIPELINE.iter().collect(),
                "Compute Pipeline",
                "Compute pipeline states, kernel dispatch, and threadgroup management",
            ),
            "metal:msl" | "msl" | "shader" | "shading" => (
                METAL_SHADING_LANGUAGE.iter().collect(),
                "Metal Shading Language",
                "MSL qualifiers, address spaces, built-ins, SIMD operations, and synchronization",
            ),
            "metal:mps" | "mps" | "performance" | "image" => (
                METAL_MPS.iter().collect(),
                "Metal Performance Shaders",
                "Image processing, convolutions, neural network layers, and ray tracing",
            ),
            "metal:mpsgraph" | "mpsgraph" | "graph" | "ml" => (
                METAL_MPSGRAPH.iter().collect(),
                "MPS Graph",
                "Compute graphs, tensor operations, automatic differentiation, and neural network training",
            ),
            "metal:metalfx" | "metalfx" | "upscaling" => (
                METAL_METALFX.iter().collect(),
                "MetalFX",
                "AI-powered spatial and temporal upscaling for games",
            ),
            "metal:raytracing" | "raytracing" | "ray" | "tracing" => (
                METAL_RAYTRACING.iter().collect(),
                "Ray Tracing",
                "Acceleration structures, ray intersection, and hardware ray tracing",
            ),
            "metal:optimization" | "optimization" | "optimize" => (
                METAL_OPTIMIZATION.iter().collect(),
                "Optimization",
                "Performance techniques for Apple Silicon GPUs",
            ),
            "metal:gpu" | "gpu" | "apple" | "m1" | "m2" | "m3" | "m4" => (
                METAL_GPU_FEATURES.iter().collect(),
                "Apple GPU Features",
                "Apple Silicon GPU capabilities, mesh shaders, hardware ray tracing, and unified memory",
            ),
            _ => anyhow::bail!("Unknown Metal category: {identifier}"),
        };

        let items = methods
            .iter()
            .map(|m| MetalCategoryItem {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind: m.kind,
                url: self.get_method_url(m),
            })
            .collect();

        Ok(MetalCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
        })
    }

    /// Get URL for a method
    fn get_method_url(&self, method: &MetalMethodIndex) -> String {
        match method.kind {
            MetalItemKind::CoreType | MetalItemKind::Function | MetalItemKind::Resource => {
                format!("{}/{}", METAL_DOCS_URL, method.name.to_lowercase())
            }
            MetalItemKind::RenderPipeline => {
                format!("{}/render_pipeline/{}", METAL_DOCS_URL, method.name.to_lowercase())
            }
            MetalItemKind::ComputePipeline => {
                format!("{}/compute_pipeline/{}", METAL_DOCS_URL, method.name.to_lowercase())
            }
            MetalItemKind::ShaderLanguage => {
                "https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf".to_string()
            }
            MetalItemKind::MPS => {
                format!("{}/{}", METAL_MPS_URL, method.name.to_lowercase())
            }
            MetalItemKind::MPSGraph => {
                format!("{}/{}", METAL_MPSGRAPH_URL, method.name.to_lowercase())
            }
            MetalItemKind::MetalFX => {
                format!("{}/{}", METAL_METALFX_URL, method.name.to_lowercase())
            }
            MetalItemKind::Optimization | MetalItemKind::GPUFeature => {
                format!("{}/gpu_features", METAL_DOCS_URL)
            }
        }
    }

    /// Get all methods as a flat list for searching
    fn all_methods() -> impl Iterator<Item = &'static MetalMethodIndex> {
        METAL_CORE_TYPES.iter()
            .chain(METAL_RESOURCES.iter())
            .chain(METAL_RENDER_PIPELINE.iter())
            .chain(METAL_COMPUTE_PIPELINE.iter())
            .chain(METAL_SHADING_LANGUAGE.iter())
            .chain(METAL_MPS.iter())
            .chain(METAL_MPSGRAPH.iter())
            .chain(METAL_METALFX.iter())
            .chain(METAL_RAYTRACING.iter())
            .chain(METAL_OPTIMIZATION.iter())
            .chain(METAL_GPU_FEATURES.iter())
    }

    /// Build detailed method documentation
    fn build_method_doc(&self, index_entry: &MetalMethodIndex) -> MetalMethod {
        let examples = self.generate_examples(index_entry);
        let parameters = self.infer_parameters(index_entry);
        let platforms = self.get_platforms(index_entry);

        MetalMethod {
            name: index_entry.name.to_string(),
            description: index_entry.description.to_string(),
            kind: index_entry.kind,
            url: self.get_method_url(index_entry),
            parameters,
            returns: self.infer_return_type(index_entry),
            examples,
            platforms,
        }
    }

    /// Get platforms for a method
    fn get_platforms(&self, method: &MetalMethodIndex) -> Vec<String> {
        match method.kind {
            MetalItemKind::MetalFX => vec!["macOS 13.0+".to_string(), "iOS 16.0+".to_string()],
            MetalItemKind::MPSGraph => vec!["macOS 11.0+".to_string(), "iOS 14.0+".to_string()],
            _ => vec![
                "macOS 10.11+".to_string(),
                "iOS 8.0+".to_string(),
                "tvOS 9.0+".to_string(),
                "visionOS 1.0+".to_string(),
            ],
        }
    }

    /// Generate example code for a method
    fn generate_examples(&self, method: &MetalMethodIndex) -> Vec<MetalExample> {
        let mut examples = Vec::new();

        match method.name {
            // Core Types
            "MTLDevice" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Get the default GPU device
guard let device = MTLCreateSystemDefaultDevice() else {
    fatalError("GPU not available")
}

// Query device capabilities
print("Device: \(device.name)")
print("Unified Memory: \(device.hasUnifiedMemory)")
print("Recommended Max Working Set: \(device.recommendedMaxWorkingSetSize / 1_000_000) MB")

// Check feature support
if device.supportsFamily(.apple7) {
    print("Supports Apple7 GPU features (M1+)")
}"#.to_string(),
                    description: Some("Get GPU device and query capabilities".to_string()),
                });
            }
            "MTLCommandQueue" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create a command queue
guard let commandQueue = device.makeCommandQueue() else {
    fatalError("Failed to create command queue")
}

// Create and commit a command buffer
guard let commandBuffer = commandQueue.makeCommandBuffer() else { return }

// Encode commands...

commandBuffer.commit()
commandBuffer.waitUntilCompleted()"#.to_string(),
                    description: Some("Create command queue and submit work".to_string()),
                });
            }
            "MTLCommandBuffer" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"guard let commandBuffer = commandQueue.makeCommandBuffer() else { return }

// Add completion handler
commandBuffer.addCompletedHandler { buffer in
    if let error = buffer.error {
        print("GPU error: \(error)")
    } else {
        print("GPU execution time: \(buffer.gpuEndTime - buffer.gpuStartTime)s")
    }
}

// Encode render pass
if let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: renderPassDesc) {
    encoder.setRenderPipelineState(pipelineState)
    encoder.setVertexBuffer(vertexBuffer, offset: 0, index: 0)
    encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
    encoder.endEncoding()
}

commandBuffer.commit()"#.to_string(),
                    description: Some("Encode and submit GPU commands".to_string()),
                });
            }

            // Resources
            "MTLBuffer" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create vertex buffer from data
let vertices: [Vertex] = [
    Vertex(position: [-1, -1, 0], color: [1, 0, 0, 1]),
    Vertex(position: [ 1, -1, 0], color: [0, 1, 0, 1]),
    Vertex(position: [ 0,  1, 0], color: [0, 0, 1, 1]),
]

guard let vertexBuffer = device.makeBuffer(
    bytes: vertices,
    length: MemoryLayout<Vertex>.stride * vertices.count,
    options: .storageModeShared  // CPU and GPU accessible
) else { return }

// For GPU-only data, use .storageModePrivate for better performance
guard let privateBuffer = device.makeBuffer(
    length: 1024,
    options: .storageModePrivate
) else { return }"#.to_string(),
                    description: Some("Create GPU buffers for vertex and compute data".to_string()),
                });
            }
            "MTLTexture" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create texture descriptor
let descriptor = MTLTextureDescriptor()
descriptor.textureType = .type2D
descriptor.pixelFormat = .rgba8Unorm
descriptor.width = 1024
descriptor.height = 1024
descriptor.usage = [.shaderRead, .shaderWrite]
descriptor.storageMode = .private

guard let texture = device.makeTexture(descriptor: descriptor) else { return }

// For render targets, add renderTarget usage
let renderTargetDesc = MTLTextureDescriptor.texture2DDescriptor(
    pixelFormat: .bgra8Unorm,
    width: 1920,
    height: 1080,
    mipmapped: false
)
renderTargetDesc.usage = [.renderTarget, .shaderRead]
renderTargetDesc.storageMode = .memoryless  // Tile-only, no DRAM backing"#.to_string(),
                    description: Some("Create textures for sampling and rendering".to_string()),
                });
            }

            // Render Pipeline
            "MTLRenderPipelineState" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create render pipeline descriptor
let pipelineDesc = MTLRenderPipelineDescriptor()
pipelineDesc.vertexFunction = library.makeFunction(name: "vertexShader")
pipelineDesc.fragmentFunction = library.makeFunction(name: "fragmentShader")
pipelineDesc.colorAttachments[0].pixelFormat = .bgra8Unorm
pipelineDesc.depthAttachmentPixelFormat = .depth32Float

// Enable blending
pipelineDesc.colorAttachments[0].isBlendingEnabled = true
pipelineDesc.colorAttachments[0].rgbBlendOperation = .add
pipelineDesc.colorAttachments[0].sourceRGBBlendFactor = .sourceAlpha
pipelineDesc.colorAttachments[0].destinationRGBBlendFactor = .oneMinusSourceAlpha

// Create pipeline state
let pipelineState = try device.makeRenderPipelineState(descriptor: pipelineDesc)"#.to_string(),
                    description: Some("Create render pipeline with blending".to_string()),
                });
            }
            "MTLRenderPassDescriptor" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create render pass descriptor
let renderPassDesc = MTLRenderPassDescriptor()

// Color attachment
renderPassDesc.colorAttachments[0].texture = drawable.texture
renderPassDesc.colorAttachments[0].loadAction = .clear
renderPassDesc.colorAttachments[0].storeAction = .store
renderPassDesc.colorAttachments[0].clearColor = MTLClearColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 1.0)

// Depth attachment
renderPassDesc.depthAttachment.texture = depthTexture
renderPassDesc.depthAttachment.loadAction = .clear
renderPassDesc.depthAttachment.storeAction = .dontCare  // Don't store depth
renderPassDesc.depthAttachment.clearDepth = 1.0

// Create encoder
guard let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: renderPassDesc) else { return }"#.to_string(),
                    description: Some("Configure render pass with color and depth".to_string()),
                });
            }
            "drawPrimitives" | "drawIndexedPrimitives" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Non-indexed drawing
encoder.setRenderPipelineState(pipelineState)
encoder.setVertexBuffer(vertexBuffer, offset: 0, index: 0)
encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)

// Indexed drawing (more efficient for complex meshes)
encoder.setVertexBuffer(vertexBuffer, offset: 0, index: 0)
encoder.drawIndexedPrimitives(
    type: .triangle,
    indexCount: 36,
    indexType: .uint16,
    indexBuffer: indexBuffer,
    indexBufferOffset: 0
)

// Instanced drawing
encoder.drawIndexedPrimitives(
    type: .triangle,
    indexCount: 36,
    indexType: .uint16,
    indexBuffer: indexBuffer,
    indexBufferOffset: 0,
    instanceCount: 100
)"#.to_string(),
                    description: Some("Draw primitives with various methods".to_string()),
                });
            }

            // Compute Pipeline
            "dispatchThreadgroups" | "dispatchThreads" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create compute encoder
guard let computeEncoder = commandBuffer.makeComputeCommandEncoder() else { return }
computeEncoder.setComputePipelineState(computePipeline)
computeEncoder.setBuffer(inputBuffer, offset: 0, index: 0)
computeEncoder.setBuffer(outputBuffer, offset: 0, index: 1)

// Option 1: Dispatch with explicit threadgroup count
let threadgroupSize = MTLSize(width: 256, height: 1, depth: 1)
let threadgroupCount = MTLSize(
    width: (elementCount + 255) / 256,
    height: 1,
    depth: 1
)
computeEncoder.dispatchThreadgroups(threadgroupCount, threadsPerThreadgroup: threadgroupSize)

// Option 2: Let Metal calculate optimal distribution
computeEncoder.dispatchThreads(
    MTLSize(width: elementCount, height: 1, depth: 1),
    threadsPerThreadgroup: MTLSize(width: 256, height: 1, depth: 1)
)

computeEncoder.endEncoding()"#.to_string(),
                    description: Some("Dispatch compute kernels".to_string()),
                });
            }

            // MSL
            "kernel" => {
                examples.push(MetalExample {
                    language: "metal".to_string(),
                    code: r#"// Simple compute kernel
kernel void vectorAdd(
    device const float* a [[buffer(0)]],
    device const float* b [[buffer(1)]],
    device float* result [[buffer(2)]],
    uint id [[thread_position_in_grid]]
) {
    result[id] = a[id] + b[id];
}

// Kernel with shared memory
kernel void reduce(
    device const float* input [[buffer(0)]],
    device float* output [[buffer(1)]],
    threadgroup float* shared [[threadgroup(0)]],
    uint tid [[thread_position_in_threadgroup]],
    uint bid [[threadgroup_position_in_grid]],
    uint blockDim [[threads_per_threadgroup]]
) {
    shared[tid] = input[bid * blockDim + tid];
    threadgroup_barrier(mem_flags::mem_threadgroup);

    for (uint s = blockDim / 2; s > 0; s >>= 1) {
        if (tid < s) {
            shared[tid] += shared[tid + s];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }

    if (tid == 0) {
        output[bid] = shared[0];
    }
}"#.to_string(),
                    description: Some("Compute kernel examples".to_string()),
                });
            }
            "vertex" | "fragment" => {
                examples.push(MetalExample {
                    language: "metal".to_string(),
                    code: r#"#include <metal_stdlib>
using namespace metal;

struct Vertex {
    float3 position [[attribute(0)]];
    float3 normal [[attribute(1)]];
    float2 texCoord [[attribute(2)]];
};

struct VertexOut {
    float4 position [[position]];
    float3 worldNormal;
    float2 texCoord;
};

struct Uniforms {
    float4x4 modelViewProjection;
    float4x4 normalMatrix;
};

vertex VertexOut vertexShader(
    Vertex in [[stage_in]],
    constant Uniforms& uniforms [[buffer(1)]]
) {
    VertexOut out;
    out.position = uniforms.modelViewProjection * float4(in.position, 1.0);
    out.worldNormal = (uniforms.normalMatrix * float4(in.normal, 0.0)).xyz;
    out.texCoord = in.texCoord;
    return out;
}

fragment float4 fragmentShader(
    VertexOut in [[stage_in]],
    texture2d<float> albedo [[texture(0)]],
    sampler texSampler [[sampler(0)]]
) {
    float3 N = normalize(in.worldNormal);
    float3 L = normalize(float3(1, 1, 1));
    float NdotL = max(dot(N, L), 0.0);

    float4 color = albedo.sample(texSampler, in.texCoord);
    return float4(color.rgb * NdotL, color.a);
}"#.to_string(),
                    description: Some("Basic vertex and fragment shaders".to_string()),
                });
            }
            "threadgroup" | "threadgroup_barrier" => {
                examples.push(MetalExample {
                    language: "metal".to_string(),
                    code: r#"// Matrix transpose using threadgroup memory
kernel void transpose(
    texture2d<float, access::read> input [[texture(0)]],
    texture2d<float, access::write> output [[texture(1)]],
    uint2 gid [[thread_position_in_grid]],
    uint2 tid [[thread_position_in_threadgroup]],
    uint2 bid [[threadgroup_position_in_grid]]
) {
    constexpr uint TILE_SIZE = 16;
    threadgroup float tile[TILE_SIZE][TILE_SIZE + 1]; // +1 avoids bank conflicts

    // Read tile with coalesced access
    tile[tid.y][tid.x] = input.read(gid).r;

    // Wait for all threads to load
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // Write transposed with coalesced access
    uint2 outPos = uint2(bid.y * TILE_SIZE + tid.x, bid.x * TILE_SIZE + tid.y);
    output.write(tile[tid.x][tid.y], outPos);
}"#.to_string(),
                    description: Some("Threadgroup memory for efficient data sharing".to_string()),
                });
            }
            "simd_shuffle" | "simd_sum" => {
                examples.push(MetalExample {
                    language: "metal".to_string(),
                    code: r#"// SIMD-efficient parallel reduction
kernel void simdReduce(
    device const float* input [[buffer(0)]],
    device float* output [[buffer(1)]],
    uint tid [[thread_position_in_threadgroup]],
    uint simd_lane [[thread_index_in_simdgroup]],
    uint simd_id [[simdgroup_index_in_threadgroup]]
) {
    float value = input[tid];

    // Reduce within SIMD group (no barriers needed!)
    float simdSum = simd_sum(value);

    // First lane of each SIMD group writes to shared memory
    threadgroup float simdResults[32]; // Max SIMD groups per threadgroup
    if (simd_lane == 0) {
        simdResults[simd_id] = simdSum;
    }

    threadgroup_barrier(mem_flags::mem_threadgroup);

    // First SIMD group reduces the partial sums
    if (simd_id == 0) {
        float partialSum = simd_lane < 32 ? simdResults[simd_lane] : 0;
        float total = simd_sum(partialSum);
        if (simd_lane == 0) {
            output[0] = total;
        }
    }
}"#.to_string(),
                    description: Some("SIMD operations for efficient reductions".to_string()),
                });
            }

            // MPS
            "MPSMatrixMultiplication" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create MPS matrix multiplication
let matMul = MPSMatrixMultiplication(
    device: device,
    transposeLeft: false,
    transposeRight: false,
    resultRows: M,
    resultColumns: N,
    interiorColumns: K,
    alpha: 1.0,
    beta: 0.0
)

// Create matrix descriptors
let descA = MPSMatrixDescriptor(rows: M, columns: K, rowBytes: K * MemoryLayout<Float>.stride, dataType: .float32)
let descB = MPSMatrixDescriptor(rows: K, columns: N, rowBytes: N * MemoryLayout<Float>.stride, dataType: .float32)
let descC = MPSMatrixDescriptor(rows: M, columns: N, rowBytes: N * MemoryLayout<Float>.stride, dataType: .float32)

let matrixA = MPSMatrix(buffer: bufferA, descriptor: descA)
let matrixB = MPSMatrix(buffer: bufferB, descriptor: descB)
let matrixC = MPSMatrix(buffer: bufferC, descriptor: descC)

// Encode multiplication: C = A * B
matMul.encode(commandBuffer: commandBuffer, leftMatrix: matrixA, rightMatrix: matrixB, resultMatrix: matrixC)"#.to_string(),
                    description: Some("MPS matrix multiplication".to_string()),
                });
            }

            // MPSGraph
            "MPSGraph" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create MPSGraph for neural network inference
let graph = MPSGraph()

// Create placeholders for input
let inputPlaceholder = graph.placeholder(
    shape: [1, 3, 224, 224],
    dataType: .float32,
    name: "input"
)

// Create convolution weights (normally loaded from checkpoint)
let weightsData = MPSGraphTensorData(device: MPSGraphDevice(mtlDevice: device), data: weightBuffer, shape: [64, 3, 7, 7], dataType: .float32)

// Build conv -> relu -> pool
let conv = graph.convolution2D(
    inputPlaceholder,
    weights: graph.constant(weightsData.copy()),
    descriptor: MPSGraphConvolution2DOpDescriptor(
        strideInX: 2, strideInY: 2,
        dilationRateInX: 1, dilationRateInY: 1,
        groups: 1,
        paddingStyle: .same,
        dataLayout: .NCHW,
        weightsLayout: .OIHW
    )!,
    name: "conv1"
)
let relu = graph.reLU(with: conv, name: "relu1")
let pool = graph.maxPooling2D(
    withSourceTensor: relu,
    descriptor: MPSGraphPooling2DOpDescriptor(kernelWidth: 3, kernelHeight: 3, strideInX: 2, strideInY: 2, paddingStyle: .same, dataLayout: .NCHW)!,
    name: "pool1"
)

// Execute graph
let results = graph.run(
    with: commandQueue,
    feeds: [inputPlaceholder: inputData],
    targetTensors: [pool],
    targetOperations: nil
)"#.to_string(),
                    description: Some("Build and execute MPSGraph for neural network".to_string()),
                });
            }

            // MetalFX
            "MTLFXTemporalScaler" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Create MetalFX temporal upscaler
let descriptor = MTLFXTemporalScalerDescriptor()
descriptor.inputWidth = 1280
descriptor.inputHeight = 720
descriptor.outputWidth = 3840
descriptor.outputHeight = 2160
descriptor.colorTextureFormat = .rgba16Float
descriptor.depthTextureFormat = .depth32Float
descriptor.motionTextureFormat = .rg16Float
descriptor.isAutoExposureEnabled = true

guard let scaler = descriptor.makeTemporalScaler(device: device) else { return }

// Configure per-frame
scaler.colorTexture = inputColor
scaler.depthTexture = inputDepth
scaler.motionTexture = motionVectors
scaler.outputTexture = upscaledOutput
scaler.jitterOffsetX = jitterX  // From TAA jitter sequence
scaler.jitterOffsetY = jitterY
scaler.reset = false  // Set true on camera cut

// Encode upscaling
scaler.encode(commandBuffer: commandBuffer)"#.to_string(),
                    description: Some("MetalFX temporal upscaling for games".to_string()),
                });
            }

            // Ray Tracing
            "MTLAccelerationStructure" | "intersector" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Build acceleration structure for ray tracing
let geometryDesc = MTLAccelerationStructureTriangleGeometryDescriptor()
geometryDesc.vertexBuffer = vertexBuffer
geometryDesc.vertexStride = MemoryLayout<Float>.stride * 3
geometryDesc.indexBuffer = indexBuffer
geometryDesc.indexType = .uint32
geometryDesc.triangleCount = triangleCount

let structureDesc = MTLPrimitiveAccelerationStructureDescriptor()
structureDesc.geometryDescriptors = [geometryDesc]

// Get sizes and allocate
let sizes = device.accelerationStructureSizes(descriptor: structureDesc)
let accelerationStructure = device.makeAccelerationStructure(size: sizes.accelerationStructureSize)!
let scratchBuffer = device.makeBuffer(length: sizes.buildScratchBufferSize, options: .storageModePrivate)!

// Build acceleration structure
let buildEncoder = commandBuffer.makeAccelerationStructureCommandEncoder()!
buildEncoder.build(
    accelerationStructure: accelerationStructure,
    descriptor: structureDesc,
    scratchBuffer: scratchBuffer,
    scratchBufferOffset: 0
)
buildEncoder.endEncoding()"#.to_string(),
                    description: Some("Build acceleration structure for ray tracing".to_string()),
                });
                if method.name == "intersector" {
                    examples.push(MetalExample {
                        language: "metal".to_string(),
                        code: r#"// Ray tracing kernel in MSL
#include <metal_raytracing>

kernel void rayTraceKernel(
    primitive_acceleration_structure accelerationStructure [[buffer(0)]],
    device Ray* rays [[buffer(1)]],
    device Intersection* intersections [[buffer(2)]],
    uint tid [[thread_position_in_grid]]
) {
    ray r;
    r.origin = rays[tid].origin;
    r.direction = rays[tid].direction;
    r.min_distance = 0.001;
    r.max_distance = 1000.0;

    intersector<triangle_data> i;
    i.accept_any_intersection(false);

    auto intersection = i.intersect(r, accelerationStructure);

    if (intersection.type == intersection_type::triangle) {
        intersections[tid].distance = intersection.distance;
        intersections[tid].primitiveIndex = intersection.primitive_id;
        intersections[tid].barycentrics = intersection.triangle_barycentric_coord;
    } else {
        intersections[tid].distance = -1.0;
    }
}"#.to_string(),
                        description: Some("MSL ray intersection kernel".to_string()),
                    });
                }
            }

            // GPU Features
            "Apple_M1" | "Apple_M2" | "Apple_M3" | "Apple_M4" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Query Apple Silicon GPU capabilities
let device = MTLCreateSystemDefaultDevice()!

// Check GPU family support
if device.supportsFamily(.apple7) {  // M1+
    print("Apple7 features: mesh shaders, hardware ray tracing (M2+)")
}
if device.supportsFamily(.apple8) {  // M2+
    print("Apple8 features: hardware ray tracing, MetalFX")
}
if device.supportsFamily(.apple9) {  // M3+
    print("Apple9 features: dynamic caching, improved ray tracing")
}

// Check specific features
print("Unified Memory: \(device.hasUnifiedMemory)")
print("Ray Tracing: \(device.supportsRaytracing)")
print("32-bit MSAA: \(device.supports32BitMSAA)")
print("Raster Order Groups: \(device.areRasterOrderGroupsSupported)")

// Get limits
let maxThreadsPerThreadgroup = device.maxThreadsPerThreadgroup
print("Max threads per threadgroup: \(maxThreadsPerThreadgroup.width)")"#.to_string(),
                    description: Some("Query Apple Silicon GPU capabilities".to_string()),
                });
            }

            // Optimization
            "triple_buffering" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Triple buffering to hide CPU-GPU latency
class Renderer {
    static let maxFramesInFlight = 3
    private let frameSemaphore = DispatchSemaphore(value: maxFramesInFlight)
    private var currentBuffer = 0
    private var uniformBuffers: [MTLBuffer] = []

    init(device: MTLDevice) {
        // Create ring buffer of uniform buffers
        for _ in 0..<Self.maxFramesInFlight {
            let buffer = device.makeBuffer(length: MemoryLayout<Uniforms>.size, options: .storageModeShared)!
            uniformBuffers.append(buffer)
        }
    }

    func render() {
        // Wait if all buffers are in flight
        frameSemaphore.wait()

        // Update uniforms for current frame
        let uniforms = uniformBuffers[currentBuffer].contents().assumingMemoryBound(to: Uniforms.self)
        uniforms.pointee = calculateUniforms()

        // Create command buffer
        let commandBuffer = commandQueue.makeCommandBuffer()!

        // Release semaphore when GPU completes
        commandBuffer.addCompletedHandler { [weak self] _ in
            self?.frameSemaphore.signal()
        }

        // Encode and commit...

        // Rotate to next buffer
        currentBuffer = (currentBuffer + 1) % Self.maxFramesInFlight
    }
}"#.to_string(),
                    description: Some("Triple buffering for optimal CPU-GPU parallelism".to_string()),
                });
            }
            "argument_buffer" => {
                examples.push(MetalExample {
                    language: "swift".to_string(),
                    code: r#"// Bindless rendering with argument buffers
// Reduces setTexture/setBuffer calls dramatically

// Create argument encoder from function
let argumentEncoder = fragmentFunction.makeArgumentEncoder(bufferIndex: 0)
let argumentBuffer = device.makeBuffer(length: argumentEncoder.encodedLength, options: .storageModeShared)!
argumentEncoder.setArgumentBuffer(argumentBuffer, offset: 0)

// Encode textures
argumentEncoder.setTexture(albedoTexture, index: 0)
argumentEncoder.setTexture(normalTexture, index: 1)
argumentEncoder.setTexture(roughnessTexture, index: 2)
argumentEncoder.setSamplerState(sampler, index: 3)

// In render loop, bind once
encoder.setFragmentBuffer(argumentBuffer, offset: 0, index: 0)
encoder.useResources([albedoTexture, normalTexture, roughnessTexture], usage: .read, stages: .fragment)
encoder.drawIndexedPrimitives(...)"#.to_string(),
                    description: Some("Argument buffers for bindless rendering".to_string()),
                });
            }

            _ => {
                // Generic example based on kind
                match method.kind {
                    MetalItemKind::ShaderLanguage => {
                        examples.push(MetalExample {
                            language: "metal".to_string(),
                            code: format!(
                                "// See Metal Shading Language Specification for '{}'\n// https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf",
                                method.name
                            ),
                            description: Some(format!("MSL reference for {}", method.name)),
                        });
                    }
                    _ => {
                        examples.push(MetalExample {
                            language: "swift".to_string(),
                            code: format!(
                                "// See Apple Developer Documentation for {}\n// {}",
                                method.name, self.get_method_url(method)
                            ),
                            description: Some(format!("Metal API reference for {}", method.name)),
                        });
                    }
                }
            }
        }

        examples
    }

    /// Infer parameters for a method
    fn infer_parameters(&self, method: &MetalMethodIndex) -> Vec<MetalParameter> {
        match method.name {
            "makeBuffer" => vec![
                MetalParameter {
                    name: "length".to_string(),
                    param_type: "Int".to_string(),
                    required: true,
                    description: "Size of the buffer in bytes".to_string(),
                    default_value: None,
                },
                MetalParameter {
                    name: "options".to_string(),
                    param_type: "MTLResourceOptions".to_string(),
                    required: false,
                    description: "Storage mode and CPU cache mode. Use .storageModeShared for CPU/GPU access, .storageModePrivate for GPU-only".to_string(),
                    default_value: Some(".storageModeShared".to_string()),
                },
            ],
            "dispatchThreadgroups" => vec![
                MetalParameter {
                    name: "threadgroupsPerGrid".to_string(),
                    param_type: "MTLSize".to_string(),
                    required: true,
                    description: "Number of threadgroups in each dimension (x, y, z)".to_string(),
                    default_value: None,
                },
                MetalParameter {
                    name: "threadsPerThreadgroup".to_string(),
                    param_type: "MTLSize".to_string(),
                    required: true,
                    description: "Number of threads in each threadgroup dimension. Total must not exceed maxThreadsPerThreadgroup.".to_string(),
                    default_value: None,
                },
            ],
            "dispatchThreads" => vec![
                MetalParameter {
                    name: "threadsPerGrid".to_string(),
                    param_type: "MTLSize".to_string(),
                    required: true,
                    description: "Total number of threads to dispatch. Metal calculates optimal threadgroup distribution.".to_string(),
                    default_value: None,
                },
                MetalParameter {
                    name: "threadsPerThreadgroup".to_string(),
                    param_type: "MTLSize".to_string(),
                    required: true,
                    description: "Threadgroup size. Should be multiple of threadExecutionWidth (32 on Apple Silicon).".to_string(),
                    default_value: None,
                },
            ],
            _ => Vec::new(),
        }
    }

    /// Infer return type for a method
    fn infer_return_type(&self, method: &MetalMethodIndex) -> Option<MetalReturnType> {
        match method.kind {
            MetalItemKind::CoreType => {
                Some(MetalReturnType {
                    type_name: method.name.to_string(),
                    description: format!("{} protocol or class instance", method.name),
                    fields: vec![],
                })
            }
            _ => None,
        }
    }

    /// Get a specific method by name
    #[instrument(name = "metal_client.get_method", skip(self))]
    pub async fn get_method(&self, name: &str) -> Result<MetalMethod> {
        let index_entry = Self::all_methods()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("Metal method not found: {name}"))?;

        Ok(self.build_method_doc(index_entry))
    }

    /// Search for methods matching a query
    #[instrument(name = "metal_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<MetalMethod>> {
        let query_lower = query.to_lowercase();

        // Split query into keywords
        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, &MetalMethodIndex)> = Vec::new();

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

            // Boost for specific searches
            if (query_lower.contains("render") || query_lower.contains("draw")) &&
               method.category == "render" {
                score += 15;
            }
            if (query_lower.contains("compute") || query_lower.contains("kernel")) &&
               method.category == "compute" {
                score += 15;
            }
            if query_lower.contains("msl") && method.category == "msl" {
                score += 20;
            }
            if query_lower.contains("mps") && method.category == "mps" {
                score += 20;
            }
            if query_lower.contains("ray") && method.category == "raytracing" {
                score += 20;
            }
            if (query_lower.contains("m1") || query_lower.contains("m2") ||
                query_lower.contains("m3") || query_lower.contains("m4") ||
                query_lower.contains("apple silicon")) && method.category == "gpu" {
                score += 25;
            }

            if score > 0 {
                scored_results.push((score, method));
            }
        }

        // Sort by score (highest first)
        scored_results.sort_by(|a, b| b.0.cmp(&a.0));

        // Convert to MetalMethod
        let results: Vec<MetalMethod> = scored_results
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
        let _client = MetalClient::new();
    }

    #[test]
    fn test_all_methods_count() {
        let count = MetalClient::all_methods().count();
        assert!(count > 80, "Expected at least 80 methods, got {}", count);
    }

    #[test]
    fn test_categories() {
        let count = METAL_CORE_TYPES.len()
            + METAL_RESOURCES.len()
            + METAL_RENDER_PIPELINE.len()
            + METAL_COMPUTE_PIPELINE.len()
            + METAL_SHADING_LANGUAGE.len()
            + METAL_MPS.len()
            + METAL_MPSGRAPH.len()
            + METAL_METALFX.len()
            + METAL_RAYTRACING.len()
            + METAL_OPTIMIZATION.len()
            + METAL_GPU_FEATURES.len();
        assert!(count > 100, "Expected comprehensive coverage, got {}", count);
    }
}
