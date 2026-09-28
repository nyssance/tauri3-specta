#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args().nth(1).expect("usage: export <output.ts>");
    let bindings = fixtures::bindings::<tauri::DynRuntime>();
    bindings.export(&destination)?;
    let configured = bindings.typescript_with(tauri3_specta::ExportOptions {
        positional_arguments: true,
        result_errors: true,
        camel_case_events: true,
        bigints_as_numbers: true,
        finite_floats: true,
    })?;
    std::fs::write(
        std::path::Path::new(&destination).with_file_name("configured.ts"),
        configured,
    )?;
    Ok(())
}
