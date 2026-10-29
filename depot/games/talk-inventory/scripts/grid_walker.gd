extends Node3D
## GameCraft grid walker: discrete moves, crate pushes, knight legality.
@export var mode: String = "maze" # maze | soko | talk | chess
@export var step_size: float = 1.0
@export var grid_w: int = 8
@export var grid_h: int = 8
var visited: Array = []
var cursor: Vector2i = Vector2i(0, 0)

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
