# Plan: send-toggle — lock sticky by default

## Intent
Remove `-l`/`--lock` CLI flag from `send-toggle`. Always set `lock_sticky: true` when deploying. For non-sticky users this is the correct default. Sticky integration will be discussed separately.

## Changes

### 1. `src/main.rs` — Remove `--lock` arg
- Remove `#[arg(short = 'l', long)] lock: bool` from `SendToggle`
- Update match: `commands::send_toggle(&mut ctx, target)?` (remove `lock` param)

### 2. `src/commands/send_toggle.rs` — Always lock
- Remove `lock: bool` parameter
- Hardcode `lock_sticky: true` in all `SentWorkspace` constructors
- Add auto_fit: on deploy, clear struts. On recall, let next reorder() restore.

### 3. Niri config keybinding — Drop `-l`
- Change `send-toggle -i 3 -l` to `send-toggle -i 3`

## Verify
- `cargo test` — all 45 pass
- `cargo build --release`
- Install, restart daemon
