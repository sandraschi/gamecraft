extends CharacterBody3D
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
