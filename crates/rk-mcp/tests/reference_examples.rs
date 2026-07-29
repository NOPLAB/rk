//! Keeps `commands_reference.md` honest: every ```json block must
//! deserialize into a `Command`, and every command variant must appear
//! in the document.
//!
//! When you add a `Command` variant the exhaustive match below stops
//! compiling — add the variant here AND document it with a JSON example
//! in `src/commands_reference.md`.

use rk_engine::Command;
use rk_mcp::server::COMMAND_REFERENCE;

fn variant_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::NewDocument => "new_document",
        Command::LoadDocument { .. } => "load_document",
        Command::SaveDocument { .. } => "save_document",
        Command::ImportMesh { .. } => "import_mesh",
        Command::ImportUrdf { .. } => "import_urdf",
        Command::ExportUrdf { .. } => "export_urdf",
        Command::RenameProject { .. } => "rename_project",
        Command::CreatePrimitive { .. } => "create_primitive",
        Command::CreateEmptyPart { .. } => "create_empty_part",
        Command::DeletePart { .. } => "delete_part",
        Command::RenamePart { .. } => "rename_part",
        Command::SetPartTransform { .. } => "set_part_transform",
        Command::SetPartColor { .. } => "set_part_color",
        Command::SetPartMaterial { .. } => "set_part_material",
        Command::SetPartMass { .. } => "set_part_mass",
        Command::SetPartInertia { .. } => "set_part_inertia",
        Command::ConnectParts { .. } => "connect_parts",
        Command::DisconnectPart { .. } => "disconnect_part",
        Command::SetJointPosition { .. } => "set_joint_position",
        Command::ResetJointPosition { .. } => "reset_joint_position",
        Command::ResetAllJointPositions => "reset_all_joint_positions",
        Command::SetJointType { .. } => "set_joint_type",
        Command::SetJointOrigin { .. } => "set_joint_origin",
        Command::SetJointAxis { .. } => "set_joint_axis",
        Command::SetJointLimits { .. } => "set_joint_limits",
        Command::AddCollision { .. } => "add_collision",
        Command::RemoveCollision { .. } => "remove_collision",
        Command::SetCollisionOrigin { .. } => "set_collision_origin",
        Command::SetCollisionGeometry { .. } => "set_collision_geometry",
        Command::CreateSketch { .. } => "create_sketch",
        Command::DeleteSketch { .. } => "delete_sketch",
        Command::AddSketchEntities { .. } => "add_sketch_entities",
        Command::UpdateSketchEntity { .. } => "update_sketch_entity",
        Command::DeleteSketchEntities { .. } => "delete_sketch_entities",
        Command::AddSketchConstraint { .. } => "add_sketch_constraint",
        Command::DeleteSketchConstraint { .. } => "delete_sketch_constraint",
        Command::SolveSketch { .. } => "solve_sketch",
        Command::SetSketchConstruction { .. } => "set_sketch_construction",
        Command::AddExtrude { .. } => "add_extrude",
        Command::AddRevolve { .. } => "add_revolve",
        Command::DeleteFeature { .. } => "delete_feature",
        Command::SetFeatureSuppressed { .. } => "set_feature_suppressed",
        Command::RollbackTo { .. } => "rollback_to",
        Command::RebuildFeatures => "rebuild_features",
        Command::Undo => "undo",
        Command::Redo => "redo",
    }
}

/// Extract the contents of every ```json fenced block
fn json_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        match &mut current {
            None if line.trim() == "```json" => current = Some(String::new()),
            None => {}
            Some(block) => {
                if line.trim() == "```" {
                    blocks.push(current.take().unwrap());
                } else {
                    block.push_str(line);
                    block.push('\n');
                }
            }
        }
    }
    assert!(
        current.is_none(),
        "unterminated ```json block in commands_reference.md"
    );
    blocks
}

#[test]
fn every_json_example_is_a_valid_command() {
    let blocks = json_blocks(COMMAND_REFERENCE);
    assert!(!blocks.is_empty(), "no ```json blocks found");
    for block in &blocks {
        let cmd: Command = serde_json::from_str(block)
            .unwrap_or_else(|e| panic!("invalid command example: {e}\n---\n{block}"));
        // Exercise the exhaustive match so new variants break this test
        let _ = variant_name(&cmd);
    }
}

#[test]
fn every_command_variant_is_documented() {
    let documented: std::collections::HashSet<String> = json_blocks(COMMAND_REFERENCE)
        .iter()
        .map(|block| {
            let cmd: Command = serde_json::from_str(block).expect("checked in the other test");
            variant_name(&cmd).to_string()
        })
        .collect();

    // The exhaustive match in `variant_name` guarantees this list can
    // only be missing a variant if this test file failed to compile
    let all = [
        "new_document",
        "load_document",
        "save_document",
        "import_mesh",
        "import_urdf",
        "export_urdf",
        "rename_project",
        "create_primitive",
        "create_empty_part",
        "delete_part",
        "rename_part",
        "set_part_transform",
        "set_part_color",
        "set_part_material",
        "set_part_mass",
        "set_part_inertia",
        "connect_parts",
        "disconnect_part",
        "set_joint_position",
        "reset_joint_position",
        "reset_all_joint_positions",
        "set_joint_type",
        "set_joint_origin",
        "set_joint_axis",
        "set_joint_limits",
        "add_collision",
        "remove_collision",
        "set_collision_origin",
        "set_collision_geometry",
        "create_sketch",
        "delete_sketch",
        "add_sketch_entities",
        "update_sketch_entity",
        "delete_sketch_entities",
        "add_sketch_constraint",
        "delete_sketch_constraint",
        "solve_sketch",
        "add_extrude",
        "add_revolve",
        "delete_feature",
        "set_feature_suppressed",
        "rollback_to",
        "rebuild_features",
        "undo",
        "redo",
    ];

    let missing: Vec<_> = all
        .iter()
        .filter(|name| !documented.contains(**name))
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing a JSON example in commands_reference.md: {missing:?}"
    );
}
