//! Release decoration paint and count retired rows by list site (@ref LLP 1142 §13.4).
use super::*;

pub(in crate::instance) fn tracks(u: &Update<'_>) -> bool {
    !u.ids.work.sites.is_empty() || !u.ids.theme.decorations.is_empty()
}

pub(in crate::instance) fn retire_node(u: &mut Update<'_>, node: &NodeInst) {
    if !tracks(u) {
        return;
    }
    u.ids.theme.decorations.remove(&node.view);
    if let Some(list) = node.list() {
        list.retire(u);
    }
    retire_children(u, &node.children);
}

pub(in crate::instance) fn retire_children(u: &mut Update<'_>, children: &[Child]) {
    for child in children {
        match child {
            Child::Node(n) => retire_node(u, n),
            Child::Region(r) => match &r.active {
                Active::Arm { roots, .. } => retire_children(u, roots),
                Active::Rows { rows } => {
                    for row in rows {
                        retire_children(u, &row.roots);
                    }
                }
            },
        }
    }
}
