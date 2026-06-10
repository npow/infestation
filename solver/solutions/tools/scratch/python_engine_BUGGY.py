#!/usr/bin/env python3
"""Solver for the Infestation puzzle game."""

import sys
import heapq
from collections import deque

# Direction constants (dx, dy)
N = (0, -1)
E = (1, 0)
S = (0, 1)
W = (-1, 0)
STALL = (0, 0)

ACTIONS = [N, E, S, W, STALL]
ACTION_NAMES = {N: '↑', E: '→', S: '↓', W: '←', STALL: '.'}
ACTION_KEYS  = {N: 'ArrowUp', E: 'ArrowRight', S: 'ArrowDown', W: 'ArrowLeft', STALL: 'Stall'}

DIR_OPPOSITE = {N: S, S: N, E: W, W: E, STALL: STALL}

# 8 directions
DIRS8 = [(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)]

# Cell constants
WALL = '#'
EMPTY = '.'
PLANK = '='
WEB = 'w'
HOLE = 'O'
EXPLOSIVE = 'X'
RAT = 'R'
CYBORG = 'C'

def player_cell(pid, direction):
    d = {(0,-1):'N',(1,0):'E',(0,1):'S',(-1,0):'W'}
    return f'P{pid}{d[direction]}'

def is_player(cell):
    return len(cell) == 3 and cell[0] == 'P' and cell[1] in '12'

_DIR_MAP = {'N':N,'E':E,'S':S,'W':W}

def player_dir(cell):
    return _DIR_MAP[cell[2]]

def player_id(cell):
    return int(cell[1])

def is_trigger(cell):
    return len(cell) == 1 and cell in '123456789'

def dist2(x1,y1,x2,y2):
    return (x1-x2)**2 + (y1-y2)**2

def parse_cell(c):
    c = c.strip()
    mp = {
        '#': WALL, '.': EMPTY, '=': PLANK, 'w': WEB,
        'O': HOLE, 'X': EXPLOSIVE, 'R': RAT, 'C': CYBORG,
        '▲': 'P1N', '▼': 'P1S', '►': 'P1E', '◄': 'P1W',
        '△': 'P2N', '▽': 'P2S', '▷': 'P2E', '◁': 'P2W',
    }
    if c in mp: return mp[c]
    if c in '123456789': return c
    return EMPTY

def parse_level(csv_text):
    grid = []
    for line in csv_text.strip().split('\n'):
        row = [parse_cell(c) for c in line.split(',')]
        grid.append(row)
    return grid

def grid_to_state(grid):
    return tuple(tuple(row) for row in grid)

def state_to_grid(state):
    return [list(row) for row in state]

def find_players_in(grid):
    players = {}
    H = len(grid); W = len(grid[0])
    for y in range(H):
        for x in range(W):
            c = grid[y][x]
            if is_player(c):
                players[player_id(c)] = (x, y, player_dir(c))
    return players

def has_rats_in(grid):
    for row in grid:
        for c in row:
            if c in (RAT, CYBORG): return True
    return False

