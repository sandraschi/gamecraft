extends Area3D
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
