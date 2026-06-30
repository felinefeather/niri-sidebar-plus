# Plan: send-toggle redesign — sticky-aware, coordinate-safe

## 1. Non-sticky `send-toggle` (default, no flag)

Purely presence-based — no `sent_workspace` state needed.

```
niri-sidebar send-toggle -i 3
```

Logic:
1. `get_windows()` → filter by sidebar IDs on **current workspace**
2. If any found: `MoveWindowToWorkspace` all tracked windows to target workspace → exit
3. If none found: find sidebar windows on **other workspaces** → `MoveWindowToWorkspace` all of them to current workspace → `reorder()` to position → exit

## 2. Sticky `send-toggle` (`--sticky` flag)

Stateful — uses `sent_workspace` to track deployed/recalled status.

```
niri-sidebar send-toggle -i 3 --sticky
```

Logic:
1. `get_windows()` → filter by sidebar IDs on **current workspace**
2. If `sent_workspace` is set (deployed mode):
   - `MoveWindowToWorkspace` all to current workspace
   - Clear `sent_workspace`
   - `reorder()` to reposition
3. If `sent_workspace` is None (local mode):
   - `MoveWindowToWorkspace` all to target workspace
   - Set `sent_workspace` (with target + lock_sticky=true)
   - Daemon skips sticky workspace-focus when lock_sticky is true

**Sticky auto-recall on new window**: When user does `toggle-window` (Mod+D) while `sent_workspace` is set:
- Before adding, trigger recall (bring deployed windows back)
- Then add the new window normally
- This keeps sidebar unified

Implementation: in `toggle_window()` (togglewindow.rs), before `add_to_sidebar()`:
```
if ctx.config.interaction.sticky && ctx.state.sent_workspace.is_some() {
    // Auto-recall deployed windows before adding new one
    commands::recall(ctx)?;
}
```

## 3. Coordinate update on recall

After moving windows to current workspace, call `reorder()` to position them with correct screen dimensions.

## 4. Config: invert `-l` to `--sticky`

- Remove `-l`/`--lock` flag
- Add `--sticky` flag (opt-in)
- Default behavior: presence-based, no state

## Files changed

| File | Change |
|------|--------|
| `src/main.rs` | Replace `-l` with `--sticky` on `SendToggle` |
| `src/commands/send_toggle.rs` | Rewrite: non-sticky presence-based + sticky stateful |
| `src/commands/togglewindow.rs` | Auto-recall on add when sticky+deployed |
| `src/commands/listen.rs` | (already handles lock_sticky — no change) |
| `src/state.rs` | (SentWorkspace already has lock_sticky — no change) |
| `~/.config/niri/config.kdl` | Drop `-l` from binding |

## Verify
- `cargo test` — 45 pass
- `cargo build --release`
- Install, restart daemon
- Test: `send-toggle -i 3` with sidebar visible → windows go to ws3
- Test: `send-toggle -i 3` again → windows come back
- Test: sticky auto-recall on Mod+D while deployed
