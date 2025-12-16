//! MLX documentation types for Apple Silicon machine learning.
//!
//! MLX is a machine learning framework from Apple's ML Research team,
//! optimized for Apple Silicon. This module covers both MLX (Python)
//! and MLX-Swift documentation.

use serde::{Deserialize, Serialize};

/// MLX technology/category representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub language: MlxLanguage,
}

/// Language variant for MLX
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlxLanguage {
    /// MLX-Swift for iOS/macOS development
    Swift,
    /// MLX Python for general ML development
    Python,
}

impl std::fmt::Display for MlxLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Swift => write!(f, "Swift"),
            Self::Python => write!(f, "Python"),
        }
    }
}

/// MLX category (e.g., Arrays, Neural Networks, Optimizers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<MlxCategoryItem>,
    pub language: MlxLanguage,
}

/// Item in an MLX category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: MlxItemKind,
    pub path: String,
    pub url: String,
}

/// Types of MLX documentation items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlxItemKind {
    /// Class (e.g., MLXArray, Module)
    Class,
    /// Function
    Function,
    /// Property
    Property,
    /// Protocol/Trait
    Protocol,
    /// Enum
    Enum,
    /// Type alias
    TypeAlias,
    /// Module/Namespace
    Module,
    /// Operator
    Operator,
    /// Extension
    Extension,
    /// Guide/Tutorial
    Guide,
}

impl std::fmt::Display for MlxItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Class => write!(f, "class"),
            Self::Function => write!(f, "function"),
            Self::Property => write!(f, "property"),
            Self::Protocol => write!(f, "protocol"),
            Self::Enum => write!(f, "enum"),
            Self::TypeAlias => write!(f, "typealias"),
            Self::Module => write!(f, "module"),
            Self::Operator => write!(f, "operator"),
            Self::Extension => write!(f, "extension"),
            Self::Guide => write!(f, "guide"),
        }
    }
}

/// Full MLX documentation article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxArticle {
    pub title: String,
    pub description: String,
    pub path: String,
    pub url: String,
    pub kind: MlxItemKind,
    pub language: MlxLanguage,
    /// Declaration/signature
    pub declaration: Option<String>,
    /// Full documentation content
    pub content: String,
    /// Code examples
    pub examples: Vec<MlxExample>,
    /// Parameters (for functions/methods)
    pub parameters: Vec<MlxParameter>,
    /// Return type/value description
    pub return_value: Option<String>,
    /// Related APIs
    pub related: Vec<String>,
    /// Platform availability (for Swift)
    pub platforms: Vec<String>,
}

/// Code example in MLX documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxExample {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

/// Parameter documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxParameter {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
}

/// Search result from MLX documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxSearchResult {
    pub name: String,
    pub path: String,
    pub url: String,
    pub kind: MlxItemKind,
    pub description: String,
    pub language: MlxLanguage,
    /// Relevance score
    pub score: i32,
}