def apply_action(state, actions):
    """
    Apply player actions and advance one full turn.
    actions: dict {pid: (dx,dy)}
    Returns: (new_state, result)  result in 'playing','won','lost'
    """
    grid = state_to_grid(state)
    H = len(grid); W = len(grid[0]) if H else 0

    def get(x,y):
        if 0<=x<W and 0<=y<H: return grid[y][x]
        return WALL

    def set_(x,y,v):
        if 0<=x<W and 0<=y<H: grid[y][x] = v

    def find_players():
        p = {}
        for y in range(H):
            for x in range(W):
                c = grid[y][x]
                if is_player(c):
                    p[player_id(c)] = (x,y,player_dir(c))
        return p

    def find_rats():
        return [(x,y) for y in range(H) for x in range(W) if grid[y][x]==RAT]

    def find_cyborgs():
        return [(x,y) for y in range(H) for x in range(W) if grid[y][x]==CYBORG]

    def has_rats():
        return any(c in (RAT,CYBORG) for row in grid for c in row)

    def chain_explosions(initials):
        queue = list(initials)
        processed = set()
        while queue:
            ex,ey = queue.pop(0)
            if (ex,ey) in processed: continue
            processed.add((ex,ey))
            for dx,dy in DIRS8:
                nx,ny = ex+dx, ey+dy
                cell = get(nx,ny)
                if cell in (RAT,CYBORG,PLANK,WEB):
                    set_(nx,ny,EMPTY)
                elif cell == EXPLOSIVE:
                    set_(nx,ny,EMPTY)
                    queue.append((nx,ny))
                elif is_player(cell):
                    set_(nx,ny,EMPTY)

    def activate_triggers(tnum, exclude=None):
        to_zap = []
        for y in range(H):
            for x in range(W):
                if grid[y][x] == tnum and (exclude is None or (x,y) != exclude):
                    to_zap.append((x,y))
        for x,y in to_zap:
            set_(x,y,WALL)
            for dx,dy in DIRS8:
                nx,ny = x+dx, y+dy
                cell = get(nx,ny)
                if cell == EMPTY:
                    set_(nx,ny,WALL)
                elif cell == EXPLOSIVE:
                    set_(nx,ny,EMPTY)
                    chain_explosions([(nx,ny)])

    def sword_blocked(move_dx, move_dy, dest_cell):
        """True if the move direction is blocked by the player's sword at dest_cell."""
        if not is_player(dest_cell): return False
        pd = player_dir(dest_cell)
        # Sword blocks if rat/entity moves in direction opposite to player facing
        # i.e., rat approaches FROM in front of the sword
        # rat move dir == player.dir (they're both pointing the same way)
        # Wait: if player faces N, sword is N. Rat at (px,py-1) moves S to (px,py).
        # move_dir = S = (0,1). player_dir = N = (0,-1).
        # S == DIR_OPPOSITE[N] = S → TRUE → blocked. Correct.
        return (move_dx, move_dy) == DIR_OPPOSITE[pd]

    # === STEP 1: PLAYER MOVEMENT ===
    players = find_players()
    if not players:
        return grid_to_state(grid), 'lost'

    # Compute intended moves
    intended = {}  # pid -> (ox,oy,nx,ny,nd)
    for pid,(ox,oy,od) in players.items():
        action = actions.get(pid, STALL)
        if action == STALL:
            intended[pid] = (ox,oy,ox,oy,od)
        else:
            dx,dy = action
            nx,ny = ox+dx, oy+dy
            dest = get(nx,ny)
            if dest in (WALL, PLANK):
                nx,ny = ox,oy  # blocked
            intended[pid] = (ox,oy,nx,ny,action if action!=STALL else od)

    # Resolve player-player conflicts (multi-player)
    if len(intended) >= 2:
        dest_to_pid = {}
        for pid,(ox,oy,nx,ny,nd) in list(intended.items()):
            if (nx,ny) in dest_to_pid:
                other = dest_to_pid[(nx,ny)]
                ox2,oy2,nx2,ny2,nd2 = intended[other]
                intended[other] = (ox2,oy2,ox2,oy2,nd2)
                intended[pid]   = (ox,oy,ox,oy,nd)
            else:
                dest_to_pid[(nx,ny)] = pid
        # Check swaps
        pids = list(intended.keys())
        for i in range(len(pids)):
            for j in range(i+1,len(pids)):
                a,b = pids[i],pids[j]
                oa,ob = intended[a], intended[b]
                if (oa[2],oa[3]) == (ob[0],ob[1]) and (ob[2],ob[3]) == (oa[0],oa[1]):
                    intended[a] = (oa[0],oa[1],oa[0],oa[1],oa[4])
                    intended[b] = (ob[0],ob[1],ob[0],ob[1],ob[4])

    # Clear player positions
    for pid,(ox,oy,od) in players.items():
        set_(ox,oy,EMPTY)

    # Place players at new positions
    surviving_pids = set()
    for pid,(ox,oy,nx,ny,nd) in intended.items():
        dest = get(nx,ny)
        if dest == HOLE:
            pass  # dies
        elif dest == EXPLOSIVE:
            set_(nx,ny,EMPTY)
            set_(nx,ny, player_cell(pid,nd))
            chain_explosions([(nx,ny)])
            if get(nx,ny) == player_cell(pid,nd):
                surviving_pids.add(pid)
        elif is_trigger(dest):
            tnum = dest
            set_(nx,ny, player_cell(pid,nd))
            activate_triggers(tnum, exclude=(nx,ny))
            if get(nx,ny) == player_cell(pid,nd):
                surviving_pids.add(pid)
        elif dest in (RAT, CYBORG):
            set_(nx,ny, player_cell(pid,nd))  # kills rat
            surviving_pids.add(pid)
        elif is_player(dest):
            # Player-player conflict not fully resolved; skip
            surviving_pids.add(pid)
        else:
            set_(nx,ny, player_cell(pid,nd))
            surviving_pids.add(pid)

    players = find_players()
    if not players:
        return grid_to_state(grid), 'lost'

    if not has_rats():
        return grid_to_state(grid), 'won'

    # === STEP 2: CYBORG RAT MOVEMENT (Dijkstra) ===
    cyborgs = find_cyborgs()
    if cyborgs:
        # Multi-source Dijkstra from all players
        # Distance metric: (ortho_steps, diag_steps), compared lexicographically
        INF = (10**9, 10**9)
        dist_map = {}
        pq = []
        for pid,(px,py,pd) in players.items():
            if (px,py) not in dist_map:
                dist_map[(px,py)] = (0,0)
                heapq.heappush(pq, (0,0,px,py))

        while pq:
            o,d,x,y = heapq.heappop(pq)
            if (o,d) > dist_map.get((x,y), INF): continue
            for dx,dy in DIRS8:
                nx,ny = x+dx, y+dy
                cell = get(nx,ny)
                if cell in (WALL, HOLE, WEB, EXPLOSIVE): continue
                is_diag = (dx!=0 and dy!=0)
                no = o + (0 if is_diag else 1)
                nd = d + (1 if is_diag else 0)
                if (no,nd) < dist_map.get((nx,ny), INF):
                    dist_map[(nx,ny)] = (no,nd)
                    heapq.heappush(pq, (no,nd,nx,ny))

        cyborgs.sort(key=lambda c: dist_map.get(c, INF))

        for cx,cy in cyborgs:
            if get(cx,cy) != CYBORG: continue
            cur = dist_map.get((cx,cy), INF)
            best_move = None
            best_dist = cur
            best_ortho = False

            for dx,dy in DIRS8:
                nx,ny = cx+dx, cy+dy
                dest = get(nx,ny)
                if dest in (WALL, WEB, CYBORG): continue
                if sword_blocked(dx,dy,dest): continue
                nd = dist_map.get((nx,ny), INF)
                is_ortho = (dx==0 or dy==0)
                if nd < best_dist or (nd == best_dist and is_ortho and not best_ortho):
                    best_dist = nd
                    best_move = (dx,dy)
                    best_ortho = is_ortho

            if best_move is None: continue
            mdx,mdy = best_move
            nx,ny = cx+mdx, cy+mdy
            dest = get(nx,ny)
            set_(cx,cy,EMPTY)
            if dest == RAT:
                set_(nx,ny,CYBORG)
            elif dest == EXPLOSIVE:
                set_(nx,ny,EMPTY)
                chain_explosions([(nx,ny)])
            elif is_trigger(dest):
                tnum = dest
                set_(nx,ny,EMPTY)
                activate_triggers(tnum, exclude=(nx,ny))
                if get(nx,ny) not in (WALL,):
                    set_(nx,ny,CYBORG)
            elif is_player(dest):
                set_(nx,ny,CYBORG)
            elif dest == HOLE:
                pass
            else:
                set_(nx,ny,CYBORG)

        players = find_players()
        if not players:
            return grid_to_state(grid), 'lost'

    if not has_rats():
        return grid_to_state(grid), 'won'

    # === STEP 3: NORMAL RAT MOVEMENT ===
    rats = find_rats()

    def rat_nearest_dist2(rx,ry):
        return min(dist2(rx,ry,px,py) for px,py,_ in players.values())

    rats.sort(key=lambda r: rat_nearest_dist2(r[0],r[1]))

    for rx,ry in rats:
        if get(rx,ry) != RAT: continue

        # Find nearest player
        nearest = min(players.values(), key=lambda p: dist2(rx,ry,p[0],p[1]))
        npx,npy,npd = nearest

        dx_raw = npx - rx
        dy_raw = npy - ry
        dx_n = 0 if dx_raw==0 else (1 if dx_raw>0 else -1)
        dy_n = 0 if dy_raw==0 else (1 if dy_raw>0 else -1)

        if dx_n != 0 and dy_n != 0:
            candidates = [(dx_n,dy_n),(dx_n,0),(0,dy_n)]
        else:
            candidates = [(dx_n,dy_n)]

        base_dist = dist2(rx,ry,npx,npy)
        best_move = None
        best_d2   = base_dist
        best_diag = True  # current best is "diagonal" (worse); ortho preferred

        for dx,dy in candidates:
            nx,ny = rx+dx, ry+dy
            dest = get(nx,ny)
            if dest in (WALL, WEB, RAT, CYBORG): continue
            if sword_blocked(dx,dy,dest): continue
            nd2 = dist2(nx,ny,npx,npy)
            is_diag = (dx!=0 and dy!=0)
            better = (nd2 < best_d2) or (nd2 == best_d2 and not is_diag and best_diag)
            if better:
                best_d2   = nd2
                best_move = (dx,dy)
                best_diag = is_diag

        if best_move is None: continue
        mdx,mdy = best_move
        nx,ny = rx+mdx, ry+mdy
        dest = get(nx,ny)
        set_(rx,ry,EMPTY)
        if dest == PLANK:
            set_(nx,ny,RAT)
        elif dest == EXPLOSIVE:
            set_(nx,ny,EMPTY)
            chain_explosions([(nx,ny)])
        elif is_trigger(dest):
            tnum = dest
            set_(nx,ny,EMPTY)
            activate_triggers(tnum, exclude=(nx,ny))
            set_(nx,ny,RAT)
        elif is_player(dest):
            set_(nx,ny,RAT)
        elif dest == HOLE:
            pass
        else:
            set_(nx,ny,RAT)

    players = find_players()
    if not players:
        return grid_to_state(grid), 'lost'

    if not has_rats():
        return grid_to_state(grid), 'won'

    return grid_to_state(grid), 'playing'


