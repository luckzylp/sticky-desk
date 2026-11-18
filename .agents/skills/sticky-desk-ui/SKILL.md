---
name: sticky-desk-ui
description: 'StickyDesk Linux desktop UI conventions built on GPUI Kit. Use when changing StickyDesk windows, notes, Explorer, Settings, dialogs, tray menus, colors, typography, spacing, or window chrome. Records the project-specific visual language and the GPUI Kit patterns used in this repository.'
---

# StickyDesk UI

StickyDesk is a Linux sticky-note desktop application built with Rust and
GPUI Kit. This skill records the UI language already used in the project so new
screens stay consistent with the existing note windows, Explorer, Settings and
dialogs.

## Product shape

- Desktop-first, quiet, compact. Notes are the primary objects; Explorer is the
  hub; Settings and About are auxiliary windows.
- A note is a borderless, translucent sticky card with a custom title bar.
- All windows use client-side decorations and a custom title bar; window
  controls are positioned where the desktop puts them.

## Window chrome and corners

- Radius is one shared value: `WINDOW_RADIUS = 12.0` in `src/app/chrome.rs`.
  Never introduce a second radius for a note/window surface.
- Windows are created with `WindowBackgroundAppearance::Transparent` and
  `WindowDecorations::Client` in `src/app/windows.rs`.
- The root surface must stay transparent. `TransparentRoot` in
  `src/app/chrome.rs` is registered after `gpui_component::init` for this.
- `content_frame()` must fill the window, apply `rounded(WINDOW_RADIUS)` and
  `overflow_hidden`, and own the background. Do not add a second border or
  shadow around it: the window border/shadow is handled by the patched
  `vendor/gpui-component/src/window_border.rs`.
- Title bars round only their top corners:
  `rounded_tl(px(WINDOW_RADIUS)).rounded_tr(px(WINDOW_RADIUS))`.
- Any full-window child background must be removed or rounded; GPUI content
  clipping is rectangular and will otherwise paint over the corner alpha.
- Do not add `.p_1()`, border or shadow to a note root: it creates a visible
  frame outside the rounded surface.
- Window controls use `chrome::control_order()` / `controls_on_left()` and
  follow GNOME `button-layout`, KDE `kdeglobals` and macOS conventions.
  Close is rendered as a text `✕`; minimize and maximize use Lucide icons.

## Sidebar (Explorer)

- Width 240px, quiet border-right, compact rows.
- Section headers are 12px semibold text (`Notebooks`, `Notes`).
- Section actions are small ghost icon buttons on the right (e.g. `+` for New
  Notebook, `⋯` for Manage Notebooks); do not add full-width action rows.
- Navigation rows use the custom `nav_item` in `src/app/explorer.rs`:
  - fixed icon lane on the left, label immediately after it;
  - 30px high, 8px horizontal padding, 4px radius;
  - hover uses `theme.muted` at low alpha; selected uses `theme.accent` at 16%;
  - notebook rows show a trailing note count in muted 12px text.
- Do not use a centered `Button` for a sidebar row.

## Icons and the asset source

- `src/main.rs` registers `gpui_kit::assets::AllAssets`, not
  `gpui_kit::assets::Assets`. `Assets` only embeds the ~106 "default" icons;
  `bold`, `italic`, `underline`, `strikethrough`, `highlighter`, `list`,
  `list-checks`, `list-ordered`, `text-align-*` and `eraser` are missing from
  that subset and silently render as blank buttons.
- When an icon button looks empty, check the icon name against the embedded
  asset set before changing layout code. `Palette`, `Search`, `Eye` and
  `Ellipsis` are in the default set, which is why partial toolbars can look
  "half working".

## Notes

- Note colors are user data, not theme tokens; alpha is applied through
  `theme::hsla(color, opacity/100)`.
- Text/font color is derived from the note background luminance
  (`theme::auto_font_color`) and the note dark-mode setting.
- Note title bar: 31px; hide/menu/collapse/window controls; title centered,
  drag on the title area only; title double-click edits.
