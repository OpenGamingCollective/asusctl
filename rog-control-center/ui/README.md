# rog-control-center — Slint UI notes

This file collects the hard-won, hard-to-see facts about writing Slint layout code
for this UI. If you edit any `.slint` file here, read this first — it will save
you the bug below.

## TL;DR (rules that prevent the bug)

- **NEVER make a `VerticalLayout` (or any layout) the FIRST child of a
  `HorizontalLayout`.** On the slint version this project is pinned to, every
  cell after it renders *empty*.
  - First cell of a horizontal row must be a non-layout element: a `Rectangle`,
    or a component rooted in `Rectangle` (e.g. `RogProductCard`).
  - If a row starts with a widget-like block, give it a fixed `width`/`height`
    and put it directly in the row (no wrapper `VerticalLayout`).
- **Never size a layout child from its parent's geometry**
  (`height: parent.width`, `width: parent.height`, `height: parent.height` etc.).
  Slint resolves a layout child's size against geometry that isn't final yet —
  the card/square hack must use a fixed length shared by both sides instead.
- **Keep the product card square with a named length, not `parent.width`.**
  See `home.slint`: `property <length> card-side: ...` is used for both the
  card's `width` and `height`.
- **Never trust a different slint version's renderer to validate these pages**
  (e.g. `slint-viewer 1.17.x`). Use the pinned-version harness in
  [Verifying the UI](#verifying-the-ui) below.

## The bug this project hit (2026-08)

Symptom: only the product card rendered on the Home dashboard. The right column
(System Usage, CPU/dGPU, Fan/Memory, Operation Mode) was a black void. It looked
like the card was "pushing everything" but it wasn't a z-order problem at all.

Root cause: **slint 1.13.1 engine bug** — when a `VerticalLayout` is the first
child of a `HorizontalLayout`, the remaining cells of that row never paint.

Minimal repro (renders green only; the red rectangle is empty):

```slint
HorizontalLayout {
    spacing: 36px;
    VerticalLayout { width: 360px; alignment: start;        // first child = layout
        Rectangle { width: 100%; height: 360px; background: green; }
    }
    Rectangle { width: 600px; height: 200px; background: red; }   // empty!
}
```

Swap the first child for a plain `Rectangle` and the red rectangle renders.
This was isolated with the exact pinned toolchain (see
[Verifying the UI](#verifying-the-ui)) and confirmed in `home.slint`:

- Before: left cell was `VerticalLayout { width: root.card-side; ... }` wrapping
  `RogProductCard` → right column empty.
- After: `RogProductCard` is a **direct** child of the row with fixed
  `width: root.card-side; height: root.card-side;` → right column renders.

### Why we're still on this slint version (do not "just upgrade")

The workspace pins rustc 1.85 (`rust-toolchain.toml`, `flake.nix`), and slint's
MSRV is tied to it:

| slint         | MSRV  |
|---------------|-------|
| 1.13.1        | 1.85  |
| 1.14 / 1.15 / 1.16 | 1.88 |
| 1.17.x (latest) | 1.92 |

So 1.13.1 is the newest release that compiles under our toolchain, and this
layout bug is live until the toolchain is bumped. When slint is upgraded, first
re-verify the pages with the harness below — the bug may be fixed *and* the
workaround (`home.slint`) should then be removable.

## How the Home dashboard is structured (so you don't break it again)

`home.slint` (`PageHome`):

- Responsive properties on the root:
  - `compact: self.width < 1000px` — tighter layout for small laptops.
  - `card-side`, `gap-lg`, `gap-md`, `metric-max`, `pad-x` — all lengths derived
    from `compact`. Metric cards use `max-width: root.metric-max`.
- Layout:
  - Root `VerticalLayout`: titlebar (52px) + dashboard (`vertical-stretch: 1`).
  - Dashboard `VerticalLayout`: padding + the columns `HorizontalLayout`.
  - Left cell: `RogProductCard` **directly** (Rectangle-rooted), square via
    `width/height = root.card-side`.
  - Right cell: `VerticalLayout { horizontal-stretch: 1 }` — fine as a *later*
    cell; only the *first* cell must not be a layout.

`main_window.slint` (hosting / responsiveness):

- Window sizing restored from `AppSize`: `min-width/height` (1100×630),
  `preferred-width/height` (1000×700).
- Page host is a `Rectangle { horizontal-stretch: 1 }`; `PageHome` gets
  `width: parent.width > 1100px ? 1100px : parent.width; height: parent.height;
  x: (parent.width - self.width)/2;` → fills monitors, capped at 1100px and
  centered; fits 1366-laptops at min size; `compact` covers narrower windows.

## Verifying the UI

`slint-viewer` shipped in nix is **1.17.x** and has no 1.17/1.13 parity for this
UI: it can't screenshot Rectangle-rooted pages ("invalid size") and its layout
behaviour differs from 1.13.1. Do not use it to validate layout.

The reliable way is a tiny standalone harness that renders the real
`main_window.slint` with the **exact pinned slint 1.13.1** (interpreter +
software renderer, no display needed):

```bash
# One-time build (out of tree):
mkdir -p /tmp/opencode/slintrender/src && cd /tmp/opencode/slintrender
# (Cargo.toml + src/main.rs — ask in the repo/git history for the current copy)
cargo build --release

# Render the real window at any size:
./target/release/slintrender 1366 768 /tmp/opencode/render.png
# SLINT_SRC=path/to/other.slint overrides the file (include path points at ui/)
```

Key points the harness relies on:

- Deps must resolve on rustc 1.85 — seed `Cargo.lock` from the workspace
  `Cargo.lock` so MSRV-compatible transitive versions are kept.
- Enable the `software-renderer-systemfonts` feature on `i-slint-core`
  (direct dep) so text renders; otherwise the renderer panics with
  "No font fallback found".
- It uses `slint::platform::set_platform` with a minimal window adapter
  (`SoftwareRenderer` + `Window::new`), then
  `spin_on::spin_on(compiler.build_from_path(...))`
  (`slint_interpreter::ComponentCompiler`, style `"fluent"`), sizes the window,
  calls `renderer.render(&mut buf, w)`, and writes a PNG.

Then inspect the PNG. For an automated/second opinion, LM Studio (local, no
network) works well:

```bash
python3 /tmp/opencode/vision.py <png> "<prompt>" <max_tokens>
```

Use model **`qwen3.6-35b-a3b-ud`** (not the ridge text model) and pass
`"reasoning_effort": "none"` in the request, otherwise it burns the budget on
thinking and returns empty.

Always verify these sizes before considering a layout change done:

- `1366 768` — laptop (full mode)
- `1920 1080` — monitor
- `2560 1440` — monitor with the page capped + centered
- `1000 700` — narrow → exercises `compact` mode

## Repo layout

- `main_window.slint` — window + sidebar + page host (toast/error overlays live
  here too).
- `pages/home.slint` — the Home dashboard (`PageHome`).
- `globals.slint` — `Theme` tokens, `AppSize`.
- `widgets/` — reusable components (`cards`, `metrics`, `controls`, `layout`,
  `sidebar`).