def solve_bfs(initial_grid, max_depth=60, verbose=False):
    """BFS for single-player levels."""
    state0 = grid_to_state(initial_grid)
    players0 = find_players_in(initial_grid)

    if not has_rats_in(initial_grid):
        return []

    queue = deque([(state0, [])])
    visited = {state0}
    nodes = 0

    while queue:
        state, path = queue.popleft()
        nodes += 1
        if nodes % 50000 == 0 and verbose:
            print(f"  BFS: {nodes} nodes, depth {len(path)}, queue {len(queue)}", file=sys.stderr)

        if len(path) >= max_depth:
            continue

        pids = list(find_players_in(state_to_grid(state)).keys())
        if not pids: continue

        for action in ACTIONS:
            action_dict = {pid: action for pid in pids}
            new_state, result = apply_action(state, action_dict)
            if result == 'won':
                return path + [action]
            elif result == 'playing' and new_state not in visited:
                visited.add(new_state)
                queue.append((new_state, path + [action]))

    return None


def solve_astar(initial_grid, max_depth=80, verbose=False):
    """A* solver — heuristic = number of remaining rats."""
    state0 = grid_to_state(initial_grid)

    def heuristic(state):
        return sum(1 for row in state for c in row if c in (RAT, CYBORG))

    if not has_rats_in(initial_grid):
        return []

    # (f, g, state, path)
    h0 = heuristic(state0)
    pq = [(h0, 0, state0, [])]
    visited = {}  # state -> best g

    nodes = 0
    while pq:
        f, g, state, path = heapq.heappop(pq)
        nodes += 1
        if nodes % 50000 == 0 and verbose:
            print(f"  A*: {nodes} nodes, g={g}, queue {len(pq)}", file=sys.stderr)

        if g > max_depth: continue
        if visited.get(state, 10**9) <= g: continue
        visited[state] = g

        pids = list(find_players_in(state_to_grid(state)).keys())
        if not pids: continue

        for action in ACTIONS:
            action_dict = {pid: action for pid in pids}
            new_state, result = apply_action(state, action_dict)
            ng = g + 1
            if result == 'won':
                return path + [action]
            elif result == 'playing' and visited.get(new_state, 10**9) > ng:
                nh = heuristic(new_state)
                heapq.heappush(pq, (ng + nh, ng, new_state, path + [action]))

    return None


