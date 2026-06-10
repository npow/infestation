"""
Infestation game simulator in Python - fixed version.
Follows the exact Rust game logic.
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
CYBORG = 'C'
PLAYER1_CHARS = '▲▼►◄'  # N S E W
PLAYER2_CHARS = '△▽▷◁'  # N S E W
TRIGGER_CHARS = '123456789'

# Direction deltas (dx, dy) - y increases downward
DIR4_DELTAS = {
    'N': (0, -1),
    'S': (0, 1),
    'E': (1, 0),
    'W': (-1, 0),
}

# Dir8: all 8 directions as (dx, dy)
DIR8_ALL = [(-1,-1), (0,-1), (1,-1), (-1,0), (1,0), (-1,1), (0,1), (1,1)]

def dir8_from_delta(dx, dy):
    """Convert delta to dir8 (dx_sign, dy_sign). Returns (dx_sign, dy_sign) or None if (0,0)."""
    def sign(x): return 1 if x > 0 else (-1 if x < 0 else 0)
    sx, sy = sign(dx), sign(dy)
    if sx == 0 and sy == 0:
        return None
    return (sx, sy)

def dir8_is_diagonal(d8):
    sx, sy = d8
    return sx != 0 and sy != 0

def dir8_x_only(d8):
    sx, sy = d8
    if sx == 0:
        return None
    return (sx, 0)

def dir8_y_only(d8):
    sx, sy = d8
    if sy == 0:
        return None
    return (0, sy)

def dir8_dist_sq(d8):
    return 2 if dir8_is_diagonal(d8) else 1

def dir8_ord(d8):
    """Return the ordinal of a Dir8 direction matching Rust's enum declaration order.
    Rust Dir8 enum: East=0, West=1, North=2, South=3, Northeast=4, Northwest=5, Southeast=6, Southwest=7
    Maps (dx, dy) tuples to ordinal.
    None (stay) is represented as -1 (sorts before any Some value, matching Rust's Option<Dir8> Ord).
    """
    if d8 is None:
        return -1  # None < Some(...) in Rust
    mapping = {
        (1, 0): 0,    # East
        (-1, 0): 1,   # West
        (0, -1): 2,   # North
        (0, 1): 3,    # South
        (1, -1): 4,   # Northeast
        (-1, -1): 5,  # Northwest
        (1, 1): 6,    # Southeast
        (-1, 1): 7,   # Southwest
    }
    return mapping.get(d8, 99)

def dir8_opposite_of_dir4(facing):
    """The opposite of a dir4 as a dir8."""
    opp = {'N': (0, 1), 'S': (0, -1), 'E': (-1, 0), 'W': (1, 0)}
    return opp[facing]

def parse_csv(csv_text):
    rows = []
    for line in csv_text.strip().split('\n'):
        row = [c.strip() for c in line.split(',')]
        rows.append(row)
    return rows

def find_player1(grid):
    """Find player1 position and facing direction."""
    p1_to_dir = {'▲': 'N', '▼': 'S', '►': 'E', '◄': 'W'}
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell in p1_to_dir:
                return (x, y), p1_to_dir[cell]
    return None, None

def find_player2(grid):
    """Find player2 position and facing direction."""
    p2_to_dir = {'△': 'N', '▽': 'S', '▷': 'E', '◁': 'W'}
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell in p2_to_dir:
                return (x, y), p2_to_dir[cell]
    return None, None

def find_all_rats(grid):
    rats = []
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell == 'R':
                rats.append((x, y))
    return rats

def get_cell(grid, x, y):
    if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
        return grid[y][x]
    return '#'  # out of bounds = wall

def set_cell(grid, x, y, val):
    grid[y][x] = val

def blocks_player(cell):
    return cell in ('#', '=')

def blocks_rat(cell):
    """Blocks normal rats. Planks (=) do NOT block rats - rats walk through and destroy them."""
    return cell in ('#', 'R', 'w', 'C')

def is_trigger(cell):
    return cell in TRIGGER_CHARS

def grid_copy(grid):
    return [row[:] for row in grid]

def encode_state(grid, p1_pos, p1_facing, p2_pos=None, p2_facing=None):
    """Encode state as a hashable tuple for visited-set."""
    cells = tuple(c for row in grid for c in row)
    return (cells, p1_pos, p1_facing, p2_pos, p2_facing)

def player1_char(facing):
    return {'N': '▲', 'S': '▼', 'E': '►', 'W': '◄'}[facing]

def player2_char(facing):
    return {'N': '△', 'S': '▽', 'E': '▷', 'W': '◁'}[facing]

def apply_explosion_chain(grid, initial_explosions):
    """Process explosion chain reactions. Returns new grid, and set of player cells removed."""
    grid = grid_copy(grid)
    pending = list(initial_explosions)
    processed = set()
    players_killed = set()

    while pending:
        next_pending = []
        for (ex, ey) in pending:
            if (ex, ey) in processed:
                continue
            processed.add((ex, ey))

            # Clear the explosive center
            if get_cell(grid, ex, ey) == 'X':
                set_cell(grid, ex, ey, '.')

            # Check 8 neighbors
            for dx, dy in DIR8_ALL:
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
                    set_cell(grid, nx, ny, '.')
                    players_killed.add((nx, ny))
                # Wall, BlackHole, Trigger, Empty - unaffected
        pending = next_pending

    return grid, players_killed

def apply_trigger(grid, trigger_num, player_pos):
    """Apply trigger activation: find same-numbered triggers, turn to walls, zap neighbors.
    player_pos is where the player stepped (already set to player char there).
    Returns (new_grid, players_killed)."""
    grid = grid_copy(grid)

    # Find all remaining triggers with this number
    trigger_positions = []
    for y, row in enumerate(grid):
        for x, cell in enumerate(row):
            if cell == trigger_num:
                trigger_positions.append((x, y))
    # Note: the player's cell was already changed to a player char,
    # so the trigger the player stepped on is no longer there

    # Turn all matching triggers to walls
    for (tx, ty) in trigger_positions:
        set_cell(grid, tx, ty, '#')

    # Zap 8-neighbors of each trigger
    pending_explosions = []
    for (tx, ty) in trigger_positions:
        for dx, dy in DIR8_ALL:
            nx, ny = tx + dx, ty + dy
            if not (0 <= ny < len(grid) and 0 <= nx < len(grid[ny])):
                continue
            cell = get_cell(grid, nx, ny)
            if cell == '.':
                set_cell(grid, nx, ny, '#')
            elif cell == 'X':
                pending_explosions.append((nx, ny))
            # Other cells unaffected by zap

    # Process explosion chain
    grid, players_killed = apply_explosion_chain(grid, pending_explosions)
    return grid, players_killed

def compute_player_dest(grid, player_pos, player_facing, action, player_num=1):
    """
    Compute where a player ends up (before trigger/explosion resolution).
    Returns (new_pos, new_facing, dest_cell_before_move, instant_death)
    instant_death is True if stepping on explosive or black hole.
    Does NOT modify the grid.
    """
    px, py = player_pos
    if action is None:
        return player_pos, player_facing, get_cell(grid, px, py), False

    dx, dy = DIR4_DELTAS[action]
    nx, ny = px + dx, py + dy
    target = get_cell(grid, nx, ny)
    new_facing = action

    if blocks_player(target):
        return player_pos, new_facing, get_cell(grid, px, py), False

    new_pos = (nx, ny)
    if target == 'X':
        return new_pos, new_facing, target, True   # instant death on explosive
    if target == 'O':
        return new_pos, new_facing, target, True   # instant death in hole

    return new_pos, new_facing, target, False


def apply_player_move(grid, player_pos, player_facing, action, player_num=1):
    """
    Apply a single player move (including trigger/explosion resolution).
    Returns (new_grid, new_pos, new_facing, alive, players_killed_set)
    action: 'N','S','E','W' or None (stall)
    player_num: 1 or 2
    NOTE: This is used for standalone player resolution (legacy).
    For correct simultaneous resolution use simulate_step.
    """
    grid = grid_copy(grid)
    px, py = player_pos
    player_ch = player1_char if player_num == 1 else player2_char

    new_pos, new_facing, dest_cell, instant_death = compute_player_dest(
        grid, player_pos, player_facing, action, player_num)

    if instant_death:
        if dest_cell == 'O':
            set_cell(grid, px, py, '.')
        return grid, new_pos, new_facing, False, set()

    # Clear old position
    if new_pos != player_pos:
        set_cell(grid, px, py, '.')

    # Place player at destination
    set_cell(grid, new_pos[0], new_pos[1], player_ch(new_facing))

    players_killed = set()

    if is_trigger(dest_cell) and new_pos != player_pos:
        grid, pk = apply_trigger(grid, dest_cell, new_pos)
        players_killed.update(pk)

    if new_pos in players_killed:
        return grid, new_pos, new_facing, False, players_killed

    return grid, new_pos, new_facing, True, players_killed

def count_rats(grid):
    count = 0
    for row in grid:
        for cell in row:
            if cell == 'R':
                count += 1
    return count

def dist_sq(x1, y1, x2, y2):
    return (x1 - x2)**2 + (y1 - y2)**2

def apply_rat_moves(grid, player_infos):
    """
    Move rats according to game logic.
    player_infos: list of (pos, facing, moved) tuples
    Returns (new_grid, all_players_alive)

    Follows exact rat.rs logic:
    - Sort rats by distance to nearest player
    - For each rat: compute face_dir to nearest player
      - If diagonal: try [face_dir, x_only, y_only]
      - If cardinal: try [face_dir]
    - Pick move that minimizes dist_sq to target player
    - Tiebreak: move_d2, player_still, player_index, dir_option_index
    """
    grid = grid_copy(grid)

    # Track player positions for death detection
    player_positions = set(pi[0] for pi in player_infos)

    # Find all rats, sort by distance to nearest player
    all_rats = find_all_rats(grid)

    def rat_min_dist2(rpos):
        rx, ry = rpos
        return min(dist_sq(rx, ry, pi[0][0], pi[0][1]) for pi in player_infos)

    # Sort rats by (min_dist2, (rx, ry)) - closest first
    all_rats.sort(key=lambda rpos: (rat_min_dist2(rpos), rpos))

    all_alive = True

    for rat_pos in all_rats:
        rx, ry = rat_pos
        if get_cell(grid, rx, ry) != 'R':
            continue  # rat was killed

        # Stay option score: min dist to any player
        stay_score = min(dist_sq(rx, ry, pi[0][0], pi[0][1]) for pi in player_infos)

        # Build candidate list: (score, move_d2, player_still, player_idx, dir_ord, dir_or_None)
        # Lower is better; dir_ord matches Rust's Dir8 enum declaration order.
        # Stay: score=min_dist2, move_d2=0, player_still=True, pi_idx=0, dir_ord=-1, dir=None
        # (None dir_ord=-1 is smallest, matches Rust Option<Dir8>::None < Some(...))
        candidates = [(stay_score, 0, True, 0, dir8_ord(None), None)]

        for pi_idx, pi in enumerate(player_infos):
            ppos, pfacing, pmoved = pi
            px, py = ppos

            # Face direction from rat to player
            fdx = px - rx
            fdy = py - ry
            face_d8 = dir8_from_delta(fdx, fdy)
            if face_d8 is None:
                continue

            # Generate direction candidates
            if dir8_is_diagonal(face_d8):
                dirs_to_try = [face_d8]
                xo = dir8_x_only(face_d8)
                yo = dir8_y_only(face_d8)
                if xo: dirs_to_try.append(xo)
                if yo: dirs_to_try.append(yo)
            else:
                dirs_to_try = [face_d8]

            for d8 in dirs_to_try:
                ddx, ddy = d8
                new_rx, new_ry = rx + ddx, ry + ddy

                cell = get_cell(grid, new_rx, new_ry)
                if blocks_rat(cell):
                    continue

                # Sword blocking check: rat can't enter player's cell from front
                # sword blocks if rat's dir == opposite of player's facing
                rat_blocked_by_sword = False
                for pi2 in player_infos:
                    ppos2, pfacing2, _ = pi2
                    if (new_rx, new_ry) == ppos2:
                        # Rat moves with direction d8
                        # Sword blocks if d8 == opposite of pfacing2 (as dir8)
                        opp_d8 = dir8_opposite_of_dir4(pfacing2)
                        if d8 == opp_d8:
                            rat_blocked_by_sword = True
                            break

                if rat_blocked_by_sword:
                    continue

                new_dist2 = dist_sq(new_rx, new_ry, px, py)
                move_d2 = dir8_dist_sq(d8)
                player_still = not pmoved

                candidates.append((new_dist2, move_d2, player_still, pi_idx, dir8_ord(d8), d8))

        # Find best candidate (min tuple comparison)
        # Uses Rust's Dir8 enum ordinal as tiebreaker (last field)
        best = min(candidates)
        best_dir = best[-1]

        if best_dir is None:
            # Rat stays - just face nearest player
            pass
        else:
            ddx, ddy = best_dir
            new_rx, new_ry = rx + ddx, ry + ddy

            # Move rat
            set_cell(grid, rx, ry, '.')

            target = get_cell(grid, new_rx, new_ry)

            if target in PLAYER1_CHARS or target in PLAYER2_CHARS:
                # Rat kills player - we consider them dead
                set_cell(grid, new_rx, new_ry, 'R')
                all_alive = False
            elif target == 'O':
                # Rat falls into black hole - disappears
                pass
            elif target == 'X':
                # Rat steps on explosive - chain explosion
                grid, players_killed = apply_explosion_chain(grid, [(new_rx, new_ry)])
                # Check if any player positions were killed by explosion
                for ppos in player_positions:
                    if ppos in players_killed:
                        all_alive = False
                # Update player positions (in case explosion cleared a player cell)
                # Remove any killed player positions from tracking
                player_positions -= players_killed
            else:
                set_cell(grid, new_rx, new_ry, 'R')

    return grid, all_alive

def simulate_step(grid, p1_pos, p1_facing, action1, p2_pos=None, p2_facing=None, action2=None):
    """
    Full game step simulation for 1 or 2 players.
    Matches Rust do_player_moves -> finish_moving -> start_zap_wave -> explosions order:
      1. Compute player destinations (no trigger yet)
      2. Place players temporarily on working grid (from cleared, dest set)
      3. Compute rat moves against this working grid (player placed, trigger NOT fired)
      4. Revert working grid to pre-move state (only from positions cleared)
      5. Place all entities (players then rats) at destinations
      6. Fire triggers -> fire explosions
    Returns: (new_grid, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, won, alive)
    """
    two_player = p2_pos is not None

    # Step 1: Compute player destinations
    p1_dest, new_p1_facing, p1_dest_cell, p1_instant_death = compute_player_dest(
        grid, p1_pos, p1_facing, action1, player_num=1)

    if p1_instant_death:
        g = grid_copy(grid)
        if p1_dest_cell == 'O':
            set_cell(g, p1_pos[0], p1_pos[1], '.')
        return g, p1_dest, new_p1_facing, p2_pos, p2_facing, False, False

    new_p2_pos, new_p2_facing = p2_pos, p2_facing
    p2_dest_cell = None

    if two_player and p2_pos is not None:
        p2_dest, new_p2_facing, p2_dest_cell, p2_instant_death = compute_player_dest(
            grid, p2_pos, p2_facing, action2, player_num=2)
        if p2_instant_death:
            g = grid_copy(grid)
            if p2_dest_cell == 'O':
                set_cell(g, p2_pos[0], p2_pos[1], '.')
            return g, p1_dest, new_p1_facing, p2_dest, new_p2_facing, False, False
        new_p2_pos = p2_dest

    new_p1_pos = p1_dest

    # Step 2: Build working grid with player destinations placed (temp for rat computation)
    # Match Rust begin_move: clear from, set dest to new cell
    working = grid_copy(grid)

    # Clear player from positions
    if new_p1_pos != p1_pos:
        set_cell(working, p1_pos[0], p1_pos[1], '.')
    # Place player at destination (overwrites whatever was there temporarily)
    set_cell(working, new_p1_pos[0], new_p1_pos[1], player1_char(new_p1_facing))

    if two_player and new_p2_pos is not None and new_p2_pos != p2_pos:
        set_cell(working, p2_pos[0], p2_pos[1], '.')
    if two_player and new_p2_pos is not None:
        set_cell(working, new_p2_pos[0], new_p2_pos[1], player2_char(new_p2_facing))

    # Step 3: Compute rat moves on working grid
    player_infos = [(new_p1_pos, new_p1_facing, action1 is not None)]
    if two_player and new_p2_pos is not None:
        player_infos.append((new_p2_pos, new_p2_facing, action2 is not None))

    working, all_alive_after_rats = apply_rat_moves(working, player_infos)

    # Step 4: Rust reverts working grid to prev_grid with from positions cleared.
    # But in our model apply_rat_moves already operates on working and returns a new grid.
    # We need the final grid to be: start from original grid, clear all from positions,
    # then the rat moves are already computed. We reconstruct:
    # Actually the Rust code is:
    #   - begin_move modifies curr_grid (clears from, sets dest) for blocking checks
    #   - After all moves queued, reset curr_grid = prev_grid, then clear from positions only
    #   - finish_moving (called in resolve_all) places entities at their to positions
    #
    # In our python model, apply_rat_moves took working (which already has player at dest),
    # computed rat movements, and returned the updated working grid.
    # The working grid at this point has: players at dests, rats at new positions,
    # trigger/explosion NOT yet fired.
    #
    # Now fire triggers and explosions:
    players_killed = set()

    # Fire trigger if player1 stepped on one
    if p1_dest_cell is not None and is_trigger(p1_dest_cell) and new_p1_pos != p1_pos:
        working, pk = apply_trigger(working, p1_dest_cell, new_p1_pos)
        players_killed.update(pk)

    # Fire trigger if player2 stepped on one
    if two_player and p2_dest_cell is not None and is_trigger(p2_dest_cell) and new_p2_pos != p2_pos:
        working, pk = apply_trigger(working, p2_dest_cell, new_p2_pos)
        players_killed.update(pk)

    # Check if players were killed by trigger explosions
    if new_p1_pos in players_killed:
        return working, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, False, False
    if two_player and new_p2_pos is not None and new_p2_pos in players_killed:
        return working, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, False, False

    if not all_alive_after_rats:
        return working, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, False, False

    # Check win condition
    if count_rats(working) == 0:
        return working, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, True, True

    return working, new_p1_pos, new_p1_facing, new_p2_pos, new_p2_facing, False, True

def bfs_solve(csv_text, max_depth=30, verbose=True):
    """BFS to solve a 1-player level. Returns action string or None."""
    grid = parse_csv(csv_text)
    p1_pos, p1_facing = find_player1(grid)

    if p1_pos is None:
        print("No player found!")
        return None

    initial_rats = count_rats(grid)
    if initial_rats == 0:
        return ""

    if verbose:
        print(f"Starting BFS: player at {p1_pos} facing {p1_facing}, {initial_rats} rats")

    initial_state = encode_state(grid, p1_pos, p1_facing)

    queue = deque()
    queue.append((grid, p1_pos, p1_facing, []))
    visited = {initial_state}

    actions = ['N', 'S', 'E', 'W', None]
    action_chars = {'N': '^', 'S': 'v', 'E': '>', 'W': '<', None: '.'}

    level_count = 0

    while queue:
        curr_grid, curr_p1, curr_f1, path = queue.popleft()

        if len(path) >= max_depth:
            continue

        if verbose and len(path) == level_count:
            print(f"  BFS depth {level_count}, queue size {len(queue)+1}")
            level_count += 1

        for action in actions:
            new_grid, np1, nf1, _, _, won, alive = simulate_step(
                curr_grid, curr_p1, curr_f1, action)

            if not alive:
                continue

            new_path = path + [action]

            if won:
                action_str = ''.join(action_chars[a] for a in new_path)
                if verbose:
                    print(f"SOLVED! Path: {action_str}")
                return action_str

            state = encode_state(new_grid, np1, nf1)
            if state not in visited:
                visited.add(state)
                queue.append((new_grid, np1, nf1, new_path))

    if verbose:
        print(f"No solution found within depth {max_depth}")
    return None

def solve_file(csv_path, max_depth=30, verbose=True):
    with open(csv_path) as f:
        csv_text = f.read()
    return bfs_solve(csv_text, max_depth=max_depth, verbose=verbose)

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python simulate2.py <csv_file> [max_depth]")
        sys.exit(1)

    csv_path = sys.argv[1]
    max_depth = int(sys.argv[2]) if len(sys.argv) > 2 else 30

    result = solve_file(csv_path, max_depth=max_depth)
    if result is not None:
        print(f"Solution: {result}")
