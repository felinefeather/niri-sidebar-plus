# niri-sidebar-plus

> ⚠️ **AI-generated fork — use at your own risk.** Built via OpenCode + DeepSeek. Expect rough edges.

A fork of [niri-sidebar](https://github.com/Vigintillionn/niri-sidebar) with alignment control, auto-fit struts, and auto-defocus.

## What's New

Three features not in upstream:

### `align` — Window Alignment
Controls overflow direction when a window's actual size exceeds the configured width/height (e.g., QQ with large `min-width`).

```toml
[interaction]
align = "top-right"  # options: top-right, top-left, bottom-right, bottom-left
```

- **`right`/`left`** — which horizontal edge anchors. Overflow goes to the opposite side (off-screen instead of into workspace).
- **`top`/`bottom`** — stacking direction (top→bottom vs bottom→top).

### `auto_fit` — Auto-Fit Struts
Dynamically writes niri layout struts to reserve screen space. When the sidebar is **hidden** with windows, applies a strut to work around niri's 75px minimum-visible constraint.

Requires one line in your niri config:

```kdl
include "/tmp/niri-sidebar-struts.kdl"
```

```toml
[interaction.auto_fit]
with_sidebar = 60   # strut when hidden with windows (px)
without_sidebar = 0  # strut otherwise (px)
```

### `auto_defocus` — Auto-Defocus
When enabled, pressing `Mod+D` adds a window to the sidebar then immediately returns focus to the tiled workspace.

```toml
[interaction]
auto_defocus = true
```

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