- The note toolbar has exactly two rows:
  - **Centred row** (`w_full().justify_center()`): font colour, find, `MD/RTF`
    toggle, preview. These four controls must stay centred at every width; do
    not move them into a left-aligned group.
  - **Formatting row** (`px_1()`): bold, italic, underline, strike, highlight,
    bullet, checkbox; numbered list, the three alignments and clear appear only
    when the window is wider than 340px. Font size stays pinned right.
- Do not rebuild a single-row left/centre/right layout: the left group
  under-renders in this GPUI version. Keep the two-row structure.
- The note body is editable in Markdown by default. Markdown formatting is
  applied through the toolbar; `MD/RTF` toggles the storage mode; the eye
  button switches to the rendered `TextView::markdown` preview.
- Locked notes are read-only and show the Markdown preview.
- Use `TextDecoration`/`HighlightStyle` for rich-mode inline styling only;
  Markdown mode relies on syntax, not decorations.

## Settings

- Settings is a vertically scrolling page of grouped card sections.
- Sections use `theme.border`, 8px card radius, 16px padding and a 12–14px
  semibold title.
- Rows are 40–44px high with the label on the left and the control on the
  right (`justify_between`); controls do not drift to the center.
- Default note color is a 16×16, 256-color grid.
- Database, sounds, hotkeys and language are data-driven; do not hardcode
  platform paths in the view.
- Links must contain only the app's supported destinations; no legacy
  "Help" link to the original Windows product.

## Dialogs and menus

- Use `window.open_dialog` and `DialogClose` from GPUI Kit.
- Titles are short nouns or commands; buttons name the action (`Save`,
  `Delete`, `Restore`) rather than `OK`.
- Destructive actions use `.danger()` and appear alone or last.
- Dialog actions are left-aligned when a footer is present; avoid centered
  footer rows that conflict with the app's left-aligned chrome.
- The note context menu (the "..." menu) uses a custom `✕` close button inside
  its title row. It follows `chrome::controls_on_left()`: left on systems whose
  window buttons are on the left, right otherwise. The built-in dialog close
  button stays disabled (`close_button(false)`) to avoid a duplicate. Render
  the title close as a plain fixed-size `Button` (28×24, padding 0); do not
  wrap it in `DialogClose`, whose renderer uses `size_full()` and stretches the
  button across the title row.
- Note context menus keep only supported actions: no legacy sharing/email
  actions.

## Visual tokens

- Use `cx.theme()` tokens (`background`, `foreground`, `muted_foreground`,
  `border`, `primary`, `accent`, `danger`, `warning`, `success`) before any
  literal color.
- Raw colors are allowed only for user-selected note colors, the 256-color
  palette and brand/icon raster data.
- Type scale: 12px metadata, 13–14px controls/title, 15px note body,
  22–26px page titles.
- Spacing: 2/4/8/12/16/24/32; compact controls use the component size API
  (`xsmall`, `small`) rather than custom heights.
- Icons are Lucide `gpui_kit::assets::IconName`. If an icon fails to render
  (as the old Trash SVG did), use a small embedded SVG via `Icon::data`
  rather than a new dependency.

## GPUI Kit patterns

- One `gpui-kit` dependency; layers are `gpui_kit::component`,
  `gpui_kit::base`, `gpui_kit::assets`.
- Stateful controls own `Entity<State>` in the view (`EditorState`,
  `InputState`); transient state stays in the view.
- Lists and repeated rows use stable IDs derived from the domain object.
- Views observe the shared `Entity<AppModel>` and call `cx.notify()` after
  mutations; DB writes go through `AppModel::mutate_note`.
- Window creation goes through `src/app/windows.rs` so app IDs, icons,
  transparency, corner radii and platform headings stay centralised.
- Do not edit GPUI itself. The only local vendor patch is gpui-component's
  `window_border.rs`, needed to remove its hardcoded square frame/shadow.

## Verification

Before finishing a UI change:

```bash
cargo fmt
cargo build --release
```

Then exercise the real window: note create/hide, Explorer sidebar rows,
Settings scrolling, dialog open/close, and resize. Check all four window
corners on X11 and Wayland if available.