# ===== LEVEL DATA =====

LEVELS = {}

LEVELS['rats'] = """\
.,.,.,R,.,.
.,#,#,#,#,.
.,#,.,.,.,#
#,.,#,#,#,.
#,.,.,.,.,.
#,.,#,#,#,#
#,.,#,▲,.,#
#,.,.,.,.,#"""

LEVELS['more_rats'] = """\
.,.,.,.,.,#,R
R,#,.,.,.,#,R
R,#,.,.,.,#,R
R,#,.,.,.,#,R
R,#,.,.,.,#,R
R,#,.,▲,.,#,R
R,#,.,.,.,.,.
"""

LEVELS['webs'] = """\
w,.,R,w,w,w,w,w,w,w,w,w
w,.,.,w,w,#,w,w,w,w,w,w
w,#,#,w,w,#,w,w,R,R,w,w
w,#,#,w,w,#,w,w,R,R,w,w
R,w,w,w,w,#,w,w,R,w,w,w
w,#,#,#,#,#,w,w,R,R,w,w
w,.,▲,w,w,#,w,w,w,w,w,w
#,#,#,#,#,#,w,w,w,w,w,w"""

LEVELS['trapped_rat'] = """\
.,.,.,.,R,.,#,.,.,.
.,.,.,.,#,.,#,.,.,.
#,#,#,#,.,#,.,#,.,.
.,#,#,.,.,#,.,.,#,.
.,#,#,.,.,.,#,#,#,.
.,#,#,#,#,.,.,#,#,.
.,.,.,.,#,#,#,#,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,▲,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,.,.,.,.,.,."""

LEVELS['trapped_rat2_v2'] = """\
.,.,#,.,R,.,#,#,.,.
.,#,.,#,#,.,#,.,#,.
.,#,#,.,.,#,#,#,.,.
.,#,#,.,.,#,#,.,#,.
.,#,.,.,#,#,.,#,#,.
.,#,#,#,#,.,.,.,#,.
.,.,.,.,#,#,#,#,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,▲,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,.,.,.,.,.,#,R
R,#,.,#,#,.,.,.,.,."""

LEVELS['planks'] = """\
.,w,w,w,=,#,.,.,.,.,.,.,#,.
.,#,#,=,=,#,.,#,#,.,.,.,#,.
.,.,#,#,.,#,.,.,#,R,.,#,#,.
.,.,.,#,.,#,#,.,#,R,.,#,.,.
.,.,.,#,.,.,#,#,#,#,.,#,.,.
.,.,.,#,.,.,.,.,.,.,.,#,.,.
.,.,.,#,#,#,#,#,#,#,#,#,.,.
.,.,.,.,.,.,▲,.,.,.,.,.,.,."""

LEVELS['explosives'] = """\
#,X,X,X,X,X,#,#,#
X,.,.,R,.,.,=,w,R
X,w,#,#,#,#,.,w,#
X,.,.,.,X,#,#,w,.
X,.,X,.,X,▼,#,.,.
X,.,X,.,X,.,#,.,.
.,.,X,.,X,.,#,.,.
#,.,X,.,.,.,#,.,.
#,.,#,#,#,#,#,.,.
#,.,.,.,.,.,.,.,.
#,#,#,#,#,.,.,.,.
.,.,.,.,.,.,.,.,.

"""

LEVELS['explosives2'] = """\
#,X,X,X,X,X,#,#,#
X,.,.,R,.,.,.,w,R
X,w,#,#,#,#,.,w,#
X,.,▼,.,X,#,#,w,.
X,.,X,X,X,.,#,.,.
X,.,X,w,w,w,#,.,.
.,.,X,w,R,w,#,.,.
#,.,X,w,w,w,#,.,.
#,.,#,#,#,#,#,.,.
#,.,.,.,.,.,.,.,.
#,#,#,#,#,.,.,.,.
.,.,.,.,.,.,.,.,.
"""

