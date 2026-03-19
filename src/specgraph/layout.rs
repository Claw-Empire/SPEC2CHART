use std::collections::{HashMap, VecDeque};
use crate::model::{FlowchartDocument, LayoutMode, NodeId};

/// Timeline grid layout.
///
/// Builds a Period × Lane grid and positions nodes into cells.
/// If `doc.timeline_dir` is "TB", periods run top-to-bottom and lanes left-to-right.
/// Otherwise (default "LR"), periods run left-to-right and lanes top-to-bottom.
pub fn timeline_layout(doc: &mut FlowchartDocument) {
    const CELL_PAD: f32 = 16.0;   // padding inside each cell
    const HEADER_H: f32 = 36.0;   // period header height (LR mode)
    const CELL_GAP: f32 = 12.0;   // gap between nodes in same cell
    const ORIGIN_X: f32 = 140.0;  // canvas left margin (LR: reserves lane label column)
    const ORIGIN_Y: f32 = 60.0;   // canvas top margin (LR: reserves period header row)
    const MIN_CELL_W: f32 = 180.0;
    const MIN_CELL_H: f32 = 80.0;

    let periods = doc.timeline_periods.clone();
    let mut lanes = doc.timeline_lanes.clone();

    if periods.is_empty() { return; }

    // Collect nodes by (period, lane)
    let mut cell_map: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    let mut unlaned: Vec<usize> = Vec::new(); // nodes with no lane
    let mut unperioded: Vec<usize> = Vec::new(); // nodes with no period

    // Auto-discover lanes from nodes in document order
    for (ni, node) in doc.nodes.iter().enumerate() {
        let p_idx = node.timeline_period.as_ref()
            .and_then(|p| periods.iter().position(|x| x == p));
        let l_idx = node.timeline_lane.as_ref().map(|l| {
            if let Some(pos) = lanes.iter().position(|x| x == l) {
                pos
            } else {
                lanes.push(l.clone());
                lanes.len() - 1
            }
        });
        match (p_idx, l_idx) {
            (Some(p), Some(l)) => { cell_map.entry((p, l)).or_default().push(ni); }
            (Some(p), None) => {
                // If lanes exist, put in implicit unlaned slot at end
                if !lanes.is_empty() {
                    unlaned.push(ni);
                } else {
                    // No lanes at all — create a single implicit "all" lane
                    cell_map.entry((p, 0)).or_default().push(ni);
                }
            }
            _ => { unperioded.push(ni); }
        }
    }

    // Update doc.timeline_lanes with any auto-discovered lanes
    doc.timeline_lanes = lanes.clone();

    let num_periods = periods.len();
    let num_lanes = if lanes.is_empty() { 1 } else { lanes.len() + if !unlaned.is_empty() { 1 } else { 0 } };

    // Compute cell sizes: each cell is sized to fit its node count
    // Column widths (one per period), row heights (one per lane)
    let mut col_w: Vec<f32> = vec![MIN_CELL_W; num_periods];
    let mut row_h: Vec<f32> = vec![MIN_CELL_H; num_lanes];

    for (&(p, l), node_indices) in &cell_map {
        if p >= num_periods || l >= num_lanes { continue; }
        let total_h: f32 = node_indices.iter().map(|&ni| doc.nodes[ni].size[1]).sum::<f32>()
            + CELL_GAP * (node_indices.len().saturating_sub(1)) as f32
            + CELL_PAD * 2.0;
        let max_w: f32 = node_indices.iter().map(|&ni| doc.nodes[ni].size[0]).fold(MIN_CELL_W, f32::max)
            + CELL_PAD * 2.0;
        col_w[p] = col_w[p].max(max_w);
        row_h[l] = row_h[l].max(total_h);
    }

    // Compute cumulative column X positions and row Y positions (LR mode)
    let mut col_x: Vec<f32> = Vec::with_capacity(num_periods);
    let mut cx = ORIGIN_X;
    for w in &col_w { col_x.push(cx); cx += w; }

    let mut row_y: Vec<f32> = Vec::with_capacity(num_lanes);
    let mut ry = ORIGIN_Y + HEADER_H;
    for h in &row_h { row_y.push(ry); ry += h; }

    // Position nodes into their cells
    for (&(p, l), node_indices) in &cell_map {
        if p >= num_periods || l >= num_lanes { continue; }
        let cell_left = col_x[p] + CELL_PAD;
        let mut y = row_y[l] + CELL_PAD;
        for &ni in node_indices {
            doc.nodes[ni].position = [cell_left, y];
            y += doc.nodes[ni].size[1] + CELL_GAP;
        }
    }

    // Position unperioded nodes far below the grid
    let grid_bottom = row_y.last().copied().unwrap_or(ORIGIN_Y + HEADER_H)
        + row_h.last().copied().unwrap_or(MIN_CELL_H) + 60.0;
    let mut ux = ORIGIN_X;
    for &ni in &unperioded {
        doc.nodes[ni].position = [ux, grid_bottom];
        ux += doc.nodes[ni].size[0] + CELL_GAP;
    }

    // Position unlaned nodes in the last implicit row
    if !unlaned.is_empty() && !row_y.is_empty() {
        let unlaned_y = row_y.last().copied().unwrap_or(0.0)
            + row_h.last().copied().unwrap_or(MIN_CELL_H);
        // Group by period
        let mut unlaned_by_period: HashMap<usize, Vec<usize>> = HashMap::new();
        for &ni in &unlaned {
            let p = doc.nodes[ni].timeline_period.as_ref()
                .and_then(|p| periods.iter().position(|x| x == p))
                .unwrap_or(0);
            unlaned_by_period.entry(p).or_default().push(ni);
        }
        for (p, nodes) in &unlaned_by_period {
            if *p >= num_periods { continue; }
            let cell_left = col_x[*p] + CELL_PAD;
            let mut y = unlaned_y + CELL_PAD;
            for &ni in nodes {
                doc.nodes[ni].position = [cell_left, y];
                y += doc.nodes[ni].size[1] + CELL_GAP;
            }
        }
    }
}

