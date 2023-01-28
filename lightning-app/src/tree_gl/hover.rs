use quadtree_f32::{Item, ItemId, QuadTree};
use lazy_static::lazy_static;
use lightning_model::data::TREE;
use lightning_model::tree::Node;
use super::draw_data::{node_pos, get_rect};
use pathfinding::prelude::bfs;
use lightning_model::tree::PassiveTree;

lazy_static! {
    /// This quadtree is used to know when tree nodes are hovered by the mouse cursor
    static ref QUADTREE: QuadTree = {
        let items = TREE.nodes
            .iter()
            .filter(|(_k,n)| n.group.is_some() && n.class_start_index.is_none() && !n.is_ascendancy_start)
            .map(|(k,n)| {
                let (x,y) = node_pos(n);
                let (rect, _) = get_rect(n).unwrap();
                let scale = 2.0;
                (
                    ItemId(*k as usize),
                    Item::Rect(quadtree_f32::Rect {
                        max_x: x + (rect.w as f32 * scale) / 2.0 + (5.0 * scale),
                        max_y: y + (rect.h as f32 * scale) / 2.0 + (5.0 * scale),
                        min_x: x - (rect.w as f32 * scale) / 2.0 - (5.0 * scale),
                        min_y: y - (rect.h as f32 * scale) / 2.0 - (5.0 * scale),
                    })
                )
            });

        QuadTree::new(items)
    };
}

pub fn get_hovered_node(x: f32, y: f32) -> Option<&'static Node> {
    let overlaps = QUADTREE.get_ids_that_overlap(
        &quadtree_f32::Rect {
            max_x: x + 1.0,
            max_y: y + 1.0,
            min_x: x - 1.0,
            min_y: y - 1.0,
        }
    );

    if overlaps.is_empty() { return None; }
    Some(&TREE.nodes[&(overlaps[0].0 as u16)])
}

fn successors(node: u16) -> Vec<u16> {
    let mut v = TREE.nodes[&node].out.as_ref().unwrap().clone();
    v.extend(TREE.nodes[&node].r#in.as_ref().unwrap().clone());
    v
}

/// When an unallocated node is hovered, find the shortest path to link
/// the rest of the tree. Using Breadth-First-Search.
pub fn find_path(node: u16, tree: &PassiveTree) -> Option<Vec<u16>> {
    bfs(&node, |p| successors(*p), |p| tree.nodes.contains(p))
}