LEVELS['tinderbox'] = """\
.,X,X,X,X,X,X,X,X,X,.
.,R,R,R,R,R,R,R,R,R,X
X,w,w,w,w,w,w,w,w,w,X
X,.,.,w,w,w,w,w,w,X,X
X,.,w,w,w,w,w,w,w,X,X
X,R,w,w,w,w,.,.,w,X,X
X,#,#,X,X,X,X,.,w,X,X
X,X,X,#,#,#,#,.,w,w,X
X,w,w,w,w,w,w,w,w,w,X
X,w,w,X,X,w,▲,w,w,.,X
X,w,w,w,w,w,w,w,R,.,X
.,X,X,X,X,X,X,X,X,X,."""

LEVELS['tinderrectangle'] = """\
.,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,.
X,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,X
X,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,X
X,.,.,w,w,w,w,w,w,w,w,#,w,w,w,w,X
X,.,w,w,w,w,w,w,#,w,w,#,w,#,w,w,X
X,.,w,w,w,w,w,w,#,#,w,#,w,#,w,w,X
X,R,w,w,w,w,w,w,◄,#,w,w,w,#,w,w,X
X,#,#,X,X,X,X,X,#,#,#,#,#,#,w,w,X
X,X,X,#,#,#,#,#,X,X,X,X,X,w,w,w,X"""

LEVELS['blackhole_v2'] = """\
.,.,.,.,.,O,.,O
.,.,w,#,O,O,.,O
.,O,w,=,w,.,.,O
.,O,#,.,O,O,.,O
.,.,#,.,O,.,.,.
.,.,#,.,O,w,#,O
.,.,#,.,#,w,.,#
.,.,#,.,#,.,.,#
.,O,#,.,#,#,.,#
.,#,.,.,.,#,#,R
.,#,#,.,.,.,#,#
.,.,#,.,.,.,.,#
.,.,#,.,#,#,#,.
.,O,#,O,.,#,#,#
►,#,R,.,.,.,#,#
.,#,#,#,#,#,.,#
.,.,.,.,.,#,#,#
.,.,.,.,.,.,.,.
"""

LEVELS['synchronicity'] = """\
R,R,R,R,R,R,R,R,2,1
w,w,w,w,w,w,w,w,X,X
X,X,X,X,X,X,X,X,#,#
.,.,#,#,#,#,#,#,#,R
.,.,w,X,X,#,#,#,w,w
.,.,w,1,1,.,.,.,2,1
.,.,w,X,X,#,#,.,.,w
►,.,w,2,2,.,.,.,.,R
.,.,w,.,#,#,#,O,.,w
.,.,#,w,w,R,#,.,.,w"""

LEVELS['release'] = """\
R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R,R
w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w,w
.,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X
X,.,.,.,.,.,#,#,#,#,.,.,.,.,.,.,.,#,#,#
X,.,w,w,w,w,#,.,.,#,.,.,.,#,.,.,.,#,R,#
X,.,.,.,.,.,#,.,.,#,.,#,#,.,#,R,.,#,w,#
.,X,X,X,.,.,#,.,.,#,.,.,#,#,#,#,.,#,X,.
.,.,.,.,X,.,#,.,.,#,#,.,.,.,#,#,.,#,X,2
w,w,.,.,X,.,#,.,.,#,#,5,4,3,#,#,.,#,X,.
R,w,.,.,X,.,#,.,.,.,#,.,.,.,#,#,.,#,O,O
.,w,.,.,X,.,#,.,▲,.,#,#,#,.,#,#,.,#,#,#
.,w,.,.,X,.,#,.,.,.,.,.,#,=,#,#,.,.,.,.
.,w,.,O,.,.,#,3,#,4,#,5,#,.,#,#,.,.,.,.
.,.,.,O,.,.,#,.,w,.,w,.,#,.,#,#,.,.,.,.
.,.,.,O,.,.,.,.,.,.,.,.,.,.,#,#,.,.,.,.
O,O,O,O,O,O,O,O,O,O,O,O,O,O,O,O,#,#,O,.
2,X,.,w,w,w,w,w,w,w,w,w,w,w,w,w,#,6,#,.
X,O,w,.,.,.,.,.,.,.,.,.,.,.,.,.,=,.,.,.
6,O,.,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,6
.,O,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,R,#,."""

