use crate::model::{Cardinality, FlowchartDocument, LayoutMode, Node, NodeKind, NodeShape, ENTITY_HEADER_HEIGHT, ENTITY_ROW_HEIGHT};
use std::path::Path;

// ---------------------------------------------------------------------------
// Embedded 5×7 pixel bitmap font (ASCII 0x20–0x7E, column-major, bit0=top)
// Each glyph is 5 bytes: one per column (left→right).
// Within each byte, bit 0 = top row … bit 6 = bottom row.
// Derived from the classic Adafruit GFX 5×7 font (MIT License).
// ---------------------------------------------------------------------------
static FONT5X7: &[u8] = &[
    0x00,0x00,0x00,0x00,0x00, // ' ' 0x20
    0x00,0x00,0x5F,0x00,0x00, // '!' 0x21
    0x00,0x07,0x00,0x07,0x00, // '"' 0x22
    0x14,0x7F,0x14,0x7F,0x14, // '#' 0x23
    0x24,0x2A,0x7F,0x2A,0x12, // '$' 0x24
    0x23,0x13,0x08,0x64,0x62, // '%' 0x25
    0x36,0x49,0x55,0x22,0x50, // '&' 0x26
    0x00,0x05,0x03,0x00,0x00, // ''' 0x27
    0x00,0x1C,0x22,0x41,0x00, // '(' 0x28
    0x00,0x41,0x22,0x1C,0x00, // ')' 0x29
    0x08,0x2A,0x1C,0x2A,0x08, // '*' 0x2A
    0x08,0x08,0x3E,0x08,0x08, // '+' 0x2B
    0x00,0x50,0x30,0x00,0x00, // ',' 0x2C
    0x08,0x08,0x08,0x08,0x08, // '-' 0x2D
    0x00,0x60,0x60,0x00,0x00, // '.' 0x2E
    0x20,0x10,0x08,0x04,0x02, // '/' 0x2F
    0x3E,0x51,0x49,0x45,0x3E, // '0' 0x30
    0x00,0x42,0x7F,0x40,0x00, // '1' 0x31
    0x42,0x61,0x51,0x49,0x46, // '2' 0x32
    0x21,0x41,0x45,0x4B,0x31, // '3' 0x33
    0x18,0x14,0x12,0x7F,0x10, // '4' 0x34
    0x27,0x45,0x45,0x45,0x39, // '5' 0x35
    0x3C,0x4A,0x49,0x49,0x30, // '6' 0x36
    0x01,0x71,0x09,0x05,0x03, // '7' 0x37
    0x36,0x49,0x49,0x49,0x36, // '8' 0x38
    0x06,0x49,0x49,0x29,0x1E, // '9' 0x39
    0x00,0x36,0x36,0x00,0x00, // ':' 0x3A
    0x00,0x56,0x36,0x00,0x00, // ';' 0x3B
    0x00,0x08,0x14,0x22,0x41, // '<' 0x3C
    0x14,0x14,0x14,0x14,0x14, // '=' 0x3D
    0x41,0x22,0x14,0x08,0x00, // '>' 0x3E
    0x02,0x01,0x51,0x09,0x06, // '?' 0x3F
    0x32,0x49,0x79,0x41,0x3E, // '@' 0x40
    0x7E,0x11,0x11,0x11,0x7E, // 'A' 0x41
    0x7F,0x49,0x49,0x49,0x36, // 'B' 0x42
    0x3E,0x41,0x41,0x41,0x22, // 'C' 0x43
    0x7F,0x41,0x41,0x22,0x1C, // 'D' 0x44
    0x7F,0x49,0x49,0x49,0x41, // 'E' 0x45
    0x7F,0x09,0x09,0x01,0x01, // 'F' 0x46
    0x3E,0x41,0x41,0x51,0x32, // 'G' 0x47
    0x7F,0x08,0x08,0x08,0x7F, // 'H' 0x48
    0x00,0x41,0x7F,0x41,0x00, // 'I' 0x49
    0x20,0x40,0x41,0x3F,0x01, // 'J' 0x4A
    0x7F,0x08,0x14,0x22,0x41, // 'K' 0x4B
    0x7F,0x40,0x40,0x40,0x40, // 'L' 0x4C
    0x7F,0x02,0x04,0x02,0x7F, // 'M' 0x4D
    0x7F,0x04,0x08,0x10,0x7F, // 'N' 0x4E
    0x3E,0x41,0x41,0x41,0x3E, // 'O' 0x4F
    0x7F,0x09,0x09,0x09,0x06, // 'P' 0x50
    0x3E,0x41,0x51,0x21,0x5E, // 'Q' 0x51
    0x7F,0x09,0x19,0x29,0x46, // 'R' 0x52
    0x46,0x49,0x49,0x49,0x31, // 'S' 0x53
    0x01,0x01,0x7F,0x01,0x01, // 'T' 0x54
    0x3F,0x40,0x40,0x40,0x3F, // 'U' 0x55
    0x1F,0x20,0x40,0x20,0x1F, // 'V' 0x56
    0x7F,0x20,0x18,0x20,0x7F, // 'W' 0x57
    0x63,0x14,0x08,0x14,0x63, // 'X' 0x58
    0x03,0x04,0x78,0x04,0x03, // 'Y' 0x59
    0x61,0x51,0x49,0x45,0x43, // 'Z' 0x5A
    0x00,0x00,0x7F,0x41,0x41, // '[' 0x5B
    0x02,0x04,0x08,0x10,0x20, // '\' 0x5C
    0x41,0x41,0x7F,0x00,0x00, // ']' 0x5D
    0x04,0x02,0x01,0x02,0x04, // '^' 0x5E
    0x40,0x40,0x40,0x40,0x40, // '_' 0x5F
    0x00,0x01,0x02,0x04,0x00, // '`' 0x60
    0x20,0x54,0x54,0x54,0x78, // 'a' 0x61
    0x7F,0x48,0x44,0x44,0x38, // 'b' 0x62
    0x38,0x44,0x44,0x44,0x20, // 'c' 0x63
    0x38,0x44,0x44,0x48,0x7F, // 'd' 0x64
    0x38,0x54,0x54,0x54,0x18, // 'e' 0x65
    0x08,0x7E,0x09,0x01,0x02, // 'f' 0x66
    0x08,0x14,0x54,0x54,0x3C, // 'g' 0x67
    0x7F,0x08,0x04,0x04,0x78, // 'h' 0x68
    0x00,0x44,0x7D,0x40,0x00, // 'i' 0x69
    0x20,0x40,0x44,0x3D,0x00, // 'j' 0x6A
    0x00,0x7F,0x10,0x28,0x44, // 'k' 0x6B
    0x00,0x41,0x7F,0x40,0x00, // 'l' 0x6C
    0x7C,0x04,0x18,0x04,0x7C, // 'm' 0x6D
    0x7C,0x08,0x04,0x04,0x78, // 'n' 0x6E
    0x38,0x44,0x44,0x44,0x38, // 'o' 0x6F
    0x7C,0x14,0x14,0x14,0x08, // 'p' 0x70
    0x08,0x14,0x14,0x18,0x7C, // 'q' 0x71
    0x7C,0x08,0x04,0x04,0x08, // 'r' 0x72
    0x48,0x54,0x54,0x54,0x24, // 's' 0x73
    0x04,0x3F,0x44,0x40,0x20, // 't' 0x74
    0x3C,0x40,0x40,0x20,0x7C, // 'u' 0x75
    0x1C,0x20,0x40,0x20,0x1C, // 'v' 0x76
    0x3C,0x40,0x30,0x40,0x3C, // 'w' 0x77
    0x44,0x28,0x10,0x28,0x44, // 'x' 0x78
    0x0C,0x50,0x50,0x50,0x3C, // 'y' 0x79
    0x44,0x64,0x54,0x4C,0x44, // 'z' 0x7A
    0x00,0x08,0x36,0x41,0x00, // '{' 0x7B
    0x00,0x00,0x7F,0x00,0x00, // '|' 0x7C
    0x00,0x41,0x36,0x08,0x00, // '}' 0x7D
    0x08,0x08,0x2A,0x1C,0x08, // '~' 0x7E
];

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
    let width = width.clamp(1, 8000);
    let height = height.clamp(1, 8000);

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

        match &node.kind {
            NodeKind::Shape { shape, .. } => match shape {
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
                    draw_filled_rect(&mut img, nx, ny, nw, nh, fill);
                    draw_rect_outline(&mut img, nx, ny, nw, nh, border, border_w);
                }
            },
            NodeKind::StickyNote { .. } => {
                draw_filled_rect(&mut img, nx, ny, nw, nh, fill);
            }
            NodeKind::Entity { .. } => {
                // Draw body
                draw_filled_rect(&mut img, nx, ny, nw, nh, fill);
                draw_rect_outline(&mut img, nx, ny, nw, nh, border, border_w);
                // Draw header bar
                let header_h = ENTITY_HEADER_HEIGHT as i32;
                draw_filled_rect(&mut img, nx, ny, nw, header_h, border);
                // Header divider
                let div_y = ny + header_h;
                draw_line(&mut img, nx, div_y, nx + nw, div_y, border, 1);
            }
            NodeKind::Text { .. } => {
                // No background for text nodes
            }
        }
    }

    // Draw edges as lines with arrowheads and labels
    let label_color = image::Rgba([76u8, 79, 105, 255]); // #4c4f69 — WCAG AA on white
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

            // Shorten the line slightly so the arrowhead tip lands at target port
            let ddx = (tx - sx) as f32;
            let ddy = (ty - sy) as f32;
            let dlen = (ddx * ddx + ddy * ddy).sqrt().max(1.0);
            let shorten = 12.0_f32; // match arrow_len in draw_arrowhead_png
            let tx_short = (tx as f32 - ddx / dlen * shorten) as i32;
            let ty_short = (ty as f32 - ddy / dlen * shorten) as i32;
            draw_line(&mut img, sx, sy, tx_short, ty_short, color, w);

            // Arrowhead at target
            draw_arrowhead_png(&mut img, sx, sy, tx, ty, color);

            // Edge label at midpoint
            if !edge.label.is_empty() {
                let mx = (sx + tx) / 2;
                let my = (sy + ty) / 2;
                draw_text_centered_png(&mut img, &edge.label, mx, my, 1, label_color);
            }
        }
    }

    // Draw node labels
    for node in &doc.nodes {
        let nx = (node.position[0] - min_x) as i32;
        let ny = (node.position[1] - min_y) as i32;
        let nw = node.size[0] as i32;
        let nh = node.size[1] as i32;

        let text_rgba = image::Rgba(node.style.text_color);
        let cx = nx + nw / 2;
        let cy = ny + nh / 2;

        match &node.kind {
            NodeKind::Shape { label, .. } => {
                if !label.is_empty() {
                    draw_text_centered_png(&mut img, label, cx, cy, 2, text_rgba);
                }
            }
            NodeKind::StickyNote { text, .. } => {
                if !text.is_empty() {
                    // Truncate to first line for simple rendering
                    let first_line = text.lines().next().unwrap_or(text);
                    draw_text_centered_png(&mut img, first_line, cx, ny + 14, 1, text_rgba);
                }
            }
            NodeKind::Entity { name, attributes } => {
                // Name in header bar (header occupies top ENTITY_HEADER_HEIGHT pixels)
                let header_h = ENTITY_HEADER_HEIGHT as i32;
                let header_center_y = ny + header_h / 2;
                draw_text_centered_png(&mut img, name, cx, header_center_y, 1, image::Rgba([255, 255, 255, 255]));
                // Attributes
                let row_h = ENTITY_ROW_HEIGHT as i32;
                let attr_color = image::Rgba([50u8, 50, 50, 255]);
                for (i, attr) in attributes.iter().enumerate() {
                    let ry = ny + header_h + (i as i32) * row_h + row_h / 2;
                    let prefix = if attr.is_primary_key { "PK " } else if attr.is_foreign_key { "FK " } else { "" };
                    let attr_text = format!("{}{}", prefix, attr.name);
                    draw_text_centered_png(&mut img, &attr_text, cx, ry, 1, attr_color);
                }
            }
            NodeKind::Text { content } => {
                if !content.is_empty() {
                    let first_line = content.lines().next().unwrap_or(content);
                    draw_text_centered_png(&mut img, first_line, cx, cy, 1, text_rgba);
                }
            }
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

/// Render a single ASCII character from FONT5X7 at pixel position (x, y).
/// `scale` pixels per font pixel (2 = 10×14 rendered size).
fn draw_char_png(img: &mut image::RgbaImage, ch: char, x: i32, y: i32, scale: i32, color: image::Rgba<u8>) {
    let code = ch as u8;
    if !(0x20..=0x7E).contains(&code) {
        return;
    }
    let idx = (code - 0x20) as usize;
    let glyph = &FONT5X7[idx * 5..(idx + 1) * 5];
    for col in 0..5_i32 {
        let col_bits = glyph[col as usize];
        for row in 0..7_i32 {
            if col_bits & (1 << row) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        put_pixel_safe(img, x + col * scale + sx, y + row * scale + sy, color);
                    }
                }
            }
        }
    }
}

