mod export_dxf;
mod export_svg;
mod font;
mod geometry;
mod procgen;
mod viewer;

use geo_types::Coord;
use geometry::{Drawing, Layer};
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub enum Shape {
    Rect,
    Circle,
    Hexagon,
    Star,
    Donut,
    Diamond,
    Heart,
}

impl Shape {
    pub fn name(&self) -> &'static str {
        match self {
            Shape::Rect => "rect",
            Shape::Circle => "circle",
            Shape::Hexagon => "hex",
            Shape::Star => "star",
            Shape::Donut => "donut",
            Shape::Diamond => "diamond",
            Shape::Heart => "heart",
        }
    }
}

pub struct Scene {
    pub grid_w: usize,
    pub grid_h: usize,
    pub cell_size: f64,
    pub shape: Shape,
    pub walls: Vec<(usize, usize, usize, usize)>, // preset rect walls
    pub painted: HashSet<(usize, usize)>,           // user-drawn cells
    pub longest: bool,
    pub cardinal_only: bool,
}

impl Scene {
    fn center(&self) -> (f64, f64) {
        (self.grid_w as f64 / 2.0, self.grid_h as f64 / 2.0)
    }

    fn radius(&self) -> f64 {
        (self.grid_w.min(self.grid_h) as f64 / 2.0) - 1.0
    }

    pub fn blocked(&self, x: usize, y: usize) -> bool {
        if self.painted.contains(&(x, y)) {
            return true;
        }
        if self.walls.iter().any(|&(x1, y1, x2, y2)| x >= x1 && x <= x2 && y >= y1 && y <= y2) {
            return true;
        }

        let (cx, cy) = self.center();
        let dx = x as f64 - cx + 0.5;
        let dy = y as f64 - cy + 0.5;
        let r = self.radius();

        match self.shape {
            Shape::Rect => {
                // Border only if no explicit walls define boundaries.
                if self.walls.is_empty() {
                    x == 0 || y == 0 || x == self.grid_w - 1 || y == self.grid_h - 1
                } else {
                    false // walls already checked above
                }
            }
            Shape::Circle => {
                dx * dx + dy * dy > r * r
            }
            Shape::Hexagon => {
                // Regular hexagon using max of three axes.
                let ax = dx.abs();
                let ay = dy.abs();
                // Hex test: max(|x|, |y|, |x+y|) > r  (pointy-top approximation)
                let s = r * 0.866; // sqrt(3)/2
                ax > r || ay > s || (ax * 0.5 + ay * 0.866) > s
            }
            Shape::Star => {
                // 5-pointed star: inside if angle-dependent radius check passes.
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > r { return true; }
                let angle = dy.atan2(dx);
                // Star oscillates between inner and outer radius.
                let inner = r * 0.38;
                let outer = r;
                let t = ((angle * 5.0 / std::f64::consts::TAU + 0.5).rem_euclid(1.0) - 0.5).abs() * 2.0;
                let threshold = inner + (outer - inner) * (1.0 - t);
                dist > threshold
            }
            Shape::Donut => {
                let dist_sq = dx * dx + dy * dy;
                let inner_r = r * 0.35;
                dist_sq > r * r || dist_sq < inner_r * inner_r
            }
            Shape::Diamond => {
                dx.abs() + dy.abs() > r
            }
            Shape::Heart => {
                // Implicit heart: (x² + y² - 1)³ - x²y³ > 0 is outside.
                // Normalize to [-1.3, 1.3] range, flip y so top is bumps.
                let nx = dx / (r * 0.85);
                let ny = -dy / (r * 0.85) + 0.2; // shift up slightly
                let a = nx * nx + ny * ny - 1.0;
                a * a * a - nx * nx * ny * ny * ny > 0.0
            }
        }
    }

