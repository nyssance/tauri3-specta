# tauri3-specta

Type-safe TypeScript commands, channels, and events for **Tauri 3**, powered by Specta.

An independent implementation. No dependency on `tauri-specta`, no Tauri 2 compatibility layer. MIT licensed.

**Alpha:** targets Tauri `3.0.0-alpha.3` and its current IPC contract. The public API can change with Tauri 3. This repository is the distribution source; crates.io publication is not yet available.

## Install

In your Tauri app's `Cargo.toml`:

```toml
[dependencies]
tauri = { version = "=3.0.0-alpha.3", features = ["specta"] }
specta = { version = "=2.0.0-rc.25", features = ["derive", "function"] }
serde = { version = "1", features = ["derive"] }
tauri3-specta = { git = "https://github.com/nyssance/tauri3-specta" }
```

Use `@tauri-apps/api@3.0.0-alpha.2` on the frontend. Both `tauri` and `specta` must be direct dependencies under their standard names because their macros generate references to those crates. Pin the Git dependency to a reviewed commit in applications.

## Commands

```rust,no_run
use serde::Serialize;
use specta::Type;
use tauri3_specta::{command, commands};

#[derive(Serialize, Type)]
struct Greeting {
    message: String,
}

#[command]
fn greet(user_name: String) -> Greeting {
    Greeting { message: format!("Hello, {user_name}!") }
}

// Use this same collection for export and registration.
let bindings: tauri3_specta::Bindings = commands![greet];
bindings.export("../src/bindings.ts")?;
let builder = tauri::Builder::default().invoke_handler(bindings.invoke_handler());
// Finish configuring and run `builder` using your Tauri 3 application's runtime.
```

The export directory must already exist. Prefer generating in a dedicated development command or example, rather than writing into application source during production startup.

```ts
import { commands } from './bindings';

const greeting = await commands.greet({ userName: 'Ada' });
console.log(greeting.message);
```

`#[command(rename = "wire_name", rename_all = "snake_case")]` follows Tauri's command naming. The default argument casing is camelCase. Functions may be synchronous or asynchronous. Register commands by their original module paths.

## Results and channels

A top-level `Result<T, E>` returns `Promise<T>`. Rust `Err(E)` and transport failures reject unchanged. `CommandErrors` records declared Rust error types, but JavaScript catch values remain `unknown`: an IPC failure is not necessarily an `E`. Nested results retain Serde's `{ Ok: T } | { Err: E }` representation. Unit returns are `null`.

`tauri::ipc::Channel<T>` command arguments become `Channel<T>` from `@tauri-apps/api/core`. Channel payload types use Rust's serialization direction.

## Events

```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri3_specta::Event;

#[derive(Clone, Serialize, Deserialize, Type, Event)]
#[event(name = "download-progress")]
struct Progress {
    percent: u32,
}

// Add `.event::<Progress>()` to your command collection.
// Send using `Progress { percent: 100 }.emit(&app_handle)?`.
```

Without an override, event names use kebab-case. Generated events expose `name`, `listen`, `once`, `emit`, and `emitTo`:

```ts
import { events } from './bindings';

const unlisten = await events['download-progress'].listen(({ payload }) => {
  console.log(payload.percent);
});
// Call unlisten() when the listener's owner is disposed.
```

Rust `Event::listen` delivers `Result<Payload, serde_json::Error>` so malformed incoming payloads remain visible. Use Tauri's `unlisten` with the returned listener ID.

## Type contract

- Serde input and output shapes are distinct, including directional renaming.
- Recursive DTOs, tagged enums, aliases, and `serde_json::Value` are covered by regression tests.
- JSON values export as a recursive JSON union. JSON numbers use JavaScript `number`, with its normal precision limits. Other 64/128-bit integers fail export unless you explicitly define a suitable serialized representation with Specta/Serde.
- Duplicate wire names and generated command names fail export. Generated internal names beginning with `__t3`, and `CommandErrors`, are reserved.
- Generic command functions and generic event declarations are rejected. Use concrete command signatures and concrete event payloads; DTOs may still be generic.
- Only `rename` and `rename_all` command options are currently supported. Plugin command namespaces, custom IPC response encodings, and alternate language exporters are outside the current API.
- Specta prerelease versions are pinned together. The wire adapter corrects the published rc.25 metadata for JSON values and nested results; it does not replace Specta's type derivation or Serde exporter.

## Development

Use the latest stable Rust and Bun. TypeScript targets `ESNext`; the test toolchain currently uses TypeScript 7.0.2 and Bun 1.4.2. Lockfiles record exact dependency resolutions.

```sh
bun install --frozen-lockfile
cargo test --workspace --lib --locked
mkdir -p tests/generated
cargo run --locked --example export -- tests/generated/bindings.ts
bun run typecheck
bun test
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Tests exercise Tauri's native mock IPC handler, event dispatch, generated TypeScript's static contract, and frontend wrappers. They do not launch a platform webview. Tauri alpha.3 requires `macos-private-api` in its macOS mock runtime; this is enabled only for this repository's test dependency.
