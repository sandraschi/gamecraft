extends StaticBody3D
## GameCraft tracker: follows a target on one axis (AI) or keys (player).
@export var axis: String = "x"
@export var target_path: NodePath = NodePath("../Ball")
@export var max_speed: float = 6.0
@export var home: float = 0.0

func _physics_process(delta: float) -> void:
    var pos := global_position
    var cur: float = pos.x if axis == "x" else pos.z
    var want := home
    if DisplayServer.get_name() == "headless":
        var target := get_node_or_null(target_path)
        if target is Node3D:
            var p: Vector3 = (target as Node3D).global_position
            want = p.x if axis == "x" else p.z
    else:
        var keys := Input.get_axis("ui_left", "ui_right")
        if axis != "x":
            keys = Input.get_axis("ui_up", "ui_down")
        want = cur + keys * max_speed * delta
    var step: float = clampf(want - cur, -max_speed * delta, max_speed * delta)
    if axis == "x":
        pos.x += step
    else:
        pos.z += step
    global_position = pos