    pub fn default_start(&self) -> (usize, usize) {
        let (cx, cy) = self.center();
        match self.shape {
            Shape::Donut => {
                // Start on the left side of the ring.
                let r_mid = self.radius() * 0.67;
                ((cx - r_mid) as usize, cy as usize)
            }
            _ => {
                // Find first non-blocked cell near top-left quadrant.
                for d in 1..self.grid_w {
                    let x = (cx as usize).saturating_sub(d);
                    let y = (cy as usize).saturating_sub(d);
                    if !self.blocked(x, y) { return (x, y); }
                }
                (1, 1)
            }
        }
    }

    pub fn default_goal(&self) -> (usize, usize) {
        let (cx, cy) = self.center();
        match self.shape {
            Shape::Donut => {
                let r_mid = self.radius() * 0.67;
                ((cx + r_mid) as usize, cy as usize)
            }
            _ => {
                for d in 1..self.grid_w {
                    let x = (cx as usize + d).min(self.grid_w - 1);
                    let y = (cy as usize + d).min(self.grid_h - 1);
                    if !self.blocked(x, y) { return (x, y); }
                }
                (self.grid_w - 2, self.grid_h - 2)
            }
        }
    }

    pub fn build(&self, start: (usize, usize), goal: (usize, usize)) -> Drawing {
        let mut drawing = Drawing::new();

        // Draw blocked cells.
        for y in 0..self.grid_h {
            for x in 0..self.grid_w {
                if self.blocked(x, y) {
                    let cx = x as f64 * self.cell_size;
                    let cy = y as f64 * self.cell_size;
                    drawing.add_closed(
                        vec![
                            Coord { x: cx, y: cy },
                            Coord { x: cx + self.cell_size, y: cy },
                            Coord { x: cx + self.cell_size, y: cy + self.cell_size },
                            Coord { x: cx, y: cy + self.cell_size },
                        ],
                        Layer::Engrave,
                    );
                }
            }
        }

        // Mark start and goal.
        let m = self.cell_size * 0.4;
        for &(gx, gy) in &[start, goal] {
            let cx = gx as f64 * self.cell_size;
            let cy = gy as f64 * self.cell_size;
            drawing.add(vec![Coord { x: cx - m, y: cy - m }, Coord { x: cx + m, y: cy + m }], Layer::Marker);
            drawing.add(vec![Coord { x: cx + m, y: cy - m }, Coord { x: cx - m, y: cy + m }], Layer::Marker);
        }

        // Pathfind.
        let blocked = |x: usize, y: usize| self.blocked(x, y);

        if self.longest {
            if let Some(segments) = procgen::find_longest_path(self.grid_w, self.grid_h, start, goal, &blocked, self.cardinal_only) {
                for seg in &segments {
                    let coords = procgen::grid_to_coords(&seg.points, self.cell_size, Coord { x: 0.0, y: 0.0 });
                    let layer = if seg.is_link { Layer::Link } else { Layer::Cut };
                    drawing.add(coords, layer);
                }
            }
        } else if let Some(path) = procgen::find_path(self.grid_w, self.grid_h, start, goal, &blocked, self.cardinal_only) {
            let coords = procgen::grid_to_coords(&path, self.cell_size, Coord { x: 0.0, y: 0.0 });
            drawing.add(coords, Layer::Cut);
        }

        drawing
    }

    /// Build drawing with walls and markers only — no pathfinding. Fast for live wall painting.
    pub fn build_walls_only(&self, start: (usize, usize), goal: (usize, usize)) -> Drawing {
        let mut drawing = Drawing::new();

        for y in 0..self.grid_h {
            for x in 0..self.grid_w {
                if self.blocked(x, y) {
                    let cx = x as f64 * self.cell_size;
                    let cy = y as f64 * self.cell_size;
                    drawing.add_closed(
                        vec![
                            Coord { x: cx, y: cy },
                            Coord { x: cx + self.cell_size, y: cy },
                            Coord { x: cx + self.cell_size, y: cy + self.cell_size },
                            Coord { x: cx, y: cy + self.cell_size },
                        ],
                        Layer::Engrave,
                    );
                }
            }
        }

        let m = self.cell_size * 0.4;
        for &(gx, gy) in &[start, goal] {
            let cx = gx as f64 * self.cell_size;
            let cy = gy as f64 * self.cell_size;
            drawing.add(vec![Coord { x: cx - m, y: cy - m }, Coord { x: cx + m, y: cy + m }], Layer::Marker);
            drawing.add(vec![Coord { x: cx + m, y: cy - m }, Coord { x: cx - m, y: cy + m }], Layer::Marker);
        }

        drawing
    }

