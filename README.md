# zPad

Minimal sticky notes for Linux, in the spirit of
[xpad](https://github.com/CaledoniaProjects/xpad): a small yellow box, plain
text only, that saves itself while you type. No formatting, no projects, no
accounts — just scratch space that is always one keystroke away.

Written in Rust on GTK4, `#![forbid(unsafe_code)]`.

## What it does

- **Just a box for text.** Every note is a small yellow window with KDE's
  own native title bar — the first line of your text becomes the title. A
  slim bar at the bottom holds `+` (new note) and the trash (delete, with a
  confirmation — it removes the file).
- **Saved on the air.** Notes autosave one second after your last keystroke,
  plus on focus loss, on close, and on quit. Writes are atomic
  (temp file + fsync + rename), so a crash or power loss cannot corrupt an
  existing note; the worst case loses under a second of typing.
- **Lives in the tray.** Closing a note hides it; zPad keeps running in the
  system tray. Clicking the tray icon opens a menu listing every note by
  title (● open, ○ hidden) — pick one to surface just that note. The menu
  also has *Show all notes*, *New note*, and *Quit*.
- **Quick capture.** Running `zpad` again while it is open instantly creates
  a new empty note.
- **Keyboard:** `Ctrl+N` new note, `Ctrl+W` close the focused note.

## Install (Fedora KDE, user-local, no root)

```sh
sudo dnf install gtk4-devel   # build dependency only
./data/install.sh
```

This builds the release binary and installs to `~/.local`: the executable in
`~/.local/bin`, the app-menu entry in `~/.local/share/applications`, and the
icon in the hicolor theme. Make sure `~/.local/bin` is on your `PATH` (it is
by default on Fedora). Launch from the application menu or run `zpad`.

## Your notes are just files

One plain UTF-8 `.txt` per note, in
`~/.local/share/zpad/notes/` (honors `XDG_DATA_HOME`). Window sizes live in
`state.toml` next to them. Nothing else — no database, no lock-in. `cat`,
`grep`, `rsync`, and any text editor in 2050 will read them.

Because notes are ordinary files, sync and encryption come for free by
delegating to the tools that already do it well:

```sh
# keep notes in sync via Syncthing/Nextcloud:
ln -s /path/to/synced/folder ~/.local/share/zpad

# or encrypt at rest with gocryptfs/fscrypt on that folder
```

## Notes on Wayland (KDE Plasma)

- Apps cannot position their own windows or set always-on-top on Wayland —
  that is the compositor's decision, true for every toolkit. zPad remembers
  each note's **size**; placement is KWin's.
- Want notes always on top? Zero code needed — add a KWin rule:
  *System Settings → Window Management → Window Rules → New* → match window
  class `io.github.zpad.Zpad` → *Keep above: Force: Yes*.
- The tray icon works over D-Bus (StatusNotifier). If it is hidden:
  right-click the system tray → *Configure System Tray* → *Entries* →
  zPad → *Always visible*.
- X11 sessions work too (XWayland).

## Building from source

```sh
cargo build --release   # target/release/zpad
cargo test              # storage layer unit tests
cargo clippy --all-targets
```

Requirements: a Rust toolchain (rustup) and `gtk4-devel` from your distro.

## Memory safety

Rust ownership plus refcounted GTK objects mean nothing is freed twice and
nothing dangles; signal handlers hold weak references so closing a note frees
its whole widget tree. The binary is built with `panic = "abort"`, LTO, and
stripped symbols.

## Design boundaries (on purpose)

- **No rich text, ever** — formatting would force a file format richer than
  `.txt`, breaking the "readable forever" guarantee. That would be a
  different app.
- **No color picker** — notes are the classic light yellow. A per-note picker
  is a small, deliberate future change if wanted.
- **No settings UI** — there is nothing to configure; if that ever changes,
  it becomes a `config.toml`, not a dialog.
- **No built-in sync/encryption** — delegate (see above).

## License

MIT — see [LICENSE](LICENSE). The icon artwork (`data/zpad.svg`) is adopted
from xpad (GPL-3.0+), at the project owner's request.
