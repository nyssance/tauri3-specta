#[path = "../../tests/fixtures/mod.rs"]
mod fixtures;

use crate::{Bindings, Event, commands};
use tauri::test::MockRuntime;

#[test]
fn command_names_and_argument_casing_match_the_wire_contract() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(
        source.contains("\"lookup\": (args: { \"userName\": string })"),
        "{source}"
    );
    assert!(
        source.contains("\"systemPing\": (args: { \"request_id\": string })"),
        "{source}"
    );
    assert!(source.contains("(\"system_ping\", args)"));
}

#[test]
fn result_aliases_resolve_to_success_and_document_the_error_type() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(source.contains("Promise<Profile>"), "{source}");
    assert!(source.contains("\"lookup\": Failure;"), "{source}");
    assert!(!source.contains("catch ("));
}

#[test]
fn unit_returns_are_json_null_not_undefined() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(
        source.contains("\"noValue\": (): Promise<null>"),
        "{source}"
    );
}

#[test]
fn serde_direction_is_preserved_for_input_and_output() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(
        source.contains("\"payload\": Directional_Deserialize"),
        "{source}"
    );
    assert!(
        source.contains("Promise<Directional_Serialize>"),
        "{source}"
    );
    assert!(source.contains("incoming"));
    assert!(source.contains("outgoing"));
}

#[test]
fn channels_carry_serialized_payloads() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(
        source.contains("\"progress\": __t3Channel<Progress>"),
        "{source}"
    );
}

#[test]
fn event_wire_name_is_shared_with_rust() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert_eq!(fixtures::Progress::NAME, "download-progress");
    assert!(source.contains("__t3Listen<Progress>(\"download-progress\", handler)"));
    assert!(source.contains("__t3Emit(\"download-progress\", payload)"));
}

#[test]
fn recursive_types_export_without_recursing_forever() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(source.contains("export type Node"));
    assert!(source.contains("Node[]") || source.contains("Array<Node>"));
}

#[test]
fn serde_json_value_exports_without_stack_overflow() {
    let bindings: Bindings<MockRuntime> = commands![fixtures::echo_json];
    let source = bindings.typescript().unwrap();
    assert!(source.contains("echoJson"));
    assert!(!source.contains(": any"));
}

#[test]
fn duplicate_commands_fail_instead_of_silently_shadowing() {
    let bindings: Bindings<MockRuntime> = commands![fixtures::lookup, fixtures::lookup];
    assert!(matches!(
        bindings.typescript(),
        Err(crate::Error::DuplicateName {
            kind: "command",
            ..
        })
    ));
}

#[test]
fn duplicate_event_names_fail() {
    let bindings = fixtures::bindings::<MockRuntime>().event::<fixtures::Progress>();
    assert!(matches!(
        bindings.typescript(),
        Err(crate::Error::DuplicateName { kind: "event", .. })
    ));
}

#[test]
fn export_is_deterministic() {
    assert_eq!(
        fixtures::bindings::<MockRuntime>().typescript().unwrap(),
        fixtures::bindings::<MockRuntime>().typescript().unwrap()
    );
}

#[test]
fn native_tauri_handler_obeys_the_exported_names_and_errors() {
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};
    let bindings = fixtures::bindings::<MockRuntime>();
    let app = mock_builder()
        .invoke_handler(bindings.invoke_handler())
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |command: &str, payload: serde_json::Value| {
        get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: command.to_owned(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(payload),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_owned(),
            },
        )
        .map(|response| response.deserialize::<serde_json::Value>().unwrap())
    };
    assert_eq!(
        invoke("system_ping", serde_json::json!({"request_id": "42"})),
        Ok(serde_json::json!("42"))
    );
    assert_eq!(
        invoke("lookup", serde_json::json!({"userName": "Ada"})),
        Ok(serde_json::json!({"display_name": "Ada", "score": 42}))
    );
    assert_eq!(
        invoke("lookup", serde_json::json!({"userName": ""})),
        Err(serde_json::json!({"kind": "not_found", "detail": ""}))
    );
    assert_eq!(
        invoke("no_value", serde_json::json!({})),
        Ok(serde_json::Value::Null)
    );
}

