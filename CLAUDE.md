# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Apple Doc MCP is a Model Context Protocol (MCP) server written in Rust that provides access to developer documentation from multiple providers. It enables AI coding assistants to search, browse, and retrieve official documentation for:
- **Apple**: SwiftUI, UIKit, Foundation, CoreML, Vision, and 60+ frameworks (including ML/AI)
- **Telegram**: Bot API methods and types
- **TON**: Comprehensive blockchain documentation including:
  - TON API (tonapi.io REST endpoints)
  - Smart contract languages (Tact, FunC, Tolk, Fift)
  - Security best practices and vulnerability patterns
  - Token standards (Jettons, NFTs, SBTs)
  - TVM (TON Virtual Machine) documentation
  - Wallet contracts and TON Connect
- **Cocoon**: Confidential computing documentation
- **Rust**: Standard library (std, core, alloc) and any crate from docs.rs
- **MDN**: JavaScript, TypeScript, Web APIs, DOM documentation
- **Web Frameworks**: React, Next.js, Node.js, and Bun documentation
- **Bun**: Fast all-in-one JavaScript runtime with bundler, transpiler, test runner, and package manager
- **MLX**: Apple's machine learning framework for Apple Silicon (Swift and Python)
- **Hugging Face**: Transformers library and swift-transformers for LLM/AI development
- **QuickNode**: Solana blockchain RPC documentation (HTTP methods, WebSocket, Marketplace add-ons)
- **Claude Agent SDK**: TypeScript and Python SDKs for building AI agents with Claude Code capabilities
- **Vertcoin**: GPU-mineable cryptocurrency with Verthash algorithm, JSON-RPC API documentation
- **CUDA**: NVIDIA GPU programming (Runtime API, kernel development, cuBLAS, cuDNN) with RTX 3070/4090 specific documentation
- **Metal**: Apple GPU programming API (MTLDevice, render/compute pipelines, MSL shaders, MPS, MetalFX)
- **Game Development**: SpriteKit (2D), SceneKit (3D), RealityKit (AR/VR), GameKit (multiplayer), GameController, GameplayKit (AI)

## Build Commands

```bash
# Build (requires Rust 1.76+)
cargo build --release

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p docs-mcp-core

# Run the server (for local development)
cargo run -p docs-mcp-cli

# Lint
cargo clippy --all-targets
```

## Architecture

### Workspace Structure

```
├── apps/cli/                    # CLI entry point (docs-mcp-cli binary)
├── crates/
│   ├── docs-mcp-client/       # HTTP client for Apple's documentation API
│   ├── docs-mcp-core/         # Core logic: tools, state, services, transport
│   ├── docs-mcp/          # MCP protocol bootstrap and config resolution
│   └── multi-provider-client/   # Clients for Telegram, TON, Cocoon, Rust, MDN, Web Frameworks, MLX, Hugging Face, QuickNode, Claude Agent SDK, Vertcoin, and CUDA APIs
│       ├── src/
│       │   ├── telegram/        # Telegram Bot API client
│       │   ├── ton/             # TON blockchain API client
│       │   ├── cocoon/          # Cocoon confidential computing client
│       │   ├── rust/            # Rust documentation client (std + docs.rs)
│       │   ├── mdn/             # MDN Web Docs client (JavaScript, Web APIs)
│       │   ├── web_frameworks/  # React, Next.js, Node.js, Bun documentation client
│       │   ├── mlx/             # MLX ML framework client (Swift + Python)
│       │   ├── huggingface/     # Hugging Face transformers client
│       │   ├── quicknode/       # QuickNode Solana RPC documentation client
│       │   ├── claude_agent_sdk/# Claude Agent SDK client (TypeScript + Python)
│       │   ├── vertcoin/        # Vertcoin blockchain RPC documentation client
│       │   ├── cuda/            # CUDA GPU programming documentation client
│       │   ├── metal/           # Metal GPU programming documentation client
│       │   ├── gamedev/         # Game development frameworks client (SpriteKit, SceneKit, etc.)
│       │   ├── types.rs         # Unified types across all providers
│       │   └── lib.rs           # ProviderClients aggregation
```

### Crate Responsibilities

