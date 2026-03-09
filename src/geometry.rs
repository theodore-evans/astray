use geo_types::{Coord, LineString};

/// A collection of 2D polylines ready for rendering or export.
pub struct Drawing {
    pub polylines: Vec<Polyline>,
    pub bounds: (f64, f64, f64, f64), // min_x, min_y, max_x, max_y
}

pub struct Polyline {
    pub points: Vec<Coord<f64>>,
    pub layer: Layer,
}

/// Laser cutter layer semantics: Cut lines vs Engrave/raster regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Cut,
    Engrave,
    Link,
    Marker,
}

impl Drawing {
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            bounds: (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        }
    }

    pub fn add(&mut self, points: Vec<Coord<f64>>, layer: Layer) {
        for p in &points {
            self.bounds.0 = self.bounds.0.min(p.x);
            self.bounds.1 = self.bounds.1.min(p.y);
            self.bounds.2 = self.bounds.2.max(p.x);
            self.bounds.3 = self.bounds.3.max(p.y);
        }
        self.polylines.push(Polyline { points, layer });
    }

    pub fn add_closed(&mut self, mut points: Vec<Coord<f64>>, layer: Layer) {
        if let Some(&first) = points.first() {
            points.push(first);
        }
        self.add(points, layer);
    }

    /// Convert a geo LineString into the drawing.
    pub fn add_linestring(&mut self, ls: &LineString<f64>, layer: Layer) {
        self.add(ls.0.clone(), layer);
    }
}
