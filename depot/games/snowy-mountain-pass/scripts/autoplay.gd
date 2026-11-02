extends Node3D
## GameCraft autoplay: plays the level through synthesized input.
@export var enabled: bool = false
@export var hold_action: String = "ui_right"
@export var jump_action: String = "ui_accept"
@export var smart_jump: bool = true
@export var probe_ahead: float = 1.0
@export var jump_frames: Array[int] = []
@export var flight: bool = false
@export var drive: bool = false
@export var flap: bool = false
@export var hover: float = 1.9
@export var ring_prefix: String = "Ring"

var tick: int = 0
var _player: Node3D = null
var _gates: Array = []
var _gate_index: int = 0

func _ready() -> void:
	# Off by default so Start Game hands the player the controls; the verifier
	# and the viewer's demo button set GAMECRAFT_AUTOPLAY=1.
	if OS.get_environment("GAMECRAFT_AUTOPLAY") == "1":
		enabled = true
	var root := get_parent()
	if root != null:
		_player = root.get_node_or_null("%Player")
		if _player == null:
			for child in root.get_children():
				if child is Node3D and str((child as Node3D).name) == "Player":
					_player = child
					break
		if flight or drive:
			for child in root.get_children():
				if child is Node3D and str(child.name).begins_with(ring_prefix):
					_gates.append(child)
			_gates.sort_custom(
				func(a, b): return (a as Node3D).global_position.x < (b as Node3D).global_position.x
			)

func _physics_process(_delta: float) -> void:
	if not enabled:
		return
	var root := get_parent()
	if root != null and "won" in root and root.won:
		_release_all()
		return
	if flight:
		_fly()
		return
	if drive:
		_drive()
		return
	if hold_action != "":
		Input.action_press(hold_action)
	var want_jump := false
	if smart_jump and _player != null and _player.is_on_floor():
		want_jump = gap_ahead()
	if not want_jump:
		want_jump = jump_frames.has(tick) or jump_frames.has(tick - 1)
	if want_jump:
		Input.action_press(jump_action)
	else:
		Input.action_release(jump_action)
	tick += 1

func _fly() -> void:
	if _player == null:
		return
	var target_y: float = _player.global_position.y
	if _gate_index < _gates.size():
		target_y = (_gates[_gate_index] as Node3D).global_position.y
	var error: float = target_y - _player.global_position.y
	if error > 0.15:
		Input.action_press("ui_up")
		Input.action_release("ui_down")
	elif error < -0.15:
		Input.action_press("ui_down")
		Input.action_release("ui_up")
	else:
		Input.action_release("ui_up")
		Input.action_release("ui_down")
	if _gate_index < _gates.size() and _player.global_position.x > (_gates[_gate_index] as Node3D).global_position.x + 1.0:
		_gate_index += 1

func _hover() -> void:
	if _player == null:
		return
	if _player.global_position.y < hover:
		# is_action_just_pressed only fires on a fresh press; holding the action
		# down permanently would flap exactly once and then fall forever.
		Input.action_release(jump_action)
		Input.action_press(jump_action)
	else:
		Input.action_release(jump_action)

func _drive() -> void:
	if _player == null or _gate_index >= _gates.size():
		return
	var target := (_gates[_gate_index] as Node3D).global_position
	var desired := atan2(-(target.z - _player.global_position.z), target.x - _player.global_position.x)
	var error := wrapf(desired - _player.rotation.y, -PI, PI)
	if error > 0.05:
		Input.action_press("ui_left")
		Input.action_release("ui_right")
	elif error < -0.05:
		Input.action_press("ui_right")
		Input.action_release("ui_left")
	else:
		Input.action_release("ui_left")
		Input.action_release("ui_right")
	if _player.global_position.distance_to(target) < 3.5:
		_gate_index += 1

func gap_ahead() -> bool:
	if _player == null:
		return false
	var space := get_world_3d().direct_space_state
	# Cast from just above the feet, far enough down to see the floor below a
	# raised platform. A short probe reports "gap" while standing high up and
	# jumps off the platform into the pit it was trying to avoid.
	var origin: Vector3 = _player.global_position + Vector3(probe_ahead, 0.1, 0.0)
	var query := PhysicsRayQueryParameters3D.create(origin, origin + Vector3(0.0, -3.5, 0.0))
	return space.intersect_ray(query).is_empty()

func _release_all() -> void:
	if hold_action != "":
		Input.action_release(hold_action)
	Input.action_release(jump_action)
	Input.action_release("ui_up")
	Input.action_release("ui_down")

func _exit_tree() -> void:
	_release_all()
