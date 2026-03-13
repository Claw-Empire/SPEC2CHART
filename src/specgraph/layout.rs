use std::collections::{HashMap, VecDeque};
use crate::model::{FlowchartDocument, NodeId};

/// Hierarchical (layered) auto-layout.
///
/// Nodes are assigned to horizontal layers based on their longest path from
/// any root (node with no incoming edges). Within each layer they are spread
/// evenly and the whole layer is centred over the widest layer.
/// Only nodes still at the origin [0, 0] are repositioned.
pub fn hierarchical_layout(doc: &mut FlowchartDocument) {
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

    // Layout constants
    const GAP_X: f32 = 60.0;  // horizontal gap between nodes in a layer
    const GAP_Y: f32 = 80.0;  // vertical gap between layers
    const START_X: f32 = 100.0;
    const START_Y: f32 = 100.0;

    // Compute the total width of each layer (sum of node widths + gaps)
    let layer_widths: Vec<f32> = layers
        .iter()
        .map(|nodes| {
            let w: f32 = nodes.iter().map(|&i| doc.nodes[i].size[0]).sum();
            let g = GAP_X * (nodes.len().saturating_sub(1) as f32);
            w + g
        })
        .collect();

    let canvas_width = layer_widths.iter().cloned().fold(0.0_f32, f32::max);
    let centre_x = START_X + canvas_width / 2.0;

    let mut y = START_Y;
    for (layer_idx, layer_nodes) in layers.iter().enumerate() {
        if layer_nodes.is_empty() {
            continue;
        }

        let lw = layer_widths[layer_idx];
        let mut x = centre_x - lw / 2.0;

        let max_h = layer_nodes
            .iter()
            .map(|&i| doc.nodes[i].size[1])
            .fold(0.0_f32, f32::max);

        for &i in layer_nodes {
            // Skip layout for pinned nodes or nodes with explicit non-origin positions
            let pos = doc.nodes[i].position;
            let has_explicit_pos = pos != [0.0, 0.0] && doc.nodes[i].pinned;
            if !has_explicit_pos {
                // Vertically centre the node within the tallest node in the layer
                let node_h = doc.nodes[i].size[1];
                let y_offset = (max_h - node_h) / 2.0;
                doc.nodes[i].position = [x, y + y_offset];
            }
            x += doc.nodes[i].size[0] + GAP_X;
        }

        y += max_h + GAP_Y;
    }
}
