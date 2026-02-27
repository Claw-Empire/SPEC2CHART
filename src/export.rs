use crate::model::{FlowchartDocument, NodeShape};
use std::path::Path;

/// Padding around the bounding box in pixels.
const EXPORT_PADDING: f32 = 40.0;

/// Calculate the bounding box of all nodes, returning (min_x, min_y, max_x, max_y).
/// Returns None if there are no nodes.
fn bounding_box(doc: &FlowchartDocument) -> Option<(f32, f32, f32, f32)> {
    if doc.nodes.is_empty() {
        return None;
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &doc.nodes {
        let x = node.position[0];
        let y = node.position[1];
        let w = node.size[0];
        let h = node.size[1];
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    Some((
        min_x - EXPORT_PADDING,
        min_y - EXPORT_PADDING,
        max_x + EXPORT_PADDING,
        max_y + EXPORT_PADDING,
    ))
}

// ---------------------------------------------------------------------------
// PNG Export
// ---------------------------------------------------------------------------

pub fn export_png(doc: &FlowchartDocument, path: &Path) -> Result<(), String> {
    let (min_x, min_y, max_x, max_y) = bounding_box(doc)
        .ok_or_else(|| "Nothing to export: document has no nodes".to_string())?;

    let width = (max_x - min_x).ceil() as u32;
    let height = (max_y - min_y).ceil() as u32;

    // Clamp to reasonable size
    let width = width.min(8000).max(1);
    let height = height.min(8000).max(1);

    let mut img = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 255, 255, 255]));

    // Draw each node
    for node in &doc.nodes {
        let nx = (node.position[0] - min_x) as i32;
        let ny = (node.position[1] - min_y) as i32;
        let nw = node.size[0] as i32;
        let nh = node.size[1] as i32;

        let fill = image::Rgba(node.style.fill_color);
        let border = image::Rgba(node.style.border_color);
        let border_w = node.style.border_width.round() as i32;

        match node.shape {
            NodeShape::Circle => {
                let cx = nx + nw / 2;
                let cy = ny + nh / 2;
                let radius = nw.min(nh) / 2;
                draw_filled_circle(&mut img, cx, cy, radius, fill);
                draw_circle_outline(&mut img, cx, cy, radius, border, border_w);
            }
            NodeShape::Diamond => {
                let cx = nx + nw / 2;
                let cy = ny + nh / 2;
                let hw = nw / 2;
                let hh = nh / 2;
                let points = vec![
                    (cx, cy - hh),
                    (cx + hw, cy),
                    (cx, cy + hh),
                    (cx - hw, cy),
                ];
                draw_filled_polygon(&mut img, &points, fill);
                draw_polygon_outline(&mut img, &points, border, border_w);
            }
            NodeShape::Parallelogram => {
                let skew = (nw as f32 * 0.15) as i32;
                let points = vec![
                    (nx + skew, ny),
                    (nx + nw, ny),
                    (nx + nw - skew, ny + nh),
                    (nx, ny + nh),
                ];
                draw_filled_polygon(&mut img, &points, fill);
                draw_polygon_outline(&mut img, &points, border, border_w);
            }
            _ => {
                // Rectangle and RoundedRect (draw as rectangle for simplicity)
                draw_filled_rect(&mut img, nx, ny, nw, nh, fill);
                draw_rect_outline(&mut img, nx, ny, nw, nh, border, border_w);
            }
        }
    }

    // Draw edges as lines
    for edge in &doc.edges {
        let src_node = doc.find_node(&edge.source.node_id);
        let tgt_node = doc.find_node(&edge.target.node_id);
        if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
            let src = sn.port_position(edge.source.side);
            let tgt = tn.port_position(edge.target.side);
            let sx = (src.x - min_x) as i32;
            let sy = (src.y - min_y) as i32;
            let tx = (tgt.x - min_x) as i32;
            let ty = (tgt.y - min_y) as i32;
            let color = image::Rgba(edge.style.color);
            let w = edge.style.width.round().max(1.0) as i32;
            draw_line(&mut img, sx, sy, tx, ty, color, w);
        }
    }

    img.save(path).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Simple drawing helpers for PNG
