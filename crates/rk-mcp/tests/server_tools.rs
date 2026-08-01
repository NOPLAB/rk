//! Tool-level tests that call the server methods directly (no MCP
//! transport involved).

use rk_mcp::RkMcpServer;
use rk_mcp::server::ApplyRequest;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

fn text_of(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
        .collect()
}

#[tokio::test]
async fn apply_creates_a_part_and_reports_events() {
    let server = RkMcpServer::new();
    let result = server
        .apply(Parameters(ApplyRequest {
            commands: vec![json!({
                "type": "create_primitive",
                "id": null,
                "primitive": {"shape": "box", "size": [0.1, 0.1, 0.1]},
                "name": "base"
            })],
        }))
        .await
        .expect("apply should not be a protocol error");

    assert_eq!(result.is_error, Some(false));
    let text = text_of(&result);
    assert!(text.contains("part_added"), "events missing: {text}");
    assert_eq!(server.engine().lock().parts().count(), 1);
}

#[tokio::test]
async fn apply_rejects_malformed_commands_before_applying_any() {
    let server = RkMcpServer::new();
    let err = server
        .apply(Parameters(ApplyRequest {
            commands: vec![
                json!({"type": "create_primitive", "id": null,
                       "primitive": {"shape": "box", "size": [0.1, 0.1, 0.1]}, "name": null}),
                json!({"type": "not_a_command"}),
            ],
        }))
        .await
        .expect_err("malformed command must be rejected");
    assert!(err.message.contains("commands[1]"), "got: {}", err.message);
    // Validation happens before application: nothing was applied
    assert_eq!(server.engine().lock().parts().count(), 0);
}

#[tokio::test]
async fn apply_reports_engine_errors_with_partial_results() {
    let server = RkMcpServer::new();
    let missing = uuid::Uuid::new_v4();
    let result = server
        .apply(Parameters(ApplyRequest {
            commands: vec![
                json!({"type": "create_primitive", "id": null,
                       "primitive": {"shape": "sphere", "radius": 0.05}, "name": null}),
                json!({"type": "delete_part", "part_id": missing}),
            ],
        }))
        .await
        .expect("engine errors are tool results, not protocol errors");

    assert_eq!(result.is_error, Some(true));
    let body: serde_json::Value = serde_json::from_str(&text_of(&result)).unwrap();
    assert_eq!(body["applied"], 1);
    assert_eq!(body["error"]["index"], 1);
    // The first command stays applied
    assert_eq!(server.engine().lock().parts().count(), 1);
}

#[tokio::test]
async fn describe_scene_reflects_document_state() {
    let server = RkMcpServer::new();
    server
        .apply(Parameters(ApplyRequest {
            commands: vec![json!({
                "type": "create_primitive",
                "id": null,
                "primitive": {"shape": "cylinder", "radius": 0.03, "height": 0.1},
                "name": "wheel"
            })],
        }))
        .await
        .unwrap();

    let result = server.describe_scene().await.unwrap();
    let desc: serde_json::Value = serde_json::from_str(&text_of(&result)).unwrap();

    assert_eq!(desc["parts"].as_array().unwrap().len(), 1);
    assert_eq!(desc["parts"][0]["name"], "wheel");
    assert_eq!(desc["history"]["can_undo"], true);
    assert_eq!(desc["modified"], true);
}

#[tokio::test]
async fn command_reference_is_served() {
    let server = RkMcpServer::new();
    let result = server.command_reference().await.unwrap();
    let text = text_of(&result);
    assert!(text.contains("create_primitive"));
    assert!(text.contains("add_extrude"));
}
