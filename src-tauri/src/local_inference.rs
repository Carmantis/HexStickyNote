//! Local Inference - Runs GGUF models using llama-cpp-2
//!
//! Handles loading and running local GGUF models for inference.

use crate::ai_manager::AiStreamChunk;
use crate::keyring_store::AiProvider;
use crate::local_model;
use crate::settings_manager::SettingsManager;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use llama_cpp_2::token::LlamaToken;
use std::num::NonZeroU32;
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
    #[error("Model not downloaded")]
    ModelNotDownloaded,
    #[error("Backend not initialized")]
    BackendNotInitialized,
    #[error("Local model error: {0}")]
    LocalModelError(#[from] local_model::LocalModelError),
}

/// Initialize the llama backend (call once at startup)
pub fn init_backend() {
    LLAMA_BACKEND
        .set(LlamaBackend::init().expect("Failed to initialize llama backend"))
        .expect("Backend already initialized");
}

/// Get the global backend instance
fn get_backend() -> Result<&'static LlamaBackend, LocalInferenceError> {
    LLAMA_BACKEND
        .get()
        .ok_or(LocalInferenceError::BackendNotInitialized)
}

/// Format prompt for the model based on provider
fn format_prompt(provider: AiProvider, prompt: &str, context: &str) -> String {
    match provider {
        AiProvider::Poro2_8B => {
            // Llama 3.1 Instruct format - act as text editor, not chatbot
            // Specifically instruct to use Finnish and Markdown
            format!(
                "<|start_header_id|>system<|end_header_id|>\n\nOlet muistiolapun tekstieditori. Päivitä lapun sisältö käyttäjän pyynnön mukaan. \nSÄÄNNÖT:\n1. Kirjoita AINA suomeksi.\n2. Käytä Markdown-muotoilua (otsikot, listat, lihavointi jne.).\n3. Tulosta VAIN päivitetty muistiolapun sisältö.\n4. Älä kirjoita mitään muuta (ei selityksiä, ei tervehdyksiä).<|eot_id|><|start_header_id|>user<|end_header_id|>\n\nNykyinen sisältö:\n{}\n\nKäyttäjän pyyntö: {}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
                context, prompt
            )
        }
        AiProvider::FinChatSummary => {
            // Simpler format for Finnish summarization model to avoid echoing
            if context.is_empty() {
                format!(
                    "Kysymys: {}\n\nVastaus: ",
                    prompt
                )
            } else {
                format!(
                    "Konteksti: {}\n\nKysymys: {}\n\nVastaus: ",
                    context, prompt
                )
            }
        }
        _ => {
            // Fallback format
            format!(
                "Context: {}\n\nUser: {}\n\nAssistant:",
                context, prompt
            )
        }
    }
}

/// Run local inference with streaming
pub async fn run_local_inference(
    app: &AppHandle,
    provider: AiProvider,
    prompt: &str,
    context: &str,
    settings: Option<&SettingsManager>,
) -> Result<(), LocalInferenceError> {
    // Check if model is downloaded
    if !local_model::is_model_downloaded(provider, settings)? {
        return Err(LocalInferenceError::ModelNotDownloaded);
    }

    let model_path = local_model::get_model_path(provider, settings)?;
    let backend = get_backend()?;

    log::info!("Loading model: {:?}", model_path);

    // Get GPU setting
    let gpu_type = settings.map(|s| s.get_gpu_type()).unwrap_or(crate::keyring_store::GpuType::Cpu);
    let n_gpu_layers = if gpu_type != crate::keyring_store::GpuType::Cpu {
        log::info!("GPU acceleration enabled ({:?}), offloading 32 layers", gpu_type);
        32
    } else {
        0
    };

    // Load model
    let model_params = LlamaModelParams::default()
        .with_n_gpu_layers(n_gpu_layers);
    let model = LlamaModel::load_from_file(backend, model_path, &model_params)
        .map_err(|e| LocalInferenceError::ModelLoadError(e.to_string()))?;

    // Create context with conservative parameters for CPU inference
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(2048)) // Increased from 512
        .with_n_batch(512); // Increased from 128

    log::info!("Creating context with n_ctx=2048, n_batch=512");

    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| LocalInferenceError::ContextError(e.to_string()))?;

    log::info!("Context created successfully");

    // Format and tokenize prompt
    let formatted_prompt = format_prompt(provider, prompt, context);
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

    // Create batch and decode
    let mut batch = LlamaBatch::new(512, 1); // Match n_batch size

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
    const MAX_TOKENS: usize = 512; // Reduced for CPU inference (was 2048)
    let mut generated_tokens = 0;
    let mut emitted_chunks = 0;
    let mut full_response = String::new();

    log::info!("Starting token generation (max {} tokens)...", MAX_TOKENS);

    while n_cur < MAX_TOKENS {
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
                full_response.push_str(&text);

                // Stop sequence detection (case insensitive-ish)
                let stop_sequences = [
                    "Kysymys:", 
                    "Käyttäjä:", 
                    "Expected Output:", 
                    "User Request:", 
                    "Instruction:",
                    "Vastaus:",
                    "<|eot_id|>",
                    "<|end_of_text|>",
                    "\n\n\n" // Stop on excessive newlines
                ];
                
                let mut should_stop = false;
                for seq in stop_sequences {
                    if full_response.contains(seq) {
                        log::info!("Stop sequence '{}' detected. Stopping.", seq);
                        should_stop = true;
                        break;
                    }
                }
                
                if should_stop {
                    break;
                }

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
