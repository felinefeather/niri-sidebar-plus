use crate::niri::NiriClient;
use crate::Ctx;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

pub fn recall<C: NiriClient>(ctx: &mut Ctx<C>) -> Result<()> {
    let active_ws = ctx.socket.get_active_workspace()?.id;
    let windows = ctx.socket.get_windows()?;
    let sidebar_windows: Vec<_> = windows
        .iter()
        .filter(|w| ctx.state.windows.iter().any(|ws| ws.id == w.id))
        .filter(|w| w.workspace_id != Some(active_ws))
        .collect();

    if sidebar_windows.is_empty() {
        anyhow::bail!("No sidebar windows on other workspaces to recall");
    }

    for w in sidebar_windows {
        ctx.socket.send_action(Action::MoveWindowToWorkspace {
            window_id: Some(w.id),
            reference: WorkspaceReferenceArg::Id(active_ws),
            focus: false,
        })?;
    }

    Ok(())
}
