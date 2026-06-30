use crate::commands::reorder;
use crate::niri::NiriClient;
use crate::state::save_state;
use crate::Ctx;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

pub fn recall<C: NiriClient>(ctx: &mut Ctx<C>) -> Result<()> {
    let active_ws = ctx.socket.get_active_workspace()?.id;
    let windows = ctx.socket.get_windows()?;
    let sidebar_ids: Vec<u64> = ctx.state.windows.iter().map(|w| w.id).collect();
    let sidebar_windows: Vec<_> = windows
        .iter()
        .filter(|w| sidebar_ids.contains(&w.id))
        .filter(|w| w.workspace_id != Some(active_ws))
        .collect();

    if sidebar_windows.is_empty() {
        eprintln!("No sidebar windows on other workspaces to recall");
        return Ok(());
    }

    for w in sidebar_windows {
        ctx.socket.send_action(Action::MoveWindowToWorkspace {
            window_id: Some(w.id),
            reference: WorkspaceReferenceArg::Id(active_ws),
            focus: false,
        })?;
    }

    ctx.state.sent_workspace = None;
    save_state(&ctx.state, &ctx.cache_dir)?;
    reorder(ctx)?;

    Ok(())
}