- **docs-mcp-client**: Fetches and caches documentation from `developer.apple.com/tutorials/data`. Uses two-tier caching (memory TTL + disk persistence). Key types: `AppleDocsClient`, `Technology`, `FrameworkData`, `SymbolData`.

- **docs-mcp-core**: Contains all MCP tool implementations, application state (`AppContext`, `ServerState`), and the stdio transport layer. Tools are registered via `tools::register_tools()`.

- **docs-mcp**: Thin wrapper that resolves environment config (`DOCSMCP_CACHE_DIR`, `DOCSMCP_HEADLESS`) and launches the core server.

- **multi-provider-client**: HTTP clients for non-Apple documentation providers:
  - `TelegramClient`: Telegram Bot API methods and types from `core.telegram.org`
  - `TonClient`: TON blockchain documentation with unified search across:
    - tonapi.io REST API endpoints (OpenAPI spec)
    - Security patterns and best practices (embedded knowledge base)
    - Smart contract documentation (Tact, FunC, Tolk)
    - Token standards (Jettons, NFTs, TON Connect)
  - `CocoonClient`: Cocoon documentation from `cocoon.org`
  - `RustClient`: Rust std library + any crate from `docs.rs`
  - `MdnClient`: JavaScript, TypeScript, Web APIs from `developer.mozilla.org`
  - `WebFrameworksClient`: React, Next.js, Node.js, Bun documentation with example extraction
  - `MlxClient`: MLX ML framework documentation (Swift DocC + Python Sphinx) from `ml-explore.github.io`
  - `HuggingFaceClient`: Transformers and swift-transformers documentation from `huggingface.co`
  - `QuickNodeClient`: Solana RPC documentation from `quicknode.com/docs/solana`
  - `ClaudeAgentSdkClient`: Claude Agent SDK documentation for TypeScript and Python from `docs.anthropic.com`
  - `VertcoinClient`: Vertcoin blockchain RPC documentation with Verthash mining support
  - `CudaClient`: CUDA GPU programming documentation (Runtime API, kernels, libraries) with RTX 3070/4090 specs
  - `MetalClient`: Metal GPU programming documentation (MTLDevice, pipelines, MSL, MPS, MetalFX, ray tracing)
  - `GameDevClient`: Game development frameworks (SpriteKit, SceneKit, RealityKit, GameKit, GameController, GameplayKit)

### Provider Architecture

All providers implement a consistent interface through unified types:

```rust
pub enum ProviderType {
    Apple,
    Telegram,
    TON,
    Cocoon,
    Rust,
    Mdn,
    WebFrameworks,
    Vertcoin,
    Mlx,
    HuggingFace,
    QuickNode,
    ClaudeAgentSdk,
    Cuda,
    Metal,
    GameDev,
}

pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
    pub rust: RustClient,
    pub mdn: MdnClient,
    pub web_frameworks: WebFrameworksClient,
    pub mlx: MlxClient,
    pub huggingface: HuggingFaceClient,
    pub quicknode: QuickNodeClient,
    pub claude_agent_sdk: ClaudeAgentSdkClient,
    pub vertcoin: VertcoinClient,
    pub cuda: CudaClient,
    pub metal: MetalClient,
    pub gamedev: GameDevClient,
}
```

Each tool dispatches to the appropriate provider based on `active_provider` state.

### MCP Tools

**Single unified tool** exposed via MCP (`crates/docs-mcp-core/src/tools/query.rs`):

| Tool | Purpose | Key Features |
|------|---------|--------------|
| `query` | Unified documentation search engine | • Natural language query parsing<br>• Automatic provider/technology detection<br>• Intelligent search with synonym expansion<br>• Returns structured context with code samples<br>• Combines search + documentation fetching |

The `query` tool acts as an intelligent entry point that:
1. Parses natural language queries to extract intent (how-to, reference, search)
2. Auto-detects the appropriate provider (Apple, Telegram, TON, Rust, Cocoon, MDN, Web Frameworks, MLX, Hugging Face, QuickNode, Claude Agent SDK, Vertcoin, CUDA)
3. Auto-selects the relevant technology/framework
4. Executes optimized search across the detected provider
5. Fetches detailed documentation for top results
6. Returns structured, AI-ready context with usage examples

