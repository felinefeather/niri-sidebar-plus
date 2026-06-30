# niri-sidebar-plus

> ⚠️ **AI-generated fork — use at your own risk.** Built via OpenCode. Expect rough edges.

A fork of [niri-sidebar](https://github.com/Vigintillionn/niri-sidebar) with alignment control, auto-fit struts, and auto-defocus.

## What's New

Features not in upstream:

### `align` — Window Alignment
Controls overflow direction when a window's actual size exceeds the configured width/height (e.g., applications with large `min-width`) to make it looks better.

```toml
[interaction]
align = "top-right"  # options: top-right, top-left, bottom-right, bottom-left
```

- **`right`/`left`** — which horizontal edge anchors. Overflow goes to the opposite side (off-screen instead of into workspace).
- **`top`/`bottom`** — stacking direction (top→bottom vs bottom→top).

### `auto_fit` — Per-Workspace Struts
Dynamically writes niri layout struts per active workspace. When the sidebar is **hidden** with windows on the **current** workspace, applies a strut to work around niri's 75px minimum-visible constraint.

Requires one line in your niri config:

```kdl
include "/tmp/niri-sidebar-struts.kdl"
```

```toml
[interaction.auto_fit]
with_sidebar = 60   # strut when hidden with windows (px)
without_sidebar = 0  # strut otherwise (px)

# Optional: preserve your existing niri struts (supports float values)
[interaction.auto_fit.base]
top = 64.0
bottom = -4.0
```

### `gap_mode` — Stack Spacing
Controls how windows stack: `dynamic` (default) uses each window's actual size + gap. `static` uses a fixed gap regardless of height, for a card-stack look.

```toml
[geometry]
gap_mode = "dynamic"  # or "static"
```

### `auto_defocus` — Auto-Defocus
When enabled, pressing `Mod+S` adds a window to the sidebar then immediately returns focus to the tiled workspace.

```toml
[interaction]
auto_defocus = true
```

### `send`, `send-toggle` & `recall` — Workspace Transfer
Forcibly move all sidebar windows to a target workspace.

```bash
niri-sidebar send -i 3            # one-way: send to workspace index 3
niri-sidebar send -n chat         # one-way: send to workspace named "chat"
niri-sidebar send-toggle -i 3     # toggle: sends if here, recalls if elsewhere
niri-sidebar send-toggle -i 3 -s  # sticky mode: stateful with auto-recall on new window
niri-sidebar recall               # bring all back to current workspace
```

**Non-sticky** (default): presence-based. If sidebar windows are on the current workspace → send to target. If elsewhere → recall. No state persisted.

**Sticky** (`-s`): stateful via `sent_workspace` in state.json. Deployed mode locks sticky (daemon won't pull windows back) and suspends auto-fit on the source workspace. Adding a new window while deployed auto-recalls first. Auto-detected when `sticky = true` in config — `-s` flag is then optional.

## Installation

```bash
git clone https://github.com/felinefeather/niri-sidebar-plus
cd niri-sidebar-plus
cargo build --release
cp target/release/niri-sidebar ~/.local/bin/
```

## Usage

Same as upstream. See the [original README](https://github.com/Vigintillionn/niri-sidebar) for full configuration and workflow tips.

## License

MIT.