/// MLX-Swift predefined topics for search index
pub const MLX_SWIFT_TOPICS: &[(&str, &str, &str)] = &[
    // Core Types
    ("MLXArray", "mlx/mlxarray", "The core array type - a multi-dimensional array with lazy evaluation on Apple Silicon"),
    ("DType", "mlx/dtype", "Data types for MLX arrays (float32, float16, bfloat16, int32, etc.)"),
    ("Device", "mlx/device", "Device abstraction for CPU and GPU computation"),
    ("Stream", "mlx/stream", "Stream for controlling execution order and synchronization"),

    // Array Operations
    ("zeros", "mlx/zeros(_:dtype:stream:)", "Create an array filled with zeros"),
    ("ones", "mlx/ones(_:dtype:stream:)", "Create an array filled with ones"),
    ("full", "mlx/full(_:values:dtype:stream:)", "Create an array filled with a constant value"),
    ("arange", "mlx/arange(_:_:_:dtype:stream:)", "Create an array with evenly spaced values"),
    ("linspace", "mlx/linspace(_:_:count:dtype:stream:)", "Create an array with evenly spaced values over a specified interval"),
    ("reshape", "mlxarray/reshaped(_:stream:)", "Reshape an array to a new shape"),
    ("transpose", "mlxarray/transposed(axes:stream:)", "Transpose array dimensions"),
    ("concatenate", "mlx/concatenated(_:axis:stream:)", "Join arrays along an axis"),
    ("split", "mlx/split(_:parts:axis:stream:)", "Split an array into multiple sub-arrays"),
    ("stack", "mlx/stacked(_:axis:stream:)", "Stack arrays along a new axis"),

    // Math Operations
    ("matmul", "mlx/matmul(_:_:stream:)", "Matrix multiplication"),
    ("conv1d", "mlx/conv1d(_:_:stride:padding:dilation:groups:stream:)", "1D convolution"),
    ("conv2d", "mlx/conv2d(_:_:stride:padding:dilation:groups:stream:)", "2D convolution"),
    ("softmax", "mlx/softmax(_:axis:stream:)", "Softmax activation function"),
    ("relu", "mlx/relu(_:stream:)", "Rectified linear unit activation"),
    ("gelu", "mlx/gelu(_:stream:)", "Gaussian Error Linear Unit activation"),
    ("sigmoid", "mlx/sigmoid(_:stream:)", "Sigmoid activation function"),
    ("tanh", "mlx/tanh(_:stream:)", "Hyperbolic tangent activation"),

    // Neural Network Modules
    ("Module", "mlx/module", "Base class for all neural network modules"),
    ("Linear", "mlx/linear", "Linear (fully connected) layer"),
    ("Conv1d", "mlx/conv1d-class", "1D convolutional layer"),
    ("Conv2d", "mlx/conv2d-class", "2D convolutional layer"),
    ("Embedding", "mlx/embedding", "Embedding layer for token lookups"),
    ("LayerNorm", "mlx/layernorm", "Layer normalization"),
    ("RMSNorm", "mlx/rmsnorm", "Root Mean Square normalization"),
    ("MultiHeadAttention", "mlx/multiheadattention", "Multi-head attention mechanism"),
    ("Transformer", "mlx/transformer", "Transformer architecture components"),
    ("LSTM", "mlx/lstm", "Long Short-Term Memory layer"),
    ("GRU", "mlx/gru", "Gated Recurrent Unit layer"),
    ("Dropout", "mlx/dropout", "Dropout regularization layer"),
    ("BatchNorm", "mlx/batchnorm", "Batch normalization layer"),

    // Optimizers
    ("SGD", "mlx/sgd", "Stochastic Gradient Descent optimizer"),
    ("Adam", "mlx/adam", "Adam optimizer"),
    ("AdamW", "mlx/adamw", "AdamW optimizer with weight decay"),
    ("Adagrad", "mlx/adagrad", "Adagrad optimizer"),
    ("RMSprop", "mlx/rmsprop", "RMSprop optimizer"),
    ("Lion", "mlx/lion", "Lion optimizer"),

    // Loss Functions
    ("crossEntropy", "mlx/crossentropy(_:_:weights:axis:labelsmoothing:reduction:stream:)", "Cross-entropy loss"),
    ("mseLoss", "mlx/mseloss(_:_:reduction:stream:)", "Mean squared error loss"),
    ("binaryCrossEntropy", "mlx/binarycrossentropy(_:_:weights:reduction:stream:)", "Binary cross-entropy loss"),
    ("klDivLoss", "mlx/kldivloss(_:_:axis:reduction:stream:)", "KL divergence loss"),

    // Random
    ("RandomState", "mlx/randomstate", "Random number generator state"),
    ("normal", "mlx/normal(_:_:dtype:stream:)", "Generate normally distributed random numbers"),
    ("uniform", "mlx/uniform(_:_:shape:dtype:stream:)", "Generate uniformly distributed random numbers"),

    // Compilation & Evaluation
    ("compile", "mlx/compile(_:inputs:outputs:)", "Compile a function for optimized execution"),
    ("eval", "mlx/eval(_:)-1094l", "Evaluate arrays, triggering computation"),
    ("grad", "mlx/grad(_:)", "Compute gradients of a function"),
    ("valueAndGrad", "mlx/valueandgrad(_:)", "Compute both value and gradients"),

    // LLM Specific
    ("generate", "llms/generate(prompt:parameters:)", "Generate text from a language model"),
    ("loadModel", "llms/loadmodel(_:)", "Load a pre-trained model"),
    ("Tokenizer", "llms/tokenizer", "Tokenizer for text encoding/decoding"),
    ("KVCache", "mlx/kvcache", "Key-value cache for efficient inference"),
    ("RotaryPositionalEncoding", "mlx/rotatypositionalencoding", "Rotary position embeddings (RoPE)"),
];