/// Bucket key for nodes that have no `timeline_lane` assignment.
/// Using a double-underscore-wrapped key prevents accidental collision with
/// user-defined lane names and avoids the sentinel being rendered as a label.
const UNLANED_KEY: &str = "__unlaned__";

/// Swimlane layout — positions nodes in horizontal rows, one row per lane.
///
/// Nodes belonging to the same `timeline_lane` are grouped on the same Y band.
/// Lane order follows `doc.timeline_lanes`; nodes not assigned to a lane are
/// placed below all named lanes. Within each lane nodes are stacked left-to-right.
pub fn swimlane_layout(doc: &mut FlowchartDocument) {
    const LANE_PAD: f32 = 16.0;   // padding inside each lane band
    const NODE_GAP: f32 = 20.0;   // horizontal gap between nodes in same lane
    const LANE_GAP: f32 = 32.0;   // vertical gap between lane bands
    const ORIGIN_X: f32 = 160.0;  // left margin (reserves space for lane label)
    const ORIGIN_Y: f32 = 60.0;   // top margin
    const MIN_LANE_H: f32 = 80.0; // minimum height of a lane band

    let lanes = doc.timeline_lanes.clone();

    // Group node indices by lane name, preserving doc order within each lane.
    // Unlaned nodes go to a catch-all bucket at the end.
    let mut lane_map: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    let mut unlaned: Vec<usize> = Vec::new();

    for (ni, node) in doc.nodes.iter().enumerate() {
        match &node.timeline_lane {
            Some(l) => lane_map.entry(l.clone()).or_default().push(ni),
            None    => unlaned.push(ni),
        }
    }

    // Walk lanes in declared order, then unlaned at the bottom.
    let mut ordered_lanes: Vec<(String, Vec<usize>)> = lanes
        .iter()
        .filter_map(|name| {
            lane_map.remove(name.as_str()).map(|v| (name.clone(), v))
        })
        .collect();
    // Any lanes discovered but not in doc.timeline_lanes (shouldn't happen, but be safe).
    for (name, indices) in lane_map {
        ordered_lanes.push((name, indices));
    }
    if !unlaned.is_empty() {
        ordered_lanes.push((UNLANED_KEY.to_string(), unlaned));
    }

    // Pre-compute band heights (separate read pass to avoid borrow conflict).
    let band_heights: Vec<f32> = ordered_lanes
        .iter()
        .map(|(_, indices)| {
            let max_h = indices
                .iter()
                .map(|&i| doc.nodes[i].size[1])
                .fold(0.0_f32, f32::max);
            (max_h + LANE_PAD * 2.0).max(MIN_LANE_H)
        })
        .collect();

    // Assign Y positions per band, then X positions within each band.
    let mut y = ORIGIN_Y;
    for ((_, indices), &bh) in ordered_lanes.iter().zip(band_heights.iter()) {
        let mut x = ORIGIN_X;
        for &ni in indices {
            let node_h = doc.nodes[ni].size[1];
            let y_center = y + LANE_PAD + (bh - LANE_PAD * 2.0 - node_h) / 2.0;
            doc.nodes[ni].position = [x, y_center.max(y + LANE_PAD)];
            x += doc.nodes[ni].size[0] + NODE_GAP;
        }
        y += bh + LANE_GAP;
    }
}

