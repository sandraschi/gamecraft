//! Shared `GDScript` templates. One copy per game lives in that game's
//! `scripts/` dir so every depot folder opens standalone.

/// Root-attached runtime: spin, score, goal, checkpoints, kill, hints,
/// talk/door/key state, pong points, autofire bullet pool.
pub const RUNTIME: &str = r#"extends Node
## Shared GameCraft runtime (attached to the scene root).
@export var dialogue: Array[String] = []
@export var needs_key: bool = false
@export var autofire: bool = false
@export var fire_interval: float = 0.4
@export var bullet_speed: float = 14.0
@export var checkpoints_to_win: int = 0
@export var rings_to_win: int = 0
@export var points_to_win: int = 0
@export var bricks_to_win: int = 0

var score: int = 0
var talk_index: int = 0
var has_key: bool = false
var won: bool = false
var checkpoints: int = 0
var rings_passed: int = 0
var points_left: int = 0
var points_right: int = 0
var fire_clock: float = 0.0
var bullets: Array = []
var bullet_index: int = 0
var enemies_down: int = 0
var total_to_win: int = 0

func _ready() -> void:
    for child in get_children():
        if child is Node3D and str(child.name).begins_with("Bullet"):
            bullets.append(child)

func _process(delta: float) -> void:
    for child in get_children():
        if child is Node3D and (child as Node3D).visible and str(child.name).begins_with("Coin"):
            (child as Node3D).rotate_y(delta * 2.0)
    if autofire and not won:
        fire_clock += delta
        if fire_clock >= fire_interval:
            fire_clock = 0.0
            fire_bullet()

func _bind_gamepad() -> void:
    # Built-in ui_* actions already cover movement on a pad, but ui_accept is
    # keys-only, so A would do nothing. Add the buttons rather than rewriting
    # project.godot for every game.
    _add_joy_button("ui_accept", JOY_BUTTON_A)
    _add_joy_button("ui_cancel", JOY_BUTTON_B)

func _add_joy_button(action: String, button: int) -> void:
    for existing in InputMap.action_get_events(action):
        if existing is InputEventJoypadButton and existing.button_index == button:
            return
    var event := InputEventJoypadButton.new()
    event.button_index = button
    InputMap.action_add_event(action, event)

func _unhandled_input(event: InputEvent) -> void:
    if event.is_action_pressed("ui_cancel"):
        get_tree().quit()

func add_score() -> void:
    score += 1

func on_brick_down() -> void:
    score += 1
    if bricks_to_win > 0 and score >= bricks_to_win and not won:
        won = true
        print("GameCraft: goal reached")

func _on_goal(body: Node3D) -> void:
    if body.has_method("is_player") and not won:
        won = true
        print("GameCraft: goal reached")

func _on_checkpoint(body: Node3D) -> void:
    if won:
        return
    if body.has_method("is_player"):
        checkpoints += 1
        print("GameCraft: checkpoint ", checkpoints)
        if checkpoints_to_win > 0 and checkpoints >= checkpoints_to_win:
            _on_goal(body)

func _on_ring(body: Node3D) -> void:
    if not body.has_method("is_player"):
        return
    rings_passed += 1
    print("GameCraft: ring ", rings_passed)
    if rings_to_win > 0 and rings_passed >= rings_to_win and not won:
        won = true
        print("GameCraft: goal reached")

func _on_kill(body: Node3D) -> void:
    if body.has_method("respawn"):
        body.respawn()

func _on_hint(body: Node3D) -> void:
    if body.has_method("do_jump"):
        body.do_jump()

func _on_talk(body: Node3D) -> void:
    if not body.has_method("is_player"):
        return
    if talk_index < dialogue.size():
        print("GameCraft NPC: ", dialogue[talk_index])
        talk_index += 1
        if talk_index >= dialogue.size():
            has_key = true
            print("GameCraft: received key")

func _on_door(body: Node3D) -> void:
    if body.has_method("is_player") and not won:
        if not needs_key or has_key:
            won = true
            print("GameCraft: door opened, win")
        else:
            print("GameCraft: the door needs a key")

func _on_point_left(_body: Node3D) -> void:
    on_point("left")

func _on_point_right(_body: Node3D) -> void:
    on_point("right")