/// MLX Python topics for search index
pub const MLX_PYTHON_TOPICS: &[(&str, &str, &str)] = &[
    // Core
    ("mlx.core", "python/ops.html", "Core array operations and primitives"),
    ("mlx.core.array", "python/_autosummary/mlx.core.array.html", "The core array class"),
    ("mlx.core.zeros", "python/_autosummary/mlx.core.zeros.html", "Create array of zeros"),
    ("mlx.core.ones", "python/_autosummary/mlx.core.ones.html", "Create array of ones"),
    ("mlx.core.reshape", "python/_autosummary/mlx.core.reshape.html", "Reshape an array"),
    ("mlx.core.matmul", "python/_autosummary/mlx.core.matmul.html", "Matrix multiplication"),
    ("mlx.core.compile", "python/_autosummary/mlx.core.compile.html", "JIT compile a function"),
    ("mlx.core.eval", "python/_autosummary/mlx.core.eval.html", "Evaluate arrays"),
    ("mlx.core.grad", "python/_autosummary/mlx.core.grad.html", "Gradient computation"),
    ("mlx.core.softmax", "python/_autosummary/mlx.core.softmax.html", "Softmax function"),

    // Neural Networks
    ("mlx.nn", "python/nn.html", "Neural network module"),
    ("mlx.nn.Module", "python/nn/module.html", "Base neural network module"),
    ("mlx.nn.layers", "python/nn/layers.html", "Neural network layers"),
    ("mlx.nn.functions", "python/nn/functions.html", "Activation and functional APIs"),
    ("mlx.nn.Linear", "python/nn/_autosummary/mlx.nn.Linear.html", "Linear layer"),
    ("mlx.nn.Conv1d", "python/nn/_autosummary/mlx.nn.Conv1d.html", "1D convolution layer"),
    ("mlx.nn.Conv2d", "python/nn/_autosummary/mlx.nn.Conv2d.html", "2D convolution layer"),
    ("mlx.nn.Embedding", "python/nn/_autosummary/mlx.nn.Embedding.html", "Embedding layer"),
    ("mlx.nn.LayerNorm", "python/nn/_autosummary/mlx.nn.LayerNorm.html", "Layer normalization"),
    ("mlx.nn.RMSNorm", "python/nn/_autosummary/mlx.nn.RMSNorm.html", "RMS normalization"),
    ("mlx.nn.MultiHeadAttention", "python/nn/_autosummary/mlx.nn.MultiHeadAttention.html", "Multi-head attention"),
    ("mlx.nn.Transformer", "python/nn/_autosummary/mlx.nn.Transformer.html", "Transformer components"),
    ("mlx.nn.ReLU", "python/nn/_autosummary/mlx.nn.ReLU.html", "ReLU activation module"),
    ("mlx.nn.relu", "python/nn/_autosummary_functions/mlx.nn.relu.html", "ReLU activation function"),
    ("mlx.nn.Softmax", "python/nn/_autosummary/mlx.nn.Softmax.html", "Softmax activation module"),
    ("mlx.nn.softmax", "python/nn/_autosummary_functions/mlx.nn.softmax.html", "Softmax activation function"),
    ("mlx.nn.GELU", "python/nn/_autosummary/mlx.nn.GELU.html", "GELU activation module"),
    ("mlx.nn.gelu", "python/nn/_autosummary_functions/mlx.nn.gelu.html", "GELU activation function"),

    // Optimizers
    ("mlx.optimizers", "python/optimizers.html", "Optimizer implementations"),
    ("mlx.optimizers.SGD", "python/optimizers/_autosummary/mlx.optimizers.SGD.html", "SGD optimizer"),
    ("mlx.optimizers.Adam", "python/optimizers/_autosummary/mlx.optimizers.Adam.html", "Adam optimizer"),
    ("mlx.optimizers.AdamW", "python/optimizers/_autosummary/mlx.optimizers.AdamW.html", "AdamW optimizer"),

    // LLM
    ("mlx_lm.load", "llm/load", "Load LLM model and tokenizer"),
    ("mlx_lm.generate", "llm/generate", "Generate text with LLM"),
    ("mlx_lm.convert", "llm/convert", "Convert models to MLX format"),
    ("mlx_lm.quantize", "llm/quantize", "Quantize models for efficiency"),
];
