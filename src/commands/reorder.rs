use crate::config::{Align, BaseStruts, GapMode, SidebarPosition};
use crate::niri::NiriClient;
use crate::state::save_state;
use crate::window_rules::{resolve_rule_focus_peek, resolve_rule_peek, resolve_window_size};
use crate::{Ctx, WindowTarget};
use anyhow::Result;
use niri_ipc::{Action, PositionChange, Window};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

const STRUT_FILE: &str = "/tmp/niri-sidebar-struts.kdl";

fn write_strut_file(
    pos: SidebarPosition,
    strut_value: i32,
    base: Option<&BaseStruts>,
) -> Result<bool> {
    let field = match pos {
        SidebarPosition::Right => "right",
        SidebarPosition::Left => "left",
        SidebarPosition::Top => "top",
        SidebarPosition::Bottom => "bottom",
    };
    let mut fields: Vec<String> = vec![format!("        {} {}", field, strut_value)];
    if let Some(b) = base {
        if field != "top" && b.top != 0.0 {
            fields.push(format!("        top {}", b.top));
        }
        if field != "right" && b.right != 0.0 {
            fields.push(format!("        right {}", b.right));
        }
        if field != "bottom" && b.bottom != 0.0 {
            fields.push(format!("        bottom {}", b.bottom));
        }
        if field != "left" && b.left != 0.0 {
            fields.push(format!("        left {}", b.left));
        }
    }
    let content = format!(
        "layout {{\n    struts {{\n{}\n    }}\n}}\n",
        fields.join("\n")
    );
    let existing = fs::read_to_string(STRUT_FILE).unwrap_or_default();
    if existing == content {
        return Ok(false);
    }
    let mut f = fs::File::create(STRUT_FILE)?;
    f.write_all(content.as_bytes())?;
    Ok(true)
}

fn clear_strut_file(base: Option<&BaseStruts>) -> Result<bool> {
    if let Some(b) = base {
        let mut fields: Vec<String> = vec![];
        if b.top != 0.0 {
            fields.push(format!("        top {}", b.top));
        }
        if b.right != 0.0 {
            fields.push(format!("        right {}", b.right));
        }
        if b.bottom != 0.0 {
            fields.push(format!("        bottom {}", b.bottom));
        }
        if b.left != 0.0 {
            fields.push(format!("        left {}", b.left));
        }
        let content = if fields.is_empty() {
            "layout { }\n".to_string()
        } else {
            format!(
                "layout {{\n    struts {{\n{}\n    }}\n}}\n",
                fields.join("\n")
            )
        };
        let existing = fs::read_to_string(STRUT_FILE).unwrap_or_default();
        if existing == content {
            return Ok(false);
        }
        fs::write(STRUT_FILE, content)?;
        Ok(true)
    } else {
        let content = "layout { }\n";
        let existing = fs::read_to_string(STRUT_FILE).unwrap_or_default();
        if existing == content {
            return Ok(false);
        }
        fs::write(STRUT_FILE, content)?;
        Ok(true)
    }
}

pub fn update_auto_fit<C: NiriClient>(ctx: &mut Ctx<C>) -> Result<()> {
    if let Some(auto_fit) = &ctx.config.interaction.auto_fit {
        let current_ws = ctx.socket.get_active_workspace()?.id;
        let windows = ctx.socket.get_windows()?;
        let sidebar_ids: Vec<u64> = ctx.state.windows.iter().map(|w| w.id).collect();
        let has_windows_on_ws = windows.iter().any(|w| {
            w.is_floating && w.workspace_id == Some(current_ws) && sidebar_ids.contains(&w.id)
        });
        let strut_active = has_windows_on_ws && ctx.state.is_hidden;
        let strut_value = if strut_active {
            auto_fit.with_sidebar
        } else {
            auto_fit.without_sidebar
        };
        let base = auto_fit.base.as_ref();
        let wrote = if strut_value > 0 {
            write_strut_file(ctx.config.interaction.position, strut_value, base)?
        } else {
            clear_strut_file(base)?
        };
        if wrote {
            let _ = ctx
                .socket
                .send_action(Action::LoadConfigFile { path: None });
        }
    }
    Ok(())
}

