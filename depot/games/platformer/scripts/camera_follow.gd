extends Camera3D
## GameCraft follow camera (side view).
@export var target_path: NodePath = NodePath("../Player")
@export var height: float = 1.6
@export var smoothing: float = 8.0

var fixed_z: float = 0.0

func _ready() -> void:
	fixed_z = global_position.z
	rotation = Vector3.ZERO

func _process(delta: float) -> void:
	var target := get_node_or_null(target_path)
	if not (target is Node3D):
		return
	var spot: Vector3 = (target as Node3D).global_position
	var want := Vector3(spot.x, spot.y + height, fixed_z)
	global_position = global_position.lerp(want, clampf(smoothing * delta, 0.0, 1.0))
