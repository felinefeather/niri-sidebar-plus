use crate::commands::reorder;
use crate::niri::NiriClient;
use crate::state::{SentWorkspace, save_state};
use crate::Ctx;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

/// Non-sticky mode (default, sticky=false):
///   - Find sidebar windows on CURRENT workspace
///   - If any: send them all to target workspace
///   - If none: find sidebar windows on OTHER workspaces, recall them to current + reorder()
///
/// Sticky mode (sticky=true):
///   - If sent_workspace is set (deployed): recall all to current workspace, clear sent_workspace, reorder()
///   - If sent_workspace is None (local): send all to target, set sent_workspace with lock_sticky=true
///   - Auto-fit: on deploy, clear strut file. On recall, next reorder() will restore.
pub fn send_toggle<C: NiriClient>(ctx: &mut Ctx<C>, target: WorkspaceReferenceArg, sticky: bool) -> Result<()> {
    let windows = ctx.socket.get_windows()?;
    let active_ws = ctx.socket.get_active_workspace()?.id;

    let all_sidebar: Vec<_> = windows
        .iter()
        .filter(|w| ctx.state.windows.iter().any(|ws| ws.id == w.id))
        .collect();

    if all_sidebar.is_empty() {
        eprintln!("No sidebar windows to toggle");
        return Ok(());
    }

    if sticky {
        // ── Sticky mode: stateful with sent_workspace tracking ──
        if ctx.state.sent_workspace.is_some() {
            // Deployed → recall all to current workspace
            for w in all_sidebar {
                ctx.socket.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(w.id),
                    reference: WorkspaceReferenceArg::Id(active_ws),
                    focus: false,
                })?;
            }
            ctx.state.sent_workspace = None;
            save_state(&ctx.state, &ctx.cache_dir)?;
            reorder(ctx)?;
        } else {
            // Local → send all to target, set sent_workspace
            let target_clone = target.clone();
            for w in all_sidebar {
                ctx.socket.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(w.id),
                    reference: target.clone(),
                    focus: false,
                })?;
            }
            // Auto-fit: clear strut file on deploy
            crate::commands::reorder::clear_strut_file_if_active(ctx)?;

            let saved = match target_clone {
                WorkspaceReferenceArg::Index(i) => SentWorkspace { index: Some(i), name: None, lock_sticky: true },
                WorkspaceReferenceArg::Name(n) => SentWorkspace { index: None, name: Some(n), lock_sticky: true },
                WorkspaceReferenceArg::Id(_) => SentWorkspace { index: None, name: None, lock_sticky: true },
            };
            ctx.state.sent_workspace = Some(saved);
            save_state(&ctx.state, &ctx.cache_dir)?;
        }
    } else {
        // ── Non-sticky mode: presence-based, no state ──
        let on_current: Vec<_> = all_sidebar
            .iter()
            .filter(|w| w.workspace_id == Some(active_ws))
            .collect();

        if !on_current.is_empty() {
            // Sidebar windows on current workspace → send to target
            for w in on_current {
                ctx.socket.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(w.id),
                    reference: target.clone(),
                    focus: false,
                })?;
            }
        } else {
            // Sidebar windows elsewhere → recall to current workspace
            for w in all_sidebar {
                ctx.socket.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(w.id),
                    reference: WorkspaceReferenceArg::Id(active_ws),
                    focus: false,
                })?;
            }
            reorder(ctx)?;
        }
    }

    Ok(())
}
