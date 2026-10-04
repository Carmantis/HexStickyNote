//! Local Inference - Runs GGUF models using llama-cpp-2
//!
//! Handles loading and running local GGUF models for inference.
//! Prompts are formatted with the chat template stored in the GGUF file.

use crate::ai_manager::AiStreamChunk;
use crate::settings_manager::GpuType;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

static LLAMA_BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

#[derive(Debug, Error)]
pub enum LocalInferenceError {
    #[error("Failed to load model: {0}")]
    ModelLoadError(String),
    #[error("Failed to create context: {0}")]
    ContextError(String),
    #[error("Tokenization failed: {0}")]
    TokenizationError(String),
    #[error("Inference failed: {0}")]
    InferenceError(String),
    #[error("Prompt is too long for the model context ({0} tokens)")]
    PromptTooLong(usize),
    #[error("Backend not initialized")]
    BackendNotInitialized,
}

/// Context window and the cap on newly generated tokens
const N_CTX: u32 = 4096;
const MAX_NEW_TOKENS: usize = 1024;

/// Initialize the llama backend (call once at startup)
/// Returns false if initialization fails (e.g. missing Vulkan drivers)
pub fn init_backend() -> bool {
    match LlamaBackend::init() {
        Ok(backend) => {
            let _ = LLAMA_BACKEND.set(backend);
            true
        }
        Err(e) => {
            log::warn!("Llama backend initialization failed (local AI will be unavailable): {}", e);
            false
        }
    }
}

/// Get the global backend instance
fn get_backend() -> Result<&'static LlamaBackend, LocalInferenceError> {
    LLAMA_BACKEND
        .get()
        .ok_or(LocalInferenceError::BackendNotInitialized)
}

