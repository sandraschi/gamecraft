extends Area3D
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
