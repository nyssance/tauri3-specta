use serde::{Serialize, de::DeserializeOwned};
use specta::Type;
use tauri::{Emitter, EventId, EventTarget, Listener, Runtime};

/// A payload with a stable event name shared by Rust and generated TypeScript.
///
/// Derive using `#[derive(tauri3_specta::Event)]`. Names default to kebab-case.
pub trait Event: Type + Serialize + DeserializeOwned + Clone + Send + 'static {
    /// The Tauri wire name of the event.
    const NAME: &'static str;

    /// Emits this payload to all listeners.
    fn emit<R: Runtime>(&self, emitter: &impl Emitter<R>) -> tauri::Result<()> {
        emitter.emit(Self::NAME, self)
    }

    /// Emits this payload to a specific Tauri target.
    fn emit_to<R: Runtime>(
        &self,
        emitter: &impl Emitter<R>,
        target: impl Into<EventTarget>,
    ) -> tauri::Result<()> {
        emitter.emit_to(target, Self::NAME, self)
    }

    /// Listens for incoming payloads. Malformed external payloads are returned
    /// as errors to the caller, never silently dropped or converted to defaults.
    fn listen<R: Runtime>(
        listener: &impl Listener<R>,
        handler: impl Fn(Result<Self, serde_json::Error>) + Send + 'static,
    ) -> EventId {
        listener.listen(Self::NAME, move |event| {
            handler(serde_json::from_str(event.payload()))
        })
    }
}
