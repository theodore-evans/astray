use geo_types::Coord;
use pathfinding::prelude::astar;

fn grid_neighbors(
    x: usize,
    y: usize,
    grid_w: usize,
    grid_h: usize,
    blocked: &dyn Fn(usize, usize) -> bool,
    cardinal_only: bool,
) -> Vec<((usize, usize), u32)> {
    let mut neighbors = Vec::with_capacity(8);
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            // Skip diagonals in cardinal-only mode.
            if cardinal_only && dx != 0 && dy != 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as usize) < grid_w && (ny as usize) < grid_h {
                let (nx, ny) = (nx as usize, ny as usize);
                if blocked(nx, ny) {
                    continue;
                }
                // Prevent diagonal corner-cutting: both cardinal neighbors must be free.
                if dx != 0 && dy != 0 {
                    let cx = (x as i32 + dx) as usize;
                    let cy = (y as i32 + dy) as usize;
                    if blocked(x, cy) || blocked(cx, y) {
                        continue;
                    }
                }
                let cost = if dx.abs() + dy.abs() == 2 { 14 } else { 10 };
                neighbors.push(((nx, ny), cost));
            }
        }
    }
    neighbors
}

/// A tagged path segment: greedy (Cut) or link (Link).
pub struct PathSegment {
    pub points: Vec<(usize, usize)>,
    pub is_link: bool,
}

/// Grid-based A* pathfinding (shortest mode).
pub fn find_path(
    grid_w: usize,
    grid_h: usize,
    start: (usize, usize),
    goal: (usize, usize),
    blocked: &dyn Fn(usize, usize) -> bool,
    cardinal_only: bool,
) -> Option<Vec<(usize, usize)>> {
    let result = astar(
        &start,
        |&(x, y)| grid_neighbors(x, y, grid_w, grid_h, blocked, cardinal_only),
        |&(x, y)| {
            let dx = (x as i32 - goal.0 as i32).unsigned_abs();
            let dy = (y as i32 - goal.1 as i32).unsigned_abs();
            (dx + dy) as u32 * 10
        },
        |pos| *pos == goal,
    );
    result.map(|(path, _cost)| path)
}

/// Longest path: greedy walk (furthest from goal) with A* links when stuck.
/// Returns tagged segments so links can be drawn in a different color.
pub fn find_longest_path(
    grid_w: usize,
    grid_h: usize,
    start: (usize, usize),
    goal: (usize, usize),
    blocked: &dyn Fn(usize, usize) -> bool,
    cardinal_only: bool,
) -> Option<Vec<PathSegment>> {
    let mut visited = vec![vec![false; grid_w]; grid_h];
    let mut segments: Vec<PathSegment> = Vec::new();
    let mut current_greedy: Vec<(usize, usize)> = vec![start];
    visited[start.1][start.0] = true;
    let mut pos = start;

    // Track used diagonal edges to prevent visual crossings.
    // Key: (min_x, min_y, is_backslash) where backslash = same-sign dx/dy.
    use std::collections::HashSet;
    let mut used_diags: HashSet<(usize, usize, bool)> = HashSet::new();

    loop {
        if pos == goal {
            if current_greedy.len() >= 2 {
                segments.push(PathSegment { points: current_greedy, is_link: false });
            }
            return Some(segments);
        }

        // Collect unvisited, unblocked neighbors.
        let mut neighbors: Vec<(usize, usize)> = Vec::new();
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                if cardinal_only && dx != 0 && dy != 0 { continue; }
                let nx = pos.0 as i32 + dx;
                let ny = pos.1 as i32 + dy;
                if nx >= 0 && ny >= 0 && (nx as usize) < grid_w && (ny as usize) < grid_h {
                    let (nx, ny) = (nx as usize, ny as usize);
                    if !blocked(nx, ny) && !visited[ny][nx] {
                        // Corner-cutting check for diagonals.
                        if dx != 0 && dy != 0 {
                            if blocked(pos.0, ny) || blocked(nx, pos.1) {
                                continue;
                            }
                            // Check if the crossing diagonal is already used.
                            let bx = pos.0.min(nx);
                            let by = pos.1.min(ny);
                            let is_backslash = (dx > 0) == (dy > 0);
                            // The crossing diagonal is the opposite type in the same 2x2 block.
                            if used_diags.contains(&(bx, by, !is_backslash)) {
                                continue;
                            }
                        }
                        neighbors.push((nx, ny));
                    }
                }
            }
        }

        if !neighbors.is_empty() {
            // Greedy: pick neighbor furthest from goal.
            neighbors.sort_by(|a, b| {
                let da = (a.0 as i32 - goal.0 as i32).pow(2) + (a.1 as i32 - goal.1 as i32).pow(2);
                let db = (b.0 as i32 - goal.0 as i32).pow(2) + (b.1 as i32 - goal.1 as i32).pow(2);
                db.cmp(&da)
            });
            let next = neighbors[0];
            visited[next.1][next.0] = true;
            // Record diagonal edge if applicable.
            let ddx = next.0 as i32 - pos.0 as i32;
            let ddy = next.1 as i32 - pos.1 as i32;
            if ddx != 0 && ddy != 0 {
                let bx = pos.0.min(next.0);
                let by = pos.1.min(next.1);
                let is_backslash = (ddx > 0) == (ddy > 0);
                used_diags.insert((bx, by, is_backslash));
            }
            current_greedy.push(next);
            pos = next;
            continue;
        }

        // Stuck. Save current greedy segment.
        if current_greedy.len() >= 2 {
            segments.push(PathSegment { points: current_greedy, is_link: false });
        }

        // Find nearest unvisited cell reachable via A* (allowing visited cells).
        // Prefer cells far from the goal to keep the wandering going.
        let link_target = find_nearest_unvisited(grid_w, grid_h, pos, goal, blocked, &visited, cardinal_only);

        if let Some((target, link_path)) = link_target {
            // Strip cells that overlap with visited greedy path, keeping only
            // the portions that pass through unvisited territory.
            add_link_segments(&link_path, &visited, &mut segments);
            visited[target.1][target.0] = true;
            current_greedy = vec![target];
            pos = target;
        } else {
            // No unvisited cells reachable. Try to link directly to goal.
            let result = astar(
                &pos,
                |&(x, y)| grid_neighbors(x, y, grid_w, grid_h, blocked, cardinal_only),
                |&(x, y)| {
                    let dx = (x as i32 - goal.0 as i32).unsigned_abs();
                    let dy = (y as i32 - goal.1 as i32).unsigned_abs();
                    (dx + dy) as u32 * 10
                },
                |p| *p == goal,
            );
            if let Some((link_path, _)) = result {
                add_link_segments(&link_path, &visited, &mut segments);
            }
            return Some(segments);
        }
    }
}

