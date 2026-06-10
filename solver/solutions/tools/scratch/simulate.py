"""
Infestation game simulator in Python.
Supports: player movement, triggers (zap), explosions (chain), rat tracking.
Assumes rats are static (won't move if walled off or solution avoids rat movement).
"""
import sys
import copy
import math
from collections import deque

# Cell types
EMPTY = '.'
WALL = '#'
PLANK = '='
WEB = 'w'
HOLE = 'O'
EXPLOSIVE = 'X'
RAT = 'R'
PLAYER1_CHARS = '▲▼►◄'  # N S E W
PLAYER2_CHARS = '△▽▷◁'  # N S E W
TRIGGER_CHARS = '123456789'

# Direction deltas (dx, dy) - y increases downward
DIR_DELTAS = {
    'N': (0, -1),
    'S': (0, 1),
    'E': (1, 0),
    'W': (-1, 0),
}
DIR_CHARS = {'N': '^', 'S': 'v', 'E': '>', 'W': '<'}
CHAR_TO_DIR = {'^': 'N', 'v': 'S', '>': 'E', '<': 'W', '.': None}

def parse_csv(csv_text):
    rows = []
    for line in csv_text.strip().split('\n'):
        row = [c.strip() for c in line.split(',')]
        rows.append(row)
    return rows

def find_player(grid):
    """Find player1 position and facing direction."""
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell in '▲▼►◄':
                facing = {'▲': 'N', '▼': 'S', '►': 'E', '◄': 'W'}[cell]
                return (x, y), facing
    return None, None

def find_all_rats(grid):
    rats = set()
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell == 'R':
                rats.add((x, y))
    return rats

def get_cell(grid, x, y):
    if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
        return grid[y][x]
    return '#'  # out of bounds = wall

def set_cell(grid, x, y, val):
    grid[y][x] = val

def blocks_player(cell):
    return cell in ('#', '=')

def is_trigger(cell):
    return cell in TRIGGER_CHARS

def grid_copy(grid):
    return [row[:] for row in grid]

def encode_state(grid, player_pos, player_facing):
    """Encode state as a hashable tuple for BFS."""
    # Only encode relevant cells for state comparison
    cells = []
    for row in grid:
        for c in row:
            cells.append(c)
    return (tuple(cells), player_pos, player_facing)

def apply_player_move(grid, player_pos, player_facing, action):
    """
    Apply a single player move. Returns (new_grid, new_pos, new_facing, alive).
    action: 'N','S','E','W' or None (stall)
    """
    grid = grid_copy(grid)
    px, py = player_pos

    if action is None:
        # Stall - player stays
        new_pos = player_pos
        new_facing = player_facing
    else:
        dx, dy = DIR_DELTAS[action]
        nx, ny = px + dx, py + dy
        target = get_cell(grid, nx, ny)

        if blocks_player(target):
            # Blocked - stay in place
            new_pos = player_pos
            new_facing = action  # still update facing
        else:
            new_pos = (nx, ny)
            new_facing = action

            # Check if target is explosive - player dies!
            if target == 'X':
                # Player dies
                return grid, player_pos, player_facing, False

    # Update player position on grid
    px, py = player_pos
    nx, ny = new_pos

    # Clear old position
    set_cell(grid, px, py, '.')

    # Check what's at new position
    target = get_cell(grid, nx, ny)

    # Kill rat by moving onto it
    if target == 'R':
        set_cell(grid, nx, ny, player_char(new_facing))
    elif target in TRIGGER_CHARS:
        # Trigger activated - don't replace trigger with wall here,
        # handle in trigger phase
        trigger_num = target
        set_cell(grid, nx, ny, player_char(new_facing))
        # Apply trigger zap
        grid = apply_trigger(grid, trigger_num)
    elif target == 'O':
        # Black hole - player disappears (dies)
        return grid, new_pos, new_facing, False
    else:
        set_cell(grid, nx, ny, player_char(new_facing))

    return grid, new_pos, new_facing, True

def player_char(facing):
    return {'N': '▲', 'S': '▼', 'E': '►', 'W': '◄'}[facing]

def apply_trigger(grid, trigger_num):
    """Apply trigger: find all triggers with same number, turn to walls, zap neighbors."""
    grid = grid_copy(grid)

    # Find all triggers with this number (excluding player's position which is already set)
    trigger_positions = []
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell == trigger_num:
                trigger_positions.append((x, y))

    # Turn all matching triggers to walls
    for (tx, ty) in trigger_positions:
        set_cell(grid, tx, ty, '#')

    # Zap 8-neighbors of each trigger
    pending_explosions = []
    for (tx, ty) in trigger_positions:
        for dx in [-1, 0, 1]:
            for dy in [-1, 0, 1]:
                if dx == 0 and dy == 0:
                    continue
                nx, ny = tx + dx, ty + dy
                if not (0 <= ny < len(grid) and 0 <= nx < len(grid[ny])):
                    continue
                cell = get_cell(grid, nx, ny)
                if cell == '.':
                    set_cell(grid, nx, ny, '#')
                elif cell == 'X':
                    pending_explosions.append((nx, ny))

    # Process explosion chain
    grid = apply_explosion_chain(grid, pending_explosions)

    return grid

