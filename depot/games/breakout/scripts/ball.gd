extends CharacterBody3D
## GameCraft ball: bounces, kicks off paddles, serves from center.
@export var speed: float = 8.0
@export var mode: int = 0 # 0 pong (keep x side), 1 breakout (force -z)
var cool: int = 0
var dir: Vector3 = Vector3(1, 0, 0.3).normalized()

func _ready() -> void:
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