LEVELS['lock_in'] = """\
R,w,5,.,.,.,.,.,.,.,.,#,.,.,#,2,.,.,.,.,.,.,.,.,.
#,#,#,#,#,#,#,#,#,#,.,#,=,.,.,#,2,.,.,.,.,.,.,.,.
R,.,.,.,.,.,w,X,1,=,.,=,=,.,.,#,#,2,.,.,.,.,.,.,.
#,.,.,.,w,w,w,=,=,=,.,=,X,.,.,.,#,#,2,.,.,.,.,.,.
#,#,.,.,.,w,X,1,=,=,.,=,X,.,.,.,.,#,#,2,.,.,.,.,.
.,#,#,.,w,w,w,=,=,=,.,=,X,.,.,.,.,.,#,#,2,.,.,.,.
O,#,#,#,.,.,w,X,1,=,▲,=,X,.,.,.,.,.,.,#,#,2,.,.,.
1,#,#,#,#,#,#,=,=,.,.,=,X,.,#,.,.,.,.,.,#,#,2,=,=
X,3,O,.,.,#,.,.,.,.,#,#,X,.,#,.,.,.,.,.,.,#,#,3,X
w,#,=,#,.,#,.,.,.,.,#,R,X,.,#,.,.,.,.,.,.,.,#,#,w
R,#,.,#,.,#,w,w,w,4,#,#,#,3,#,.,.,.,.,.,.,.,.,#,R
#,#,=,#,.,#,w,5,X,.,4,.,.,.,#,.,.,.,.,.,.,.,.,#,#
X,7,=,#,.,#,.,7,.,5,.,#,#,#,#,#,#,#,#,#,#,#,#,#,#
w,w,6,#,.,#,.,.,.,.,#,=,=,X,w,R,O,O,O,O,O,O,O,O,O
R,w,.,#,.,O,.,.,.,.,.,O,6,O,O,O,O,O,O,O,O,O,O,O,O"""

LEVELS['no_retreat'] = """\
.,.,.,.,.,.,.,.,.,.,#,#,#,#,#,.,.,w,=,.
#,#,#,#,#,#,#,#,#,#,#,#,.,.,w,=,.,#,#,#,.
.,.,.,.,.,.,.,.,.,.,w,=,.,#,#,#,.,#,X,#,.
.,#,#,#,#,#,#,#,#,#,#,#,.,#,X,#,.,#,R,w,.
.,.,.,.,.,.,.,.,.,#,X,#,.,#,R,w,.,#,#,#,.
#,#,#,#,#,#,#,#,.,#,R,w,.,#,#,#,.,#,X,#,.
.,.,.,.,.,.,.,.,.,#,#,#,.,#,X,#,.,#,R,w,.
.,=,=,=,=,=,=,=,=,#,X,#,.,#,R,w,.,#,#,#,.
.,.,.,.,.,=,.,.,.,#,R,w,.,#,#,#,.,#,X,#,.
#,.,.,.,.,.,.,=,.,#,#,#,.,#,X,#,.,#,R,w,.
#,#,#,=,=,=,.,=,.,#,X,#,.,#,R,w,.,#,#,#,.
#,.,#,.,.,.,.,.,.,#,R,w,.,#,#,#,.,#,X,#,.
.,#,#,.,R,=,=,R,.,#,#,#,.,#,X,#,.,#,R,w,.
#,.,w,=,#,#,#,#,=,#,X,#,.,#,R,w,.,#,#,#,.
.,#,#,.,.,.,.,.,.,#,R,w,.,#,#,#,.,#,X,#,.
#,R,#,.,.,.,.,.,▲,#,#,#,R,#,X,#,R,#,R,w,R"""

LEVELS['triggers'] = """\
.,.,.,.,w,w,=,.,.,.,.,.
w,w,w,w,w,#,=,.,#,.,.,.
w,1,w,w,1,#,#,.,#,.,.,.
w,.,R,.,#,#,2,.,#,#,.,.
w,#,#,#,#,.,.,#,1,.,.,.
.,.,.,.,.,.,#,#,#,#,#,.
.,.,.,2,.,.,#,.,.,.,.,.
.,.,.,.,.,.,#,.,#,#,#,#
.,.,.,.,.,.,#,.,.,.,.,#
.,.,▲,#,#,#,#,#,.,=,.,.
2,.,.,#,.,.,.,#,=,1,=,.
.,.,.,#,.,#,.,#,.,=,.,.
.,.,.,.,.,#,.,.,.,.,.,.
"""

LEVELS['triggers2'] = """\
.,.,#,.,.,#,R,#,.,.,.,w,R,#
.,#,#,.,.,#,w,#,.,.,.,#,#,4
.,w,R,#,.,#,1,#,.,#,.,.,#,#
.,#,#,#,.,#,.,#,.,#,#,.,.,#
.,.,#,.,#,#,.,#,.,#,#,#,.,#
.,.,#,#,#,.,.,.,.,.,#,#,.,#
.,.,.,.,.,.,.,.,.,.,#,#,.,#
.,.,.,.,.,.,.,.,.,.,#,#,.,#
.,.,.,.,▲,.,.,.,.,.,#,.,.,.
#,O,#,.,.,.,.,.,#,#,#,#,#,#
#,1,#,2,.,#,#,#,#,4,#,#,R,R
.,#,#,.,2,.,3,#,#,.,.,.,.,.
O,R,#,.,#,3,w,.,w,.,.,.,.,.
"""

