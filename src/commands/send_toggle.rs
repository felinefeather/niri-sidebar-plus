use crate::niri::NiriClient;
use crate::state::{SentWorkspace, save_state};
use crate::Ctx;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

pub fn send_toggle<C: NiriClient>(ctx: &mut Ctx<C>, target: WorkspaceReferenceArg, lock: bool) -> anyhow::Result<()> {
    let windows = ctx.socket.get_windows()?;
    let sidebar_windows: Vec<_> = windows
        .iter()
        .filter(|w| ctx.state.windows.iter().any(|ws| ws.id == w.id))
        .collect();

    if sidebar_windows.is_empty() {
        eprintln!("No sidebar windows to toggle");
        return Ok(());
    }

    if ctx.state.sent_workspace.is_some() {
        // Recall: bring all back to current workspace
        let active_ws = ctx.socket.get_active_workspace()?.id;
        for w in sidebar_windows {
            ctx.socket.send_action(Action::MoveWindowToWorkspace {
                window_id: Some(w.id),
                reference: WorkspaceReferenceArg::Id(active_ws),
                focus: false,
            })?;
        }
        ctx.state.sent_workspace = None;
    } else {
        // Send to target workspace
        let target_clone = target.clone();
        for w in sidebar_windows {
            ctx.socket.send_action(Action::MoveWindowToWorkspace {
                window_id: Some(w.id),
                reference: target.clone(),
                focus: false,
            })?;
        }
        let saved = match target_clone {
            WorkspaceReferenceArg::Index(i) => SentWorkspace { index: Some(i), name: None, lock_sticky: lock },
            WorkspaceReferenceArg::Name(n) => SentWorkspace { index: None, name: Some(n), lock_sticky: lock },
            WorkspaceReferenceArg::Id(_) => SentWorkspace { index: None, name: None, lock_sticky: lock },
        };
        ctx.state.sent_workspace = Some(saved);
    }

    save_state(&ctx.state, &ctx.cache_dir)?;
    Ok(())
}
