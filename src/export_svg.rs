use crate::geometry::{Drawing, Layer};
use svg::node::element::path::Data;
use svg::node::element::Path;
use svg::Document;
use std::path::Path as FsPath;

pub fn save_svg(drawing: &Drawing, path: &FsPath, margin: f64) -> std::io::Result<()> {
    let (min_x, min_y, max_x, max_y) = drawing.bounds;
    let w = max_x - min_x + margin * 2.0;
    let h = max_y - min_y + margin * 2.0;

    let mut doc = Document::new()
        .set("viewBox", (min_x - margin, min_y - margin, w, h))
        .set("width", format!("{w}mm"))
        .set("height", format!("{h}mm"))
        .set("xmlns", "http://www.w3.org/2000/svg");

    for polyline in &drawing.polylines {
        if polyline.points.len() < 2 {
            continue;
        }

        let mut data = Data::new().move_to((polyline.points[0].x, polyline.points[0].y));
        for p in &polyline.points[1..] {
            data = data.line_to((p.x, p.y));
        }

        let (color, width) = match polyline.layer {
            Layer::Cut => ("red", 0.1),
            Layer::Engrave => ("blue", 0.2),
            Layer::Link => ("#ccaa00", 0.15),
            Layer::Marker => ("lime", 0.2),
        };

        let path = Path::new()
            .set("d", data)
            .set("fill", "none")
            .set("stroke", color)
            .set("stroke-width", width);

        doc = doc.add(path);
    }

    svg::save(path, &doc)
}
