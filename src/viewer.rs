use crate::font::draw_text;
use crate::geometry::{Drawing, Layer};
use crate::{export_dxf, export_svg, Scene, Shape};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use std::collections::HashSet;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Bresenham's line algorithm: returns all grid cells on the line from (x0,y0) to (x1,y1).
fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cx = x0;
    let mut cy = y0;
    loop {
        if cx >= 0 && cy >= 0 {
            cells.push((cx as usize, cy as usize));
        }
        if cx == x1 && cy == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; cx += sx; }
        if e2 <= dx { err += dx; cy += sy; }
    }
    cells
}

struct Visibility {
    walls: bool,
    links: bool,
    markers: bool,
}

fn rasterize(drawing: &Drawing, width: usize, height: usize, vis: &Visibility) -> Vec<u32> {
    let bg = Color::from_rgba8(20, 20, 25, 255);
    let (min_x, min_y, max_x, max_y) = drawing.bounds;
    let draw_w = max_x - min_x;
    let draw_h = max_y - min_y;

    let margin = 30.0;
    let scale = ((width as f64 - margin * 2.0) / draw_w)
        .min((height as f64 - margin * 2.0) / draw_h);

    let ox = (width as f64 - draw_w * scale) / 2.0 - min_x * scale;
    let oy = (height as f64 - draw_h * scale) / 2.0 - min_y * scale;

    let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();
    pixmap.fill(bg);

    let transform = Transform::from_scale(scale as f32, scale as f32)
        .post_translate(ox as f32, oy as f32);

    for polyline in &drawing.polylines {
        if polyline.points.len() < 2 {
            continue;
        }
        if polyline.layer == Layer::Engrave && !vis.walls {
            continue;
        }
        if polyline.layer == Layer::Link && !vis.links {
            continue;
        }
        if polyline.layer == Layer::Marker && !vis.markers {
            continue;
        }

        let mut pb = PathBuilder::new();
        pb.move_to(polyline.points[0].x as f32, polyline.points[0].y as f32);
        for p in &polyline.points[1..] {
            pb.line_to(p.x as f32, p.y as f32);
        }
        let Some(path) = pb.finish() else { continue };

        let mut paint = Paint::default();
        paint.anti_alias = true;
        match polyline.layer {
            Layer::Cut => paint.set_color(Color::from_rgba8(255, 60, 60, 255)),
            Layer::Engrave => paint.set_color(Color::from_rgba8(60, 120, 255, 140)),
            Layer::Link => paint.set_color(Color::from_rgba8(255, 200, 40, 200)),
            Layer::Marker => paint.set_color(Color::from_rgba8(0, 255, 120, 255)),
        };

        let mut stroke = Stroke::default();
        stroke.width = match polyline.layer {
            Layer::Cut => 1.5 / scale as f32,
            Layer::Engrave => 0.8 / scale as f32,
            Layer::Link => 1.0 / scale as f32,
            Layer::Marker => 2.0 / scale as f32,
        };

        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
    }

    let rgba = pixmap.data();
    let mut buffer: Vec<u32> = Vec::with_capacity(width * height);
    for pixel in rgba.chunks_exact(4) {
        let (r, g, b) = (pixel[0] as u32, pixel[1] as u32, pixel[2] as u32);
        buffer.push((r << 16) | (g << 8) | b);
    }
    buffer
}

fn pixel_to_cell(scene: &Scene, px: f64, py: f64, win_w: usize, win_h: usize) -> Option<(usize, usize)> {
    let draw_w = scene.grid_w as f64 * scene.cell_size;
    let draw_h = scene.grid_h as f64 * scene.cell_size;
    let margin = 30.0;
    let scale = ((win_w as f64 - margin * 2.0) / draw_w)
        .min((win_h as f64 - margin * 2.0) / draw_h);
    let ox = (win_w as f64 - draw_w * scale) / 2.0;
    let oy = (win_h as f64 - draw_h * scale) / 2.0;
    let gx = ((px - ox) / scale / scene.cell_size).floor() as i32;
    let gy = ((py - oy) / scale / scene.cell_size).floor() as i32;
    if gx >= 0 && gy >= 0 && (gx as usize) < scene.grid_w && (gy as usize) < scene.grid_h {
        Some((gx as usize, gy as usize))
    } else {
        None
    }
}

