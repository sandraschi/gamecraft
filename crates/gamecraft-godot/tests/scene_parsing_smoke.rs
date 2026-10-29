//! Headless scene-parsing smoke: fixture file -> parser -> graph -> checks.
//!
//! Run via `just smoke-godot`. No `unwrap`/`expect` (workspace denies them).

use gamecraft_godot::{build_graph, build_registry, parse_tscn};

/// Load the shared 3D-world fixture next to the workspace root.
fn fixture_text() -> Option<String> {
    let candidates = [
        "tests/fixtures/sample_3d_world.tscn",
        "../tests/fixtures/sample_3d_world.tscn",
        "../../tests/fixtures/sample_3d_world.tscn",
    ];
    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(path) {
            return Some(text);
        }
    }
    None
}

#[test]
fn fixture_parses_and_builds() {
    let text = fixture_text();
    assert!(text.is_some());
    if let Some(text) = text {
        let scene = parse_tscn(text.as_str());
        assert!(scene.is_ok());
        if let Ok(scene) = scene {
            assert_eq!(scene.header.format, 3);
            assert_eq!(scene.nodes.len(), 5);
            assert_eq!(scene.sub_resources.len(), 1);
            assert_eq!(scene.ext_resources.len(), 1);
            assert_eq!(scene.connections.len(), 1);
            let graph = build_graph(&scene);
            assert!(graph.is_ok());
            if let Ok(graph) = graph {
                let root = graph.root;
                assert!(root.is_some());
                if let Some(root_id) = root {
                    assert!(graph.resolve_path(root_id, "Player/Body").is_some());
                    assert!(graph.resolve_path(root_id, "Crate").is_some());
                    let registry = build_registry(&scene, &graph);
                    let (_, unresolved) = registry.resolve(&graph, root_id);
                    assert_eq!(unresolved, Vec::new());
                }
            }
        }
        println!("smoke-godot: fixture OK");
    }
}

#[test]
fn uid_round_trip_is_stable() {
    let first = gamecraft_godot::deterministic_uid("res://scenes/sample_3d_world.tscn");
    let second = gamecraft_godot::deterministic_uid("res://scenes/sample_3d_world.tscn");
    assert_eq!(first, second);
    assert!(first.starts_with("uid://"));
}