fn resolve_dimensions<C: NiriClient>(window: &Window, ctx: &Ctx<C>) -> WindowTarget {
    let (width, height) = resolve_window_size(
        &ctx.config.window_rule,
        window,
        ctx.config.geometry.width,
        ctx.config.geometry.height,
    );

    WindowTarget { width, height }
}

fn calculate_coordinates<C: NiriClient>(
    pos: SidebarPosition,
    align: Align,
    dims: WindowTarget,
    actual_size: (i32, i32),
    screen: (i32, i32),
    stack_offset: i32,
    active_peek: i32,
    ctx: &Ctx<C>,
) -> (i32, i32) {
    let state = &ctx.state;
    let margins = &ctx.config.margins;
    let (sw, sh) = screen;
    let (w, h) = (dims.width, dims.height);
    let (aw, _ah) = actual_size;

    match pos {
        SidebarPosition::Right => {
            let visible_x = if align.is_right_aligned() {
                sw - margins.right - aw
            } else {
                sw - margins.right - w
            };
            let hidden_x = sw - active_peek;
            let x = if state.is_hidden { hidden_x } else { visible_x };

            let y = if align.stacks_downward() {
                margins.top + stack_offset
            } else {
                sh - h - margins.bottom - stack_offset
            };
            (x, y)
        }
        SidebarPosition::Left => {
            let visible_x = if align.is_right_aligned() {
                margins.left + w - aw
            } else {
                margins.left
            };
            let hidden_x = if align.is_right_aligned() {
                margins.left - aw + active_peek
            } else {
                -w + active_peek
            };
            let x = if state.is_hidden { hidden_x } else { visible_x };

            let y = if align.stacks_downward() {
                margins.top + stack_offset
            } else {
                sh - h - margins.bottom - stack_offset
            };
            (x, y)
        }
        SidebarPosition::Bottom => {
            let x = if align.is_right_aligned() {
                sw - margins.right - aw - stack_offset
            } else {
                margins.left + stack_offset
            };

            let visible_y = if state.is_hidden {
                sh - active_peek
            } else {
                sh - h - margins.bottom
            };
            let hidden_y = sh - active_peek;
            let y = if state.is_hidden { hidden_y } else { visible_y };
            (x, y)
        }
        SidebarPosition::Top => {
            let x = if align.is_right_aligned() {
                sw - margins.right - aw - stack_offset
            } else {
                margins.left + stack_offset
            };

            let visible_y = margins.top;
            let hidden_y = -h + active_peek;
            let y = if state.is_hidden { hidden_y } else { visible_y };
            (x, y)
        }
    }
}