pub fn show_interactive(scene: &mut Scene, initial_start: (usize, usize), initial_goal: (usize, usize), title: &str) {
    let init_w = 1024;
    let init_h = 768;

    let opts = WindowOptions {
        resize: true,
        ..WindowOptions::default()
    };
    let mut window = Window::new(title, init_w, init_h, opts)
        .expect("Failed to create window");

    window.set_target_fps(60);

    let mut start = initial_start;
    let mut goal = initial_goal;
    let mut click_sets_start = true;
    let mut vis = Visibility { walls: true, links: false, markers: false };
    let mut prev_w = false;
    let mut prev_c = false;
    let mut prev_d = false;
    let mut prev_l = false;
    let mut prev_m = false;
    let mut prev_s = false;

    // Left mouse: wall line drawing.
    let mut lmb_was_down = false;
    let mut lmb_dragged = false;
    let mut lmb_erasing = false;  // ctrl+lmb erases
    let mut lmb_start_pos: Option<(f32, f32)> = None;
    let mut lmb_start_cell: Option<(usize, usize)> = None;
    let mut line_preview: HashSet<(usize, usize)> = HashSet::new();

    // Right mouse: start/goal placement.
    let mut rmb_was_down = false;

    let mut cur_w = init_w;
    let mut cur_h = init_h;
    let mut drawing = scene.build(start, goal);
    let mut buffer = rasterize(&drawing, cur_w, cur_h, &vis);
    let mut dirty = false;

    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        let (w, h) = window.get_size();
        if w != cur_w || h != cur_h {
            cur_w = w.max(1);
            cur_h = h.max(1);
            dirty = true;
        }

        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);

        // Toggle keys (on key-down edge).
        let c_down = window.is_key_down(Key::C);
        if c_down && !prev_c {
            scene.painted.clear();
            drawing = scene.build(start, goal);
            dirty = true;
        }
        prev_c = c_down;

        let d_down = window.is_key_down(Key::D);
        if d_down && !prev_d {
            scene.cardinal_only = !scene.cardinal_only;
            drawing = scene.build(start, goal);
            dirty = true;
        }
        prev_d = d_down;

        let w_down = window.is_key_down(Key::W);
        if w_down && !prev_w {
            vis.walls = !vis.walls;
            dirty = true;
        }
        prev_w = w_down;

        let l_down = window.is_key_down(Key::L);
        if l_down && !prev_l {
            vis.links = !vis.links;
            dirty = true;
        }
        prev_l = l_down;

        let m_down = window.is_key_down(Key::M);
        if m_down && !prev_m {
            vis.markers = !vis.markers;
            dirty = true;
        }
        prev_m = m_down;

        // Number keys: select boundary shape.
        // 1=Rect 2=Circle 3=Hexagon 4=Star 5=Donut 6=Diamond 7=Heart
        let shape_keys: [(Key, Shape); 7] = [
            (Key::Key1, Shape::Rect),
            (Key::Key2, Shape::Circle),
            (Key::Key3, Shape::Hexagon),
            (Key::Key4, Shape::Star),
            (Key::Key5, Shape::Donut),
            (Key::Key6, Shape::Diamond),
            (Key::Key7, Shape::Heart),
        ];
        for (key, shape) in shape_keys {
            if window.is_key_down(key) {
                scene.shape = shape;
                scene.painted.clear();
                start = scene.default_start();
                goal = scene.default_goal();
                click_sets_start = true;
                drawing = scene.build(start, goal);
                dirty = true;
                break;
            }
        }

        let s_down = window.is_key_down(Key::S);
        if s_down && !prev_s {
            let out_dir = std::env::current_dir().unwrap().join("output");
            std::fs::create_dir_all(&out_dir).ok();
            // Find next available number for this shape.
            let shape_name = scene.shape.name();
            let mut n = 1u32;
            loop {
                let candidate = out_dir.join(format!("{shape_name}-{n}.svg"));
                if !candidate.exists() { break; }
                n += 1;
            }
            let svg_path = out_dir.join(format!("{shape_name}-{n}.svg"));
            let dxf_path = out_dir.join(format!("{shape_name}-{n}.dxf"));
            export_svg::save_svg(&drawing, &svg_path, 5.0).expect("Failed to write SVG");
            export_dxf::save_dxf(&drawing, &dxf_path).expect("Failed to write DXF");
            println!("Wrote {}", svg_path.display());
            println!("Wrote {}", dxf_path.display());
        }
        prev_s = s_down;

        // --- Left mouse: wall line drawing / ctrl+lmb line erasing ---
        let lmb_down = window.get_mouse_down(MouseButton::Left);
        if lmb_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
                if !lmb_was_down {
                    // Press: record start pos and cell, determine mode.
                    lmb_start_pos = Some((mx, my));
                    lmb_start_cell = pixel_to_cell(scene, mx as f64, my as f64, cur_w, cur_h);
                    lmb_dragged = false;
                    lmb_erasing = ctrl;
                    line_preview.clear();
                } else if let Some((sx, sy)) = lmb_start_pos {
                    if (mx - sx).abs() > 3.0 || (my - sy).abs() > 3.0 {
                        lmb_dragged = true;
                    }
                }

                if lmb_dragged {
                    if let (Some(sc), Some(ec)) = (
                        lmb_start_cell,
                        pixel_to_cell(scene, mx as f64, my as f64, cur_w, cur_h),
                    ) {
                        let new_line: HashSet<(usize, usize)> = bresenham(
                            sc.0 as i32, sc.1 as i32, ec.0 as i32, ec.1 as i32,
                        ).into_iter().collect();

                        if new_line != line_preview {
                            // Remove old preview from painted.
                            for &c in &line_preview {
                                scene.painted.remove(&c);
                            }
                            // Add new preview (only non-shape-blocked cells for drawing, or existing painted for erasing).
                            line_preview = new_line;
                            if !lmb_erasing {
                                for &c in &line_preview {
                                    scene.painted.insert(c);
                                }
                            }
                            drawing = scene.build_walls_only(start, goal);
                            dirty = true;
                        }
                    }
                }
            }
        }
        if !lmb_down && lmb_was_down {
            if lmb_dragged && !line_preview.is_empty() {
                if lmb_erasing {
                    // Erase all cells along the line.
                    for &c in &line_preview {
                        scene.painted.remove(&c);
                    }
                }
                // For drawing mode, preview cells are already in painted — keep them.
                line_preview.clear();
                drawing = scene.build(start, goal);
                dirty = true;
            } else if !lmb_dragged {
                // Single click: toggle wall.
                if let Some(cell) = lmb_start_cell {
                    if !scene.painted.remove(&cell) {
                        scene.painted.insert(cell);
                    }
                    drawing = scene.build(start, goal);
                    dirty = true;
                }
            }
            lmb_start_pos = None;
            lmb_start_cell = None;
        }
        lmb_was_down = lmb_down;

        // --- Right mouse: set start/goal ---
        let rmb_down = window.get_mouse_down(MouseButton::Right);
        if rmb_down && !rmb_was_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
                if let Some(cell) = scene.pixel_to_grid(mx as f64, my as f64, cur_w, cur_h) {
                    if click_sets_start {
                        start = cell;
                    } else {
                        goal = cell;
                    }
                    click_sets_start = !click_sets_start;
                    drawing = scene.build(start, goal);
                    dirty = true;
                }
            }
        }
        rmb_was_down = rmb_down;

        if dirty {
            buffer = rasterize(&drawing, cur_w, cur_h, &vis);
            // Draw HUD overlay.
            let dim = 0x55_99_99_99u32; // gray
            let bright = 0xFF_CC_CC_CCu32; // white-ish
            let active = 0xFF_FF_CC_44u32; // yellow highlight
            let scale = 2;
            let pad = 6;

            // Top: shape selection.
            let shapes = [
                ("1", "Rect", matches!(scene.shape, Shape::Rect)),
                ("2", "Circle", matches!(scene.shape, Shape::Circle)),
                ("3", "Hex", matches!(scene.shape, Shape::Hexagon)),
                ("4", "Star", matches!(scene.shape, Shape::Star)),
                ("5", "Donut", matches!(scene.shape, Shape::Donut)),
                ("6", "Diamond", matches!(scene.shape, Shape::Diamond)),
                ("7", "Heart", matches!(scene.shape, Shape::Heart)),
            ];
            let mut tx = pad;
            for (key, name, is_active) in shapes {
                let label = format!("{key}:{name}");
                let col = if is_active { active } else { dim };
                draw_text(&mut buffer, cur_w, cur_h, &label, tx, pad, col, scale);
                tx += (label.len() * (5 + 1) * scale) + pad * 2;
            }

            // Bottom: controls.
            let controls = [
                ("LMB", "Draw", true),
                ("RMB", "Start/Goal", true),
                ("C", "Clear", true),
                ("D", "Diag", !scene.cardinal_only),
                ("L", "Links", vis.links),
                ("M", "Markers", vis.markers),
                ("W", "Walls", vis.walls),
                ("S", "Save", true),
                ("Q", "Quit", true),
            ];
            let bot_y = cur_h.saturating_sub(7 * scale + pad);
            let mut bx = pad;
            for (key, label, is_on) in controls {
                let text = format!("{key}:{label}");
                let col = if is_on { dim } else { 0x55_66_66_66 };
                draw_text(&mut buffer, cur_w, cur_h, &text, bx, bot_y, col, scale);
                bx += (text.len() * (5 + 1) * scale) + pad * 2;
            }

            dirty = false;
        }

        window
            .update_with_buffer(&buffer, cur_w, cur_h)
            .expect("Failed to update window");
    }
}
