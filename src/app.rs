//! Application state: the set of open note windows, remembered window sizes,
//! and the actions shared by the tray, accelerators, and repeat launches.
//!
//! Owned as a single `Rc` for the lifetime of `main`; every GTK signal
//! handler keeps only a `Weak` back-reference so dropping a note window
//! frees it completely.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;

use crate::config::Config;
use crate::note::NoteWindow;
use crate::storage::Store;
use crate::tray::{SharedNotes, TrayNote, ZpadTray};

pub const DEFAULT_SIZE: (i32, i32) = (260, 220);

/// The font/colors stylesheet, rewritten whenever Preferences changes.
fn dynamic_css(config: &Config) -> String {
    let (text, background) = if config.use_custom_colors {
        (config.custom_text.clone(), config.custom_background.clone())
    } else {
        // Theme colors: let the system palette show through.
        ("inherit".to_string(), "transparent".to_string())
    };
    let mut css = format!(
        ".zpad-note textview, .zpad-note textview text {{ \
             background-color: {background}; color: {text}; caret-color: {text}; }}\n"
    );
    if config.use_custom_colors {
        css.push_str(&format!(
            ".zpad-note scrolledwindow {{ background-color: {background}; }}\n"
        ));
    }
    if !config.font_from_theme && !config.font_family.is_empty() {
        // Pango description ("Sans 9") → CSS family + pt size.
        let mut parts: Vec<&str> = config.font_family.split_whitespace().collect();
        let mut size = String::new();
        if let Some(last) = parts.last() {
            if last.chars().all(|c| c.is_ascii_digit()) {
                size = parts.pop().unwrap_or_default().to_string();
            }
        }
        let family = parts.join(" ");
        if !family.is_empty() {
            css.push_str(&format!(".zpad-note textview {{ font-family: \"{family}\";"));
            if !size.is_empty() {
                css.push_str(&format!(" font-size: {size}pt;"));
            }
            css.push_str(" }\n");
        }
    }
    css
}

fn autostart_dir() -> PathBuf {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() && PathBuf::from(&v).is_absolute() => PathBuf::from(v),
        _ => {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        }
    };
    base.join("autostart")
}

pub struct ZpadState {
    app: gtk::Application,
    store: Store,
    windows: RefCell<HashMap<u64, Rc<NoteWindow>>>,
    sizes: RefCell<HashMap<u64, (i32, i32)>>,
    tray: RefCell<Option<ksni::blocking::Handle<ZpadTray>>>,
    tray_menu: SharedNotes,
    about_window: RefCell<glib::WeakRef<gtk::AboutDialog>>,
    prefs_slot: crate::prefs::Slot,
    config: RefCell<Config>,
    dynamic_css: RefCell<Option<gtk::CssProvider>>,
    styles_installed: Cell<bool>,
    has_tray: Cell<bool>,
    started: Cell<bool>,
    /// Monotonic within the session. Seeded from disk (max existing id + 1)
    /// so an empty note that has never been saved can't collide with the
    /// next one — files only appear after the first autosave.
    next_id: Cell<u64>,
}

impl ZpadState {
    pub fn new(app: gtk::Application) -> Rc<Self> {
        let store = match Store::default_dir() {
            Ok(dir) => Store::new(dir),
            Err(err) => {
                eprintln!("zpad: {err}; falling back to a temporary directory");
                Store::new(std::env::temp_dir().join("zpad"))
            }
        };
        if let Err(err) = store.ensure_dirs() {
            eprintln!("zpad: cannot create data dir: {err}");
        }
        let sizes = store.load_sizes();
        let next_id = store.next_id();
        Rc::new(Self {
            app,
            store,
            windows: RefCell::new(HashMap::new()),
            sizes: RefCell::new(sizes),
            tray: RefCell::new(None),
            tray_menu: crate::tray::shared_notes(),
            about_window: RefCell::new(glib::WeakRef::new()),
            prefs_slot: crate::prefs::Slot::new(),
            config: RefCell::new(Config::load()),
            dynamic_css: RefCell::new(None),
            styles_installed: Cell::new(false),
            has_tray: Cell::new(false),
            started: Cell::new(false),
            next_id: Cell::new(next_id),
        })
    }

