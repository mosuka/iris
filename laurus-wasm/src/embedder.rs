//! JavaScript callback embedder for WASM environments.
//!
//! Bridges the [`Embedder`] trait to a JavaScript function, enabling
//! in-engine automatic embedding powered by browser-side models
//! (e.g. Transformers.js). This gives WASM users the same Unified Query
//! DSL experience as native platforms.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use wasm_bindgen::prelude::*;

use laurus::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use laurus::vector::core::vector::Vector;
use laurus::{LaurusError, Result};

// ---------------------------------------------------------------------------
// Send wrapper for !Send futures (safe in single-threaded WASM)
// ---------------------------------------------------------------------------

/// A wrapper that marks a `!Send` future as `Send`.
///
/// # Safety
///
/// This is only safe in single-threaded environments (i.e. `wasm32-unknown-unknown`).
/// The future will never actually be sent across threads.
struct AssertSend<F>(F);

// SAFETY: WASM is single-threaded. The future will only ever be polled
// on the main (and only) thread.
unsafe impl<F> Send for AssertSend<F> {}

impl<F: Future> Future for AssertSend<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We never move the inner future after pinning.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        inner.poll(cx)
    }
}

// ---------------------------------------------------------------------------
// JsFunction wrapper (Send + Sync for single-threaded WASM)
// ---------------------------------------------------------------------------

struct JsFunction(js_sys::Function);

// SAFETY: WASM is single-threaded.
unsafe impl Send for JsFunction {}
unsafe impl Sync for JsFunction {}

// ---------------------------------------------------------------------------
// JsCallbackEmbedder
// ---------------------------------------------------------------------------

/// An [`Embedder`] implementation that delegates to a JavaScript callback.
///
/// The JS function receives a string and must return a `Promise<number[]>`
/// (an array of floats representing the embedding vector).
///
/// # Example (JavaScript)
///
/// ```javascript
/// import { pipeline } from '@huggingface/transformers';
///
/// const model = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');
///
/// schema.addEmbedder("my-bert", {
///   type: "callback",
///   embed: async (text) => {
///     const output = await model(text, { pooling: 'mean', normalize: true });
///     return Array.from(output.data);
///   }
/// });
/// ```
pub struct JsCallbackEmbedder {
    func: Arc<JsFunction>,
    name: String,
}

impl fmt::Debug for JsCallbackEmbedder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsCallbackEmbedder")
            .field("name", &self.name)
            .finish()
    }
}

impl JsCallbackEmbedder {
    /// Create a new JS callback embedder.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for this embedder (used in logging).
    /// * `func` - A JS function `(text: string) => Promise<number[]>`.
    pub fn new(name: String, func: js_sys::Function) -> Self {
        Self {
            func: Arc::new(JsFunction(func)),
            name,
        }
    }

    /// Call the JS function and await the result.
    async fn embed_text(&self, text: &str) -> Result<Vector> {
        // Call: func(text) -> Promise<number[]>
        let js_text = JsValue::from_str(text);
        let promise = self
            .func
            .0
            .call1(&JsValue::NULL, &js_text)
            .map_err(|e| LaurusError::internal(format!("JS embedder call failed: {e:?}")))?;

        // Await the Promise
        let js_result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
            .await
            .map_err(|e| LaurusError::internal(format!("JS embedder promise rejected: {e:?}")))?;

        // Convert number[] to Vec<f32>
        let js_array = js_sys::Array::from(&js_result);
        let vec: Vec<f32> = js_array
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        if vec.is_empty() {
            return Err(LaurusError::internal(
                "JS embedder returned an empty vector",
            ));
        }

        Ok(Vector::new(vec))
    }
}

// Manual `Embedder` trait implementation.
//
// We do NOT use `#[async_trait]` here because the macro generates
// `Pin<Box<dyn Future + Send>>` and `JsFuture` is `!Send`.
// Instead we manually return `AssertSend`-wrapped futures, which is
// safe because WASM is single-threaded.
impl Embedder for JsCallbackEmbedder {
    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn embed<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        input: &'life1 EmbedInput<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<Vector>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        let text = input.as_text().map(|s| s.to_string());
        Box::pin(AssertSend(async move {
            let text = text.ok_or_else(|| {
                LaurusError::invalid_argument("JsCallbackEmbedder only supports text input")
            })?;
            self.embed_text(&text).await
        }))
    }
}
