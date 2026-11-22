use super::*;
use crate::paint::scrollbar::{contains, Bar};

impl<D: DataSource> Presenter<D> {
    /// Refresh Windows' automatic scrollbar visibility after returning to the app.
    #[cfg(windows)]
    pub fn refresh_scrollbar_preferences(&mut self) {
        if crate::com::ensure_mta().is_err() {
            self.set_scrollbars(true);
            return;
        }
        self.set_scrollbars(
            windows::UI::ViewManagement::UISettings::new()
                .and_then(|settings| settings.AutoHideScrollBars())
                .unwrap_or(true),
        );
    }
    /// Desktop scrollbars, with the person's automatic visibility preference.
    pub fn set_scrollbars(&mut self, auto_hide: bool) {
        self.brush.scrollbars.enabled = true;
        self.brush.scrollbars.auto_hide = auto_hide;
        self.dirty = true;
    }

    fn scrollbar_exposed(&self, bar: &Bar, x: f32, y: f32) -> bool {
        let Some(owner) = self.boxes.get(bar.owner) else {
            return false;
        };
        owner.id == bar.id
            && owner.contains(x, y)
            && self
                .painted_hit(std::slice::from_ref(owner), x, y, PaintHit::Published)
                .is_some()
            && self.boxes.get(bar.above..).is_some_and(|above| {
                self.painted_hit(above, x, y, PaintHit::ScrollbarCover)
                    .is_none()
            })
    }

    /// Refresh the look from the same published boxes used for presses.
    pub(crate) fn refresh_scrollbar_hover(&mut self) -> bool {
        if !self.brush.scrollbars.enabled {
            return false;
        }
        let mut hovered = Vec::new();
        if let Some((x, y)) = self
            .hover_point
            .filter(|&(x, y)| !self.shape_surface_open() && self.display.contains(x, y))
        {
            let mut over_track = false;
            for bar in self.brush.scrollbars.bars.iter().rev() {
                if contains(bar, bar.port, x, y) && self.scrollbar_exposed(bar, x, y) {
                    let track = !over_track && contains(bar, bar.track, x, y);
                    over_track |= track;
                    hovered.push((bar.key, bar.vertical, track));
                }
            }
        }
        if self.brush.scrollbars.hovered == hovered {
            return false;
        }
        self.brush.scrollbars.hovered = hovered;
        self.dirty = true;
        true
    }

    pub(crate) fn scrollbars_shown(&mut self) -> bool {
        let state = &mut self.brush.scrollbars;
        if state.drag.is_some_and(|(key, vertical, _)| {
            !state
                .bars
                .iter()
                .any(|b| (b.key, b.vertical) == (key, vertical))
        }) {
            state.drag = None;
        }
        self.refresh_scrollbar_hover()
    }

