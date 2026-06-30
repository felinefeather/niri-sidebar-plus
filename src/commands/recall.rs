use crate::Ctx;
use crate::commands::reorder;
use crate::niri::NiriClient;
use crate::state::save_state;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

pub fn recall<C: NiriClient>(
    ctx: &mut Ctx<C>,
    source: Option<WorkspaceReferenceArg>,
) -> Result<()> {
    let active_ws = ctx.socket.get_active_workspace()?.id;
    let windows = ctx.socket.get_windows()?;
    let sidebar_ids: Vec<u64> = ctx.state.windows.iter().map(|w| w.id).collect();

    // Resolve workspace ID from index/name
    let source_ws: Option<u64> = if let Some(ref src) = source {
        let workspaces = ctx.socket.get_workspaces()?;
        match src {
            WorkspaceReferenceArg::Index(idx) => {
                workspaces.iter().find(|w| w.idx == *idx).map(|w| w.id)
            }
            WorkspaceReferenceArg::Name(name) => workspaces
                .iter()
                .find(|w| w.name.as_deref() == Some(name.as_str()))
                .map(|w| w.id),
            WorkspaceReferenceArg::Id(id) => Some(*id),
        }
    } else {
        None
    };

    let sidebar_windows: Vec<_> = windows
        .iter()
        .filter(|w| sidebar_ids.contains(&w.id))
        .filter(|w| {
            w.workspace_id != Some(active_ws)
                && source_ws.is_none_or(|sw| w.workspace_id == Some(sw))
        })
        .collect();

    if sidebar_windows.is_empty() {
        let hint = match &source {
            Some(WorkspaceReferenceArg::Index(i)) => format!("workspace index {}", i),
            Some(WorkspaceReferenceArg::Name(n)) => format!("\"{}\"", n),
            _ => "other workspaces".into(),
        };
        eprintln!("No sidebar windows on {} to recall", hint);
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