func on_point(side: String) -> void:
    if side == "left":
        points_left += 1
    else:
        points_right += 1
    print("GameCraft: point ", side, " ", points_left, "-", points_right)
    if points_to_win > 0 and not won:
        if points_left >= points_to_win or points_right >= points_to_win:
            won = true
            print("GameCraft: goal reached")
    var ball := get_node_or_null("Ball")
    if ball != null and ball.has_method("serve"):
        if side == "left":
            ball.serve(-1.0)
        else:
            ball.serve(1.0)

func on_grid_win() -> void:
    if not won:
        won = true
        print("GameCraft: puzzle solved")

func on_enemy_down() -> void:
    enemies_down += 1
    if total_to_win > 0 and enemies_down >= total_to_win and not won:
        won = true
        print("GameCraft: all hostiles down, win")

func fire_bullet() -> void:
    if bullets.is_empty():
        return
    var player := get_node_or_null("%Player")
    var muzzle := Vector3.ZERO
    if player is Node3D:
        muzzle = (player as Node3D).global_position
    var target: Variant = nearest_enemy(muzzle)
    if target == null:
        return
    var bullet: Node3D = bullets[bullet_index % bullets.size()]
    bullet_index += 1
    var dir: Vector3 = (target as Vector3) - muzzle
    if dir.length() < 0.01:
        return
    dir = dir.normalized()
    if bullet.has_method("launch"):
        bullet.launch(muzzle + dir, dir * bullet_speed)

func nearest_enemy(from: Vector3) -> Variant:
    var best: Variant = null
    var best_d := 1e20
    for child in get_children():
        if child is Node3D and (child as Node3D).visible and str(child.name).begins_with("Enemy"):
            var d := from.distance_squared_to((child as Node3D).global_position)
            if d < best_d:
                best_d = d
                best = (child as Node3D).global_position
    return best
"#;

/// 2.5D platforming player: platform / runner / flap / drive modes.
pub const PLAYER: &str = r#"extends CharacterBody3D
## GameCraft shared player controller.
@export var mode: int = 0 # 0 platform, 1 runner, 2 flap, 3 drive
@export var speed: float = 4.0
@export var jump_velocity: float = 7.0
@export var gravity: float = 20.0
@export var steer_speed: float = 3.0
@export var climb_speed: float = 6.0
@export var turn_rate: float = 3.0
var hp: int = 3
var spawn_at: Vector3 = Vector3.ZERO

func _ready() -> void:
    spawn_at = global_position

func is_player() -> bool:
    return true

func do_jump() -> void:
    if is_on_floor() or mode == 2:
        velocity.y = jump_velocity

func take_hit(damage: int) -> void:
    hp -= damage
    if hp <= 0:
        respawn()

func respawn() -> void:
    global_position = spawn_at
    velocity = Vector3.ZERO
    hp = 3

func _physics_process(delta: float) -> void:
    if mode == 6:
        # Car: forward is local +X, steering rotates the body.
        var steer := Input.get_axis("ui_left", "ui_right")
        rotate_y(-steer * turn_rate * delta)
        velocity = global_transform.basis.x * speed
        velocity.y = 0.0
        move_and_slide()
        return
    if mode == 5:
        # Flight: constant forward speed, vertical rate follows the pitch input.
        velocity.x = speed
        var climb := Input.get_axis("ui_down", "ui_up")
        velocity.y = lerpf(velocity.y, climb * climb_speed, 2.5 * delta)
        move_and_slide()
        return
    if mode == 3:
        var steer := Input.get_axis("ui_left", "ui_right")
        if DisplayServer.get_name() == "headless":
            steer = 0.0
        velocity.x = speed
        velocity.z = steer * steer_speed
        if not is_on_floor():
            velocity.y -= gravity * delta
        else:
            velocity.y = -0.5
    elif mode == 2:
        velocity.x = speed
        velocity.z = 0.0
        if not is_on_floor():
            velocity.y -= gravity * delta
        else:
            velocity.y = -0.5
        if Input.is_action_just_pressed("ui_accept"):
            velocity.y = jump_velocity
    elif mode == 4:
        var sx := Input.get_axis("ui_left", "ui_right")
        var sz := Input.get_axis("ui_up", "ui_down")
        if DisplayServer.get_name() == "headless":
            sx = 0.0
            sz = 0.0
        velocity.x = sx * speed
        velocity.z = sz * speed
        if not is_on_floor():
            velocity.y -= gravity * delta
        else:
            velocity.y = -0.5
    else:
        var dir := 0.0
        if DisplayServer.get_name() == "headless" and mode == 1:
            dir = 1.0
        else:
            dir = Input.get_axis("ui_left", "ui_right")
        velocity.x = dir * speed
        velocity.z = 0.0
        if not is_on_floor():
            velocity.y -= gravity * delta
        elif Input.is_action_pressed("ui_accept"):
            # Held rather than edge-triggered: a jump requested on the frame the
            # body is still settling would otherwise be dropped forever. The body
            # leaves the floor immediately after jumping, so a hold cannot re-fire.
            velocity.y = jump_velocity
    move_and_slide()
    if global_position.y < -20.0:
        respawn()