**Legacy tools** (`discover_technologies`, `choose_technology`, `search_symbols`, etc.) remain in the codebase for reference but are not exposed via MCP.

### Provider-Specific Features

#### Apple
- 50+ frameworks (SwiftUI, UIKit, Foundation, etc.)
- Platform availability info (iOS, macOS, watchOS, tvOS)
- Design guidance from Human Interface Guidelines
- Knowledge base overlays with tips and related APIs

#### Telegram
- Bot API methods (sendMessage, getUpdates, etc.)
- Type definitions (Update, Message, User, etc.)
- Parameter documentation with required/optional flags

#### TON (Telegram Open Network)
- **TON API**: 100+ REST endpoints from tonapi.io (accounts, transactions, NFTs, jettons)
- **Smart Contract Languages**:
  - Tact: High-level TypeScript-like language with code examples
  - FunC: Low-level C-like language documentation
  - Tolk: Next-generation language replacing FunC
  - Fift: Stack-based assembler language
- **Security Best Practices** (embedded knowledge base):
  - Integer handling (overflow/underflow protection)
  - Message handling and replay attack prevention
  - Gas management and unbounded loop patterns
  - Access control and authorization
  - Secure randomness and race condition avoidance
  - Code upgrade vulnerability patterns
  - Front-running protection
- **Token Standards**:
  - Jettons (TEP-74): Fungible tokens with transfer patterns
  - NFT (TEP-62): Non-fungible token documentation
  - SBT (TEP-85): Soul Bound Token standard
- **TVM (TON Virtual Machine)**: Opcodes, instruction categories, gas costs
- **Wallet Documentation**: v3r2, v4r2, v5 wallet versions and features
- **TON Connect**: dApp wallet connection protocol with TypeScript examples
- **Development Tools**: Blueprint framework, Sandbox testing

#### Cocoon
- Confidential computing documentation
- Architecture and TDX sections
- Smart contract documentation

#### Rust
- Standard library: std, core, alloc
- Dynamic crate loading from docs.rs
- Search index parsing from rustdoc
- Module and symbol documentation

#### MDN Web Docs
- JavaScript core language features (Array, Object, Promise, etc.)
- Web APIs (DOM, Fetch, WebSocket, Canvas, etc.)
- TypeScript type documentation
- Code examples with quality scoring

#### Web Frameworks
- **React**: Hooks (useState, useEffect, etc.), components, patterns
- **Next.js**: App Router, Server Components, API routes
- **Node.js**: Core modules (fs, path, http, crypto, stream)
- **Bun**: Runtime APIs, HTTP server, WebSocket, SQLite, bundler, test runner
- Usage examples prioritized by completeness and runnability

#### Bun Runtime
- **Runtime APIs**: Bun.serve, Bun.file, Bun.write, Bun.spawn, Bun.sleep, Bun.env
- **HTTP/WebSocket**: Fast HTTP server with WebSocket pub/sub support
- **File I/O**: BunFile with streaming, lazy loading, and efficient writes
- **Database**: Built-in SQLite driver with prepared statements and transactions
- **Bundler**: Bun.build with code splitting, minification, and plugin support
- **Test Runner**: bun test with Jest-compatible API, mocking, and snapshots
- **Package Manager**: Fast npm-compatible package manager with bun.lockb
- **FFI**: Foreign Function Interface for calling native libraries
- **Crypto**: Password hashing (bcrypt/argon2), cryptographic hashers
- **Node.js Compatibility**: Compatible with most Node.js APIs

#### MLX (Apple Silicon ML)
- **MLX Swift**: MLXArray, MLXRandom, MLXLinalg, MLXNN, MLXFFT, MLXOptimizers
- **MLX Python**: mlx.core, mlx.nn, mlx.optimizers, operations
- Apple Silicon optimized machine learning primitives
- Documentation from ml-explore.github.io (DocC for Swift, Sphinx for Python)

#### Hugging Face
- **Transformers**: AutoModel, AutoTokenizer, pipeline, training utilities
- **swift-transformers**: Hub, Models, Tokenizers for iOS/macOS
- LLM model families: GPT, LLaMA, BERT, T5, Whisper, CLIP, etc.
- Model card documentation and usage patterns