// ---------------------------------------------------------------------------

fn draw_filled_rect(
    img: &mut image::RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: image::Rgba<u8>,
) {
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for py in y.max(0)..(y + h).min(ih) {
        for px in x.max(0)..(x + w).min(iw) {
            img.put_pixel(px as u32, py as u32, color);
        }
    }
}

fn draw_rect_outline(
    img: &mut image::RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: image::Rgba<u8>,
    bw: i32,
) {
    // Top and bottom
    for t in 0..bw {
        for px in x..(x + w) {
            put_pixel_safe(img, px, y + t, color);
            put_pixel_safe(img, px, y + h - 1 - t, color);
        }
        // Left and right
        for py in y..(y + h) {
            put_pixel_safe(img, x + t, py, color);
            put_pixel_safe(img, x + w - 1 - t, py, color);
        }
    }
}

fn draw_filled_circle(
    img: &mut image::RgbaImage,
    cx: i32,
    cy: i32,
    radius: i32,
    color: image::Rgba<u8>,
) {
    let r2 = (radius * radius) as f32;
    for py in (cy - radius)..(cy + radius + 1) {
        for px in (cx - radius)..(cx + radius + 1) {
            let dx = (px - cx) as f32;
            let dy = (py - cy) as f32;
            if dx * dx + dy * dy <= r2 {
                put_pixel_safe(img, px, py, color);
            }
        }
    }
}

fn draw_circle_outline(
    img: &mut image::RgbaImage,
    cx: i32,
    cy: i32,
    radius: i32,
    color: image::Rgba<u8>,
    bw: i32,
) {
    let r_outer = radius as f32;
    let r_inner = (radius - bw) as f32;
    let r_outer2 = r_outer * r_outer;
    let r_inner2 = r_inner * r_inner;
    for py in (cy - radius - bw)..(cy + radius + bw + 1) {
        for px in (cx - radius - bw)..(cx + radius + bw + 1) {
            let dx = (px - cx) as f32;
            let dy = (py - cy) as f32;
            let d2 = dx * dx + dy * dy;
            if d2 <= r_outer2 && d2 >= r_inner2 {
                put_pixel_safe(img, px, py, color);
            }
        }
    }
}

fn draw_filled_polygon(
    img: &mut image::RgbaImage,
    points: &[(i32, i32)],
    color: image::Rgba<u8>,
) {
    if points.is_empty() {
        return;
    }
    let min_y = points.iter().map(|p| p.1).min().unwrap();
    let max_y = points.iter().map(|p| p.1).max().unwrap();

    for y in min_y..=max_y {
        let mut intersections = Vec::new();
        let n = points.len();
        for i in 0..n {
            let (x0, y0) = points[i];
            let (x1, y1) = points[(i + 1) % n];
            if (y0 <= y && y1 > y) || (y1 <= y && y0 > y) {
                let t = (y - y0) as f32 / (y1 - y0) as f32;
                let x_intersect = x0 as f32 + t * (x1 - x0) as f32;
                intersections.push(x_intersect as i32);
            }
        }
        intersections.sort();
        for chunk in intersections.chunks(2) {
            if chunk.len() == 2 {
                for x in chunk[0]..=chunk[1] {
                    put_pixel_safe(img, x, y, color);
                }
            }
        }
    }
}

fn draw_polygon_outline(
    img: &mut image::RgbaImage,
    points: &[(i32, i32)],
    color: image::Rgba<u8>,
    bw: i32,
) {
    let n = points.len();
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        draw_line(img, x0, y0, x1, y1, color, bw);
    }
}

fn draw_line(
    img: &mut image::RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: image::Rgba<u8>,
    width: i32,
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut cx = x0;
    let mut cy = y0;

    let half_w = width / 2;

    loop {
        // Draw a small rect to get line width
        for oy in -half_w..=half_w {
            for ox in -half_w..=half_w {
                put_pixel_safe(img, cx + ox, cy + oy, color);
            }
        }

        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            cx += sx;
        }
        if e2 < dx {
            err += dx;
            cy += sy;
        }
    }
}

fn put_pixel_safe(img: &mut image::RgbaImage, x: i32, y: i32, color: image::Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, color);
    }
}