    pub(super) fn scrollbar_pointer(&mut self, phase: i32, x: f32, y: f32) -> bool {
        if self.shape_surface_open() || !self.brush.scrollbars.enabled {
            return false;
        }
        if let Some((key, vertical, grab)) = self.brush.scrollbars.drag {
            let bar = self
                .brush
                .scrollbars
                .bars
                .iter()
                .find(|b| (b.key, b.vertical) == (key, vertical))
                .copied();
            if phase == 1 {
                self.brush.scrollbars.drag = None;
                self.brush.scrollbars.pressed = false;
            }
            if let Some(bar) = bar.filter(|b| {
                self.host
                    .kernel()
                    .node_by_key(key)
                    .is_some_and(|n| n.id == b.id)
                    && self.alert_allows(b.id)
                    && !self.host.route_visibility(b.id).1
            }) {
                let mut to = self.scroll_of(bar.id);
                let value = bar.scroll_offset(bar.destination(bar.coordinate(x, y), grab));
                if vertical {
                    to.1 = value;
                } else {
                    to.0 = value;
                }
                self.scroll_to(bar.id, to);
            } else {
                self.brush.scrollbars.drag = None;
            }
            self.dirty = true;
            return true;
        }
        if phase == 1 {
            return std::mem::take(&mut self.brush.scrollbars.pressed);
        }
        if phase != 0 || !self.display.contains(x, y) {
            return false;
        }
        self.boxes();
        let bar = self
            .brush
            .scrollbars
            .bars
            .iter()
            .rev()
            .find(|b| contains(b, b.track, x, y) && self.scrollbar_exposed(b, x, y))
            .copied();
        let Some(bar) = bar else {
            return false;
        };
        self.brush.scrollbars.pressed = true;
        if contains(&bar, bar.thumb, x, y) {
            self.brush.scrollbars.drag = Some((
                bar.key,
                bar.vertical,
                bar.coordinate(x, y) - bar.start(bar.thumb),
            ));
        } else {
            let direction = if bar.coordinate(x, y) < bar.start(bar.thumb) {
                -1.
            } else {
                1.
            };
            let mut to = self.scroll_of(bar.id);
            let value = bar.scroll_offset(bar.offset + direction * bar.length(bar.port));
            if bar.vertical {
                to.1 = value;
            } else {
                to.0 = value;
            }
            self.scroll_to(bar.id, to);
        }
        self.dirty = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: &str = r##"      column position="absolute" left=150 top=25 width=132 height=145 z-index=10 background-color="#e1eddf"
        button "Panel action" press=click appearance="none" width="100%" height="100%" background-color="#e1eddf" border-width=0"##;

    fn covered(panel: &str) -> Presenter<()> {
        let source = format!(
            r##"component App
  state count = 0
  action click
    count = count + 1
  view
    column width=400 height=260 position="relative" background-color="#ffffff"
      textarea testId="editor" value="one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nthirteen\nfourteen\nfifteen\nsixteen\nseventeen\neighteen\nnineteen\ntwenty" position="absolute" left=20 top=15 width=200 height=180 box-sizing="border-box" font-size=16 line-height=1.4 resize="none" scrollbar-width=(count == 7 ? "none" : "auto")
{panel}
      text `Presses: ${{count}}` position="absolute" left=20 top=210
"##
        );
        let (mut p, error) = Presenter::boot_with_fixture_fonts(
            &crate::test_plan(&source),
            (),
            (400., 260.),
            1.,
            PathBuf::new(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(error.is_none(), "{error:?}");
        p.set_scrollbars(false);
        p
    }

    #[test]
    fn a_higher_panel_covers_the_textarea_scrollbar() {
        let mut p = covered(PANEL);
        p.pointer_move(215., 53., 0.).unwrap();
        let pixels = p.frame();
        let pixel = pixels.pixel(215, 53).unwrap();
        let rgb = [pixel.red(), pixel.green(), pixel.blue()];
        for (index, x) in [180., 208., 211., 215., 219., 220.].into_iter().enumerate() {
            p.pointer_down(x, 53., 10. * index as f64 + 1.).unwrap();
            p.pointer_up(x, 53., 10. * index as f64 + 2.).unwrap();
            p.frame();
        }
        assert_eq!(
            (rgb, p.host.runner().slot("count")),
            ([225, 237, 223], Some(&exact_runner::Value::Number(6.)))
        );
        let kernel = p.host.kernel();
        let editor = kernel
            .node_by_key(kernel.find_by_test_id("editor")[0])
            .unwrap()
            .id;
        p.pointer_down(215., 21., 61.).unwrap();
        p.pointer_move(215., 61., 62.).unwrap();
        p.pointer_up(215., 61., 63.).unwrap();
        let dragged = p.scroll_of(editor).1;
        assert!(dragged > 0., "the exposed thumb can be grabbed");
        assert_eq!(
            p.host.runner().slot("count"),
            Some(&exact_runner::Value::Number(6.))
        );

        p.pointer_down(220., 53., 64.).unwrap();
        p.pointer_up(220., 53., 65.).unwrap();
        p.frame();
        assert!(
            p.brush.scrollbars.bars.is_empty(),
            "scrollbar-width is none"
        );
        p.wheel_at(145., 53., 0., 48.);
        assert_eq!(p.scroll_of(editor).1, dragged + 48.);

        p.pointer_down(220., 53., 66.).unwrap();
        p.pointer_up(220., 53., 67.).unwrap();
        p.frame();
        assert_eq!(p.brush.scrollbars.bars.len(), 1);
        assert_eq!(p.scroll_of(editor).1, dragged + 48.);
    }

    #[test]
    fn a_covered_scrollbar_stays_hidden_until_the_pointer_leaves_the_panel() {
        let mut p = covered(PANEL);
        p.set_scrollbars(true);
        let points = [(221., 53.), (215., 53.), (215., 21.)];
        let reds = points.map(|(x, y)| {
            p.pointer_move(x, y, 0.).unwrap();
            p.frame().pixel(214, 21).unwrap().red()
        });
        assert_eq!(
            reds[0], reds[1],
            "hover over the panel leaves the thumb hidden"
        );
        assert!(
            reds[2] < reds[0],
            "the exposed track still reveals the thumb"
        );
    }

    #[test]
    fn a_rounded_button_takes_presses_in_its_hit_slop_over_a_scrollbar() {
        let mut p = covered(
            r##"      column position="absolute" left=150 top=25 width=132 height=145 z-index=10 background-color="#e1eddf" pointer-events="none"
        button "Panel action" testId="panel" press=click appearance="none" position="absolute" left=70 top=0 width=62 height=145 background-color="#e1eddf" border-width=0 border-radius=4 pointer-events="auto" -exact-hit-slop="12px""##,
        );
        p.frame();
        let kernel = p.host.kernel();
        let button = kernel
            .node_by_key(kernel.find_by_test_id("panel")[0])
            .unwrap()
            .id;
        assert_eq!(p.hit(215., 53.), Some(button));
        p.pointer_down(215., 53., 0.).unwrap();
        p.pointer_up(215., 53., 1.).unwrap();
        assert_eq!(
            p.host.runner().slot("count"),
            Some(&exact_runner::Value::Number(1.))
        );
    }

    #[test]
    fn a_native_button_face_covers_the_textarea_scrollbar() {
        let mut p = covered(
            r##"      button "Panel action" testId="panel" press=click appearance="auto" buttonStyle="filled" position="absolute" left=190 top=50 width=120 height=40 z-index=10 border-radius="50%""##,
        );
        let pixels = p.frame();
        let face = pixels.pixel(215, 53).unwrap();
        assert_eq!(Some(face), pixels.pixel(250, 53));
        assert!(face.blue() > face.red());
        let kernel = p.host.kernel();
        let editor = kernel
            .node_by_key(kernel.find_by_test_id("editor")[0])
            .unwrap()
            .id;
        let button = kernel
            .node_by_key(kernel.find_by_test_id("panel")[0])
            .unwrap()
            .id;
        assert_eq!(p.hit(215., 53.), Some(button));

        p.pointer_down(215., 53., 0.).unwrap();
        p.pointer_up(215., 53., 1.).unwrap();
        let presses = p.host.runner().slot("count").cloned();
        p.pointer_down(215., 53., 2.).unwrap();
        p.pointer_move(215., 70., 3.).unwrap();
        p.pointer_up(215., 70., 4.).unwrap();
        assert_eq!(
            (presses, p.scroll_of(editor).1),
            (Some(exact_runner::Value::Number(1.)), 0.)
        );
    }

    #[test]
    fn a_closed_select_face_covers_the_textarea_scrollbar() {
        const NEUTRAL: &str = "EXACT_SELECT_SCROLLBAR_NEUTRAL_TEST";
        let neutral = std::env::var_os(NEUTRAL).is_some();
        if neutral {
            crate::app::show_neutral_look();
        }
        let mut p = covered(
            r##"      select pointerdown=click position="absolute" left=210 top=50 width=120 height=40 z-index=10 border-radius="50%"
        option "One" value="one""##,
        );
        let pixels = p.frame();
        let face = pixels.pixel(215, 53).unwrap();
        assert_eq!(Some(face), pixels.pixel(250, 53));
        assert_ne!(Some(face), pixels.pixel(215, 21));
        let kernel = p.host.kernel();
        let editor = kernel
            .node_by_key(kernel.find_by_test_id("editor")[0])
            .unwrap()
            .id;

        p.pointer_down(215., 53., 0.).unwrap();
        p.pointer_move(215., 73., 1.).unwrap();
        p.pointer_up(215., 73., 2.).unwrap();
        assert_eq!(
            (p.host.runner().slot("count"), p.scroll_of(editor).1),
            (Some(&exact_runner::Value::Number(1.)), 0.)
        );
        if !neutral {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", std::thread::current().name().unwrap()])
                .env(NEUTRAL, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    fn drag_through_panel_corner(panel: &str) {
        let mut p = covered(panel);
        let pixels = p.frame();
        assert_eq!(pixels.pixel(215, 53), pixels.pixel(215, 21));
        let kernel = p.host.kernel();
        let editor = kernel
            .node_by_key(kernel.find_by_test_id("editor")[0])
            .unwrap()
            .id;

        p.pointer_down(216., 76., 0.).unwrap();
        p.pointer_up(216., 76., 1.).unwrap();
        assert_eq!(
            p.host.runner().slot("count"),
            Some(&exact_runner::Value::Number(1.)),
            "the part of the button that covers the track takes the click"
        );

        p.pointer_down(215., 53., 2.).unwrap();
        p.pointer_move(215., 84., 3.).unwrap();
        p.pointer_up(215., 84., 4.).unwrap();
        assert!(
            p.scroll_of(editor).1 > 0.,
            "the exposed thumb can be grabbed"
        );
    }

    #[test]
    fn a_scrollbar_can_be_grabbed_outside_a_rounded_border() {
        drag_through_panel_corner(
            r##"      button "Panel action" press=click appearance="none" position="absolute" left=210 top=50 width=80 height=80 z-index=10 border-radius=40 background-color="#e1eddf" border-width=0"##,
        );
    }

    #[test]
    fn a_scrollbar_can_be_grabbed_outside_a_projected_rounded_border() {
        drag_through_panel_corner(
            r##"      button "Panel action" press=click appearance="none" position="absolute" left=210 top=50 width=80 height=80 z-index=10 rotate="y 5deg" border-radius=40 background-color="#e1eddf" border-width=0"##,
        );
    }

    #[test]
    fn a_scrollbar_can_be_grabbed_outside_a_rotated_overflow_clip() {
        drag_through_panel_corner(
            r##"      column position="absolute" left=210 top=50 width=80 height=80 z-index=10 rotate="45deg" overflow="hidden"
        button "Panel action" press=click appearance="none" position="absolute" left=-60 top=-60 width=200 height=200 background-color="#e1eddf" border-width=0"##,
        );
    }

    #[test]
    fn a_scrollbar_drag_lasts_until_the_display_acknowledges_its_removal() {
        let source = r#"component App
  state shown = true
  action hide
    shown = false
  view
    column
      column testId="port" width=200 height=180 overflow-y="auto" scrollbar-width=(shown ? "auto" : "none")
        box height=620 flex-shrink=0
      button "Hide" testId="hide" press=hide
"#;
        let (mut p, error) = Presenter::boot_with_fixture_fonts(
            &crate::test_plan(source),
            (),
            (260., 260.),
            1.,
            PathBuf::new(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(error.is_none(), "{error:?}");
        p.set_scrollbars(false);
        let shown = p.display_frame().unwrap();
        assert!(p.display_complete(&shown));
        let bar = p.brush.scrollbars.bars[0];
        p.pointer_down(194., 10., 0.).unwrap();
        assert!(p.brush.scrollbars.drag.is_some());
        let kernel = p.host.kernel();
        let hide = kernel
            .node_by_key(kernel.find_by_test_id("hide")[0])
            .unwrap()
            .id;
        assert_eq!(p.host.dispatch_at(hide, Event::Press, 1.), None);
        assert_eq!(p.after_commit(), None);
        let hidden = p.display_frame().unwrap();
        p.pointer_move(194., 50., 2.).unwrap();
        assert!(
            p.scroll_of(bar.id).1 > 0.,
            "the displayed thumb can still drag"
        );
        assert!(p.brush.scrollbars.drag.is_some());
        assert!(p.display_complete(&hidden));
        assert!(p.brush.scrollbars.drag.is_none());
        let offset = p.scroll_of(bar.id);
        p.pointer_move(194., 90., 3.).unwrap();
        p.pointer_up(194., 90., 4.).unwrap();
        assert_eq!(p.scroll_of(bar.id), offset);
    }

    #[test]
    #[cfg(not(target_os = "android"))]
    fn a_scrollbar_in_a_retained_row_uses_its_scroller_clip() {
        let source = r##"component App
  view
    column width=320 height=360 overflow-y="auto"
      column testId="inner" width=200 height=240 overflow-y="auto" flex-shrink=0 background-color="#ffffff"
        box height=1020 flex-shrink=0
      box height=200 flex-shrink=0
"##;
        let plan = crate::test_plan(source);
        let offsets = [PainterChoice::Cpu, PainterChoice::Ink].map(|choice| {
            let (mut p, error) = Presenter::boot_with_fixture_fonts(
                &plan,
                (),
                (320., 180.),
                1.,
                PathBuf::new(),
                choice,
            )
            .unwrap();
            assert!(error.is_none(), "{error:?}");
            p.set_scrollbars(false);
            let pixels = p.frame();
            let drawn = pixels.pixel(194, 10).unwrap().red() < 255;
            let kernel = p.host.kernel();
            let inner = kernel
                .node_by_key(kernel.find_by_test_id("inner")[0])
                .unwrap()
                .id;
            p.pointer_down(194., 10., 0.).unwrap();
            p.pointer_move(194., 50., 1.).unwrap();
            p.pointer_up(194., 50., 2.).unwrap();
            (drawn, p.scroll_of(inner).1)
        });
        assert!(offsets[0].0 && offsets[0].1 > 0., "{offsets:?}");
        assert_eq!(offsets[1], offsets[0]);
    }

    fn input(direction: &str) -> (Presenter<()>, ViewId) {
        let value = if direction == "rtl" {
            "אבגדהוזחטי "
        } else {
            "abcdefghij "
        }
        .repeat(12);
        let source = format!(
            "component App\n  view\n    input testId=\"editor\" direction=\"{direction}\" value=\"{value}\" width=200 height=64 padding=4 appearance=\"none\" overflow-x=\"auto\" font-size=16\n"
        );
        let (mut p, error) = Presenter::boot_with_fixture_fonts(
            &crate::test_plan(&source),
            (),
            (300., 200.),
            1.,
            PathBuf::new(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(error.is_none(), "{error:?}");
        let k = p.host.kernel();
        let id = k.node_by_key(k.find_by_test_id("editor")[0]).unwrap().id;
        p.set_scrollbars(false);
        p.set_focus(Some(id), 0.);
        p.mark_field(id, &value, exact_runner::FieldSelection::caret(66), false);
        p.frame();
        let rect = p.rect_of(id).unwrap();
        p.press_field(id, rect.0 + 104., rect.1 + 32.);
        p.frame();
        (p, id)
    }

    #[test]
    fn single_line_scrollbar_thumb_marks_the_physical_edges() {
        for direction in ["ltr", "rtl"] {
            let (mut p, id) = input(direction);
            let max = p.brush.field_text[&id].scroll_max.0;
            for offset in [0., max] {
                p.scroll_to(id, (offset, 0.));
                p.frame();
                let bar = p.brush.scrollbars.bars[0];
                assert!(!bar.vertical);
                let at_right = (offset == max) != (direction == "rtl");
                let expected = bar.track.0
                    + if at_right {
                        bar.track.2 - bar.thumb.2
                    } else {
                        0.
                    };
                assert!(
                    (bar.thumb.0 - expected).abs() < 0.01,
                    "{direction}, offset={offset}"
                );
            }
        }
    }

    #[test]
    fn single_line_scrollbar_drag_and_track_click_move_text_in_physical_direction() {
        for (direction, sign) in [("ltr", 1.), ("rtl", -1.)] {
            let (mut p, id) = input(direction);
            let selection = p.field_selection(id);
            for dx in [40., -40.] {
                let bar = p.brush.scrollbars.bars[0];
                let before = p.caret_frame().unwrap();
                let offset = p.scroll_of(id).0;
                let x = bar.thumb.0 + bar.thumb.2 / 2.;
                let y = bar.thumb.1 + bar.thumb.3 / 2.;
                let to = x + dx / bar.max * (bar.track.2 - bar.thumb.2);
                p.pointer_down(x, y, 1.).unwrap();
                p.pointer_move(to, y, 2.).unwrap();
                p.pointer_up(to, y, 3.).unwrap();
                p.frame();
                assert!(
                    (p.scroll_of(id).0 - offset - sign * dx).abs() < 0.05,
                    "{direction}"
                );
                let after = p.caret_frame().unwrap();
                assert!((after.0 - before.0 + dx).abs() < 0.05, "{direction}");
            }
            for right in [true, false] {
                let bar = p.brush.scrollbars.bars[0];
                let offset = p.scroll_of(id).0;
                let x = if right {
                    bar.track.0 + bar.track.2 - 1.
                } else {
                    bar.track.0 + 1.
                };
                let y = bar.track.1 + bar.track.3 / 2.;
                p.pointer_down(x, y, 4.).unwrap();
                p.pointer_up(x, y, 5.).unwrap();
                p.frame();
                let delta = if right { bar.port.2 } else { -bar.port.2 };
                assert!(
                    (p.scroll_of(id).0 - offset - sign * delta).abs() < 0.05,
                    "{direction}"
                );
            }
            assert_eq!(p.field_selection(id), selection);
        }
    }

    #[test]
    fn a_wheel_ends_a_scrollbar_drag_without_reverting_its_scroll() {
        for vertical in [false, true] {
            for redraw in [false, true] {
                let source = r#"component App
  state presses = 0
  action press
    presses = presses + 1
  view
    column width=240 height=200 overflow="auto"
      button press=press width=1000 height=1000 flex-shrink=0
        text "A large row"
"#;
                let (mut p, error) = Presenter::boot_with_fixture_fonts(
                    &crate::test_plan(source),
                    (),
                    (240., 200.),
                    1.,
                    PathBuf::new(),
                    PainterChoice::Cpu,
                )
                .unwrap();
                assert!(error.is_none(), "{error:?}");
                p.set_scrollbars(false);
                p.frame();
                let bar = *p
                    .brush
                    .scrollbars
                    .bars
                    .iter()
                    .find(|b| b.vertical == vertical)
                    .unwrap();
                let (x, y) = (
                    bar.thumb.0 + bar.thumb.2 / 2.,
                    bar.thumb.1 + bar.thumb.3 / 2.,
                );
                p.pointer_down(x, y, 0.).unwrap();
                assert!(p.brush.scrollbars.drag.is_some());
                let offset = if vertical { (0., 40.) } else { (40., 0.) };
                p.wheel_at(x, y, offset.0, offset.1);
                assert_eq!(p.scroll_of(bar.id), offset);
                let dragging = p.brush.scrollbars.drag.is_some();
                let pressed = p.brush.scrollbars.pressed;
                if redraw {
                    p.frame();
                }
                p.pointer_up(x, y, 1.).unwrap();
                assert_eq!(
                    p.scroll_of(bar.id),
                    offset,
                    "vertical={vertical}, redraw={redraw}"
                );
                assert!(!dragging && !pressed);
                assert_eq!(
                    p.host.runner().slot("presses"),
                    Some(&exact_runner::Value::Number(0.))
                );
            }
        }
    }

    #[test]
    fn a_horizontal_track_leaves_the_corner_only_for_a_vertical_bar() {
        for (overflow_y, corner) in [("hidden", 0.), ("clip", 0.), ("auto", 12.)] {
            let source = format!(
                "component App\n  view\n    column width=240 height=200 overflow-x=\"auto\" overflow-y=\"{overflow_y}\"\n      column width=1000 height=1000 flex-shrink=0\n"
            );
            let (mut p, error) = Presenter::boot_with_fixture_fonts(
                &crate::test_plan(&source),
                (),
                (240., 200.),
                1.,
                PathBuf::new(),
                PainterChoice::Cpu,
            )
            .unwrap();
            assert!(error.is_none(), "{error:?}");
            p.set_scrollbars(false);
            p.frame();
            let bars = &p.brush.scrollbars.bars;
            let bar = bars.iter().find(|b| !b.vertical).unwrap();
            assert_eq!(bars.len(), if corner > 0. { 2 } else { 1 }, "{overflow_y}");
            assert_eq!(bar.track.2, bar.port.2 - 4. - corner, "{overflow_y}");
        }
    }

    #[test]
    fn desktop_scrollbars_drag_page_and_cancel_without_pressing_content() {
        let source = r#"component App
  state presses = 0
  action press
    presses = presses + 1
  view
    column width=240 height=200 overflow-y="auto" testId="list"
      button press=press width=240 height=1000 flex-shrink=0
        text "A long row"
"#;
        let (mut p, error) = Presenter::boot_with_fixture_fonts(
            &crate::test_plan(source),
            (),
            (240., 200.),
            1.,
            PathBuf::new(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(error.is_none());
        p.set_scrollbars(false);
        p.frame();
        let bar = p.brush.scrollbars.bars[0];
        assert!(bar.vertical);
        assert!(bar.thumb.3 < bar.track.3);
        let (x, y) = (
            bar.thumb.0 + bar.thumb.2 / 2.,
            bar.thumb.1 + bar.thumb.3 / 2.,
        );
        p.pointer_down(x, y, 0.).unwrap();
        p.pointer_move(x, bar.track.1 + bar.track.3, 1.).unwrap();
        p.frame();
        p.pointer_up(x, bar.track.1 + bar.track.3, 2.).unwrap();
        assert_eq!(p.scroll_of(bar.id).1, bar.max);
        assert_eq!(
            p.host.runner().slot("presses"),
            Some(&exact_runner::Value::Number(0.))
        );
        p.frame();
        let bar = p.brush.scrollbars.bars[0];
        p.pointer_down(x, bar.track.1 + 1., 3.).unwrap();
        p.pointer_up(x, bar.track.1 + 1., 4.).unwrap();
        assert_eq!(p.scroll_of(bar.id).1, (bar.max - bar.port.3).max(0.));
        assert_eq!(
            p.host.runner().slot("presses"),
            Some(&exact_runner::Value::Number(0.))
        );
        p.frame();
        let bar = p.brush.scrollbars.bars[0];
        p.pointer_down(x, bar.thumb.1 + 2., 5.).unwrap();
        p.pointer_cancel(6.).unwrap();
        assert!(p.brush.scrollbars.drag.is_none());
    }
}