#[test]
fn nested_results_use_serdes_externally_tagged_representation() {
    let source = fixtures::bindings::<MockRuntime>().typescript().unwrap();
    assert!(
        source.contains("Ok: T") && source.contains("Err: E"),
        "{source}"
    );
    assert!(!source.contains("ok: T"), "{source}");
}

#[test]
fn large_integer_precision_loss_is_not_silently_allowed() {
    let bindings: Bindings<MockRuntime> = commands![fixtures::large_integer];
    assert!(bindings.typescript().is_err());
}

#[test]
fn native_event_listeners_receive_payloads_and_parse_errors() {
    use tauri::{Emitter, Listener};
    let app = tauri::test::mock_app();
    let (sender, receiver) = std::sync::mpsc::channel();
    let id = fixtures::Progress::listen(&app, move |payload| sender.send(payload).unwrap());
    fixtures::Progress { percent: 25 }.emit(&app).unwrap();
    assert_eq!(receiver.recv().unwrap().unwrap().percent, 25);
    app.emit(fixtures::Progress::NAME, "invalid payload")
        .unwrap();
    assert!(receiver.recv().unwrap().is_err());
    app.unlisten(id);
}

#[test]
fn inlined_results_keep_the_serde_wire_contract() {
    let source = fixtures::bindings::<MockRuntime>()
        .register::<fixtures::InlineResult>()
        .typescript()
        .unwrap();
    assert!(!source.contains("ok:"), "{source}");
    assert!(source.contains("Ok:"), "{source}");
}

#[test]
fn explicit_number_policy_handles_wide_integers() {
    let bindings: Bindings<MockRuntime> = commands![fixtures::large_integer];
    let source = bindings
        .typescript_with(crate::ExportOptions {
            bigints_as_numbers: true,
            ..Default::default()
        })
        .unwrap();
    assert!(source.contains("Promise<number>"), "{source}");
}

#[test]
fn positional_result_contract_preserves_wire_names() {
    let source = fixtures::bindings::<MockRuntime>()
        .typescript_with(crate::ExportOptions {
            positional_arguments: true,
            result_errors: true,
            camel_case_events: true,
            bigints_as_numbers: false,
            finite_floats: false,
        })
        .unwrap();
    assert!(
        source.contains("\"systemPing\": (request_id: string)"),
        "{source}"
    );
    assert!(
        source.contains("(\"system_ping\", { request_id })"),
        "{source}"
    );
    assert!(source.contains("__t3Result<Profile, Failure>"), "{source}");
    assert!(source.contains("\"downloadProgress\": {"), "{source}");
    assert!(
        source.contains("__t3Emit(\"download-progress\", payload)"),
        "{source}"
    );
}

#[test]
fn explicit_number_policy_preserves_json_map_keys() {
    #[derive(specta::Type, serde::Serialize)]
    struct Row {
        cells: std::collections::HashMap<usize, String>,
    }
    let bindings = fixtures::bindings::<MockRuntime>().register::<Row>();
    let source = bindings
        .typescript_with(crate::ExportOptions {
            bigints_as_numbers: true,
            ..Default::default()
        })
        .unwrap();
    assert!(source.contains("export type Row"), "{source}");
}

#[test]
fn finite_float_policy_is_explicit() {
    #[derive(specta::Type, serde::Serialize)]
    struct Coordinates {
        x: f64,
    }
    let bindings = fixtures::bindings::<MockRuntime>().register::<Coordinates>();
    assert!(bindings.typescript().unwrap().contains("x: number | null"));
    let source = bindings
        .typescript_with(crate::ExportOptions {
            finite_floats: true,
            ..Default::default()
        })
        .unwrap();
    assert!(source.contains("x: number,"), "{source}");
}
