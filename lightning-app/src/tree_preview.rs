use pathfinding::prelude::bfs;
use lightning_model::data::TREE;
use lightning_model::tree::PassiveTree;

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
