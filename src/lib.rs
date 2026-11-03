//! Generate TypeScript command and event bindings for Tauri 3.
//!
//! Use [`command`] on command functions, then pass the same [`commands!`]
//! collection to the exporter and Tauri's invoke handler.

extern crate self as tauri3_specta;

mod event;
mod export;
mod wire;

use std::{collections::BTreeSet, path::Path, sync::Arc};

use specta::{
    Type, Types,
    datatype::{DataType, Function},
};
use tauri::{DynRuntime, Runtime, ipc::Invoke};

pub use event::Event;
pub use tauri3_specta_macros::{Event, command, commands};

/// An invalid schema or a failed TypeScript export.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("duplicate {kind} name: {name}")]
    DuplicateName { kind: &'static str, name: String },
    #[error(transparent)]
    Typescript(#[from] specta_typescript::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid plugin name: {0}")]
    PluginName(String),
}

/// Explicit choices for the generated frontend contract.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExportOptions {
    /// Generate positional parameters instead of an argument object.
    pub positional_arguments: bool,
    /// Wrap declared Rust errors in a status-discriminated union.
    pub result_errors: bool,
    /// Use camelCase event property names while preserving wire names.
    pub camel_case_events: bool,
    /// Represent wide integers as JS numbers, accepting JS precision limits.
    pub bigints_as_numbers: bool,
    /// Assert that application floats are finite; represent them as numbers.
    pub finite_floats: bool,
}

/// Commands and events backed by one type graph and one IPC handler.
pub struct Bindings<R: Runtime = DynRuntime> {
    handler: Arc<dyn Fn(Invoke<R>) -> bool + Send + Sync>,
    types: Types,
    commands: Vec<Command>,
    events: Vec<EventSchema>,
    plugin: Option<String>,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct Command {
    pub(crate) function: Function,
    pub(crate) name: String,
    pub(crate) snake_case: bool,
}

impl Command {
    #[doc(hidden)]
    pub fn new(function: Function, name: &str, snake_case: bool) -> Self {
        Self {
            function,
            name: name.to_owned(),
            snake_case,
        }
    }
}

#[derive(Clone)]
struct EventSchema {
    name: &'static str,
    datatype: DataType,
}

impl<R: Runtime> Bindings<R> {
    /// Called by [`commands!`]. Prefer that macro to keep registration and export aligned.
    #[doc(hidden)]
    pub fn from_commands(
        handler: impl Fn(Invoke<R>) -> bool + Send + Sync + 'static,
        collect: fn(&mut Types) -> Vec<Command>,
    ) -> Self {
        let mut types = Types::default();
        let commands = collect(&mut types);
        Self {
            handler: Arc::new(handler),
            types,
            commands,
            events: Vec::new(),
            plugin: None,
        }
    }

    /// Address the commands of a Tauri plugin: the generated client invokes
    /// `plugin:<name>|<command>`, while the plugin's handler keeps the bare command names.
    /// The name must equal the one passed to `tauri::plugin::Builder::new`.
    pub fn plugin(mut self, name: &str) -> Result<Self, Error> {
        let valid = !name.is_empty()
            && !name.starts_with('-')
            && !name.ends_with('-')
            && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
        if !valid {
            return Err(Error::PluginName(name.to_owned()));
        }
        self.plugin = Some(name.to_owned());
        Ok(self)
    }

    /// Adds an event using the name defined by its [`Event`] implementation.
    pub fn event<E: Event>(mut self) -> Self {
        self.events.push(EventSchema {
            name: E::NAME,
            datatype: E::definition(&mut self.types),
        });
        self
    }

    /// Adds a type that is not reachable from a command or event.
    pub fn register<T: Type>(mut self) -> Self {
        T::definition(&mut self.types);
        self
    }

    /// Generates bindings in memory. Export errors are never suppressed.
    pub fn typescript(&self) -> Result<String, Error> {
        self.typescript_with(ExportOptions::default())
    }

    /// Generates bindings with an explicit frontend contract.
    pub fn typescript_with(&self, options: ExportOptions) -> Result<String, Error> {
        self.validate()?;
        export::generate(&self.types, &self.commands, &self.events, self.plugin.as_deref(), options).map_err(Into::into)
    }

    /// Writes a generated TypeScript module. The parent directory must exist.
    pub fn export(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let source = self.typescript()?;
        std::fs::write(path, source)?;
        Ok(())
    }

    /// Returns the handler for exactly the commands exported by this collection.
    pub fn invoke_handler(&self) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
        let handler = self.handler.clone();
        move |invoke| handler(invoke)
    }

    fn validate(&self) -> Result<(), Error> {
        use heck::ToLowerCamelCase;
        for (kind, names) in [
            (
                "command",
                self.commands
                    .iter()
                    .map(|command| command.name.clone())
                    .collect::<Vec<_>>(),
            ),
            (
                "TypeScript command",
                self.commands
                    .iter()
                    .map(|command| command.name.to_lower_camel_case())
                    .collect(),
            ),
            (
                "event",
                self.events
                    .iter()
                    .map(|event| event.name.to_owned())
                    .collect(),
            ),
        ] {
            let mut used = BTreeSet::new();
            for name in names {
                if !used.insert(name.clone()) {
                    return Err(Error::DuplicateName { kind, name });
                }
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub mod __private {
    pub use crate::Command;
}

#[cfg(test)]
#[path = "__tests__/bindings.rs"]
mod tests;