    pub fn pixel_to_grid(&self, px: f64, py: f64, win_w: usize, win_h: usize) -> Option<(usize, usize)> {
        let draw_w = self.grid_w as f64 * self.cell_size;
        let draw_h = self.grid_h as f64 * self.cell_size;
        let margin = 30.0;
        let scale = ((win_w as f64 - margin * 2.0) / draw_w)
            .min((win_h as f64 - margin * 2.0) / draw_h);
        let ox = (win_w as f64 - draw_w * scale) / 2.0;
        let oy = (win_h as f64 - draw_h * scale) / 2.0;

        let world_x = (px - ox) / scale;
        let world_y = (py - oy) / scale;
        let gx = (world_x / self.cell_size).floor() as i32;
        let gy = (world_y / self.cell_size).floor() as i32;

        if gx >= 0 && gy >= 0 && (gx as usize) < self.grid_w && (gy as usize) < self.grid_h {
            let (gx, gy) = (gx as usize, gy as usize);
            if !self.blocked(gx, gy) {
                return Some((gx, gy));
            }
        }
        None
    }

}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let longest = args.iter().any(|a| a == "--longest");
    let shape_name = args.iter()
        .position(|a| a == "--shape")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("circle");

    let (mut scene, start, goal) = if shape_name == "maze" {
        let s = Scene {
            grid_w: 50, grid_h: 50, cell_size: 3.0,
            shape: Shape::Rect,
            walls: vec![
                (0, 0, 49, 0), (0, 49, 49, 49), (0, 0, 0, 49), (49, 0, 49, 49),
                (1, 16, 34, 16), (41, 16, 48, 16),
                (1, 33, 4, 33), (11, 33, 48, 33),
                (25, 1, 25, 14), (25, 18, 25, 31), (25, 35, 25, 48),
            ],
            painted: HashSet::new(),
            longest,
            cardinal_only: false,
        };
        ((s), (2, 2), (47, 47))
    } else {
        let shape = match shape_name {
            "rect" => Shape::Rect,
            "circle" => Shape::Circle,
            "hex" | "hexagon" => Shape::Hexagon,
            "star" => Shape::Star,
            "donut" => Shape::Donut,
            "diamond" => Shape::Diamond,
            "heart" => Shape::Heart,
            other => {
                eprintln!("Unknown shape '{other}'. Options: maze, rect, circle, hex, star, donut, diamond, heart");
                std::process::exit(1);
            }
        };
        let s = Scene {
            grid_w: 60, grid_h: 60, cell_size: 2.5,
            shape, walls: vec![], painted: HashSet::new(), longest, cardinal_only: false,
        };
        let start = s.default_start();
        let goal = s.default_goal();
        (s, start, goal)
    };

    // Export.
    let drawing = scene.build(start, goal);
    let out_dir = std::env::current_dir().unwrap().join("output");
    std::fs::create_dir_all(&out_dir).ok();
    let sn = scene.shape.name();
    let mut n = 1u32;
    loop {
        if !out_dir.join(format!("{sn}-{n}.svg")).exists() { break; }
        n += 1;
    }
    let svg_path = out_dir.join(format!("{sn}-{n}.svg"));
    let dxf_path = out_dir.join(format!("{sn}-{n}.dxf"));
    export_svg::save_svg(&drawing, &svg_path, 5.0).expect("Failed to write SVG");
    println!("Wrote {}", svg_path.display());
    export_dxf::save_dxf(&drawing, &dxf_path).expect("Failed to write DXF");
    println!("Wrote {}", dxf_path.display());

    // Interactive viewer.
    viewer::show_interactive(&mut scene, start, goal, &format!("astar — {shape_name}"));
}
