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

// ---------------------------------------------------------------------------
// SVG Export
// ---------------------------------------------------------------------------

pub fn export_svg(doc: &FlowchartDocument, path: &Path) -> Result<(), String> {
    let (min_x, min_y, max_x, max_y) = bounding_box(doc)
        .ok_or_else(|| "Nothing to export: document has no nodes".to_string())?;

    let width = max_x - min_x;
    let height = max_y - min_y;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<rect width="100%" height="100%" fill="white"/>
"#,
        width.ceil() as i32,
        height.ceil() as i32,
        width.ceil() as i32,
        height.ceil() as i32,
    ));

    // Draw edges first (behind nodes)
    for edge in &doc.edges {
        let src_node = doc.find_node(&edge.source.node_id);
        let tgt_node = doc.find_node(&edge.target.node_id);
        if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
            let src = sn.port_position(edge.source.side);
            let tgt = tn.port_position(edge.target.side);
            let sx = src.x - min_x;
            let sy = src.y - min_y;
            let tx = tgt.x - min_x;
            let ty = tgt.y - min_y;
            let color = rgba_to_svg_color(edge.style.color);
            let opacity = edge.style.color[3] as f32 / 255.0;
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}" stroke-opacity="{:.2}"/>"#,
                sx, sy, tx, ty, color, edge.style.width, opacity,
            ));
            svg.push('\n');

            // Edge label at midpoint
            if !edge.label.is_empty() {
                let mx = (sx + tx) / 2.0;
                let my = (sy + ty) / 2.0;
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="12" fill="{}">{}</text>"#,
                    mx, my, color, xml_escape(&edge.label),
                ));
                svg.push('\n');
            }
        }
    }

    // Draw nodes
    for node in &doc.nodes {
        let nx = node.position[0] - min_x;
        let ny = node.position[1] - min_y;
        let nw = node.size[0];
        let nh = node.size[1];

        let fill = rgba_to_svg_color(node.style.fill_color);
        let fill_opacity = node.style.fill_color[3] as f32 / 255.0;
        let stroke = rgba_to_svg_color(node.style.border_color);
        let stroke_opacity = node.style.border_color[3] as f32 / 255.0;
        let stroke_width = node.style.border_width;

        match node.shape {
            NodeShape::Rectangle => {
                svg.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    nx, ny, nw, nh, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
            }
            NodeShape::RoundedRect => {
                svg.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="10" ry="10" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    nx, ny, nw, nh, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
            }
            NodeShape::Circle => {
                let cx = nx + nw / 2.0;
                let cy = ny + nh / 2.0;
                let r = nw.min(nh) / 2.0;
                svg.push_str(&format!(
                    r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    cx, cy, r, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
            }
            NodeShape::Diamond => {
                let cx = nx + nw / 2.0;
                let cy = ny + nh / 2.0;
                let hw = nw / 2.0;
                let hh = nh / 2.0;
                let points_str = format!(
                    "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    cx, cy - hh,
                    cx + hw, cy,
                    cx, cy + hh,
                    cx - hw, cy,
                );
                svg.push_str(&format!(
                    r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    points_str, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
            }
            NodeShape::Parallelogram => {
                let skew = nw * 0.15;
                let points_str = format!(
                    "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    nx + skew, ny,
                    nx + nw, ny,
                    nx + nw - skew, ny + nh,
                    nx, ny + nh,
                );
                svg.push_str(&format!(
                    r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    points_str, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
            }
        }
        svg.push('\n');

        // Text label centered in node
        if !node.label.is_empty() {
            let text_color = rgba_to_svg_color(node.style.text_color);
            let text_opacity = node.style.text_color[3] as f32 / 255.0;
            let text_x = nx + nw / 2.0;
            let text_y = ny + nh / 2.0;
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="{:.0}" fill="{}" fill-opacity="{:.2}">{}</text>"#,
                text_x, text_y, node.style.font_size, text_color, text_opacity, xml_escape(&node.label),
            ));
            svg.push('\n');
        }
    }

    svg.push_str("</svg>\n");

    std::fs::write(path, svg).map_err(|e| e.to_string())
}

/// Convert [r,g,b,a] to an SVG-compatible hex color string (ignoring alpha).
fn rgba_to_svg_color(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Minimal XML escaping for text content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