LEVELS['order_of_operations_new_v2'] = """\
#,#,#,#,#,#,#,#,#,#,X,X,#,#,.
.,X,X,X,X,#,#,.,.,.,2,#,R,3,.
X,.,.,.,w,R,#,.,#,.,.,w,R,3,.
X,.,#,1,#,#,#,.,#,.,.,w,#,.,X
X,.,►,#,#,#,1,.,#,O,w,.,X,4,X
.,X,.,.,.,.,.,.,#,2,#,.,X,.,X
.,#,X,.,.,#,.,.,#,.,w,.,X,.,X
.,#,X,.,w,#,.,.,#,.,w,.,X,.,X
.,#,X,w,w,#,1,.,#,.,w,.,X,.,X
.,#,#,1,w,#,.,.,#,.,#,.,X,.,X
.,#,#,.,w,#,.,.,#,.,#,.,X,.,X
.,#,#,.,w,#,1,.,#,.,#,.,X,4,X
.,#,#,.,w,#,.,#,#,.,#,#,#,.,X
.,#,#,.,w,#,.,#,R,.,#,#,#,.,X
.,#,#,.,w,#,1,.,.,.,.,#,#,.,X
#,#,R,.,O,#,.,.,.,2,.,#,R,.,X"""

LEVELS['guidance'] = """\
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,▼,.,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,#,#,#,#,#,#,#,#,#,#,#,#,#,#,.,.,.,.
.,.,#,.,.,.,.,#,.,.,.,.,.,.,R,#,.,.,.,.
.,.,#,.,#,O,.,#,#,.,.,.,.,#,#,#,.,.,.,.
.,.,#,.,#,O,.,#,O,O,.,O,O,O,O,#,.,.,.,.
.,.,#,.,#,O,.,#,.,.,.,#,.,.,.,#,.,.,.,.
.,.,#,.,#,O,.,#,O,.,O,#,#,#,O,#,.,.,.,.
.,.,#,.,#,O,.,#,#,.,.,.,.,#,#,#,.,.,.,.
.,.,#,.,#,O,.,#,.,#,#,#,#,.,.,#,.,.,.,.
.,.,#,.,#,O,.,#,O,O,.,.,#,#,.,#,.,.,.,.
.,.,#,.,#,w,.,#,.,.,#,#,.,#,.,#,.,.,.,.
.,.,#,.,#,.,.,.,O,#,.,#,.,.,.,#,.,.,.,.
.,.,#,.,#,#,#,#,#,#,#,#,#,#,#,#,.,.,.,.
.,.,#,.,.,.,.,.,.,.,.,.,.,w,R,#,.,.,.,.
.,.,#,#,#,#,#,#,=,#,#,#,#,#,#,#,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
"""

LEVELS['limited2'] = """\
.,.,.,.,.,R,R,R,R,R,R,.,.,.,.,.,.,.,.
.,.,.,.,1,#,#,#,#,#,#,#,#,#,#,#,#,#,.
#,#,#,X,.,.,2,#,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,2,O,.,.,.,.,.,.,.,#,R,#,.
.,.,.,X,3,#,#,#,.,.,.,.,.,.,.,#,#,#,.
.,.,.,X,.,.,4,#,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,4,O,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,5,#,#,#,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,6,#,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,6,O,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,#,#,#,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,X,.,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,X,X,X,X,X,X,X,X,X,X,X,X,#,#,#,#,.
.,.,X,.,.,.,.,.,.,.,.,5,.,.,3,.,.,1,.
.,X,.,.,.,.,.,.,#,.,.,#,.,.,#,.,.,#,.
.,X,.,.,.,.,.,.,#,.,6,#,.,4,#,.,2,#,.
.,X,.,.,.,.,.,.,#,O,#,#,O,#,#,O,#,#,.
.,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X
.,.,.,=,▼,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,X,.,X,X,X,X,X,X,X,X,X,X,X,X,X,X,X
.,5,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
"""

LEVELS['reload_v3'] = """\
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,.
.,#,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,#,R,.,.,.,.
.,#,.,.,=,.,.,.,#,#,#,#,#,#,#,#,.,.,#,#,.,.,.,.
.,#,.,=,4,=,.,.,#,6,#,.,.,.,.,#,.,.,.,.,.,.,.,.
.,#,.,.,=,.,.,.,#,5,w,.,#,#,R,#,.,.,.,.,.,.,.,.
.,#,.,.,.,.,.,.,#,X,#,#,O,#,#,#,.,.,.,.,.,.,.,.
.,#,.,.,.,.,.,.,#,#,#,#,#,#,#,#,w,#,#,#,.,.,.,.
.,#,.,.,.,.,.,7,.,.,.,w,w,w,w,w,w,#,#,#,.,.,.,.
.,#,.,.,.,.,.,.,#,7,#,#,#,#,#,#,#,#,#,#,.,.,.,.
.,#,.,.,=,.,.,.,#,#,#,#,#,#,#,#,#,#,#,#,.,.,.,.
.,#,.,=,3,=,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,#,.,.,=,.,.,.,.,.,.,.,.,.,.,.,.,=,.,.,.,.,.,.
.,#,.,.,.,.,.,.,.,.,.,.,.,.,.,.,=,2,=,.,.,.,.,.
.,#,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,=,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
.,.,.,.,.,.,.,.,.,.,.,.,.,▲,.,.,.,.,#,.,.,.,.,.
.,.,.,.,=,.,.,.,.,.,.,.,.,.,.,.,#,.,.,#,.,.,.,.
.,.,.,=,1,=,.,.,.,.,.,.,.,.,.,.,#,#,#,.,.,.,.,.
.,.,.,.,=,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.,.
#,#,#,.,.,.,.,.,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#,#
R,w,X,w,.,.,.,.,#,1,#,#,2,#,#,3,#,#,4,#,#,#,#,#
#,#,6,#,.,.,.,.,w,X,w,w,X,w,w,X,w,w,X,w,5,#,#,#
"""

