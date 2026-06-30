use crate::Ctx;
use crate::niri::NiriClient;
use anyhow::Result;
use niri_ipc::{Action, WorkspaceReferenceArg};

pub fn send<C: NiriClient>(ctx: &mut Ctx<C>, target: WorkspaceReferenceArg) -> Result<()> {
    let windows = ctx.socket.get_windows()?;
    let sidebar_windows: Vec<_> = windows
        .iter()
        .filter(|w| ctx.state.windows.iter().any(|ws| ws.id == w.id))
        .collect();

    if sidebar_windows.is_empty() {
        eprintln!("No sidebar windows to send");
        return Ok(());
    }

    for w in sidebar_windows {
        ctx.socket.send_action(Action::MoveWindowToWorkspace {
            window_id: Some(w.id),
            reference: target.clone(),
            focus: false,
        })?;
    }

    Ok(())
}