"#;

/// Self-collecting coin: hides itself, credits the root runtime.
pub const COIN: &str = r#"extends Area3D
## GameCraft coin: touch to collect.
func _ready() -> void:
    body_entered.connect(_take)

func _take(body: Node3D) -> void:
    if body.has_method("is_player") and visible:
        hide()
        set_deferred("monitoring", false)
        var root := get_tree().current_scene
        if root != null and root.has_method("add_score"):
            root.add_score()
"#;

/// Constant-velocity ball with paddle kick (pong/breakout, XZ plane).
pub const BALL: &str = r#"extends CharacterBody3D
## GameCraft ball: bounces, kicks off paddles, serves from center.
@export var speed: float = 8.0
@export var mode: int = 0 # 0 pong (keep x side), 1 breakout (force -z)
var cool: int = 0
var dir: Vector3 = Vector3(1, 0, 0.3).normalized()
var sweep: float = 0.45

func _ready() -> void:
    if mode == 1:
        dir = Vector3(0.3, 0.0, -1.0).normalized()
    velocity = dir * speed

func serve(side: float) -> void:
    global_position = Vector3.ZERO
    var z := 0.3
    if mode == 1:
        z = -0.8
    dir = Vector3(side, 0.0, z).normalized()
    velocity = dir * speed

func _physics_process(delta: float) -> void:
    if cool > 0:
        cool -= 1
    if global_position.y < -7.0 or absf(global_position.x) > 30.0 or absf(global_position.z) > 30.0:
        global_position = Vector3(0, 0.5, 3.0)
        velocity = Vector3(0.3, 0.0, -1.0).normalized() * speed
    var hit := move_and_collide(velocity * delta)
    if hit:
        var normal := hit.get_normal()
        velocity = velocity.bounce(normal)
        var collider := hit.get_collider()
        if collider is Node3D and str((collider as Node3D).name).begins_with("Brick"):
            var brick := collider as Node3D
            brick.visible = false
            brick.process_mode = Node.PROCESS_MODE_DISABLED
            var brick_shape := brick.get_node_or_null("Shape")
            if brick_shape is CollisionShape3D:
                (brick_shape as CollisionShape3D).disabled = true
            var host := get_tree().current_scene
            if host != null and host.has_method("on_brick_down"):
                host.on_brick_down()
            sweep = -sweep
            velocity = Vector3(velocity.x + sweep, 0.0, velocity.z).normalized() * speed
        if cool == 0 and collider is Node3D and str((collider as Node3D).name).begins_with("Paddle"):
            speed += 0.4
            cool = 6
            var off: float = 0.0
            if mode == 0:
                var side := 1.0
                if velocity.x > 0.0:
                    side = -1.0
                off = clampf((global_position.z - (collider as Node3D).global_position.z) * 0.6, -0.7, 0.7)
                velocity = Vector3(side, 0.0, off).normalized() * speed
            else:
                off = clampf((global_position.x - (collider as Node3D).global_position.x) * 0.6, -0.7, 0.7)
                velocity = Vector3(off, 0.0, -1.0).normalized() * speed
"#;

/// Replays a verified trace inside the engine.
///
/// Two modes. `smart_jump` looks at the level (a downward probe ahead of the
/// player) and jumps at gaps, which survives the small integrator differences
/// between the Rust stepper and Godot. `jump_frames` replays exact frames, which
/// is what the mirrored sims used and is only safe when the engine matches.
pub const AUTOPLAY: &str = r#"extends Node3D
## GameCraft autoplay: plays the level through synthesized input.
@export var enabled: bool = true
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
"#;

/// Side-scroller camera: tracks the player on x, fixed height and distance.
///
/// A static pitched camera turned a 12:1 level into a strip at the bottom of the
/// screen; this keeps the player centred and the ground on screen.
pub const CAMERA_FOLLOW: &str = r#"extends Camera3D
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
"#;

/// Axis tracker: AI paddle follow in headless, input control for players.
pub const TRACKER: &str = r#"extends StaticBody3D
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
"#;