LEVELS['triggering_explosives_v3'] = """\
.,.,3,X,2,X,X,X,X,X
#,#,#,.,.,.,#,R,R,.
w,2,.,.,#,.,#,#,#,.
.,.,#,X,.,.,◄,.,#,#
=,1,X,#,#,.,#,.,X,4
.,.,#,3,#,.,#,.,X,3
w,w,#,#,#,.,#,.,X,#
X,X,1,.,.,.,#,.,.,.
w,w,#,.,#,.,#,#,#,w
R,R,#,.,#,.,4,#,.,w
w,w,#,.,#,#,4,#,#,.
.,.,.,.,.,.,.,w,R,#
w,w,#,#,#,#,4,#,4,#
"""

LEVELS['chase'] = """\
.,.,O,.,.,.,.,.,.,.,R,#,X,#,#,2,w,.,.,.
3,.,O,.,#,#,#,#,#,#,#,w,#,X,#,R,#,.,#,.
.,1,O,#,#,#,#,#,#,#,#,.,#,X,#,#,#,.,#,.
.,.,.,.,.,.,.,.,.,.,1,.,#,R,#,.,.,.,#,.
.,.,.,.,.,.,.,.,.,.,.,.,#,#,#,.,.,.,#,.
.,.,.,.,.,.,.,.,.,.,O,.,O,3,.,.,.,.,#,.
.,.,.,.,.,.,.,O,.,.,.,.,.,3,.,.,.,.,2,.
.,.,.,.,.,.,.,O,.,.,.,.,.,3,.,.,.,.,#,w
.,.,.,.,.,.,.,#,w,w,w,w,w,3,.,.,.,.,#,.
.,.,.,.,.,.,.,#,X,X,X,X,X,3,.,.,.,.,#,.
.,.,.,.,.,.,.,.,=,=,=,=,=,#,#,.,.,.,#,.
.,.,.,.,.,w,.,.,4,.,5,.,.,.,.,.,.,.,#,.
.,.,.,.,R,w,.,.,.,4,.,5,.,.,.,.,.,.,#,.
.,.,.,.,.,w,.,.,.,.,.,.,.,.,.,.,.,.,#,.
.,.,.,.,.,w,.,.,.,.,#,.,.,.,.,.,.,.,#,.
.,.,.,.,#,.,.,#,w,#,#,=,=,.,.,.,.,.,#,.
#,#,#,#,#,.,.,#,=,#,#,w,#,.,.,.,.,.,#,.
.,.,▲,.,.,.,.,#,R,#,#,R,#,.,.,.,.,.,#,R
#,#,4,X,3,w,.,#,#,#,#,#,#,.,.,.,.,2,w,w
R,w,4,X,w,w,.,.,.,.,.,.,.,.,.,.,.,.,.,1
"""


if __name__ == '__main__':
    import time

    # Solve simpler levels first
    simple_levels = [
        'rats', 'webs', 'trapped_rat', 'trapped_rat2_v2',
        'planks', 'explosives', 'explosives2',
        'tinderbox', 'tinderrectangle',
        'blackhole_v2', 'synchronicity',
        'triggers', 'triggers2', 'triggering_explosives_v3',
        'order_of_operations_new_v2',
        'guidance', 'no_retreat',
        'more_rats',
        'limited2', 'reload_v3', 'release', 'lock_in', 'chase',
    ]

    all_solutions = {}

    for name in simple_levels:
        if name not in LEVELS:
            print(f"\n[{name}] Level data not available")
            continue

        csv_text = LEVELS[name]
        grid = parse_level(csv_text)

        players = find_players_in(grid)
        rats_start = sum(1 for row in grid for c in row if c in (RAT, CYBORG))
        W = len(grid[0]); H = len(grid)
        print(f"\n{'='*60}")
        print(f"Level: {name}  ({W}×{H} grid, {rats_start} rats, {len(players)} player(s))")

        t0 = time.time()
        max_depth = 40

        # For small grids with few rats, try BFS first
        solution = solve_astar(grid, max_depth=max_depth, verbose=True)
        elapsed = time.time() - t0

        if solution:
            moves = [ACTION_NAMES[a] for a in solution]
            keys  = [ACTION_KEYS[a] for a in solution]
            all_solutions[name] = solution
            print(f"  SOLVED in {len(solution)} moves ({elapsed:.1f}s)")
            print(f"  Moves: {' '.join(moves)}")
            print(f"  Keys:  {' '.join(keys)}")
        else:
            print(f"  No solution found (depth={max_depth}, time={elapsed:.1f}s)")

    print(f"\n\n{'='*60}")
    print(f"Summary: solved {len(all_solutions)}/{len(simple_levels)} levels")
    for name, sol in all_solutions.items():
        moves = ''.join(ACTION_NAMES[a] for a in sol)
        print(f"  {name}: {len(sol)} moves — {moves}")
