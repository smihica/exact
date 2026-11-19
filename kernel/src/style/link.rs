use super::*;

/// Link every grammar a host may link by use (LLP 1047 D2): what an artifact
/// that links everything, the compiler and [`crate::Kernel::new`] do.
pub fn link_all() {
    link_backdrop_filter();
    link_segments();
    link_wide_colors();
    link_effects();
    link_grid();
    link_animations();
    link_transitions();
    link_gradients();
    link_corners();
    link_shadows();
    link_rotate();
    link_text_stroke();
    link_symbol_palette();
    crate::timeline::link();
    crate::layout::link_grid_layout();
}
