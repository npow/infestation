"""
IDA* solver for Infestation game.
"""
import sys
sys.path.insert(0, '/tmp/infestation')
from simulate2 import (parse_csv, find_player1, find_player2, simulate_step,
                        count_rats, encode_state, find_all_rats, dist_sq)
import math

def heuristic(grid, p1_pos, rat_positions):
    """Min distance from player to nearest rat (lower bound on steps needed)."""
    if not rat_positions:
        return 0
    px, py = p1_pos
    min_dist = min(math.sqrt(dist_sq(px, py, rx, ry)) for rx, ry in rat_positions)
    return max(0, int(min_dist) - 1)  # very weak heuristic

def idas_search(csv_path, max_cost=40):
    with open(csv_path) as f:
        grid = parse_csv(f.read())
    
    p1_pos, p1_facing = find_player1(grid)
    initial_rats = count_rats(grid)
    
    print(f"IDA*: player={p1_pos}, {initial_rats} rats")
    
    if initial_rats == 0:
        return ""
    
    actions = ['N', 'S', 'E', 'W', None]
    action_chars = {'N': '^', 'S': 'v', 'E': '>', 'W': '<', None: '.'}
    
    rat_positions = find_all_rats(grid)
    bound = heuristic(grid, p1_pos, rat_positions)
    path = []
    
    found = [None]
    
    def search(grid, p1_pos, p1_facing, g, bound, path, visited):
        rat_positions = find_all_rats(grid)
        h = heuristic(grid, p1_pos, rat_positions)
        f = g + h
        
        if f > bound:
            return f
        
        if count_rats(grid) == 0:
            found[0] = list(path)
            return -1
        
        if g >= max_cost:
            return float('inf')
        
        minimum = float('inf')
        
        for action in actions:
            new_grid, np1, nf1, _, _, won, alive = simulate_step(
                grid, p1_pos, p1_facing, action)
            
            if not alive:
                continue
            
            state = encode_state(new_grid, np1, nf1)
            if state in visited:
                continue
            
            path.append(action)
            visited.add(state)
            
            t = search(new_grid, np1, nf1, g + 1, bound, path, visited)
            
            visited.discard(state)
            path.pop()
            
            if t == -1:
                return -1
            if t < minimum:
                minimum = t
        
        return minimum
    
    while bound <= max_cost:
        print(f"  IDA* bound={bound}")
        visited = {encode_state(grid, p1_pos, p1_facing)}
        t = search(grid, p1_pos, p1_facing, 0, bound, path, visited)
        
        if found[0] is not None:
            action_str = ''.join(action_chars[a] for a in found[0])
            print(f"SOLVED! Path: {action_str}")
            return action_str
        
        if t == float('inf'):
            print("No solution found")
            return None
        
        bound = t
    
    return None

if __name__ == '__main__':
    result = idas_search(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 40)
    if result:
        print(f"Solution: {result}")
