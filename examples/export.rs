#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args().nth(1).expect("usage: export <output.ts>");
    fixtures::bindings::<tauri::DynRuntime>().export(destination)?;
    Ok(())
}