def apply_explosion_chain(grid, initial_explosions):
    """Process explosion chain reactions."""
    grid = grid_copy(grid)
    pending = list(initial_explosions)
    processed = set()

    while pending:
        next_pending = []
        for (ex, ey) in pending:
            if (ex, ey) in processed:
                continue
            processed.add((ex, ey))

            # Clear the explosive
            set_cell(grid, ex, ey, '.')

            # Check 8 neighbors
            for dx in [-1, 0, 1]:
                for dy in [-1, 0, 1]:
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = ex + dx, ey + dy
                    if not (0 <= ny < len(grid) and 0 <= nx < len(grid[ny])):
                        continue
                    cell = get_cell(grid, nx, ny)
                    if cell == 'X':
                        if (nx, ny) not in processed:
                            next_pending.append((nx, ny))
                    elif cell in ('R', 'C', 'w', '='):
                        set_cell(grid, nx, ny, '.')
                    elif cell in PLAYER1_CHARS or cell in PLAYER2_CHARS:
                        # Player killed by explosion - mark somehow
                        # We'll handle this by checking after
                        pass
                    # Wall, BlackHole, Trigger, Empty - unaffected
        pending = next_pending

    return grid

def apply_rat_moves(grid, player_pos, player_facing):
    """
    Move rats toward nearest player using Euclidean distance minimization.
    Returns new grid. If rat kills player, returns (grid, False).
    """
    grid = grid_copy(grid)
    px, py = player_pos

    # Find all rats, sorted by distance to player (closest first)
    rats = []
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell == 'R':
                dist2 = (x - px)**2 + (y - py)**2
                rats.append(((x, y), dist2))

    rats.sort(key=lambda r: (r[1], r[0]))

    player_alive = True

    for (rx, ry), _ in rats:
        if get_cell(grid, rx, ry) != 'R':
            continue  # rat may have been killed

        current_dist2 = (rx - px)**2 + (ry - py)**2

        # Find best move direction (Euclidean distance reduction)
        # Face direction from rat to player
        face_dx = px - rx
        face_dy = py - ry

        # Candidate directions: the 8-way face direction decomposed
        # Determine primary direction
        if face_dx == 0 and face_dy == 0:
            continue  # on top of player somehow

        # Generate candidate moves (try to minimize distance)
        candidates = []

        # Try all 8 directions
        for dx in [-1, 0, 1]:
            for dy in [-1, 0, 1]:
                if dx == 0 and dy == 0:
                    continue
                nx, ny = rx + dx, ry + dy
                cell = get_cell(grid, nx, ny)

                # Rats are blocked by walls, other rats, webs
                if cell in ('#', 'R', 'w', '='):
                    continue

                # Check sword blocking: rat can't enter from directly in front of sword
                if (nx, ny) == player_pos:
                    # Check if rat approaches from player's facing direction
                    rat_approach_dir = get_approach_dir(rx, ry, nx, ny)
                    if is_sword_blocked(rat_approach_dir, player_facing):
                        continue

                new_dist2 = (nx - px)**2 + (ny - py)**2
                candidates.append((new_dist2, (nx, ny), (dx, dy)))

        if not candidates:
            continue

        # Stay option
        stay_dist2 = current_dist2

        # Find minimum distance among moves
        best_move = min(candidates, key=lambda c: c[0])

        if best_move[0] < stay_dist2:
            # Move rat
            new_pos = best_move[1]
            set_cell(grid, rx, ry, '.')
            nx, ny = new_pos
            target = get_cell(grid, nx, ny)

            if (nx, ny) == player_pos:
                # Rat kills player
                player_alive = False
                set_cell(grid, nx, ny, 'R')
            elif target == 'O':
                # Rat falls into black hole
                pass  # rat disappears
            elif target == 'X':
                # Rat steps on explosive - explodes
                grid = apply_explosion_chain(grid, [(nx, ny)])
            else:
                set_cell(grid, nx, ny, 'R')

    return grid, player_alive

def get_approach_dir(from_x, from_y, to_x, to_y):
    """Direction of movement from (from_x, from_y) to (to_x, to_y)."""
    dx = to_x - from_x
    dy = to_y - from_y
    if dx > 0: return 'E'
    if dx < 0: return 'W'
    if dy > 0: return 'S'
    if dy < 0: return 'N'
    return None

