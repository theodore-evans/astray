use crate::geometry::{Drawing, Layer};
use dxf::entities::*;
use dxf::tables::Layer as DxfLayer;
use dxf::{Color, Drawing as DxfDrawing, Point};
use std::path::Path;

pub fn save_dxf(drawing: &Drawing, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut dxf = DxfDrawing::new();

    // Create layers matching laser cutter conventions.
    let mut cut_layer = DxfLayer::default();
    cut_layer.name = "CUT".to_string();
    cut_layer.color = Color::from_index(1); // red
    dxf.add_layer(cut_layer);

    let mut engrave_layer = DxfLayer::default();
    engrave_layer.name = "ENGRAVE".to_string();
    engrave_layer.color = Color::from_index(5); // blue
    dxf.add_layer(engrave_layer);

    let mut link_layer = DxfLayer::default();
    link_layer.name = "LINK".to_string();
    link_layer.color = Color::from_index(2); // yellow
    dxf.add_layer(link_layer);

    let mut marker_layer = DxfLayer::default();
    marker_layer.name = "MARKER".to_string();
    marker_layer.color = Color::from_index(3); // green
    dxf.add_layer(marker_layer);

    for polyline in &drawing.polylines {
        if polyline.points.len() < 2 {
            continue;
        }

        let layer_name = match polyline.layer {
            Layer::Cut => "CUT",
            Layer::Engrave => "ENGRAVE",
            Layer::Link => "LINK",
            Layer::Marker => "MARKER",
        };

        // Export as individual line segments for maximum compatibility.
        for pair in polyline.points.windows(2) {
            let mut line = Line::default();
            line.p1 = Point::new(pair[0].x, pair[0].y, 0.0);
            line.p2 = Point::new(pair[1].x, pair[1].y, 0.0);
            let mut entity = Entity::new(EntityType::Line(line));
            entity.common.layer = layer_name.to_string();
            dxf.add_entity(entity);
        }
    }

    dxf.save_file(path)?;
    Ok(())
}
