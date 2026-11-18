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
- The Notebooks header has exactly one action: a small ghost `+` for New
  Notebook. Do not add a `⋯` manager entry or any other header action.
- Notebooks have no right-click context menu. Rename and delete are the two
  inline row icons on non-Default notebooks only.
- `Default` is protected: it never shows rename/delete icons, and the rename
  and delete dialogs return early for it.
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
- Note title bar: 31px; title centred, drag on the title area only, title
  double-click edits. Window controls are rendered on the left when the
  desktop puts them there and on the right otherwise; `note_controls` must be
  added on both sides.
- The window controls keep the system side and order (`control_order`): Minus
  minimizes with `window.minimize_window()`, Maximize zooms, and the close
  button hides the note. Do not give the minimize button a custom behaviour.
- Collapsing to a 32px title bar is a menu action (`Collapse note`), not the
  window minimize button. A collapsed note shows an explicit maximize button
  in its title bar (`Expand note`); the note minimum size is 200×32 so the
  collapse can apply.
- The note toolbar is a single formatting row (`px_1()`): bold, italic,
  underline, strike, highlight, bullet, checkbox; numbered list, the three
  alignments and clear appear only when the window is wider than 340px. Font
  size stays pinned right. There is no centred toolbar row.
- The note title bar only carries Preview (eye) and the ellipsis menu after
  the title area; Preview stays visible in preview mode and is the way back.
  Star and Find live in the note menu (`Add/Remove star`, `Find…`), not in the
  title bar.
- Font colour is not in the toolbar: the note menu's `Font color…` item opens
  the Markdown colour dialog for the selection or current line.
- Markdown is the only note format (GitHub-compatible). There is no RTF mode
  and no MD/RTF toggle: the toolbar inserts Markdown syntax, and the eye
  button switches to the rendered `TextView::markdown` preview.
- The shortcut toolbar is shown in edit mode only. Hide it while previewing
  and while a note is locked; never show it above the preview.
- Because the toolbar disappears in preview, the `Preview` toggle in the
  note menu is the way back. Keep it checkable in the dropdown menu.
- Locked notes are read-only and show the Markdown preview.
- The note body uses the GPUI Kit `Textarea` (not the code `Editor`): the
  code editor reserves a fold/gutter strip that pushes text far from the
  border. Wrap it in `px_1().py_1()` and use `Size::XSmall` so the text sits
  close to the note edge.
- Text edits update the model in memory only (`AppModel::update_doc_text`).
  The background loop flushes dirty notes every ~400ms, but the actual SQLite
  writes happen on the dedicated `DbWriter` thread (`src/store.rs`): the UI
  thread only clones the dirty document and sends it over a channel. Call
  `flush_now()` (which waits for the writer) on quit paths. Never call
  `save_note`/`save_doc` or notify the model from a keystroke handler.

## Settings

- Settings is a vertically scrolling page of grouped card sections.
- Sections use `theme.border`, 8px card radius, 16px padding and a 12–14px
  semibold title.
- Rows are 40–44px high with the label on the left and the control on the
  right (`justify_between`); controls do not drift to the center.
- Color choice uses the circular HSV wheel in `src/app/color_wheel.rs`
  (`ColorWheelView`): a hue/saturation disc, brightness slider and editable
  `#RRGGBB` field. Settings opens it as a dialog; the note menu embeds the
  same view inline. Do not reintroduce a 256-swatch grid, palettes or the
  GPUI Kit `ColorSelect` popover.
- Database, sounds, hotkeys and language are data-driven; do not hardcode
  platform paths in the view.
- Links must contain only the app's supported destinations; no legacy
  "Help" link to the original Windows product.

## Window lifecycle and tray

- The app uses `QuitMode::Explicit`. A `cx.on_window_closed` observer quits
  only when no windows remain and `settings.close_to_tray` is off.
- `close_to_tray` defaults to true (serde default) and is exposed in Settings
  as "Minimize to tray on close". With it on, closing the last window keeps
  the process and its tray icon alive; the tray's `Explorer`, `Show All Notes`
  and `Exit` actions are the way back.
- Do not switch back to `QuitMode::LastWindowClosed`: it quits before the
  tray-only state can be reached.

## Dialogs and menus

- The note "..." menu is a `DropdownMenu` on the ellipsis button, built from
  `PopupMenu` items: grouped sections with separators, icons, checkable
  toggles and submenus (Notebook, Opacity, Size). Do not replace it with a
  flat dialog again.
- A menu item that needs a dialog (Color…) must open it through
  `window.defer(cx, ...)` so the dialog survives the menu dismissing.
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