def is_sword_blocked(approach_dir, player_facing):
    """
    Rat approaches player from approach_dir.
    Player is facing player_facing.
    Blocked if rat comes from the direction player faces INTO (rat moves INTO the sword).
    i.e., rat's movement direction == player's facing direction.
    """
    # Sword blocks rat entering FROM the direction the player faces
    # The rat enters from direction approach_dir
    # Blocked if approach_dir == player_facing
    # Wait: "A rat CANNOT kill the player by entering from directly in front of the sword
    # (i.e. moving in the player's facing direction)"
    # So if player faces N (sword points N), rat moving S into player is killed/blocked
    # Rat moving S means it came FROM north...
    # Actually: approach_dir is the rat's movement direction
    # If rat moves S (south), it enters from north, which is where the sword points if player faces N
    # So: blocked if approach_dir == player_facing (both moving same direction = rat comes from front)

    # Re-reading: "entering from directly in front of the sword (moving in the player's facing direction)"
    # If player faces N, sword is in front (north). Rat moving S approaches from north.
    # Rat "moving in the player's facing direction" means rat moves N? No...
    # Let me re-read: rat moves IN player's facing direction = rat approaches from player's back?

    # Actually from rat.rs:
    # "sword_blocked = players.iter().any(|p| new_pos == p.pos && dir == p.dir.opposite())"
    # dir is the rat's movement direction (8-way), p.dir is player's facing direction
    # sword blocks if rat's movement direction == OPPOSITE of player facing
    # i.e., rat moves NORTH and player faces SOUTH -> blocked (rat attacks from the front when facing south)
    # Wait: rat dir == player.dir.opposite()
    # player faces N, player.dir = N, opposite = S
    # So blocked if rat moves S (south)
    # A rat moving south attacks from north... that's in FRONT of a northward-facing player
    # YES: player faces N, sword points N. Rat moving S enters from north. Sword blocks it.

    opposites = {'N': 'S', 'S': 'N', 'E': 'W', 'W': 'E'}
    return approach_dir == opposites.get(player_facing)

def count_rats(grid):
    count = 0
    for row in grid:
        for cell in row:
            if cell == 'R':
                count += 1
    return count

def grid_to_str(grid):
    return '\n'.join(','.join(row) for row in grid)

def simulate_step(grid, player_pos, player_facing, action, simulate_rats=True):
    """
    Full game step simulation.
    Returns: (new_grid, new_pos, new_facing, won, alive)
    """
    # Player moves first
    grid, new_pos, new_facing, alive = apply_player_move(grid, player_pos, player_facing, action)
    if not alive:
        return grid, new_pos, new_facing, False, False

    # Check if any player is killed by explosion neighbor (from trigger/explosion)
    if new_pos not in [(y, x) for y in range(len(grid)) for x in range(len(grid[y]))]:
        pass

    px, py = new_pos
    if get_cell(grid, px, py) not in PLAYER1_CHARS:
        # Player was removed (maybe by explosion)
        return grid, new_pos, new_facing, False, False

    # Check win condition after player moves (all rats dead)
    if count_rats(grid) == 0:
        return grid, new_pos, new_facing, True, True

    if not simulate_rats:
        return grid, new_pos, new_facing, False, True

    # Rats move
    grid, alive = apply_rat_moves(grid, new_pos, new_facing)
    if not alive:
        return grid, new_pos, new_facing, False, False

    # Check win condition after rats move
    if count_rats(grid) == 0:
        return grid, new_pos, new_facing, True, True

    return grid, new_pos, new_facing, False, True

def bfs_solve(csv_text, max_depth=30, verbose=True):
    """BFS to solve the level. Returns action string or None."""
    grid = parse_csv(csv_text)
    player_pos, player_facing = find_player(grid)

    if player_pos is None:
        print("No player found!")
        return None

    initial_rats = count_rats(grid)
    if initial_rats == 0:
        return ""

    print(f"Starting BFS: player at {player_pos} facing {player_facing}, {initial_rats} rats")

    # State: (grid_tuple, player_pos, player_facing)
    initial_state = encode_state(grid, player_pos, player_facing)

    queue = deque()
    queue.append((grid, player_pos, player_facing, []))
    visited = {initial_state}

    actions = ['N', 'S', 'E', 'W', None]
    action_chars = {'N': '^', 'S': 'v', 'E': '>', 'W': '<', None: '.'}

    level_count = 0

    while queue:
        if not queue:
            break

        curr_grid, curr_pos, curr_facing, path = queue.popleft()

        if len(path) >= max_depth:
            continue

        if verbose and len(path) == level_count:
            print(f"  BFS depth {level_count}, queue size {len(queue)+1}")
            level_count += 1

        for action in actions:
            new_grid, new_pos, new_facing, won, alive = simulate_step(
                curr_grid, curr_pos, curr_facing, action, simulate_rats=True)

            if not alive:
                continue

            new_path = path + [action]

            if won:
                action_str = ''.join(action_chars[a] for a in new_path)
                print(f"SOLVED! Path: {action_str}")
                return action_str

            state = encode_state(new_grid, new_pos, new_facing)
            if state not in visited:
                visited.add(state)
                queue.append((new_grid, new_pos, new_facing, new_path))

    print(f"No solution found within depth {max_depth}")
    return None

def solve_file(csv_path, max_depth=25):
    with open(csv_path) as f:
        csv_text = f.read()
    return bfs_solve(csv_text, max_depth=max_depth)

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python simulate.py <csv_file> [max_depth]")
        sys.exit(1)

    csv_path = sys.argv[1]
    max_depth = int(sys.argv[2]) if len(sys.argv) > 2 else 25

    result = solve_file(csv_path, max_depth=max_depth)
    if result is not None:
        print(f"Solution: {result}")