#### QuickNode (Solana)
- **HTTP Methods**: 50+ JSON-RPC methods (getAccountInfo, getBalance, sendTransaction, etc.)
- **WebSocket Methods**: Real-time subscriptions (accountSubscribe, logsSubscribe, etc.)
- **Marketplace Add-ons**: JITO bundles, Metaplex DAS API, Yellowstone gRPC
- Solana-specific documentation with code examples

#### Claude Agent SDK
- **TypeScript SDK**: ClaudeClient, query function, ClaudeAgentOptions, hooks, MCP servers
- **Python SDK**: ClaudeSDKClient, @tool decorator, async context manager, hooks
- Core concepts: query, hooks (PreToolUse, PostToolUse), MCP server integration
- Configuration: systemPrompt, maxTurns, allowedTools, permissionMode
- Message types: AssistantMessage, UserMessage, SystemMessage, ResultMessage
- Content blocks: TextBlock, ToolUseBlock, ToolResultBlock
- Authentication: ANTHROPIC_API_KEY, Bedrock, Vertex AI

#### Vertcoin
- **Blockchain RPC**: 80+ JSON-RPC methods (getblockchaininfo, getbalance, sendtoaddress, etc.)
- **Wallet Methods**: Address management, transaction signing, wallet encryption
- **Mining (Verthash)**: GPU-optimized ASIC-resistant mining documentation
- **Network**: Peer management, node discovery, ban lists
- **Specifications**: 2.5 minute block time, 84M supply, SegWit, Kimoto Gravity Well difficulty
- CLI examples with vertcoin-cli and JSON-RPC curl commands

#### CUDA (NVIDIA GPU Programming)
- **Runtime API**: 50+ functions for memory management (cudaMalloc, cudaMemcpy, cudaFree)
- **Device Management**: cudaGetDeviceProperties, cudaSetDevice, cudaDeviceSynchronize
- **Kernel Programming**: __global__, __device__, __shared__, __constant__ qualifiers
- **Thread Indexing**: threadIdx, blockIdx, blockDim, gridDim built-in variables
- **Synchronization**: __syncthreads, __syncwarp, atomic operations (atomicAdd, atomicCAS)
- **Warp Primitives**: __shfl_sync, __ballot_sync for efficient parallel reductions
- **Streams & Events**: Asynchronous execution and GPU timing
- **Libraries**: cuBLAS (GEMM), cuDNN (convolutions), cuFFT, cuRAND, NCCL (multi-GPU)
- **RTX 3070**: GA104, Compute 8.6, 5888 CUDA cores, 8GB GDDR6, 4MB L2 cache
- **RTX 4090**: AD102, Compute 8.9, 16384 CUDA cores, 24GB GDDR6X, 72MB L2 cache
- **Optimization**: Memory coalescing, occupancy, warp divergence, grid-stride loops, Tensor Cores
- Comprehensive code examples for kernel development

#### Metal (Apple GPU Programming)
- **Core Types**: MTLDevice, MTLCommandQueue, MTLCommandBuffer, MTLCommandEncoder
- **Resources**: MTLBuffer, MTLTexture, MTLSampler with shared/managed/private storage modes
- **Render Pipeline**: MTLRenderPipelineState, MTLRenderPassDescriptor, vertex/fragment shaders
- **Compute Pipeline**: MTLComputePipelineState, MTLComputeCommandEncoder, threadgroup memory
- **MSL (Metal Shading Language)**: Vertex/fragment/kernel functions, data types, attributes
- **MPS (Metal Performance Shaders)**: Neural networks, image processing, matrix operations
- **MPSGraph**: Graph-based ML operations with automatic differentiation
- **MetalFX**: Temporal and spatial upscaling for performance optimization
- **Ray Tracing**: Acceleration structures, intersection functions, ray queries
- **GPU Features**: Tile shaders, indirect command buffers, argument buffers
- **Optimization**: Resource heaps, memory-less render targets, GPU profiling
- Code examples for all major Metal operations