pub fn reorder<C: NiriClient>(ctx: &mut Ctx<C>) -> Result<()> {
    let display_w;
    let display_h;
    
    

    let sidebar_ids: Vec<u64> = ctx.state.windows.iter().map(|w| w.id).collect();

    update_auto_fit(ctx)?;

    // Fetch fresh data (after possible config reload)
    (display_w, display_h) = ctx.socket.get_screen_dimensions()?;
    let current_ws = ctx.socket.get_active_workspace()?.id;
    let all_windows = ctx.socket.get_windows()?;

    let mut sidebar_windows: Vec<_> = all_windows
        .iter()
        .filter(|w| {
            w.is_floating && w.workspace_id == Some(current_ws) && sidebar_ids.contains(&w.id)
        })
        .collect();

    let initial_len = ctx.state.windows.len();
    let active_ids: HashSet<u64> = all_windows.iter().map(|w| w.id).collect();

    ctx.state.windows.retain(|w| active_ids.contains(&w.id));
    if ctx.state.windows.len() != initial_len {
        save_state(&ctx.state, &ctx.cache_dir)?;
    }

    sidebar_windows.sort_by_key(|w| {
        sidebar_ids
            .iter()
            .position(|id| *id == w.id)
            .unwrap_or(usize::MAX)
    });
    if ctx.state.is_flipped {
        sidebar_windows.reverse();
    }

    let position = ctx.config.interaction.position;
    let align = ctx.config.interaction.align;
    let gap = ctx.config.geometry.gap;
    let gap_mode = ctx.config.geometry.gap_mode;
    let mut current_stack_offset = 0;

    for window in sidebar_windows.iter() {
        let dims = resolve_dimensions(window, ctx);
        let (aw, ah) = window.layout.window_size;
        let active_peek = if window.is_focused {
            resolve_rule_focus_peek(
                &ctx.config.window_rule,
                window,
                ctx.config.interaction.get_focus_peek(),
            )
        } else {
            resolve_rule_peek(&ctx.config.window_rule, window, ctx.config.interaction.peek)
        };
        let (target_x, target_y) = calculate_coordinates(
            position,
            align,
            dims,
            (aw, ah),
            (display_w, display_h),
            current_stack_offset,
            active_peek,
            ctx,
        );
        match position {
            SidebarPosition::Left | SidebarPosition::Right => {
                let step = match gap_mode {
                    GapMode::Static => gap,
                    GapMode::Dynamic => ah + gap,
                };
                current_stack_offset += step;
            }
            SidebarPosition::Top | SidebarPosition::Bottom => {
                let step = match gap_mode {
                    GapMode::Static => gap,
                    GapMode::Dynamic => aw + gap,
                };
                current_stack_offset += step;
            }
        }
        let _ = ctx.socket.send_action(Action::MoveFloatingWindow {
            id: Some(window.id),
            x: PositionChange::SetFixed(target_x.into()),
            y: PositionChange::SetFixed(target_y.into()),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WindowRule;
    use crate::state::{AppState, WindowState};
    use crate::test_utils::{MockNiri, mock_config, mock_window, mock_window_with_size};
    use niri_ipc::{Action, PositionChange};
    use regex::Regex;
    use tempfile::tempdir;

    #[test]
    fn test_standard_stacking_order() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Two windows, visible. Check Y-axis stacking.
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let w2 = mock_window(2, true, true, 1, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w1, w2]);

        let mut state = AppState::default();
        // 1 is bottom, 2 is top
        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: true,
            position: Some((1.0, 2.0)),
        };
        state.windows.push(w1);
        state.windows.push(w2);

        let mut ctx = Ctx {
            state,
            config: mock_config(),
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;
        assert_eq!(actions.len(), 2);

        // Screen W: 1920, H: 1080
        // Config: W: 300, H: 200, Gap: 10, Top: 50, Right: 20
        let base_x = 1920 - 300 - 20; // 1600
        let base_y = 1080 - 200 - 50; // 830 (Bottom-most slot)

        // Window 1 (Index 0)
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                x: PositionChange::SetFixed(x),
                y: PositionChange::SetFixed(y)
            } if *x == f64::from(base_x) && *y == f64::from(base_y)
        )));

        // Window 2 (Index 1) -> Stacked above
        // Y = BaseY - (Height + Gap) = 830 - (200 + 10) = 620
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(2),
                x: PositionChange::SetFixed(x),
                y: PositionChange::SetFixed(y)
            } if *x == f64::from(base_x) && *y == 620.0
        )));
    }

    #[test]
    fn test_hidden_mode_with_focus_peek() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Hidden mode. Focused window should stick out more.
        let w_focused = mock_window(1, true, true, 1, Some((1.0, 2.0)));
        let w_bg = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w_focused, w_bg]);

        let mut state = AppState {
            is_hidden: true,
            ..Default::default()
        };
        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: true,
            position: Some((1.0, 2.0)),
        };
        state.windows.push(w1);
        state.windows.push(w2);

        let mut ctx = Ctx {
            state,
            config: mock_config(),
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;

        // Config: Peek: 10, FocusPeek: 50
        // 1. Unfocused Window (ID 2) -> Should be at 1920 - 10 = 1910
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(2), x: PositionChange::SetFixed(x), .. }
            if *x == 1910.0
        )));

        // 2. Focused Window (ID 1) -> Should be at 1920 - 50 = 1870
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(1), x: PositionChange::SetFixed(x), .. }
            if *x == 1870.0
        )));
    }

    #[test]
    fn test_filters_wrong_workspace_and_cleanup_zombies() {
        let temp_dir = tempdir().unwrap();
        // Scenario:
        // - Window 1: On workspace 1 (Correct)
        // - Window 2: On workspace 99 (Should be ignored)
        // - Window 3: In State, but does not exist in Niri

        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let w2 = mock_window(2, false, true, 99, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w1, w2]);

        let mut state = AppState::default();
        let w1 = WindowState {
            id: 1,
            width: 100,
            height: 100,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 100,
            height: 100,
            is_floating: true,
            position: Some((1.0, 2.0)),
        };
        let w3 = WindowState {
            id: 3,
            width: 100,
            height: 100,
            is_floating: true,
            position: Some((1.0, 2.0)),
        };
        state.windows.push(w1);
        state.windows.push(w2);
        state.windows.push(w3);

        let mut ctx = Ctx {
            state,
            config: mock_config(),
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).unwrap();

        // Check Logic:
        // 1. Window 3 should be removed from state
        // 2. Window 2 should NOT be moved
        // 3. Window 1 SHOULD be moved

        let ids: Vec<u64> = ctx.state.windows.iter().map(|w| w.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(
            !ids.contains(&3),
            "Zombie window 3 should be removed from state"
        );

        // Assert Actions
        let actions = &ctx.socket.sent_actions;

        // Should move ID 1
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::MoveFloatingWindow { id: Some(1), .. }))
        );
        // Should NOT move ID 2 (Wrong WS)
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::MoveFloatingWindow { id: Some(2), .. }))
        );
        // Should NOT move ID 3 (Doesn't exist)
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::MoveFloatingWindow { id: Some(3), .. }))
        );
    }

    #[test]
    fn test_flipped_order() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Flipped mode reverses the visual stack
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let w2 = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w1, w2]);

        let mut state = AppState {
            is_flipped: true,
            ..Default::default()
        };
        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: true,
            position: Some((1.0, 2.0)),
        };
        state.windows.push(w1);
        state.windows.push(w2);

        let mut ctx = Ctx {
            state,
            config: mock_config(),
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).unwrap();

        let actions = &ctx.socket.sent_actions;

        // Normal Order: 1 is bottom (idx 0), 2 is top (idx 1)
        // Flipped Order: 2 becomes bottom (idx 0), 1 becomes top (idx 1)
        // Check Window 2 is now at the Bottom (BaseY)
        // BaseY = 1080 - 200 - 50 = 830
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(2), y: PositionChange::SetFixed(y), .. }
            if *y == 830.0
        )));
        // Check Window 1 is now stacked above
        // Y = 830 - (200 + 10) = 620
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(1), y: PositionChange::SetFixed(y), .. }
            if *y == 620.0
        )));
    }

    #[test]
    fn test_position_left_hidden() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Left side, Hidden.
        // Window Width: 300. Peek: 10.
        // Expected X = -300 + 10 = -290.
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w1]);

        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Left;
        config.interaction.peek = 10;
        config.geometry.width = 300;
        config.margins.left = 0;

        let mut state = AppState {
            is_hidden: true,
            ..Default::default()
        };

        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1);

        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == -290.0 // Verify negative coordinate
        )));
    }

    #[test]
    fn test_position_bottom_stacking() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Bottom bar.
        // Windows should stack Left-to-Right (X axis changes, Y is fixed).
        let w1 = mock_window_with_size(1, false, true, 1, Some((1.0, 2.0)), (100, 200));
        let w2 = mock_window_with_size(2, false, true, 1, Some((1.0, 2.0)), (100, 200));
        let mock = MockNiri::new(vec![w1, w2]);

        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Bottom;
        config.interaction.align = crate::config::Align::BottomLeft;
        config.geometry.width = 100;
        config.geometry.gap = 10;
        config.margins.left = 20;

        let mut state = AppState::default();

        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1);
        state.windows.push(w2);

        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;

        // Window 1 (First): X = Margin Left = 20
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(1), x: PositionChange::SetFixed(x), .. }
            if *x == 20.0
        )));

        // Window 2 (Second): X = Margin + Width + Gap = 20 + 100 + 10 = 130
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow { id: Some(2), x: PositionChange::SetFixed(x), .. }
            if *x == 130.0
        )));
    }

    #[test]
    fn test_window_rules_override_behavior() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Two windows. One with a rule, one default.
        // Window 1: Default (Width 300, Peek 10)
        // Window 2: Rule (Width 500, Peek 100)
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let mut w2 = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        w2.app_id = Some("special".into());

        let mock = MockNiri::new(vec![w1, w2]);

        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Right;
        config.geometry.width = 300;
        config.interaction.peek = 10;

        config.window_rule = vec![WindowRule {
            app_id: Some(Regex::new("special").unwrap()),
            width: Some(500),
            peek: Some(100),
            ..Default::default()
        }];

        let mut state = AppState::default();

        // 1 is bottom, 2 is top
        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1);
        state.windows.push(w2);

        state.is_hidden = true;

        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;

        // Screen W: 1920

        // Window 1 (Default):
        // Width 300. Peek 10.
        // Hidden X = ScreenW - Peek = 1920 - 10 = 1910
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == 1910.0
        )));

        // Window 2 (Special Rule):
        // Width 500. Peek 100.
        // Hidden X = ScreenW - Peek = 1920 - 100 = 1820
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(2),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == 1820.0
        )));
    }

    #[test]
    fn test_window_rules_left_hidden_mixed() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Left side, Hidden.
        // Window 1: Default (Width 300, Peek 10)
        // Window 2: Special (Width 400, Peek 50)
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let mut w2 = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        w2.app_id = Some("special".into());

        let mock = MockNiri::new(vec![w1, w2]);

        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Left;
        config.interaction.align = crate::config::Align::BottomLeft;
        config.interaction.peek = 10;
        config.geometry.width = 300;
        config.margins.left = 0;

        config.window_rule = vec![WindowRule {
            app_id: Some(Regex::new("special").unwrap()),
            width: Some(400),
            peek: Some(50),
            ..Default::default()
        }];

        let mut state = AppState::default();

        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1);
        state.windows.push(w2);

        state.is_hidden = true;

        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;

        // Window 1 (Default):
        // X = -Width + Peek = -300 + 10 = -290
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == -290.0
        )));

        // Window 2 (Special):
        // X = -Width + Peek = -400 + 50 = -350
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(2),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == -350.0
        )));
    }

    #[test]
    fn test_window_rules_bottom_visible_mixed() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Bottom side, Visible.
        // Window 1: Special (Width 200)
        // Window 2: Default (Width 100)
        let mut w1 = mock_window_with_size(2, false, true, 1, Some((1.0, 2.0)), (100, 200));
        let w2 = mock_window_with_size(1, false, true, 1, Some((1.0, 2.0)), (100, 200));
        w1.app_id = Some("wide".into());

        let mock = MockNiri::new(vec![w1, w2]);

        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Bottom;
        config.interaction.align = crate::config::Align::BottomLeft;
        config.geometry.width = 100;
        config.geometry.gap = 10;
        config.margins.left = 0;

        config.window_rule = vec![WindowRule {
            app_id: Some(Regex::new("wide").unwrap()),
            width: Some(200),
            ..Default::default()
        }];

        let mut state = AppState::default();

        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1); // Will be processed first
        state.windows.push(w2); // Will be processed second

        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).expect("Reorder failed");

        let actions = &ctx.socket.sent_actions;

        // Window 1 (Special):
        // X = Start (0) + Offset (0) = 0
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == 0.0
        )));

        // Window 2 (Default):
        // X = Start (0) + Offset (Width1 + Gap) = 0 + 200 + 10 = 110
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(2),
                x: PositionChange::SetFixed(x),
                ..
            } if *x == 110.0
        )));
    }

    #[test]
    fn test_window_rules_right_height_stacking_mixed() {
        let temp_dir = tempdir().unwrap();
        // Scenario: Right side.
        // Window 1: Default (Height 200)
        // Window 2: Special (Height 400)
        // Window 3: Default (Height 200)
        let w1 = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let mut w2 = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        w2.app_id = Some("tall".into());
        let w3 = mock_window(3, false, true, 1, Some((1.0, 2.0)));
        let mock = MockNiri::new(vec![w1, w2, w3]);
        let mut config = mock_config();
        config.interaction.position = SidebarPosition::Right;
        config.geometry.height = 200;
        config.geometry.gap = 10;
        config.margins.top = 0;
        config.margins.right = 0;
        config.margins.bottom = 0;
        config.window_rule = vec![WindowRule {
            app_id: Some(Regex::new("tall").unwrap()),
            height: Some(400),
            ..Default::default()
        }];
        let mut state = AppState::default();
        let w1 = WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w2 = WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        let w3 = WindowState {
            id: 3,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        };
        state.windows.push(w1);
        state.windows.push(w2);
        state.windows.push(w3);
        let mut ctx = Ctx {
            state,
            config,
            socket: mock,
            cache_dir: temp_dir.path().to_path_buf(),
        };
        reorder(&mut ctx).expect("Reorder failed");
        let actions = &ctx.socket.sent_actions;
        // Screen H: 1080

        // Window 1 (Default, actual_h=200):
        // Y = ScreenH - configH - MarginBottom - Offset
        // Y = 1080 - 200 - 0 - 0 = 880
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(1),
                y: PositionChange::SetFixed(y),
                ..
            } if *y == 880.0
        )));
        // Window 2 (Tall rule, config_h=400, actual_h=200):
        // Gap mode Dynamic: offset = actual_h1 + gap = 200 + 10 = 210
        // Y = ScreenH - config_h2 - MarginBottom - Offset
        // Y = 1080 - 400 - 0 - 210 = 470
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(2),
                y: PositionChange::SetFixed(y),
                ..
            } if *y == 470.0
        )));
        // Window 3 (Default, actual_h=200):
        // Previous Offset = 210 + actual_h2 + Gap = 210 + 200 + 10 = 420
        // Y = ScreenH - config_h3 - MarginBottom - Offset
        // Y = 1080 - 200 - 0 - 420 = 460
        assert!(actions.iter().any(|a| matches!(a,
            Action::MoveFloatingWindow {
                id: Some(3),
                y: PositionChange::SetFixed(y),
                ..
            } if *y == 460.0
        )));
    }

    #[test]
    fn test_focus_peek_transitions_on_focus_change() {
        let temp_dir = tempdir().unwrap();
        // Two hidden windows. Focus moves from 1→2→1. Verify peek retracts/extends.
        let w1a = mock_window(1, true, true, 1, Some((1.0, 2.0)));
        let w2a = mock_window(2, false, true, 1, Some((1.0, 2.0)));
        let mock1 = MockNiri::new(vec![w1a, w2a]);

        let mut config = mock_config();
        config.interaction.peek = 10;
        config.interaction.focus_peek = Some(50);

        let mut state = AppState {
            is_hidden: true,
            ..Default::default()
        };
        state.windows.push(WindowState {
            id: 1,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        });
        state.windows.push(WindowState {
            id: 2,
            width: 300,
            height: 200,
            is_floating: false,
            position: None,
        });

        let mut ctx = Ctx {
            state,
            config,
            socket: mock1,
            cache_dir: temp_dir.path().to_path_buf(),
        };

        reorder(&mut ctx).unwrap();

        // After first reorder: id=1 focused (x=1920-50=1870), id=2 unfocused (x=1920-10=1910)
        let actions = &ctx.socket.sent_actions;
        assert!(
            actions.iter().any(|a| matches!(
                a,
                Action::MoveFloatingWindow {
                    id: Some(1),
                    x: PositionChange::SetFixed(1870.0),
                    ..
                }
            )),
            "focused window 1 should peek 50px"
        );
        assert!(
            actions.iter().any(|a| matches!(
                a,
                Action::MoveFloatingWindow {
                    id: Some(2),
                    x: PositionChange::SetFixed(1910.0),
                    ..
                }
            )),
            "unfocused window 2 should peek 10px"
        );

        // Now focus moves: id=1 loses focus, id=2 gains focus
        let w1b = mock_window(1, false, true, 1, Some((1.0, 2.0)));
        let w2b = mock_window(2, true, true, 1, Some((1.0, 2.0)));
        let mock2 = MockNiri::new(vec![w1b, w2b]);
        ctx.socket = mock2;

        reorder(&mut ctx).unwrap();

        let actions2 = &ctx.socket.sent_actions;
        // id=1 should now retract to peek=10 (x=1910), id=2 should extend to focus_peek=50 (x=1870)
        assert!(
            actions2.iter().any(|a| matches!(
                a,
                Action::MoveFloatingWindow {
                    id: Some(1),
                    x: PositionChange::SetFixed(1910.0),
                    ..
                }
            )),
            "unfocused window 1 should retract to 10px peek"
        );
        assert!(
            actions2.iter().any(|a| matches!(
                a,
                Action::MoveFloatingWindow {
                    id: Some(2),
                    x: PositionChange::SetFixed(1870.0),
                    ..
                }
            )),
            "focused window 2 should extend to 50px peek"
        );
    }
}
