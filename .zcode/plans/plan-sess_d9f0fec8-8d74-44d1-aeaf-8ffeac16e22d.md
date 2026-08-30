# zPad — minimal sticky notes for Linux (Rust + GTK4)

## Goal
A tiny, xpad-spirited sticky note app: plain text only, saves instantly, zero fuss, no memory leaks, ready-to-use on KDE Fedora (Wayland-first).

## Decisions
- **Language/Toolkit:** Rust 1.95 + gtk4-rs (GTK4, no libadwaita — uses the KDE system theme, minimal deps).
- **Crates:** `gtk4`, `ksni` (pure-D-Bus tray, no toolkit dep), `serde` + `toml` (state file only). No async runtime, no network. Crate gets `#![forbid(unsafe_code)]`.

## One-time system prep
`sudo dnf install gtk4-devel` (pulls glib/pango/cairo devel deps; fallback: also `glib2-devel pango-devel cairo-devel gdk-pixbuf2-devel`).

## Project layout
```
Cargo.toml
src/main.rs      # entry, single-instance activate logic, --new flag
src/app.rs       # app state, note lifecycle, debounced autosave, tray wiring
src/note.rs      # note window: slim header [+ new | trash] + plain GtkTextView
src/tray.rs      # StatusNotifier item: click = show notes, menu = New note / Quit
src/storage.rs   # load/save, atomic writes, state file
data/zpad.desktop, data/zpad.svg, data/install.sh
README.md, LICENSE (MIT)
```

## Behavior (v1)
- Each note = a small window: first line of text becomes the window title; body is a plain-text view (word wrap, zero formatting). **All notes have a fixed light-yellow background** (#FFF9C4-style pastel, applied once via CSS provider) — classic sticky look, no picker, no settings.
- Header bar: `+` (new note) and trash (delete, with confirmation — it removes the file).
- **Autosave "on the air":** debounced save 1s after last keystroke, plus on focus-out, close, and quit. Atomic writes (temp + rename) — crash-safe, worst case loses <1s of typing.
- Launch restores all previously open notes (or creates one if none). Running `zpad` again = instant new note (quick capture).
- **Tray icon (Plasma system tray):** left-click/Activate presents all note windows (or spawns one if none); right-click menu: New note / Quit. Works over D-Bus on Wayland. README notes how to force it visible: System Tray Settings → Entries → zPad → Always visible.
- Shortcuts: Ctrl+N new note, Ctrl+W close (autosaves).

## Storage (no lock-in)
- `~/.local/share/zpad/notes/<id>.txt` — ordinary UTF-8 files (grep/edit/backup freely).
- `~/.local/share/zpad/state.toml` — open-session list + per-note window size; corrupt state rebuilds from the .txt files.
- Sync/encryption delegation recipe in README: symlink or point the folder at Syncthing/Nextcloud/gocryptfs — plain files make this work with zero code.

## Wayland honesty (README)
KWin/Wayland doesn't let apps self-position or set always-on-top (true for all toolkits). v1 remembers sizes, not positions. README includes the zero-code KWin Window Rule recipe for "keep above". X11/XWayland also works.

## Leak & safety verification
- RAII + refcounted GTK objects owned by the widget tree; `#![forbid(unsafe_code)]`.
- `cargo test` (storage layer, temp-dir based), `cargo clippy` clean.
- Smoke test on this KDE Wayland session: open/close 100 notes in a loop → RSS stays flat; valgrind pass with GLib's expected one-time allocations noted; tray thread shuts down cleanly on quit.

## Milestones
1. Scaffold + `cargo build` (after gtk4-devel) — proves toolchain.
2. `storage.rs` + unit tests (atomic save/load, state rebuild).
3. Note window: yellow styling, autosave, header buttons.
4. Multi-note, session restore, repeat-invocation new note, delete confirm.
5. Tray icon (ksni): activate + menu.
6. Packaging: `.desktop`, yellow sticky SVG icon, `install.sh` (binary → `~/.local/bin`, menu entry + icon), README.
7. Verification: tests, clippy, release build, leak smoke test, final deliverable.

## Out of scope (v1, on purpose) — with reasons
- **Rich text:** permanently out — it would force a format richer than `.txt`, breaking the "readable forever with `cat`" guarantee. That would be a different app.
- **Color picker / skins:** deferred — v1 ships the fixed light-yellow sticky look instead; a per-note picker is ~50 lines if ever wanted.
- **Sync/encryption:** deferred — delegation covers it (see Storage); no sync code to leak or maintain.
- **Settings UI:** nothing to configure by design; if settings ever appear they'd be a `config.toml`, not a GUI.
- **RPM spec / COPR:** future — proper `dnf install zpad` packaging, worth doing once v1 is proven.
- **Flathub manifest:** future — reaches all distros but adds sandbox permissions for the notes dir and build infra; RPM path is lighter for Fedora.
- **Always-on-top via KWin scripting:** future — fragile, KDE-only; the zero-code KWin Window Rule already covers it today.

## Deliverable
Release binary `zpad` + `data/install.sh` + README. After `install.sh`: app-menu entry, tray icon, plain-text notes that save themselves.