/// Add link path segments, stripping any cells already visited by the greedy walk.
/// This prevents visual overlap between link and greedy segments.
fn add_link_segments(
    link_path: &[(usize, usize)],
    visited: &[Vec<bool>],
    segments: &mut Vec<PathSegment>,
) {
    let mut current_run: Vec<(usize, usize)> = Vec::new();
    for &(x, y) in link_path {
        if visited[y][x] {
            // This cell overlaps with greedy path — break the run.
            if current_run.len() >= 2 {
                segments.push(PathSegment { points: current_run, is_link: true });
            }
            current_run = Vec::new();
        } else {
            current_run.push((x, y));
        }
    }
    if current_run.len() >= 2 {
        segments.push(PathSegment { points: current_run, is_link: true });
    }
}

/// Find the nearest unvisited cell from `pos` using A* over all passable cells.
/// Returns the target cell and the A* path to reach it.
fn find_nearest_unvisited(
    grid_w: usize,
    grid_h: usize,
    pos: (usize, usize),
    goal: (usize, usize),
    blocked: &dyn Fn(usize, usize) -> bool,
    visited: &[Vec<bool>],
    cardinal_only: bool,
) -> Option<((usize, usize), Vec<(usize, usize)>)> {
    // BFS from pos to find closest unvisited cell.
    use std::collections::VecDeque;
    let mut seen = vec![vec![false; grid_w]; grid_h];
    let mut parent: Vec<Vec<Option<(usize, usize)>>> = vec![vec![None; grid_w]; grid_h];
    let mut queue = VecDeque::new();
    queue.push_back(pos);
    seen[pos.1][pos.0] = true;

    // Collect all unvisited cells found, pick the one furthest from goal.
    let mut candidates: Vec<((usize, usize), u32)> = Vec::new();

    while let Some((x, y)) = queue.pop_front() {
        if (x, y) != pos && !visited[y][x] {
            let dist = (x as i32 - goal.0 as i32).pow(2) + (y as i32 - goal.1 as i32).pow(2);
            candidates.push(((x, y), dist as u32));
            continue;
        }
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                if cardinal_only && dx != 0 && dy != 0 { continue; }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 && (nx as usize) < grid_w && (ny as usize) < grid_h {
                    let (nx, ny) = (nx as usize, ny as usize);
                    if !blocked(nx, ny) && !seen[ny][nx] {
                        seen[ny][nx] = true;
                        parent[ny][nx] = Some((x, y));
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    // Pick the candidate furthest from goal.
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    let target = candidates[0].0;

    // Reconstruct path from pos to target.
    let mut path = vec![target];
    let mut cur = target;
    while let Some(p) = parent[cur.1][cur.0] {
        path.push(p);
        cur = p;
        if cur == pos { break; }
    }
    path.reverse();
    Some((target, path))
}

/// Grid coordinates to world-space polyline points.
pub fn grid_to_coords(
    path: &[(usize, usize)],
    cell_size: f64,
    offset: Coord<f64>,
) -> Vec<Coord<f64>> {
    path.iter()
        .map(|&(x, y)| Coord {
            x: x as f64 * cell_size + offset.x,
            y: y as f64 * cell_size + offset.y,
        })
        .collect()
}