#### Game Development Frameworks
- **SpriteKit (2D Games)**:
  - Core: SKScene, SKView, SKNode, SKSpriteNode, SKLabelNode
  - Actions: SKAction sequences, groups, repeats, timing functions
  - Physics: SKPhysicsBody, SKPhysicsWorld, collision detection, joints
  - Effects: Particle emitters, shaders, blend modes, lighting
- **SceneKit (3D Games)**:
  - Core: SCNScene, SCNView, SCNNode, SCNCamera, SCNLight
  - Materials: SCNMaterial, shaders, PBR rendering, environment maps
  - Animation: SCNAnimation, physics, morph targets
  - Physics: SCNPhysicsBody, SCNPhysicsWorld, vehicle physics
- **RealityKit (AR/VR)**:
  - Core: ARView, Entity, AnchorEntity, ModelEntity
  - Components: Transform, ModelComponent, PhysicsBody, Collision
  - AR Features: Scene understanding, occlusion, face tracking
  - visionOS: Immersive spaces, hand tracking, eye tracking
- **GameKit (Multiplayer)**:
  - Core: GKLocalPlayer, GKPlayer, authentication
  - Matchmaking: GKMatch, GKMatchRequest, real-time/turn-based
  - Leaderboards: GKLeaderboard, score submission
  - Achievements: GKAchievement, progress tracking
- **GameController**:
  - Controller types: GCController, GCExtendedGamepad, GCMicroGamepad
  - Input handling: Button presses, thumbstick movement, triggers
  - Haptics: GCDeviceHaptics for vibration feedback
- **GameplayKit (AI & Logic)**:
  - Entities: GKEntity, GKComponent, component system
  - State machines: GKStateMachine, GKState transitions
  - Agents: GKAgent, behaviors, goals, obstacles
  - Pathfinding: GKGraph, GKGridGraph, navigation meshes
  - Randomization: GKRandomSource, deterministic sequences
- Platform availability and code examples for all frameworks

### Unified Query Tool Features

The `query` tool implements advanced natural language processing:

#### Natural Language Query Parsing
Automatically extracts intent, provider, technology, and keywords from queries:
```rust
// Example: "how to use SwiftUI NavigationStack"
QueryIntent {
    query_type: HowTo,
    provider: Some(Apple),
    technology: Some("swiftui"),
    keywords: ["navigationstack"]
}
```

#### Provider Auto-Detection
Intelligently detects the target provider from query context:
- **Apple**: SwiftUI, UIKit, iOS, macOS keywords + 60+ framework names (including CoreML, Vision, NaturalLanguage)
- **Rust**: std, tokio, serde, and other popular crate names
- **Telegram**: bot, sendmessage, telegram, webhook keywords
- **TON**: ton, tact, func, tolk, jetton, tvm, cell, boc, tonconnect, security, blueprint, seqno keywords
- **Cocoon**: confidential computing, TDX keywords
- **MDN**: javascript, js, dom, fetch, promise, array, web, browser keywords
- **React**: react, jsx, hook, usestate, useeffect, component keywords
- **Next.js**: nextjs, next, approuter, servercomponent keywords
- **Bun**: bun, bunjs, bun.serve, bun.file, bunx, bun.spawn, bun:sqlite, bun:test, bunfig keywords
- **Node.js**: nodejs, node, fs, path, http, stream keywords
- **MLX**: mlx, mlxarray, mlxnn, apple silicon, ml-explore keywords
- **Hugging Face**: huggingface, transformers, automodel, autotokenizer, swift-transformers keywords
- **QuickNode**: quicknode, solana, getaccountinfo, getbalance, lamports, pubkey keywords
- **Claude Agent SDK**: claude, agent sdk, claudeagentsdk, claudesdkclient, query, mcp, hooks keywords
- **Vertcoin**: vertcoin, vtc, verthash, vertcoin-cli, getblockchaininfo, getnewaddress, one click miner keywords
- **CUDA**: cuda, nvcc, cudamalloc, cudamemcpy, __global__, __shared__, cublas, cudnn, rtx 3070, rtx 4090, tensor cores keywords
- **Metal**: metal, mtldevice, mtlbuffer, mtlcommandqueue, msl, mps, mpsgraph, metalfx, render pipeline, compute kernel keywords
- **GameDev**: spritekit, scenekit, realitykit, gamekit, gamecontroller, gameplaykit, skscene, sknode, scnmaterial, gkmatch, gccontroller, 2d game, 3d game, physics simulation keywords