/// Drifting enemy: seeks the player or marches a straight lane.
pub const ENEMY: &str = r#"extends CharacterBody3D
## GameCraft enemy: seek or lane march, takes hits, dies flat.
@export var mode: int = 0 # 0 seek player, 1 straight +x lane
@export var speed: float = 1.5
@export var hp: int = 2
@export var target_path: NodePath = NodePath("../Player")
var dead: bool = false

func take_hit(damage: int) -> void:
    if dead:
        return
    hp -= damage
    if hp <= 0:
        dead = true
        hide()
        get_node("Shape").set_deferred("disabled", true)
        var root := get_tree().current_scene
        if root != null and root.has_method("on_enemy_down"):
            root.on_enemy_down()

func _physics_process(delta: float) -> void:
    if dead:
        return
    var dir := Vector3.ZERO
    if mode == 0:
        var target := get_node_or_null(target_path)
        if target is Node3D:
            dir = (target as Node3D).global_position - global_position
            dir.y = 0.0
            if dir.length() > 0.05:
                dir = dir.normalized()
    else:
        dir = Vector3(1, 0, 0)
    velocity = dir * speed
    velocity.y = -1.0
    move_and_slide()
"#;

/// Tower: nearest-foe cooldown fire by name prefix.
pub const TOWER: &str = r#"extends Node3D
## GameCraft tower: fires at the nearest visible Enemy* in range.
@export var fire_range: float = 6.0
@export var cooldown: float = 0.8
@export var damage: int = 1
var clock: float = 0.0

func _physics_process(delta: float) -> void:
    clock += delta
    if clock < cooldown:
        return
    clock = 0.0
    var foe: Variant = nearest_foe()
    if foe != null:
        foe.take_hit(damage)

func nearest_foe() -> Variant:
    var root := get_parent()
    var best: Variant = null
    var best_d := fire_range * fire_range
    for child in root.get_children():
        if child is Node3D and (child as Node3D).visible and str(child.name).begins_with("Enemy"):
            if not child.has_method("take_hit"):
                continue
            var d := global_position.distance_squared_to((child as Node3D).global_position)
            if d < best_d:
                best_d = d
                best = child
    return best
"#;

/// Pooled bullet: launched by the runtime, parks on hit or exit.
pub const BULLET: &str = r#"extends Area3D
## GameCraft pooled bullet.
var vel: Vector3 = Vector3.ZERO
var damage: int = 1
var live: bool = false

func _ready() -> void:
    body_entered.connect(_strike)

func launch(from: Vector3, v: Vector3) -> void:
    global_position = from
    vel = v
    live = true
    show()

func _physics_process(delta: float) -> void:
    if not live:
        return
    global_position += vel * delta
    if absf(global_position.x) > 60.0 or absf(global_position.z) > 60.0 or global_position.y < -5.0:
        park()

func _strike(body: Node3D) -> void:
    if not live:
        return
    if body.has_method("take_hit"):
        body.take_hit(damage)
        park()

func park() -> void:
    live = false
    hide()
"#;

/// Grid walker: maze / sokoban / talk / chess-knight movement and rules.
pub const WALKER: &str = r#"extends Node3D
## GameCraft grid walker: discrete moves, crate pushes, knight legality.
@export var mode: String = "maze" # maze | soko | talk | chess
@export var step_size: float = 1.0
@export var grid_w: int = 8
@export var grid_h: int = 8
var visited: Array = []
var cursor: Vector2i = Vector2i(0, 0)
@export var autoplay: bool = false
@export var autoplay_plan: Array[int] = []
@export var autoplay_interval: int = 3
var _plan_index: int = 0
var _plan_clock: int = 0

func _ready() -> void:
    cursor = cell()
    if mode == "chess":
        draw_cursor()

func draw_cursor() -> void:
    var marker := get_node_or_null("Cursor")
    if marker is Node3D:
        (marker as Node3D).global_position = cell_pos(cursor) + Vector3(0, -0.35, 0)

func cell() -> Vector2i:
    return Vector2i(int(round(global_position.x / step_size)), int(round(global_position.z / step_size)))

func cell_pos(c: Vector2i) -> Vector3:
    return Vector3(float(c.x) * step_size, 0.5, float(c.y) * step_size)

func crate_at(c: Vector2i) -> Node3D:
    for child in get_parent().get_children():
        if child is Node3D and (child as Node3D).visible and str(child.name).begins_with("Crate"):
            if cell_of(child as Node3D) == c:
                return child as Node3D
    return null