/// Render a string centered at pixel (cx, cy) using the 5×7 bitmap font.
fn draw_text_centered_png(img: &mut image::RgbaImage, text: &str, cx: i32, cy: i32, scale: i32, color: image::Rgba<u8>) {
    if text.is_empty() { return; }
    let char_w = (5 + 1) * scale; // 5 pixels wide + 1px gap
    let char_h = 7 * scale;
    let total_w = text.len() as i32 * char_w - scale; // remove trailing gap
    let x_start = cx - total_w / 2;
    let y_start = cy - char_h / 2;
    for (i, ch) in text.chars().enumerate() {
        draw_char_png(img, ch, x_start + i as i32 * char_w, y_start, scale, color);
    }
}

/// Draw a filled arrowhead triangle pointing from (sx,sy) toward (tx,ty).
/// The tip is at (tx,ty).
fn draw_arrowhead_png(img: &mut image::RgbaImage, sx: i32, sy: i32, tx: i32, ty: i32, color: image::Rgba<u8>) {
    let dx = (tx - sx) as f32;
    let dy = (ty - sy) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 { return; }
    let dirx = dx / len;
    let diry = dy / len;
    let perpx = -diry;
    let perpy = dirx;

    let arrow_len = 12.0_f32;
    let arrow_half_w = 5.0_f32;

    // Base center of the arrowhead triangle
    let bx = tx as f32 - dirx * arrow_len;
    let by = ty as f32 - diry * arrow_len;

    let p1 = (tx, ty); // tip
    let p2 = ((bx + perpx * arrow_half_w) as i32, (by + perpy * arrow_half_w) as i32);
    let p3 = ((bx - perpx * arrow_half_w) as i32, (by - perpy * arrow_half_w) as i32);

    draw_filled_polygon(img, &[p1, p2, p3], color);
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
<defs>
  <marker id="arrowhead" viewBox="0 0 10 10" refX="9" refY="5"
          markerUnits="strokeWidth" markerWidth="6" markerHeight="6" orient="auto">
    <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke"/>
  </marker>
</defs>
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
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}" stroke-opacity="{:.2}" marker-end="url(#arrowhead)"/>"#,
                sx, sy, tx, ty, color, edge.style.width, opacity,
            ));
            svg.push('\n');

            // Edge label at midpoint — use accessible dark color (#4c4f69) for WCAG AA contrast on white
            if !edge.label.is_empty() {
                let mx = (sx + tx) / 2.0;
                let my = (sy + ty) / 2.0;
                svg.push_str(&format!(
                    r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="12" fill="#4c4f69">{}</text>"##,
                    mx, my, xml_escape(&edge.label),
                ));
                svg.push('\n');
            }

            // Crow's foot symbols
            let ew = edge.style.width;
            svg_crow_foot(&mut svg, sx, sy, tx, ty, edge.source_cardinality, &color, ew, true);
            svg_crow_foot(&mut svg, sx, sy, tx, ty, edge.target_cardinality, &color, ew, false);

            // Source/target text labels — use accessible dark color (#4c4f69) for WCAG AA contrast on white
            if !edge.source_label.is_empty() {
                let lx = sx + (tx - sx) * 0.08;
                let ly = sy + (ty - sy) * 0.08 - 10.0;
                svg.push_str(&format!(
                    r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-family="sans-serif" font-size="11" fill="#4c4f69">{}</text>"##,
                    lx, ly, xml_escape(&edge.source_label),
                ));
                svg.push('\n');
            }
            if !edge.target_label.is_empty() {
                let lx = sx + (tx - sx) * 0.92;
                let ly = sy + (ty - sy) * 0.92 - 10.0;
                svg.push_str(&format!(
                    r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-family="sans-serif" font-size="11" fill="#4c4f69">{}</text>"##,
                    lx, ly, xml_escape(&edge.target_label),
                ));
                svg.push('\n');
            }
        }
    }

    // Swimlane dividers: horizontal separator lines + lane labels
    if doc.layout_mode == LayoutMode::Swimlane {
        use std::collections::HashMap;
        let canonical_lanes = &doc.timeline_lanes;
        let mut lane_order: Vec<String> = Vec::new();
        let mut lane_y_min: HashMap<String, f32> = HashMap::new();
        let mut lane_y_max: HashMap<String, f32> = HashMap::new();

        for node in &doc.nodes {
            let lane = match &node.timeline_lane {
                Some(l) if !l.is_empty() => l.clone(),
                _ => continue,
            };
            let ny = node.position[1] - min_y;
            let nh = node.size[1];
            let entry_min = lane_y_min.entry(lane.clone()).or_insert(f32::MAX);
            *entry_min = entry_min.min(ny);
            let entry_max = lane_y_max.entry(lane.clone()).or_insert(f32::MIN);
            *entry_max = entry_max.max(ny + nh);
            if !lane_order.contains(&lane) {
                lane_order.push(lane);
            }
        }

        let ordered: Vec<&String> = if !canonical_lanes.is_empty() {
            let mut v: Vec<&String> = canonical_lanes
                .iter()
                .filter(|l| lane_y_min.contains_key(l.as_str()))
                .collect();
            for l in &lane_order {
                if !canonical_lanes.contains(l) {
                    v.push(l);
                }
            }
            v
        } else {
            lane_order.iter().collect()
        };

        for (i, lane) in ordered.iter().enumerate() {
            if i > 0 {
                let prev = &ordered[i - 1];
                let prev_max = lane_y_max.get(prev.as_str()).copied().unwrap_or(0.0);
                let curr_min = lane_y_min.get(lane.as_str()).copied().unwrap_or(0.0);
                let div_y = (prev_max + curr_min) / 2.0;
                svg.push_str(&format!(
                    r##"<line x1="0" y1="{:.1}" x2="{}" y2="{:.1}" stroke="#969696" stroke-width="1" stroke-dasharray="6,4" stroke-opacity="0.5"/>"##,
                    div_y, width.ceil() as i32, div_y,
                ));
                svg.push('\n');
            }
            // Lane label on the left
            let y_min = lane_y_min.get(lane.as_str()).copied().unwrap_or(0.0);
            let y_max = lane_y_max.get(lane.as_str()).copied().unwrap_or(0.0);
            let label_y = (y_min + y_max) / 2.0;
            let label_x = 6.0;
            let lw = lane.len() as f32 * 7.0 + 16.0;
            let lh = 20.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" ry="4" fill="#1e1e2e" fill-opacity="0.78"/>"##,
                label_x - 2.0, label_y - lh / 2.0, lw, lh,
            ));
            svg.push('\n');
            svg.push_str(&format!(
                r##"<text x="{:.1}" y="{:.1}" font-family="sans-serif" font-size="11" fill="#b4befe" font-weight="bold">{}</text>"##,
                label_x + 6.0, label_y + 4.0, xml_escape(lane),
            ));
            svg.push('\n');
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

        match &node.kind {
            NodeKind::Shape { shape, label, description } => {
                match shape {
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
                    NodeShape::Connector => {
                        let ry = nh / 2.0;
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx, ny, nw, nh, ry, ry, fill, fill_opacity * 0.4, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Hexagon => {
                        let cx = nx + nw / 2.0;
                        let cy = ny + nh / 2.0;
                        let hw = nw / 2.0;
                        let hh = nh / 2.0;
                        let inset = hw * 0.3;
                        let points_str = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            cx - hw, cy,
                            cx - inset, cy - hh,
                            cx + inset, cy - hh,
                            cx + hw, cy,
                            cx + inset, cy + hh,
                            cx - inset, cy + hh,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            points_str, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Triangle => {
                        let cx = nx + nw / 2.0;
                        let points_str = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            cx, ny,          // apex top-center
                            nx + nw, ny + nh, // bottom-right
                            nx, ny + nh,     // bottom-left
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            points_str, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Callout => {
                        // Body: rounded rect
                        let body_h = nh * 0.75;
                        let tail_w = nw * 0.15;
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx, ny, nw, body_h, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        // Tail
                        let tail_pts = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            nx + 3.0, ny + body_h,
                            nx + tail_w, ny + body_h,
                            nx - 2.0, ny + nh,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            tail_pts, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Person => {
                        let cx = nx + nw / 2.0;
                        let head_r = nh * 0.22;
                        let neck_y = ny + head_r * 2.0;
                        let bottom = ny + nh;
                        // Head circle
                        svg.push_str(&format!(
                            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            cx, ny + head_r, head_r, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Body trapezoid
                        let pts = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            cx - nw * 0.28, neck_y,
                            cx + nw * 0.28, neck_y,
                            cx + nw * 0.18, bottom,
                            cx - nw * 0.18, bottom,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            pts, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Screen => {
                        let cx = nx + nw / 2.0;
                        // Darker fill for chrome details (clamp to avoid underflow)
                        let dr = (node.style.fill_color[0] as i32 - 30).clamp(0, 255) as u8;
                        let dg = (node.style.fill_color[1] as i32 - 30).clamp(0, 255) as u8;
                        let db = (node.style.fill_color[2] as i32 - 30).clamp(0, 255) as u8;
                        let darker = format!("#{:02x}{:02x}{:02x}", dr, dg, db);
                        // Main bezel
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx, ny, nw, nh * 0.75, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Chrome bar at top
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                            nx, ny, nw, nh * 0.1, darker, fill_opacity,
                        ));
                        svg.push('\n');
                        // Stand
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                            cx - nw * 0.06, ny + nh * 0.75, nw * 0.12, nh * 0.15, fill, fill_opacity,
                        ));
                        svg.push('\n');
                        // Base
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                            cx - nw * 0.2, ny + nh * 0.9, nw * 0.4, nh * 0.1, fill, fill_opacity,
                        ));
                    }
                    NodeShape::Cylinder => {
                        let cx = nx + nw / 2.0;
                        let ell_h = nh * 0.18;
                        let dr = (node.style.fill_color[0] as i32 - 25).clamp(0, 255) as u8;
                        let dg = (node.style.fill_color[1] as i32 - 25).clamp(0, 255) as u8;
                        let db = (node.style.fill_color[2] as i32 - 25).clamp(0, 255) as u8;
                        let darker = format!("#{:02x}{:02x}{:02x}", dr, dg, db);
                        // Body rect (no stroke so edges are covered by ellipses/lines)
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                            nx, ny + ell_h / 2.0, nw, nh - ell_h, fill, fill_opacity,
                        ));
                        svg.push('\n');
                        // Bottom ellipse
                        svg.push_str(&format!(
                            r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            cx, ny + nh - ell_h / 2.0, nw / 2.0, ell_h / 2.0, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Top ellipse (slightly darker)
                        svg.push_str(&format!(
                            r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            cx, ny + ell_h / 2.0, nw / 2.0, ell_h / 2.0, darker, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Left side stroke line
                        svg.push_str(&format!(
                            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx, ny + ell_h / 2.0, nx, ny + nh - ell_h / 2.0, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Right side stroke line
                        svg.push_str(&format!(
                            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx + nw, ny + ell_h / 2.0, nx + nw, ny + nh - ell_h / 2.0, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Cloud => {
                        // Base rounded rect (no stroke — bumps provide the outline)
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                            nx + nw * 0.1, ny + nh * 0.3, nw * 0.8, nh * 0.7, nh * 0.2, fill, fill_opacity,
                        ));
                        svg.push('\n');
                        // Four bump circles
                        for (bx, by, br) in [
                            (nx + nw * 0.2, ny + nh * 0.3, nh * 0.2),
                            (nx + nw * 0.4, ny + nh * 0.15, nh * 0.22),
                            (nx + nw * 0.6, ny + nh * 0.2, nh * 0.2),
                            (nx + nw * 0.8, ny + nh * 0.3, nh * 0.18),
                        ] {
                            svg.push_str(&format!(
                                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="none"/>"#,
                                bx, by, br, fill, fill_opacity,
                            ));
                            svg.push('\n');
                        }
                        // Outline stroke on base rect
                        svg.push_str(&format!(
                            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="none" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            nx + nw * 0.1, ny + nh * 0.3, nw * 0.8, nh * 0.7, nh * 0.2, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Document => {
                        let fold = nw.min(nh) * 0.2;
                        // Main body (5-point polygon with folded corner)
                        let pts = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            nx,           ny,
                            nx + nw - fold, ny,
                            nx + nw,      ny + fold,
                            nx + nw,      ny + nh,
                            nx,           ny + nh,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            pts, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                        svg.push('\n');
                        // Fold triangle (slightly darker)
                        let dr = (node.style.fill_color[0] as i32 - 20).clamp(0, 255) as u8;
                        let dg = (node.style.fill_color[1] as i32 - 20).clamp(0, 255) as u8;
                        let db = (node.style.fill_color[2] as i32 - 20).clamp(0, 255) as u8;
                        let darker = format!("#{:02x}{:02x}{:02x}", dr, dg, db);
                        let fold_pts = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            nx + nw - fold, ny,
                            nx + nw,        ny + fold,
                            nx + nw - fold, ny + fold,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            fold_pts, darker, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Channel => {
                        // Funnel / trapezoid
                        let pts = format!(
                            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                            nx,             ny,
                            nx + nw,        ny,
                            nx + nw * 0.72, ny + nh,
                            nx + nw * 0.28, ny + nh,
                        );
                        svg.push_str(&format!(
                            r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                            pts, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                        ));
                    }
                    NodeShape::Segment => {
                        // Two overlapping person shapes: background (82% scale, offset left)
                        // then foreground (full scale, shifted right).
                        let draw_person = |svg: &mut String, ox: f32, scale: f32, fi: &str, fo: f32, st: &str, so: f32, sw: f32| {
                            let pw = nw * scale;
                            let ph = nh * scale;
                            let pcx = ox + pw / 2.0;
                            let py = ny + (nh - ph); // align to bottom
                            let head_r = ph * 0.22;
                            let neck_y = py + head_r * 2.0;
                            let bottom = py + ph;
                            svg.push_str(&format!(
                                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                                pcx, py + head_r, head_r, fi, fo, st, so, sw,
                            ));
                            svg.push('\n');
                            let pts = format!(
                                "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                                pcx - pw * 0.28, neck_y,
                                pcx + pw * 0.28, neck_y,
                                pcx + pw * 0.18, bottom,
                                pcx - pw * 0.18, bottom,
                            );
                            svg.push_str(&format!(
                                r#"<polygon points="{}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                                pts, fi, fo, st, so, sw,
                            ));
                            svg.push('\n');
                        };
                        // Background person: 82% scale, offset to the left quarter
                        let dr = (node.style.fill_color[0] as i32 - 20).clamp(0, 255) as u8;
                        let dg = (node.style.fill_color[1] as i32 - 20).clamp(0, 255) as u8;
                        let db = (node.style.fill_color[2] as i32 - 20).clamp(0, 255) as u8;
                        let darker = format!("#{:02x}{:02x}{:02x}", dr, dg, db);
                        draw_person(&mut svg, nx + nw * 0.0, 0.82, &darker, fill_opacity * 0.85, &stroke, stroke_opacity, stroke_width);
                        // Foreground person: full scale, shifted right
                        draw_person(&mut svg, nx + nw * 0.18, 1.0, &fill, fill_opacity, &stroke, stroke_opacity, stroke_width);
                    }
                }
                svg.push('\n');

                // Compute vertical stacking for label + sublabel + tag badge.
                // We center the stack vertically around the node's mid-Y.
                // Sublabel source: explicit `node.sublabel` (set by some parsers) or
                // the indented continuation description parsed into `description`.
                let sublabel_text: &str = if !node.sublabel.is_empty() {
                    &node.sublabel
                } else if !description.is_empty() {
                    // Use only the first line of description to keep nodes compact.
                    description.lines().next().unwrap_or("")
                } else {
                    ""
                };
                let has_sublabel = !sublabel_text.is_empty();
                let has_tag = node.tag.is_some();
                let label_h = node.style.font_size;
                let sub_h = if has_sublabel { 14.0 } else { 0.0 };
                let tag_badge_h = if has_tag { 16.0 } else { 0.0 };
                let gap = if has_sublabel || has_tag { 4.0 } else { 0.0 };
                let stack_h = label_h + sub_h + tag_badge_h + gap * 2.0;
                let stack_top = ny + nh / 2.0 - stack_h / 2.0;

                // Shape label (top of stack)
                if !label.is_empty() {
                    let text_color = rgba_to_svg_color(node.style.text_color);
                    let text_opacity = node.style.text_color[3] as f32 / 255.0;
                    let text_x = nx + nw / 2.0;
                    let text_y = stack_top + label_h / 2.0;
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="{:.0}" fill="{}" fill-opacity="{:.2}">{}</text>"#,
                        text_x, text_y, node.style.font_size, text_color, text_opacity, xml_escape(label),
                    ));
                    svg.push('\n');
                }

                // Sublabel (second line, smaller, muted)
                if has_sublabel {
                    let sub_color = {
                        // Derive a muted variant of the text color (55% opacity mixed look)
                        let [r, g, b, _] = node.style.text_color;
                        format!("#{:02x}{:02x}{:02x}", r, g, b)
                    };
                    let sub_x = nx + nw / 2.0;
                    let sub_y = stack_top + label_h + gap + sub_h / 2.0;
                    // Truncate overly long sublabels to fit the node width.
                    // Approx 6px per character at font-size 10.
                    let max_chars = ((nw - 16.0) / 6.0).max(8.0) as usize;
                    let display_sub: String = if sublabel_text.chars().count() > max_chars {
                        let truncated: String = sublabel_text.chars().take(max_chars.saturating_sub(1)).collect();
                        format!("{}…", truncated)
                    } else {
                        sublabel_text.to_string()
                    };
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="10" fill="{}" fill-opacity="0.65">{}</text>"#,
                        sub_x, sub_y, sub_color, xml_escape(&display_sub),
                    ));
                    svg.push('\n');
                }

                // Tag badge (bottom of stack)
                if let Some(tag) = &node.tag {
                    let tag_rgba = tag.color();
                    let tag_fill = rgba_to_svg_color(tag_rgba);
                    let tag_text = tag.label();
                    let badge_w = tag_text.len() as f32 * 7.0 + 12.0;
                    let badge_x = nx + nw / 2.0 - badge_w / 2.0;
                    let badge_y = stack_top + label_h + if has_sublabel { gap + sub_h + gap } else { gap };
                    svg.push_str(&format!(
                        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" ry="8" fill="{}" fill-opacity="0.85"/>"#,
                        badge_x, badge_y, badge_w, tag_badge_h, tag_fill,
                    ));
                    svg.push('\n');
                    svg.push_str(&format!(
                        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="10" fill="#1e1e2e" font-weight="bold">{}</text>"##,
                        badge_x + badge_w / 2.0, badge_y + tag_badge_h / 2.0, xml_escape(tag_text),
                    ));
                    svg.push('\n');
                }
            }
            NodeKind::StickyNote { text, .. } => {
                svg.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" ry="8" fill="{}" fill-opacity="{:.2}"/>"#,
                    nx, ny, nw, nh, fill, fill_opacity,
                ));
                svg.push('\n');
                if !text.is_empty() {
                    let text_color = rgba_to_svg_color(node.style.text_color);
                    let text_x = nx + 10.0;
                    let text_y = ny + 20.0;
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" font-family="sans-serif" font-size="{:.0}" fill="{}">{}</text>"#,
                        text_x, text_y, node.style.font_size, text_color, xml_escape(text),
                    ));
                    svg.push('\n');
                }
            }
            NodeKind::Entity { name, attributes } => {
                // Body
                svg.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="3" ry="3" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
                    nx, ny, nw, nh, fill, fill_opacity, stroke, stroke_opacity, stroke_width,
                ));
                svg.push('\n');
                // Header
                let header_h = ENTITY_HEADER_HEIGHT;
                svg.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="3" ry="3" fill="{}" fill-opacity="{:.2}"/>"#,
                    nx, ny, nw, header_h, stroke, stroke_opacity,
                ));
                svg.push('\n');
                // Header divider
                let div_y = ny + header_h;
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                    nx, div_y, nx + nw, div_y, stroke,
                ));
                svg.push('\n');
                // Entity name
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="{:.0}" fill="white" font-weight="bold">{}</text>"#,
                    nx + nw / 2.0, ny + header_h / 2.0, node.style.font_size + 1.0, xml_escape(name),
                ));
                svg.push('\n');
                // Attributes
                let row_h = ENTITY_ROW_HEIGHT;
                let text_color = rgba_to_svg_color(node.style.text_color);
                for (i, attr) in attributes.iter().enumerate() {
                    let row_y = div_y + (i as f32) * row_h + row_h / 2.0;
                    let prefix = if attr.is_primary_key { "PK " } else if attr.is_foreign_key { "FK " } else { "" };
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" dominant-baseline="middle" font-family="sans-serif" font-size="{:.0}" fill="{}">{}{}</text>"#,
                        nx + 8.0, row_y, node.style.font_size, text_color, prefix, xml_escape(&attr.name),
                    ));
                    svg.push('\n');
                    let dim_color = "#6c7086";
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" text-anchor="end" dominant-baseline="middle" font-family="monospace" font-size="{:.0}" fill="{}">{}</text>"#,
                        nx + nw - 8.0, row_y, node.style.font_size * 0.85, dim_color, xml_escape(&attr.attr_type),
                    ));
                    svg.push('\n');
                }
            }
            NodeKind::Text { content } => {
                if !content.is_empty() {
                    let text_color = rgba_to_svg_color(node.style.text_color);
                    let text_opacity = node.style.text_color[3] as f32 / 255.0;
                    svg.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" font-family="sans-serif" font-size="{:.0}" fill="{}" fill-opacity="{:.2}">{}</text>"#,
                        nx, ny + node.style.font_size, node.style.font_size, text_color, text_opacity, xml_escape(content),
                    ));
                    svg.push('\n');
                }
            }
        }
    }

    svg.push_str("</svg>\n");

    std::fs::write(path, svg).map_err(|e| e.to_string())
}

