//! Hugging Face documentation types for LLM and ML model documentation.
//!
//! Provides access to Hugging Face model documentation, transformers library,
//! and swift-transformers for iOS/macOS development.

use serde::{Deserialize, Serialize};

/// Hugging Face technology/category representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub kind: HfTechnologyKind,
}

/// Types of HF technologies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HfTechnologyKind {
    /// Transformers Python library
    Transformers,
    /// Swift Transformers for iOS/macOS
    SwiftTransformers,
    /// Model Hub
    Models,
    /// Datasets
    Datasets,
    /// Tokenizers library
    Tokenizers,
    /// Diffusers library
    Diffusers,
    /// PEFT (Parameter-Efficient Fine-Tuning)
    Peft,
    /// Hub Python library
    Hub,
}

impl std::fmt::Display for HfTechnologyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transformers => write!(f, "transformers"),
            Self::SwiftTransformers => write!(f, "swift-transformers"),
            Self::Models => write!(f, "models"),
            Self::Datasets => write!(f, "datasets"),
            Self::Tokenizers => write!(f, "tokenizers"),
            Self::Diffusers => write!(f, "diffusers"),
            Self::Peft => write!(f, "peft"),
            Self::Hub => write!(f, "hub"),
        }
    }
}

/// Hugging Face category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<HfCategoryItem>,
    pub kind: HfTechnologyKind,
}

/// Item in a HF category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: HfItemKind,
    pub path: String,
    pub url: String,
}

/// Types of HF documentation items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HfItemKind {
    /// Model (e.g., Llama, Mistral)
    Model,
    /// Class (e.g., AutoModel, Pipeline)
    Class,
    /// Function
    Function,
    /// Configuration
    Config,
    /// Tokenizer
    Tokenizer,
    /// Trainer
    Trainer,
    /// Pipeline task
    Pipeline,
    /// Guide/Tutorial
    Guide,
    /// Dataset
    Dataset,
}

impl std::fmt::Display for HfItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model => write!(f, "model"),
            Self::Class => write!(f, "class"),
            Self::Function => write!(f, "function"),
            Self::Config => write!(f, "config"),
            Self::Tokenizer => write!(f, "tokenizer"),
            Self::Trainer => write!(f, "trainer"),
            Self::Pipeline => write!(f, "pipeline"),
            Self::Guide => write!(f, "guide"),
            Self::Dataset => write!(f, "dataset"),
        }
    }
}

/// Full HF documentation article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfArticle {
    pub title: String,
    pub description: String,
    pub path: String,
    pub url: String,
    pub kind: HfItemKind,
    pub technology: HfTechnologyKind,
    /// Declaration/signature
    pub declaration: Option<String>,
    /// Full documentation content
    pub content: String,
    /// Code examples
    pub examples: Vec<HfExample>,
    /// Parameters (for functions/classes)
    pub parameters: Vec<HfParameter>,
    /// Return type/value description
    pub return_value: Option<String>,
    /// Related items
    pub related: Vec<String>,
    /// Supported languages (Python, Swift, etc.)
    pub languages: Vec<String>,
}

/// Code example in HF documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfExample {
    pub code: String,
    pub language: String,
    pub description: Option<String>,
}

/// Parameter documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfParameter {
    pub name: String,
    pub description: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
}

/// Search result from HF documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSearchResult {
    pub name: String,
    pub path: String,
    pub url: String,
    pub kind: HfItemKind,
    pub technology: HfTechnologyKind,
    pub description: String,
    /// Relevance score
    pub score: i32,
}

/// Model info from Hugging Face Hub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModelInfo {
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub likes: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "pipeline_tag")]
    pub pipeline_tag: Option<String>,
    #[serde(default, rename = "library_name")]
    pub library_name: Option<String>,
}