    pub fn app(&self) -> &gtk::Application {
        &self.app
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn set_tray(&self, handle: ksni::blocking::Handle<ZpadTray>) {
        self.has_tray.set(true);
        *self.tray.borrow_mut() = Some(handle);
    }

    /// A handle to the shared tray-menu snapshot, handed to the ksni thread
    /// at spawn time.
    pub fn tray_menu_clone(&self) -> SharedNotes {
        self.tray_menu.clone()
    }

    /// Refresh the tray menu's snapshot of the note list. The ksni thread
    /// reads this when the user opens the menu, so it is always current.
    pub fn sync_tray_menu(&self) {
        let mut snapshot = match self.tray_menu.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        snapshot.clear();
        for window in self.windows.borrow().values() {
            let (id, title, visible) = window.tray_entry();
            snapshot.push(TrayNote { id, title, visible });
        }
    }

    /// Like `sync_tray_menu`, plus a tray update so shells with a cached
    /// menu see the change immediately. Used on discrete events (new,
    /// delete, show, hide); typing only refreshes the snapshot.
    pub fn sync_tray_menu_now(&self) {
        self.sync_tray_menu();
        if let Some(handle) = self.tray.borrow().as_ref() {
            handle.update(|_| {});
        }
    }

    /// First activation honors the Startup preferences: restore every saved
    /// note, or start fresh with a single empty one — plus an optional
    /// additional blank note. Every later activation is quick capture.
    pub fn on_activated(self: &Rc<Self>) {
        if !self.started.replace(true) {
            let config = self.config();
            match config.display_pads.as_str() {
                "new" => {
                    self.new_note();
                }
                _ => match self.store.load_notes() {
                    Ok(notes) if !notes.is_empty() => {
                        let sizes = self.sizes.borrow().clone();
                        for note in notes {
                            let (width, height) =
                                sizes.get(&note.id).copied().unwrap_or(DEFAULT_SIZE);
                            self.open_note(note.id, note.text, width, height);
                        }
                    }
                    Ok(_) => {
                        self.new_note();
                    }
                    Err(err) => {
                        eprintln!("zpad: cannot read notes: {err}");
                        self.new_note();
                    }
                },
            }
            if config.open_new_pad_on_start {
                self.new_note();
            }
        } else {
            self.new_note();
        }
    }

    pub fn new_note(self: &Rc<Self>) -> Rc<NoteWindow> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let config = self.config();
        let (width, height) = (config.new_pad_width, config.new_pad_height);
        self.open_note(id, String::new(), width, height)
    }

    /// Present every open note; if none exist (all deleted), start one.
    /// This is the "Show all notes" path.
    pub fn show_all_notes(self: &Rc<Self>) {
        let all: Vec<Rc<NoteWindow>> = self.windows.borrow().values().cloned().collect();
        if all.is_empty() {
            self.new_note();
        } else {
            for window in all {
                window.present();
            }
        }
        self.sync_tray_menu_now();
    }

    /// Surface exactly one note, chosen from the tray menu.
    pub fn show_note(&self, id: u64) {
        if let Some(window) = self.windows.borrow().get(&id) {
            window.present();
        }
        self.sync_tray_menu_now();
    }

    /// Hide every note (saving each first). The app keeps running in the
    /// tray; nothing is deleted.
    pub fn hide_all_notes(self: &Rc<Self>) {
        let all: Vec<Rc<NoteWindow>> = self.windows.borrow().values().cloned().collect();
        for window in all {
            window.flush_save(self);
            window.remember_size_into(self);
            window.hide();
        }
        self.sync_tray_menu_now();
        self.check_idle_after_hide();
    }

    /// Small About window: what zPad is, who made it, when.
    pub fn show_about(&self) {        let slot = self.about_window.borrow_mut();
        let dialog = match slot.upgrade() {
            Some(dialog) => dialog,
            None => {
                let dialog = gtk::AboutDialog::new();
                dialog.set_program_name(Some("zPad"));
                dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
                dialog.set_comments(Some(
                    "Minimal sticky notes for Linux — plain text, saved on the air.",
                ));
                dialog.set_copyright(Some("© 2026 mosafer"));
                dialog.set_authors(&["mosafer"]);
                dialog.set_license_type(gtk::License::MitX11);
                dialog.set_logo_icon_name(Some("zpad"));
                dialog.set_modal(false);
                slot.set(Some(&dialog));
                dialog
            }
        };
        dialog.present();
    }

    /// The Preferences window: live-applied switches persisted to config.toml.
    pub fn show_preferences(self: &Rc<Self>) {
        crate::prefs::open(self, &self.prefs_slot);
    }