/// Org-tree (top-down tree) layout using BFS depth assignment.
///
/// Finds all root nodes (in-degree 0), assigns each node a depth level via BFS,
/// then positions nodes in rows: depth 0 at top, increasing depth downward.
/// Within each depth row, nodes are spread evenly and centred.
pub fn orgtree_layout(doc: &mut FlowchartDocument) {
    let node_count = doc.nodes.len();
    if node_count == 0 { return; }

    // Map NodeId -> index in doc.nodes
    let id_to_idx: HashMap<_, _> = doc.nodes.iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect();

    // Compute in-degrees to find roots
    let mut in_degree = vec![0u32; node_count];
    for edge in &doc.edges {
        if let Some(&tgt_i) = id_to_idx.get(&edge.target.node_id) {
            in_degree[tgt_i] += 1;
        }
    }

    // BFS to assign depth level
    let mut depth = vec![0usize; node_count];
    let roots: Vec<usize> = (0..node_count).filter(|&i| in_degree[i] == 0).collect();
    let mut queue = VecDeque::from(roots.clone());
    let mut visited = vec![false; node_count];
    for &r in &roots { visited[r] = true; }

    while let Some(i) = queue.pop_front() {
        let node_id = doc.nodes[i].id;
        for edge in &doc.edges {
            if edge.source.node_id == node_id {
                if let Some(&tgt_i) = id_to_idx.get(&edge.target.node_id) {
                    if !visited[tgt_i] {
                        depth[tgt_i] = depth[i] + 1;
                        visited[tgt_i] = true;
                        queue.push_back(tgt_i);
                    }
                }
            }
        }
    }

    // Note: nodes in a cycle are never enqueued (visited remains false, depth stays 0).
    // They will be placed at the root row (y=0). This is intentional silent handling —
    // OrgTree specs are expected to be DAGs.
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let gap_y = doc.layout_gap_main.max(120.0);
    let gap_x = doc.layout_gap_cross.max(160.0);

    for d in 0..=max_depth {
        let at_depth: Vec<usize> = (0..node_count).filter(|&i| depth[i] == d).collect();
        let count = at_depth.len() as f32;
        let start_x = -(count - 1.0) * gap_x / 2.0;
        for (j, &i) in at_depth.iter().enumerate() {
            doc.nodes[i].position = [
                start_x + j as f32 * gap_x,
                d as f32 * gap_y,
            ];
        }
    }
}