#### Query Type Classification
Three query types with specialized handling:
- **HowTo**: Recipes and guided steps with knowledge base tips
- **Reference**: Detailed documentation with code samples
- **Search**: General symbol search with synonym expansion

#### Tool Use Examples
The query tool includes diverse usage examples for natural language understanding:
```rust
input_examples: Some(vec![
    json!({"query": "how to use SwiftUI NavigationStack"}),
    json!({"query": "Rust tokio async task spawning"}),
    json!({"query": "Telegram Bot API sendMessage parameters"}),
    json!({"query": "CoreData fetch request predicates", "maxResults": 5}),
    json!({"query": "JavaScript Array map filter"}),
    json!({"query": "React useState hook"}),
    json!({"query": "Next.js server components"}),
    json!({"query": "Node.js fs readFile"}),
    json!({"query": "Bun serve HTTP server"}),
    json!({"query": "Bun.file read write"}),
    json!({"query": "Bun SQLite database"}),
    json!({"query": "MLX array operations Swift"}),
    json!({"query": "Hugging Face AutoModel from_pretrained"}),
    json!({"query": "Solana getAccountInfo"}),
    json!({"query": "QuickNode getBalance"}),
    json!({"query": "Claude Agent SDK query function typescript"}),
    json!({"query": "agent sdk python ClaudeSDKClient"}),
    json!({"query": "Vertcoin getblockchaininfo"}),
    json!({"query": "Verthash mining algorithm"}),
    json!({"query": "vertcoin-cli sendtoaddress"}),
    json!({"query": "CUDA cudaMalloc cudaMemcpy"}),
    json!({"query": "CUDA __global__ kernel example"}),
    json!({"query": "RTX 4090 specs"}),
    json!({"query": "cuBLAS matrix multiplication"}),
    json!({"query": "Metal MTLDevice creation"}),
    json!({"query": "Metal render pipeline state"}),
    json!({"query": "MSL shader function"}),
    json!({"query": "MPS neural network"}),
    json!({"query": "SpriteKit SKScene setup"}),
    json!({"query": "SceneKit 3D materials"}),
    json!({"query": "RealityKit AR anchor"}),
    json!({"query": "GameKit multiplayer match"}),
    json!({"query": "GameController button input"}),
])
```

### ToolDefinition Structure

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub input_examples: Option<Vec<serde_json::Value>>,  // Usage demonstrations
    pub allowed_callers: Option<Vec<String>>,            // Programmatic calling
}
```

### State Flow

1. `AppContext` holds the `AppleDocsClient`, `ServerState`, `ToolRegistry`, and `ProviderClients`
2. `ServerState` tracks: active technology, active provider, framework cache, search indexes, telemetry
3. Tool handlers receive `Arc<AppContext>` and return `ToolResponse` with optional metadata

### Search System

The unified `query` tool uses sophisticated ranking (in `tools/query.rs`):
- **Token matching**: camelCase splitting and multi-token queries
- **Synonym expansion**: e.g., "list" → "table", "collection", "tableview"
- **Natural language parsing**: extracts intent (how-to, reference, search)
- **Provider auto-detection**: routes to appropriate search backend
- **Smart scoring**: exact title match (15pts), abstract match (5pts), token match (2pts)
- **Knowledge base overlays**: tips and design guidance for Apple symbols
- **Code sample extraction**: automatically fetches and includes example code
- **Related APIs**: surfaces 5 related symbols for context

### Caching Strategy

All providers use two-tier caching:

| Data Type | Memory TTL | Disk TTL |
|-----------|------------|----------|
| Apple frameworks | 30min | 24h |
| Telegram spec | 1h | 24h |
| TON OpenAPI | 1h | 24h |
| Cocoon docs | 1h | 24h |
| Rust std index | 24h | 7d |
| Rust crate metadata | 30min | 24h |
| MDN search index | 1h | 24h |
| MDN article content | 30min | 24h |
| React/Next.js docs | 1h | 24h |
| Node.js API index | 24h | 7d |
| Bun docs | 1h | 24h |
| MLX Swift docs | 1h | 24h |
| MLX Python docs | 1h | 24h |
| Hugging Face docs | 1h | 24h |
| QuickNode methods | 30min | 24h |
| Claude Agent SDK docs | 24h | 24h |
| Vertcoin RPC docs | 1h | 24h |

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `DOCSMCP_CACHE_DIR` | Override disk cache location |
| `DOCSMCP_HEADLESS` | Set to `1` or `true` to skip stdio transport (testing) |
| `RUST_LOG` | Control tracing output (e.g., `info`, `debug`) |

## Testing the MCP Server

```bash
# Test MCP handshake and tools/list
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}\n' | ./target/release/docs-mcp-cli