    /// Called once from `connect_startup`, when the display exists: installs
    /// the static stylesheet and the font/colors stylesheet that Preferences
    /// rewrites.
    pub fn install_styles(&self) {
        if self.styles_installed.replace(true) {
            return;
        }
        let dynamic = gtk::CssProvider::new();
        dynamic.load_from_string(&dynamic_css(&self.config()));
        let standard = gtk::CssProvider::new();
        standard.load_from_string(crate::STATIC_STYLE);
        if let Some(display) = gtk::gdk::Display::default() {
            // Dynamic first: on equal specificity the later provider wins, so
            // the static sheet (menu guards) takes precedence over it.
            gtk::style_context_add_provider_for_display(
                &display,
                &dynamic,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            gtk::style_context_add_provider_for_display(
                &display,
                &standard,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        *self.dynamic_css.borrow_mut() = Some(dynamic);
    }

    /// Current preferences snapshot.
    pub fn config(&self) -> Config {
        self.config.borrow().clone()
    }

    /// Mutate, persist, and live-apply the preferences: window flags, the
    /// font/colors stylesheet, and the autostart entry.
    pub fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let config = {
            let mut config = self.config.borrow_mut();
            f(&mut config);
            if let Err(err) = config.save() {
                eprintln!("zpad: saving config: {err}");
            }
            config.clone()
        };
        if let Some(provider) = self.dynamic_css.borrow().as_ref() {
            provider.load_from_string(&dynamic_css(&config));
        }
        for window in self.windows.borrow().values() {
            window.apply_settings(&config);
        }
        self.apply_autostart(&config);
    }

    /// Create or remove the XDG autostart entry for "start after login".
    fn apply_autostart(&self, config: &Config) {
        let dir = autostart_dir();
        let path = dir.join("zpad.desktop");
        if !config.autostart {
            let _ = std::fs::remove_file(path);
            return;
        }
        if let Err(err) = std::fs::create_dir_all(&dir) {
            eprintln!("zpad: cannot create autostart dir: {err}");
            return;
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("zpad"));
        let entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=zPad\n\
             Icon=zpad\n\
             Exec={}\n\
             Terminal=false\n\
             NoDisplay=true\n\
             X-GNOME-Autostart-delay={}\n\
             X-KDE-autostart-delay={}\n",
            exe.display(),
            config.startup_delay_secs,
            config.startup_delay_secs
        );
        if let Err(err) = std::fs::write(&path, entry) {
            eprintln!("zpad: cannot write autostart entry: {err}");
        }
    }

    /// Tray left-click: "toggle" hides everything if a note is visible and
    /// surfaces all notes otherwise; "show-all" always surfaces.
    pub fn tray_activate(self: &Rc<Self>) {
        match self.config().tray_click.as_str() {
            "show-all" => self.show_all_notes(),
            _ => {
                let any_visible = self
                    .windows
                    .borrow()
                    .values()
                    .any(|window| window.is_visible());
                if any_visible {
                    self.hide_all_notes();
                } else {
                    self.show_all_notes();
                }
            }
        }
    }

    /// Ctrl+J: flip read-only mode for every note.
    pub fn toggle_read_only(&self) {
        self.update_config(|config| config.read_only = !config.read_only);
    }

    pub fn delete_note(&self, id: u64) {
        let window = self.windows.borrow_mut().remove(&id);
        if let Some(window) = &window {
            window.cancel_save();
        }
        if let Err(err) = self.store.delete_note(id) {
            eprintln!("zpad: deleting note {id}: {err}");
        }
        if self.sizes.borrow_mut().remove(&id).is_some() {
            self.persist_sizes();
        }
        if let Some(window) = window {
            window.destroy();
        }
        self.sync_tray_menu_now();
    }

    pub fn remember_size(&self, id: u64, size: (i32, i32)) {
        let mut sizes = self.sizes.borrow_mut();
        if sizes.get(&id) != Some(&size) {
            sizes.insert(id, size);
            drop(sizes);
            self.persist_sizes();
        }
    }

    /// Save now — called by note windows on the debounced timer, on focus
    /// loss, on close, and on quit. Also refreshes the tray menu snapshot,
    /// since titles (first lines) may have changed.
    pub fn flush_window(&self, window: &NoteWindow) {
        window.flush_save(self);
        self.sync_tray_menu();
    }

    /// Closing a note normally just hides it while the app lives in the
    /// tray. Without a tray there is no way back, so once every note is
    /// hidden, quit cleanly instead of becoming an invisible zombie.
    pub fn check_idle_after_hide(&self) {
        if self.has_tray.get() {
            return;
        }
        let any_visible = self
            .windows
            .borrow()
            .values()
            .any(|window| window.is_visible());
        if !any_visible {
            self.quit();
        }
    }

    pub fn quit(&self) {
        for window in self.windows.borrow().values() {
            self.flush_window(window);
        }
        self.persist_sizes();
        if let Some(handle) = self.tray.borrow_mut().take() {
            handle.shutdown().wait();
        }
        self.app.quit();
    }

    fn open_note(self: &Rc<Self>, id: u64, text: String, width: i32, height: i32) -> Rc<NoteWindow> {
        let window = NoteWindow::new(self, id, text, width, height);
        self.windows.borrow_mut().insert(id, window.clone());
        window.present();
        self.sync_tray_menu_now();
        window
    }

    fn persist_sizes(&self) {
        if let Err(err) = self.store.save_sizes(&self.sizes.borrow()) {
            eprintln!("zpad: saving window sizes: {err}");
        }
    }
}