/// Kanban layout — positions cards in vertical columns, one column per `doc.kanban_columns` entry.
///
/// Nodes with `timeline_lane == Some(col_name)` are placed in the matching column,
/// stacked top-to-bottom. Column order follows `doc.kanban_columns`. Nodes with no
/// lane assignment are placed in a row below all columns.
pub fn kanban_layout(doc: &mut FlowchartDocument) {
    let col_width = 200.0f32;
    let card_height = 80.0f32;
    let gap_y = 20.0f32;
    let col_gap = 40.0f32;
    let padding_top = 60.0f32; // space for column header label

    for (col_idx, col_name) in doc.kanban_columns.clone().iter().enumerate() {
        let col_x = col_idx as f32 * (col_width + col_gap);
        let mut row = 0usize;
        for node in doc.nodes.iter_mut() {
            if node.timeline_lane.as_deref() == Some(col_name.as_str()) {
                node.position = [
                    col_x,
                    padding_top + row as f32 * (card_height + gap_y),
                ];
                node.size = [col_width - 20.0, card_height];
                row += 1;
            }
        }
    }

    // Nodes without a column: place below all columns
    let unassigned_y = doc.kanban_columns.len() as f32 * 200.0 + 40.0;
    let mut unassigned_x = 0.0f32;
    for node in doc.nodes.iter_mut() {
        if node.timeline_lane.is_none() {
            node.position = [unassigned_x, unassigned_y];
            unassigned_x += col_width + col_gap;
        }
    }
}

/// Dispatch helper — calls the appropriate layout function based on doc state.
///
/// Priority:
/// 1. `layout_mode` field (set by parser) — OrgTree, Kanban, Swimlane each route directly.
/// 2. `timeline_mode = true` or non-empty periods → timeline_layout (period × lane grid)
/// 3. otherwise → hierarchical_layout
pub fn auto_layout(doc: &mut FlowchartDocument) {
    if doc.layout_mode == LayoutMode::OrgTree {
        orgtree_layout(doc);
        return;
    }
    if doc.layout_mode == LayoutMode::Kanban {
        kanban_layout(doc);
        return;
    }
    if doc.layout_mode == LayoutMode::Swimlane {
        swimlane_layout(doc);
        return;
    }
    if doc.timeline_mode || !doc.timeline_lanes.is_empty() {
        timeline_layout(doc);
    } else {
        hierarchical_layout(doc);
    }
}

/// Hierarchical (layered) auto-layout.
///
/// Nodes are assigned to layers based on their longest path from
/// any root (node with no incoming edges). Within each layer they are spread
/// evenly and the whole layer is centred over the widest layer.
/// Only nodes still at the origin [0, 0] are repositioned.
///
/// `dir` controls the primary flow axis: "TB" (default), "LR", "RL", "BT".
pub fn hierarchical_layout(doc: &mut FlowchartDocument) {
    let dir = doc.layout_dir.clone();
    hierarchical_layout_dir(doc, &dir);
}