/// Transformers library predefined topics
pub const TRANSFORMERS_TOPICS: &[(&str, &str, &str, HfItemKind)] = &[
    // AutoClasses
    ("AutoModel", "model_doc/auto#transformers.AutoModel", "Auto class for loading models automatically based on config", HfItemKind::Class),
    ("AutoTokenizer", "model_doc/auto#transformers.AutoTokenizer", "Auto class for loading tokenizers automatically", HfItemKind::Tokenizer),
    ("AutoConfig", "model_doc/auto#transformers.AutoConfig", "Auto class for loading model configurations", HfItemKind::Config),
    ("AutoModelForCausalLM", "model_doc/auto#transformers.AutoModelForCausalLM", "Auto class for causal language modeling (text generation)", HfItemKind::Class),
    ("AutoModelForSeq2SeqLM", "model_doc/auto#transformers.AutoModelForSeq2SeqLM", "Auto class for sequence-to-sequence language modeling", HfItemKind::Class),
    ("AutoModelForSequenceClassification", "model_doc/auto#transformers.AutoModelForSequenceClassification", "Auto class for sequence classification", HfItemKind::Class),
    ("AutoModelForTokenClassification", "model_doc/auto#transformers.AutoModelForTokenClassification", "Auto class for token classification (NER)", HfItemKind::Class),

    // Pipelines
    ("pipeline", "main_classes/pipelines#transformers.pipeline", "High-level API for inference with pretrained models", HfItemKind::Pipeline),
    ("TextGenerationPipeline", "main_classes/pipelines#transformers.TextGenerationPipeline", "Pipeline for text generation tasks", HfItemKind::Pipeline),
    ("TextClassificationPipeline", "main_classes/pipelines#transformers.TextClassificationPipeline", "Pipeline for text classification", HfItemKind::Pipeline),
    ("QuestionAnsweringPipeline", "main_classes/pipelines#transformers.QuestionAnsweringPipeline", "Pipeline for question answering", HfItemKind::Pipeline),
    ("SummarizationPipeline", "main_classes/pipelines#transformers.SummarizationPipeline", "Pipeline for text summarization", HfItemKind::Pipeline),
    ("TranslationPipeline", "main_classes/pipelines#transformers.TranslationPipeline", "Pipeline for translation", HfItemKind::Pipeline),
    ("ConversationalPipeline", "main_classes/pipelines#transformers.ConversationalPipeline", "Pipeline for conversational AI", HfItemKind::Pipeline),

    // Training
    ("Trainer", "main_classes/trainer#transformers.Trainer", "Training API for fine-tuning models", HfItemKind::Trainer),
    ("TrainingArguments", "main_classes/trainer#transformers.TrainingArguments", "Arguments for the Trainer", HfItemKind::Config),
    ("Seq2SeqTrainer", "main_classes/trainer#transformers.Seq2SeqTrainer", "Trainer for sequence-to-sequence models", HfItemKind::Trainer),

    // Tokenization
    ("PreTrainedTokenizer", "main_classes/tokenizer#transformers.PreTrainedTokenizer", "Base class for all tokenizers", HfItemKind::Tokenizer),
    ("PreTrainedTokenizerFast", "main_classes/tokenizer#transformers.PreTrainedTokenizerFast", "Fast tokenizer implementation", HfItemKind::Tokenizer),
    ("BatchEncoding", "main_classes/tokenizer#transformers.BatchEncoding", "Output of tokenizer encoding", HfItemKind::Class),

    // Configuration
    ("PretrainedConfig", "main_classes/configuration#transformers.PretrainedConfig", "Base class for model configurations", HfItemKind::Config),

    // Model Base
    ("PreTrainedModel", "main_classes/model#transformers.PreTrainedModel", "Base class for all models", HfItemKind::Class),
    ("GenerationMixin", "main_classes/text_generation#transformers.GenerationMixin", "Mixin for text generation methods", HfItemKind::Class),
    ("GenerationConfig", "main_classes/text_generation#transformers.GenerationConfig", "Configuration for text generation", HfItemKind::Config),

    // Popular Models
    ("LlamaModel", "model_doc/llama#transformers.LlamaModel", "LLaMA model for causal language modeling", HfItemKind::Model),
    ("LlamaForCausalLM", "model_doc/llama#transformers.LlamaForCausalLM", "LLaMA model with LM head", HfItemKind::Model),
    ("LlamaTokenizer", "model_doc/llama#transformers.LlamaTokenizer", "Tokenizer for LLaMA models", HfItemKind::Tokenizer),
    ("MistralModel", "model_doc/mistral#transformers.MistralModel", "Mistral model", HfItemKind::Model),
    ("MistralForCausalLM", "model_doc/mistral#transformers.MistralForCausalLM", "Mistral model with LM head", HfItemKind::Model),
    ("Qwen2Model", "model_doc/qwen2#transformers.Qwen2Model", "Qwen2 model", HfItemKind::Model),
    ("GemmaModel", "model_doc/gemma#transformers.GemmaModel", "Gemma model from Google", HfItemKind::Model),
    ("Phi3Model", "model_doc/phi3#transformers.Phi3Model", "Phi-3 model from Microsoft", HfItemKind::Model),
    ("BertModel", "model_doc/bert#transformers.BertModel", "BERT model for language understanding", HfItemKind::Model),
    ("GPT2Model", "model_doc/gpt2#transformers.GPT2Model", "GPT-2 model", HfItemKind::Model),
    ("T5Model", "model_doc/t5#transformers.T5Model", "T5 encoder-decoder model", HfItemKind::Model),
    ("WhisperModel", "model_doc/whisper#transformers.WhisperModel", "Whisper model for speech recognition", HfItemKind::Model),
    ("CLIPModel", "model_doc/clip#transformers.CLIPModel", "CLIP model for vision-language", HfItemKind::Model),

    // Utilities
    ("from_pretrained", "main_classes/model#transformers.PreTrainedModel.from_pretrained", "Load a pretrained model from Hub or local path", HfItemKind::Function),
    ("save_pretrained", "main_classes/model#transformers.PreTrainedModel.save_pretrained", "Save model to a directory", HfItemKind::Function),
    ("push_to_hub", "main_classes/model#transformers.PreTrainedModel.push_to_hub", "Push model to Hugging Face Hub", HfItemKind::Function),
    ("generate", "main_classes/text_generation#transformers.GenerationMixin.generate", "Generate sequences using the model", HfItemKind::Function),
];