/// Render a crow's foot cardinality symbol in SVG at an edge endpoint.
/// If `is_source` is true, symbol is at (sx,sy); otherwise at (tx,ty).
#[allow(clippy::too_many_arguments)]
fn svg_crow_foot(
    svg: &mut String,
    sx: f32, sy: f32, tx: f32, ty: f32,
    cardinality: Cardinality,
    color: &str,
    stroke_w: f32,
    is_source: bool,
) {
    if cardinality == Cardinality::None {
        return;
    }
    // Direction from the approach side toward the endpoint
    let (ex, ey, fx, fy) = if is_source {
        // endpoint is source, approach from target direction
        (sx, sy, tx, ty)
    } else {
        // endpoint is target, approach from source direction
        (tx, ty, sx, sy)
    };
    let dx = ex - fx;
    let dy = ey - fy;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let dirx = dx / len;
    let diry = dy / len;
    let perpx = -diry;
    let perpy = dirx;

    let bar_half = 8.0;
    let circle_r = 5.0;
    let foot_spread = 8.0;
    let foot_len = 12.0;
    let outer_dist = 3.0;
    let inner_dist = 15.0;

    match cardinality {
        Cardinality::None => {}
        Cardinality::ExactlyOne => {
            let ox = ex - dirx * outer_dist;
            let oy = ey - diry * outer_dist;
            let ix = ex - dirx * inner_dist;
            let iy = ey - diry * inner_dist;
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                ox + perpx * bar_half, oy + perpy * bar_half,
                ox - perpx * bar_half, oy - perpy * bar_half, color, stroke_w,
            ));
            svg.push('\n');
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                ix + perpx * bar_half, iy + perpy * bar_half,
                ix - perpx * bar_half, iy - perpy * bar_half, color, stroke_w,
            ));
            svg.push('\n');
        }
        Cardinality::ZeroOrOne => {
            let ox = ex - dirx * outer_dist;
            let oy = ey - diry * outer_dist;
            let ccx = ex - dirx * (inner_dist + circle_r);
            let ccy = ey - diry * (inner_dist + circle_r);
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                ox + perpx * bar_half, oy + perpy * bar_half,
                ox - perpx * bar_half, oy - perpy * bar_half, color, stroke_w,
            ));
            svg.push('\n');
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="none" stroke="{}" stroke-width="{:.1}"/>"#,
                ccx, ccy, circle_r, color, stroke_w,
            ));
            svg.push('\n');
        }
        Cardinality::OneOrMany => {
            let ix = ex - dirx * inner_dist;
            let iy = ey - diry * inner_dist;
            let cx = ex - dirx * foot_len;
            let cy = ey - diry * foot_len;
            // Crow's foot prongs
            for sign in [-1.0_f32, 0.0, 1.0] {
                let px = ex + perpx * foot_spread * sign;
                let py = ey + perpy * foot_spread * sign;
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                    cx, cy, px, py, color, stroke_w,
                ));
                svg.push('\n');
            }
            // Inner bar
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                ix + perpx * bar_half, iy + perpy * bar_half,
                ix - perpx * bar_half, iy - perpy * bar_half, color, stroke_w,
            ));
            svg.push('\n');
        }
        Cardinality::ZeroOrMany => {
            let cx = ex - dirx * foot_len;
            let cy = ey - diry * foot_len;
            let ccx = ex - dirx * (inner_dist + circle_r);
            let ccy = ey - diry * (inner_dist + circle_r);
            // Crow's foot prongs
            for sign in [-1.0_f32, 0.0, 1.0] {
                let px = ex + perpx * foot_spread * sign;
                let py = ey + perpy * foot_spread * sign;
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{:.1}"/>"#,
                    cx, cy, px, py, color, stroke_w,
                ));
                svg.push('\n');
            }
            // Inner circle
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="none" stroke="{}" stroke-width="{:.1}"/>"#,
                ccx, ccy, circle_r, color, stroke_w,
            ));
            svg.push('\n');
        }
    }
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