# Test unified query tool with Apple/SwiftUI
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"SwiftUI Button styling"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Rust
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"tokio spawn async task"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Telegram
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"telegram bot sendMessage"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test how-to query
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"how to use SwiftUI NavigationStack"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with MDN (JavaScript)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"JavaScript Array map"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with React
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"React useState hook"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Next.js
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Next.js server components"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Node.js
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Node.js fs readFile"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Bun
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Bun serve HTTP server"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Bun SQLite
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Bun SQLite database"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with MLX (Apple Silicon ML)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"MLX array operations Swift"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Hugging Face
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Hugging Face AutoModel from_pretrained"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with TON (security best practices)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"TON security best practices"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with TON (smart contracts)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"tact"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with TON (wallet)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"TON wallet"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with TON (jetton transfer)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"jetton transfer"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with QuickNode (Solana)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Solana getAccountInfo"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Claude Agent SDK (TypeScript)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Claude Agent SDK query function"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Claude Agent SDK (Python)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"agent sdk python ClaudeSDKClient"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Vertcoin (RPC)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Vertcoin getblockchaininfo"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Vertcoin (Mining)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Verthash mining algorithm"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Vertcoin (Wallet)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"vertcoin-cli sendtoaddress"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with CUDA (Memory)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"cudaMalloc cudaMemcpy"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with CUDA (Kernel)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"CUDA __global__ __shared__ kernel"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with CUDA (RTX GPU specs)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"RTX 4090 specs CUDA cores"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with CUDA (Libraries)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"cuBLAS GEMM matrix multiplication"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Metal (Core)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Metal MTLDevice MTLCommandQueue"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Metal (Render Pipeline)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"Metal render pipeline state descriptor"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Metal (MSL Shader)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"MSL vertex fragment shader"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with Metal (MPS)
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"MPS neural network inference"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with SpriteKit
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"SpriteKit SKScene SKNode"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with SceneKit
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"SceneKit SCNMaterial PBR"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with RealityKit
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"RealityKit Entity AnchorEntity"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with GameKit
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"GameKit multiplayer matchmaking"}},"id":3}\n' | ./target/release/docs-mcp-cli

# Test query with GameController
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"query","arguments":{"query":"GameController GCController input"}},"id":3}\n' | ./target/release/docs-mcp-cli
```

## Adding a New Provider

1. Create a new module in `crates/multi-provider-client/src/<provider>/`
   - `mod.rs` - exports
   - `types.rs` - provider-specific data models
   - `client.rs` - HTTP client with caching implementing `search()` and `get_documentation()` methods

2. Update `crates/multi-provider-client/src/types.rs`:
   - Add variant to `ProviderType` enum
   - Add variant to `TechnologyKind` enum
   - Add variant to `SymbolContent` enum (if needed)
   - Add `from_<provider>()` conversion methods

3. Update `crates/multi-provider-client/src/lib.rs`:
   - Add client to `ProviderClients` struct
   - Initialize client in `ProviderClients::new()`

4. Update the unified query tool in `crates/docs-mcp-core/src/tools/query.rs`:
   - Add provider keywords to detection lists (`<PROVIDER>_KEYWORDS`)
   - Update `detect_provider_and_technology()` to detect your provider
   - Add `search_<provider>()` function implementing search logic
   - Add match arm in `execute_search_query()` to route to your search function
   - Update `resolve_technology()` to handle technology selection for your provider

## Maintenance Protocol

**IMPORTANT**: Review and update `agents.md` before finishing any session. It contains the retrieval enhancement roadmap and phase completion status.