/// Swift Transformers topics
pub const SWIFT_TRANSFORMERS_TOPICS: &[(&str, &str, &str, HfItemKind)] = &[
    // Core
    ("Hub", "hub", "Download models and tokenizers from Hugging Face Hub", HfItemKind::Class),
    ("LanguageModel", "languagemodel", "Protocol for language models", HfItemKind::Class),
    ("Tokenizer", "tokenizer", "Tokenizer protocol and implementations", HfItemKind::Tokenizer),
    ("Generation", "generation", "Text generation utilities", HfItemKind::Class),
    ("TextGenerationParameters", "textgenerationparameters", "Parameters for text generation", HfItemKind::Config),

    // Models
    ("LlamaModel", "models/llama", "LLaMA model implementation in Swift", HfItemKind::Model),
    ("MistralModel", "models/mistral", "Mistral model implementation", HfItemKind::Model),
    ("Phi3Model", "models/phi3", "Phi-3 model implementation", HfItemKind::Model),
    ("GemmaModel", "models/gemma", "Gemma model implementation", HfItemKind::Model),
    ("QwenModel", "models/qwen", "Qwen model implementation", HfItemKind::Model),

    // Tokenizers
    ("BPETokenizer", "tokenizers/bpe", "Byte-Pair Encoding tokenizer", HfItemKind::Tokenizer),
    ("PreTrainedTokenizer", "tokenizers/pretrained", "Load pretrained tokenizers", HfItemKind::Tokenizer),

    // Utilities
    ("loadModel", "loading", "Load models from Hub or local files", HfItemKind::Function),
    ("generate", "generation/generate", "Generate text with a language model", HfItemKind::Function),
    ("encode", "tokenizers/encode", "Encode text to token IDs", HfItemKind::Function),
    ("decode", "tokenizers/decode", "Decode token IDs to text", HfItemKind::Function),

    // MLX Integration
    ("MLXModel", "mlx/model", "MLX-backed model for Apple Silicon", HfItemKind::Model),
    ("MLXLanguageModel", "mlx/languagemodel", "MLX language model protocol", HfItemKind::Class),
];

/// Common LLM model families for search
pub const LLM_MODEL_FAMILIES: &[(&str, &str)] = &[
    ("llama", "Meta's LLaMA family of models"),
    ("mistral", "Mistral AI models"),
    ("qwen", "Alibaba's Qwen models"),
    ("gemma", "Google's Gemma models"),
    ("phi", "Microsoft's Phi models"),
    ("falcon", "Technology Innovation Institute's Falcon models"),
    ("mpt", "MosaicML's MPT models"),
    ("starcoder", "BigCode's StarCoder models for code"),
    ("codellama", "Meta's Code Llama models"),
    ("deepseek", "DeepSeek models"),
    ("yi", "01.AI's Yi models"),
    ("internlm", "InternLM models"),
    ("baichuan", "Baichuan models"),
    ("chatglm", "ChatGLM models"),
    ("zephyr", "HuggingFace's Zephyr fine-tuned models"),
    ("openchat", "OpenChat models"),
    ("neural-chat", "Intel's Neural Chat models"),
    ("stablelm", "Stability AI's StableLM models"),
    ("tinyllama", "TinyLlama small models"),
    ("orca", "Orca fine-tuned models"),
];
