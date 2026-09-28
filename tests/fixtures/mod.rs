#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri3_specta::{Event, command};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Profile {
    pub display_name: String,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum Failure {
    NotFound(String),
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[event(name = "download-progress")]
pub struct Progress {
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Directional {
    #[serde(rename(serialize = "outgoing", deserialize = "incoming"))]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Node {
    pub name: String,
    pub children: Vec<Node>,
}

pub type LookupResult = Result<Profile, Failure>;

#[command]
pub fn lookup(user_name: String) -> LookupResult {
    if user_name.is_empty() {
        return Err(Failure::NotFound(user_name));
    }
    Ok(Profile {
        display_name: user_name,
        score: 42,
    })
}

#[command(rename = "system_ping", rename_all = "snake_case")]
pub fn ping(request_id: String) -> String {
    request_id
}

#[command]
pub async fn async_echo(message: String) -> String {
    message
}

#[command]
pub fn no_value() {}

#[command]
pub fn directional(payload: Directional) -> Directional {
    payload
}

#[command]
pub fn tree(root: Node) -> Node {
    root
}

#[command]
pub fn stream(progress: tauri::ipc::Channel<Progress>) {
    progress
        .send(Progress { percent: 100 })
        .expect("channel send");
}

#[command]
pub fn echo_json(value: serde_json::Value) -> serde_json::Value {
    value
}

pub fn bindings<R: tauri::Runtime>() -> tauri3_specta::Bindings<R> {
    tauri3_specta::commands![
        lookup,
        ping,
        async_echo,
        no_value,
        directional,
        tree,
        stream,
        echo_json,
        nested_result
    ]
    .event::<Progress>()
}

#[command]
pub fn nested_result(value: Vec<Result<Profile, Failure>>) -> Vec<Result<Profile, Failure>> {
    value
}

#[command]
pub fn large_integer(value: u64) -> u64 {
    value
}

#[derive(Serialize, Deserialize, Type)]
pub struct InlineResult {
    #[specta(inline)]
    pub value: Result<Profile, Failure>,
}
