extends Node
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