fn hierarchical_layout_dir(doc: &mut FlowchartDocument, dir: &str) {
    let n = doc.nodes.len();
    if n == 0 {
        return;
    }

    // Index map: NodeId -> sequential index
    let node_index: HashMap<NodeId, usize> = doc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id, i))
        .collect();

    // Build adjacency list and in-degree from edges
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_deg: Vec<i32> = vec![0; n];
    for edge in &doc.edges {
        if let (Some(&from), Some(&to)) = (
            node_index.get(&edge.source.node_id),
            node_index.get(&edge.target.node_id),
        ) {
            if from != to {
                adj[from].push(to);
                in_deg[to] += 1;
            }
        }
    }

    // Kahn's BFS topological sort with longest-path-from-root layering.
    // layer[i] = max number of edges from any root to node i.
    let mut layer: Vec<i32> = vec![0; n];
    let mut rem_in: Vec<i32> = in_deg.clone();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut topo: Vec<usize> = Vec::with_capacity(n);

    for i in 0..n {
        if rem_in[i] == 0 {
            queue.push_back(i);
        }
    }

    while let Some(u) = queue.pop_front() {
        topo.push(u);
        for &v in &adj[u] {
            let candidate = layer[u] + 1;
            if candidate > layer[v] {
                layer[v] = candidate;
            }
            rem_in[v] -= 1;
            if rem_in[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    // Nodes in cycles never entered the queue; assign them to layer 0.
    // (Their topo position doesn't matter for the layout.)

    // Group nodes by layer, preserving topo order within each layer.
    let max_layer = layer.iter().copied().max().unwrap_or(0) as usize;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];

    // First add nodes in topo order so within-layer order is deterministic
    for &i in &topo {
        layers[layer[i] as usize].push(i);
    }
    // Then add any remaining (cycle) nodes
    for i in 0..n {
        if rem_in[i] > 0 {
            // still had unresolved in-edges → part of a cycle
            layers[0].push(i);
        }
    }

    // Layout constants — use config values when non-zero, else defaults
    let gap_main_default = 80.0_f32;
    let gap_cross_default = 60.0_f32;
    let GAP_MAIN: f32 = if doc.layout_gap_main > 0.0 { doc.layout_gap_main } else { gap_main_default };
    let GAP_CROSS: f32 = if doc.layout_gap_cross > 0.0 { doc.layout_gap_cross } else { gap_cross_default };
    const START: f32 = 100.0;

    // Determine if this is an LR/RL layout (horizontal main axis)
    let horizontal = matches!(dir, "LR" | "RL");
    let reverse = matches!(dir, "RL" | "BT");

    if horizontal {
        // LR/RL: layers advance along X, nodes within a layer stacked along Y
        let layer_heights: Vec<f32> = layers.iter().map(|nodes| {
            let h: f32 = nodes.iter().map(|&i| doc.nodes[i].size[1]).sum();
            let g = GAP_CROSS * (nodes.len().saturating_sub(1) as f32);
            h + g
        }).collect();

        let canvas_height = layer_heights.iter().cloned().fold(0.0_f32, f32::max);
        let centre_y = START + canvas_height / 2.0;

        let total_layers = layers.len();
        let mut x = START;
        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            if layer_nodes.is_empty() { continue; }
            let actual_idx = if reverse { total_layers - 1 - layer_idx } else { layer_idx };
            let lh = layer_heights[actual_idx];
            let mut y = centre_y - lh / 2.0;

            let max_w = layer_nodes.iter().map(|&i| doc.nodes[i].size[0]).fold(0.0_f32, f32::max);
            for &i in layer_nodes {
                let pos = doc.nodes[i].position;
                let has_explicit_pos = pos != [0.0, 0.0] && doc.nodes[i].pinned;
                if !has_explicit_pos {
                    let node_w = doc.nodes[i].size[0];
                    let x_offset = (max_w - node_w) / 2.0;
                    doc.nodes[i].position = [x + x_offset, y];
                }
                y += doc.nodes[i].size[1] + GAP_CROSS;
            }
            x += max_w + GAP_MAIN;
        }
    } else {
        // TB/BT (default): layers advance along Y, nodes within a layer spread along X
        let layer_widths: Vec<f32> = layers.iter().map(|nodes| {
            let w: f32 = nodes.iter().map(|&i| doc.nodes[i].size[0]).sum();
            let g = GAP_CROSS * (nodes.len().saturating_sub(1) as f32);
            w + g
        }).collect();

        let canvas_width = layer_widths.iter().cloned().fold(0.0_f32, f32::max);
        let centre_x = START + canvas_width / 2.0;

        let total_layers = layers.len();
        let mut y = START;
        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            if layer_nodes.is_empty() { continue; }
            let actual_idx = if reverse { total_layers - 1 - layer_idx } else { layer_idx };
            let lw = layer_widths[actual_idx];
            let mut x = centre_x - lw / 2.0;

            let max_h = layer_nodes.iter().map(|&i| doc.nodes[i].size[1]).fold(0.0_f32, f32::max);
            for &i in layer_nodes {
                let pos = doc.nodes[i].position;
                let has_explicit_pos = pos != [0.0, 0.0] && doc.nodes[i].pinned;
                if !has_explicit_pos {
                    let node_h = doc.nodes[i].size[1];
                    let y_offset = (max_h - node_h) / 2.0;
                    doc.nodes[i].position = [x, y + y_offset];
                }
                x += doc.nodes[i].size[0] + GAP_CROSS;
            }
            y += max_h + GAP_MAIN;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orgtree_layout_root_at_top() {
        use crate::specgraph::hrf::parse_hrf;
        let spec = "## OrgTree\n- [ceo] CEO\n  - [cto] CTO\n  - [coo] COO\n";
        let mut doc = parse_hrf(spec).unwrap();
        crate::specgraph::layout::auto_layout(&mut doc);
        let ceo = doc.nodes.iter().find(|n| n.hrf_id == "ceo").unwrap();
        let cto = doc.nodes.iter().find(|n| n.hrf_id == "cto").unwrap();
        let coo = doc.nodes.iter().find(|n| n.hrf_id == "coo").unwrap();
        // CEO (root, depth 0) should be above CTO and COO (depth 1)
        assert!(ceo.position[1] < cto.position[1],
            "root should have smaller Y: ceo.y={}, cto.y={}", ceo.position[1], cto.position[1]);
        // CTO and COO are siblings — same depth → same Y
        assert!((cto.position[1] - coo.position[1]).abs() < 5.0,
            "siblings should share Y: cto.y={}, coo.y={}", cto.position[1], coo.position[1]);
    }

    #[test]
    fn test_kanban_layout_columns_side_by_side() {
        use crate::specgraph::hrf::parse_hrf;
        let spec = "## Kanban: Todo\n- [ta] Task A\n- [tb] Task B\n\n## Kanban: Done\n- [tc] Task C\n";
        let mut doc = parse_hrf(spec).unwrap();
        crate::specgraph::layout::auto_layout(&mut doc);
        let a = doc.nodes.iter().find(|n| n.hrf_id == "ta").unwrap();
        let b = doc.nodes.iter().find(|n| n.hrf_id == "tb").unwrap();
        let c = doc.nodes.iter().find(|n| n.hrf_id == "tc").unwrap();
        // Todo column should be left of Done column
        assert!(a.position[0] < c.position[0],
            "Todo column should be left of Done: a.x={}, c.x={}", a.position[0], c.position[0]);
        // Task A and Task B in same column should stack vertically
        assert!((a.position[1] - b.position[1]).abs() > 40.0,
            "cards in same column should stack: a.y={}, b.y={}", a.position[1], b.position[1]);
    }

    #[test]
    fn test_swimlane_layout_positions_nodes_in_rows() {
        use crate::specgraph::hrf::parse_hrf;
        let spec = "## Swimlane: Awareness\n- [a] Alpha\n\n## Swimlane: Revenue\n- [b] Beta\n";
        let mut doc = parse_hrf(spec).unwrap();
        auto_layout(&mut doc);
        // find nodes by label
        let alpha = doc.nodes.iter().find(|n| {
            if let crate::model::NodeKind::Shape { label, .. } = &n.kind {
                label == "Alpha"
            } else {
                false
            }
        }).unwrap();
        let beta = doc.nodes.iter().find(|n| {
            if let crate::model::NodeKind::Shape { label, .. } = &n.kind {
                label == "Beta"
            } else {
                false
            }
        }).unwrap();
        assert!(
            (alpha.position[1] - beta.position[1]).abs() > 50.0,
            "lanes should have different Y positions: alpha_y={}, beta_y={}",
            alpha.position[1],
            beta.position[1]
        );
    }
}