func cell_of(node: Node3D) -> Vector2i:
    return Vector2i(int(round(node.global_position.x / step_size)), int(round(node.global_position.z / step_size)))

func is_wall(c: Vector2i) -> bool:
    if c.x < 0 or c.y < 0 or c.x >= grid_w or c.y >= grid_h:
        return true
    for child in get_parent().get_children():
        if child is Node3D and str(child.name).begins_with("Wall") and cell_of(child as Node3D) == c:
            return true
    return false

func try_step(d: Vector2i) -> void:
    var to: Vector2i = cell() + d
    if mode == "chess":
        to = cursor + d
        if to.x < 0 or to.y < 0 or to.x >= grid_w or to.y >= grid_h:
            return
        cursor = to
        draw_cursor()
        return
    if is_wall(to):
        return
    if mode == "soko":
        if crate_at(to) != null:
            var beyond: Vector2i = to + d
            if is_wall(beyond) or crate_at(beyond) != null:
                return
            (crate_at(to) as Node3D).global_position = cell_pos(beyond)
            check_soko_win()
    global_position = cell_pos(to)
    _check_reach()

func is_player() -> bool:
    return true

func _check_reach() -> void:
    var here := cell()
    var host := get_tree().current_scene
    if host == null:
        return
    for child in get_parent().get_children():
        if not (child is Node3D):
            continue
        var name_text := str((child as Node3D).name)
        if cell_of(child as Node3D) != here:
            continue
        if name_text.begins_with("Goal"):
            if host.has_method("on_grid_win"):
                host.on_grid_win()
            return
        if name_text.begins_with("Npc") or name_text.begins_with("NPCTalk"):
            if host.has_method("_on_talk"):
                host.call("_on_talk", self)
            return
        if name_text.begins_with("Door"):
            if host.has_method("_on_door"):
                host.call("_on_door", self)
            return

func _physics_process(_delta: float) -> void:
    if not autoplay or _plan_index >= autoplay_plan.size():
        return
    _plan_clock += 1
    if _plan_clock < autoplay_interval:
        return
    _plan_clock = 0
    var code: int = autoplay_plan[_plan_index]
    _plan_index += 1
    if mode == "chess":
        var hop_x: int = int(floor(code / 5.0)) - 2
        var hop_z: int = code % 5 - 2
        knight_to(cursor + Vector2i(hop_x, hop_z))
        return
    match code:
        0:
            try_step(Vector2i(1, 0))
        1:
            try_step(Vector2i(-1, 0))
        2:
            try_step(Vector2i(0, 1))
        3:
            try_step(Vector2i(0, -1))

func knight_to(to: Vector2i) -> void:
    var from := cell()
    var dx: int = abs(to.x - from.x)
    var dy: int = abs(to.y - from.y)
    var legal := (dx == 1 and dy == 2) or (dx == 2 and dy == 1)
    if not legal:
        print("GameCraft: illegal knight move")
        return
    if is_wall(to):
        return
    global_position = cell_pos(to)
    cursor = to
    draw_cursor()
    if not visited.has(to):
        visited.append(to)
    check_chess_win()

func check_soko_win() -> void:
    for child in get_parent().get_children():
        if child is Node3D and str(child.name).begins_with("Target"):
            if crate_at(cell_of(child as Node3D)) == null:
                return
    var root := get_tree().current_scene
    if root != null and root.has_method("on_grid_win"):
        root.on_grid_win()

func check_chess_win() -> void:
    var need := 0
    var total := 0
    for child in get_parent().get_children():
        if child is Node3D and str(child.name).begins_with("Target"):
            total += 1
            if visited.has(cell_of(child as Node3D)):
                need += 1
    if total > 0 and need >= total:
        var root := get_tree().current_scene
        if root != null and root.has_method("on_grid_win"):
            root.on_grid_win()

func _unhandled_input(event: InputEvent) -> void:
    var d := Vector2i.ZERO
    if event.is_action_pressed("ui_left"):
        d = Vector2i(-1, 0)
    elif event.is_action_pressed("ui_right"):
        d = Vector2i(1, 0)
    elif event.is_action_pressed("ui_up"):
        d = Vector2i(0, -1)
    elif event.is_action_pressed("ui_down"):
        d = Vector2i(0, 1)
    if d != Vector2i.ZERO:
        try_step(d)
    if mode == "chess" and event.is_action_pressed("ui_accept"):
        knight_to(cursor)
"#;