// ---------------------------------------------------------------------------
// PDF Export
// ---------------------------------------------------------------------------

/// Pixels to millimeters conversion factor (approximately 96 DPI).
const PX_TO_MM: f32 = 0.264583;

/// Convert an RGBA [u8;4] color to a printpdf RGB Color.
fn rgba_to_pdf_color(c: [u8; 4]) -> printpdf::Color {
    printpdf::Color::Rgb(printpdf::Rgb::new(
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        None,
    ))
}

/// Draw a single node as a PDF rectangle (or polygon) on the given layer.
/// PDF coordinates have origin at bottom-left, so we flip Y.
fn draw_pdf_node(
    layer: &printpdf::PdfLayerReference,
    node: &Node,
    min_x: f32,
    min_y: f32,
    page_height_mm: f32,
) {
    use printpdf::{Mm, Point, Polygon};
    use printpdf::path::PaintMode;

    let nx = (node.position[0] - min_x) * PX_TO_MM;
    let ny = (node.position[1] - min_y) * PX_TO_MM;
    let nw = node.size[0] * PX_TO_MM;
    let nh = node.size[1] * PX_TO_MM;

    // Flip Y: PDF bottom-left origin => top_y = page_height - ny, bottom_y = page_height - (ny+nh)
    let top_y = page_height_mm - ny;
    let bottom_y = page_height_mm - (ny + nh);

    // Set colors
    layer.set_fill_color(rgba_to_pdf_color(node.style.fill_color));
    layer.set_outline_color(rgba_to_pdf_color(node.style.border_color));
    layer.set_outline_thickness(node.style.border_width * PX_TO_MM);

    match &node.kind {
        NodeKind::Shape { shape, .. } => match shape {
            NodeShape::Rectangle | NodeShape::RoundedRect => {
                let rect = printpdf::Rect::new(
                    Mm(nx),
                    Mm(bottom_y),
                    Mm(nx + nw),
                    Mm(top_y),
                )
                .with_mode(PaintMode::FillStroke);
                layer.add_rect(rect);
            }
            NodeShape::Circle => {
                let cx = nx + nw / 2.0;
                let cy = (top_y + bottom_y) / 2.0;
                let rx = nw / 2.0;
                let ry = nh / 2.0;
                let segments = 32;
                let points: Vec<(Point, bool)> = (0..segments)
                    .map(|i| {
                        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
                        let px = cx + rx * angle.cos();
                        let py = cy + ry * angle.sin();
                        (Point::new(Mm(px), Mm(py)), false)
                    })
                    .collect();
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Diamond => {
                let cx = nx + nw / 2.0;
                let cy = (top_y + bottom_y) / 2.0;
                let hw = nw / 2.0;
                let hh = nh / 2.0;
                let points = vec![
                    (Point::new(Mm(cx), Mm(cy + hh)), false),
                    (Point::new(Mm(cx + hw), Mm(cy)), false),
                    (Point::new(Mm(cx), Mm(cy - hh)), false),
                    (Point::new(Mm(cx - hw), Mm(cy)), false),
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Parallelogram => {
                let skew = nw * 0.15;
                let points = vec![
                    (Point::new(Mm(nx + skew), Mm(top_y)), false),
                    (Point::new(Mm(nx + nw), Mm(top_y)), false),
                    (Point::new(Mm(nx + nw - skew), Mm(bottom_y)), false),
                    (Point::new(Mm(nx), Mm(bottom_y)), false),
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Connector => {
                // Render as rounded rectangle in PDF
                let points = vec![
                    (Point::new(Mm(nx), Mm(top_y)), false),
                    (Point::new(Mm(nx + nw), Mm(top_y)), false),
                    (Point::new(Mm(nx + nw), Mm(bottom_y)), false),
                    (Point::new(Mm(nx), Mm(bottom_y)), false),
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Hexagon => {
                let cx = nx + nw / 2.0;
                let cy = (top_y + bottom_y) / 2.0;
                let hw = nw / 2.0;
                let hh = nh / 2.0;
                let inset = hw * 0.3;
                let points = vec![
                    (Point::new(Mm(cx - hw),    Mm(cy)), false),
                    (Point::new(Mm(cx - inset), Mm(cy + hh)), false),
                    (Point::new(Mm(cx + inset), Mm(cy + hh)), false),
                    (Point::new(Mm(cx + hw),    Mm(cy)), false),
                    (Point::new(Mm(cx + inset), Mm(cy - hh)), false),
                    (Point::new(Mm(cx - inset), Mm(cy - hh)), false),
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Triangle => {
                let cx = nx + nw / 2.0;
                let points = vec![
                    (Point::new(Mm(cx),      Mm(top_y)),    false), // apex
                    (Point::new(Mm(nx + nw), Mm(bottom_y)), false), // bottom-right
                    (Point::new(Mm(nx),      Mm(bottom_y)), false), // bottom-left
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::FillStroke,
                    winding_order: printpdf::path::WindingOrder::NonZero,
                };
                layer.add_polygon(polygon);
            }
            NodeShape::Callout => {
                // Draw body as a rectangle (PDF export approximation)
                let rect = printpdf::Rect::new(
                    Mm(nx), Mm(bottom_y + nh * 0.25), Mm(nx + nw), Mm(top_y)
                ).with_mode(PaintMode::FillStroke);
                layer.add_rect(rect);
            }
            // New shapes — fall back to rounded rectangle for PDF until Task 3.4
            _ => {
                let rect = printpdf::Rect::new(
                    Mm(nx), Mm(bottom_y), Mm(nx + nw), Mm(top_y)
                ).with_mode(PaintMode::FillStroke);
                layer.add_rect(rect);
            }
        },
        NodeKind::StickyNote { .. } | NodeKind::Entity { .. } => {
            // Draw as rectangle for both sticky notes and entities
            let rect = printpdf::Rect::new(
                Mm(nx),
                Mm(bottom_y),
                Mm(nx + nw),
                Mm(top_y),
            )
            .with_mode(PaintMode::FillStroke);
            layer.add_rect(rect);
        }
        NodeKind::Text { .. } => {
            // No shape to draw for text nodes
        }
    }
}

pub fn export_pdf(doc: &FlowchartDocument, path: &Path) -> Result<(), String> {
    use printpdf::{BuiltinFont, Mm, PdfDocument};
    use std::fs::File;
    use std::io::BufWriter;

    let (min_x, min_y, max_x, max_y) = bounding_box(doc)
        .ok_or_else(|| "Nothing to export: document has no nodes".to_string())?;

    let width_mm = (max_x - min_x) * PX_TO_MM;
    let height_mm = (max_y - min_y) * PX_TO_MM;

    let (pdf_doc, page_idx, layer_idx) =
        PdfDocument::new("Flowchart", Mm(width_mm), Mm(height_mm), "Layer 1");

    let font = pdf_doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;

    let layer = pdf_doc.get_page(page_idx).get_layer(layer_idx);

    // Draw edges first (behind nodes)
    for edge in &doc.edges {
        let src_node = doc.find_node(&edge.source.node_id);
        let tgt_node = doc.find_node(&edge.target.node_id);
        if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
            let src = sn.port_position(edge.source.side);
            let tgt = tn.port_position(edge.target.side);
            let sx = (src.x - min_x) * PX_TO_MM;
            let sy = height_mm - (src.y - min_y) * PX_TO_MM;
            let tx = (tgt.x - min_x) * PX_TO_MM;
            let ty = height_mm - (tgt.y - min_y) * PX_TO_MM;

            layer.set_outline_color(rgba_to_pdf_color(edge.style.color));
            layer.set_outline_thickness(edge.style.width * PX_TO_MM);

            let line = printpdf::Line {
                points: vec![
                    (printpdf::Point::new(Mm(sx), Mm(sy)), false),
                    (printpdf::Point::new(Mm(tx), Mm(ty)), false),
                ],
                is_closed: false,
            };
            layer.add_line(line);
        }
    }

    // Draw nodes
    for node in &doc.nodes {
        draw_pdf_node(&layer, node, min_x, min_y, height_mm);
    }

    // Draw node labels
    for node in &doc.nodes {
        let label = node.display_label();
        if !label.is_empty() {
            let nx = (node.position[0] - min_x) * PX_TO_MM;
            let ny = (node.position[1] - min_y) * PX_TO_MM;
            let nw = node.size[0] * PX_TO_MM;
            let nh = node.size[1] * PX_TO_MM;

            let font_size_mm = node.style.font_size * PX_TO_MM;
            let approx_text_width = label.len() as f32 * font_size_mm * 0.5;
            let text_x = nx + nw / 2.0 - approx_text_width / 2.0;
            let text_y = height_mm - (ny + nh / 2.0) - font_size_mm * 0.3;

            layer.set_fill_color(rgba_to_pdf_color(node.style.text_color));
            layer.use_text(label, font_size_mm * 2.83465, Mm(text_x), Mm(text_y), &font);
        }
    }

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    pdf_doc.save(&mut writer).map_err(|e| e.to_string())
}
