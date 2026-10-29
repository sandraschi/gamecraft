extends CharacterBody3D
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