/// Build the prompt with the model's own chat template (ChatML if it has none)
fn format_prompt(
    model: &LlamaModel,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, LocalInferenceError> {
    let template = model.chat_template(None).or_else(|e| {
        log::warn!("Model has no usable chat template ({}), falling back to ChatML", e);
        LlamaChatTemplate::new("chatml")
            .map_err(|e| LocalInferenceError::TokenizationError(e.to_string()))
    })?;

    let messages = [("system", system_prompt), ("user", user_message)]
        .into_iter()
        .map(|(role, content)| LlamaChatMessage::new(role.to_string(), content.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LocalInferenceError::TokenizationError(e.to_string()))?;

    model
        .apply_chat_template(&template, &messages, true)
        .map_err(|e| LocalInferenceError::TokenizationError(e.to_string()))
}

/// Run local inference with streaming
pub async fn run_local_inference(
    app: &AppHandle,
    model_path: &Path,
    system_prompt: &str,
    user_message: &str,
    gpu_type: GpuType,
) -> Result<(), LocalInferenceError> {
    let backend = get_backend()?;

    log::info!("Loading model: {:?}", model_path);

    // Offload all layers when GPU acceleration is enabled (llama.cpp clamps the count)
    let n_gpu_layers = if gpu_type != GpuType::Cpu {
        log::info!("GPU acceleration enabled ({:?}), offloading all layers", gpu_type);
        999
    } else {
        0
    };

    // Load model
    let mut model_params = LlamaModelParams::default()
        .with_n_gpu_layers(n_gpu_layers);
    
    let mut current_n_gpu_layers = n_gpu_layers;
    let model = match LlamaModel::load_from_file(backend, model_path, &model_params) {
        Ok(m) => m,
        Err(e) => {
            if n_gpu_layers > 0 {
                log::warn!("Failed to load model with GPU ({} layers): {}. Falling back to CPU.", n_gpu_layers, e);
                current_n_gpu_layers = 0;
                model_params = LlamaModelParams::default().with_n_gpu_layers(0);
                LlamaModel::load_from_file(backend, model_path, &model_params)
                    .map_err(|e2| LocalInferenceError::ModelLoadError(format!("CPU fallback also failed: {}", e2)))?
            } else {
                return Err(LocalInferenceError::ModelLoadError(e.to_string()));
            }
        }
    };

    let actual_device = if current_n_gpu_layers > 0 {
        "GPU".to_string()
    } else {
        "CPU".to_string()
    };

    // The whole prompt is decoded in one batch, so n_batch matches the context size
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(N_CTX))
        .with_n_batch(N_CTX);

    log::info!("Creating context with n_ctx={}", N_CTX);

    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| LocalInferenceError::ContextError(e.to_string()))?;

    log::info!("Context created successfully");

    // Format and tokenize prompt
    let formatted_prompt = format_prompt(&model, system_prompt, user_message)?;
    let tokens = model
        .str_to_token(&formatted_prompt, AddBos::Always)
        .map_err(|e| LocalInferenceError::TokenizationError(e.to_string()))?;

    log::info!("Prompt tokenized: {} tokens", tokens.len());
    for i in 0..std::cmp::min(10, tokens.len()) {
        if let Ok(piece) = model.token_to_str(tokens[i], llama_cpp_2::model::Special::Plaintext) {
            log::info!("Prompt token {}: id={} ({:?})", i, tokens[i], piece);
        } else {
            log::info!("Prompt token {}: id={} (undecodable)", i, tokens[i]);
        }
    }

    if tokens.len() >= N_CTX as usize {
        return Err(LocalInferenceError::PromptTooLong(tokens.len()));
    }

    // Create batch and decode
    let mut batch = LlamaBatch::new(N_CTX as usize, 1);

    log::info!("Adding {} tokens to batch", tokens.len());

    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch
            .add(*token, i as i32, &[0], is_last)
            .map_err(|e| LocalInferenceError::InferenceError(e.to_string()))?;
    }

    log::info!("Starting initial decode (this may take a moment on CPU)...");

    ctx.decode(&mut batch)
        .map_err(|e| LocalInferenceError::InferenceError(e.to_string()))?;

    log::info!("Initial decode completed");

    // Generate tokens
    let mut all_tokens = tokens.clone();
    let mut n_cur = tokens.len();
    let max_tokens = (tokens.len() + MAX_NEW_TOKENS).min(N_CTX as usize);
    let mut generated_tokens = 0;
    let mut emitted_chunks = 0;

    log::info!("Starting token generation (max {} new tokens)...", max_tokens - n_cur);

    while n_cur < max_tokens {
        // Sample next token
        let candidates = ctx.candidates();
        let mut candidates_array = LlamaTokenDataArray::from_iter(candidates, false);
        
        // Manual repetition penalty (1.2)
        let penalty = 1.2f32;
        let last_n = 64;
        let recent_tokens = &all_tokens[all_tokens.len().saturating_sub(last_n)..];
        
        for cand in &mut candidates_array.data {
            if recent_tokens.contains(&cand.id()) {
                if cand.logit() <= 0.0 {
                    cand.set_logit(cand.logit() * penalty);
                } else {
                    cand.set_logit(cand.logit() / penalty);
                }
            }
        }

        // Sort by logit for greedy sampling (after penalty)
        candidates_array.data.sort_by(|a, b| {
            b.logit().partial_cmp(&a.logit()).unwrap_or(std::cmp::Ordering::Equal)
        });

        if generated_tokens == 0 {
            log::info!("Got {} candidates", candidates_array.data.len());
            for i in 0..std::cmp::min(5, candidates_array.data.len()) {
                let cand = &candidates_array.data[i];
                log::info!("Candidate {}: id={}, logit={}", i, cand.id(), cand.logit());
            }
        }

        // Greedy sampling: take the token with highest logit (first in sorted array)
        let token = if let Some(first_candidate) = candidates_array.data.first() {
            let token_id = first_candidate.id();
            if generated_tokens < 5 {
                log::info!("Token {}: Selected ID {} with logit {}", generated_tokens + 1, token_id, first_candidate.logit());
            }
            token_id
        } else {
            log::info!("No more candidate tokens available");
            break; // No more tokens
        };

        generated_tokens += 1;
        all_tokens.push(token);

        // Check for EOS
        if model.is_eog_token(token) {
            log::info!("EOS token reached after {} tokens", generated_tokens);
            break;
        }

        // Decode token to text
        let text_res = model.token_to_str(token, llama_cpp_2::model::Special::Plaintext);
        
        match text_res {
            Ok(text) => {
                // Log first 5 tokens to see what we're getting
                if generated_tokens <= 5 {
                    log::info!("Token {}: id={} text={:?}", generated_tokens, token, text);
                }

                // Skip empty strings and unknown tokens
                if text.is_empty() {
                    if generated_tokens <= 10 {
                        log::info!("Skipping empty token {} (id: {})", generated_tokens, token);
                    }
                } else if text == "<unk>" || text == " <unk>" {
                    log::info!("Skipping <unk> token {} (id: {})", generated_tokens, token);
                } else {
                    // Emit chunk to frontend
                    if emitted_chunks < 5 {
                        log::info!("Emitting chunk {}: {:?}", emitted_chunks + 1, text);
                    }
                    app.emit(
                        "ai-stream-chunk",
                        AiStreamChunk {
                            chunk: text.clone(),
                            done: false,
                            gpu_info: Some(actual_device.clone()),
                        },
                    )
                    .ok();
                    emitted_chunks += 1;
                }
            }
            Err(e) => {
                if generated_tokens <= 10 {
                    log::warn!("Failed to decode token {} (id: {}): {}", generated_tokens, token, e);
                }
            }
        }

        // Log progress every 50 tokens
        if generated_tokens % 50 == 0 {
            log::info!("Progress: generated {} tokens, emitted {} chunks", generated_tokens, emitted_chunks);
        }

        // Prepare next batch
        batch.clear();
        batch
            .add(token, n_cur as i32, &[0], true)
            .map_err(|e| LocalInferenceError::InferenceError(e.to_string()))?;

        ctx.decode(&mut batch)
            .map_err(|e| LocalInferenceError::InferenceError(e.to_string()))?;

        n_cur += 1;
    }

    // Emit done signal
    app.emit(
        "ai-stream-chunk",
        AiStreamChunk {
            chunk: String::new(),
            done: true,
            gpu_info: Some(actual_device),
        },
    )
    .ok();

    log::info!(
        "Local inference completed: generated {} tokens, emitted {} chunks",
        generated_tokens,
        emitted_chunks
    );
    Ok(())
}
