use crate::Ctx;
use crate::commands::reorder as reorder_mod;
use crate::commands::reorder::update_auto_fit;
use crate::niri::NiriClient;
use crate::state::{SentWorkspace, save_state};
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

/// Automatically uses sticky mode when `sticky = true` in config,
/// otherwise uses presence-based non-sticky mode.
pub fn send_toggle<C: NiriClient>(
    ctx: &mut Ctx<C>,
    target: WorkspaceReferenceArg,
    sticky_flag: bool,
) -> Result<()> {
    let sticky = sticky_flag || ctx.config.interaction.sticky;
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
            reorder_mod(ctx)?;
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
            // Auto-fit will be handled naturally by reorder() on workspace transition

            let saved = match target_clone {
                WorkspaceReferenceArg::Index(i) => SentWorkspace {
                    index: Some(i),
                    name: None,
                    lock_sticky: true,
                },
                WorkspaceReferenceArg::Name(n) => SentWorkspace {
                    index: None,
                    name: Some(n),
                    lock_sticky: true,
                },
                WorkspaceReferenceArg::Id(_) => SentWorkspace {
                    index: None,
                    name: None,
                    lock_sticky: true,
                },
            };
            ctx.state.sent_workspace = Some(saved);
            save_state(&ctx.state, &ctx.cache_dir)?;
            update_auto_fit(ctx)?;
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
            update_auto_fit(ctx)?;
        } else {
            // Sidebar windows elsewhere → recall to current workspace
            for w in all_sidebar {
                ctx.socket.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(w.id),
                    reference: WorkspaceReferenceArg::Id(active_ws),
                    focus: false,
                })?;
            }
            reorder_mod(ctx)?;
        }
    }

    Ok(())
}
