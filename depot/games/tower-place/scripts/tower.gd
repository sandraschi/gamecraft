extends Node3D
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
