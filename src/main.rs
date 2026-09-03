//! zPad — minimal sticky notes for Linux.
//!
//! Plain text only, saved on the air: one second after the last keystroke,
//! on focus loss, on close, on quit. Lives in the tray; every later launch
//! is quick capture.

#![forbid(unsafe_code)]

mod app;
mod config;
mod note;
mod prefs;
mod storage;
mod tray;

use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use app::ZpadState;

const APP_ID: &str = "io.github.zpad.Zpad";

/// Structural styles only: the note colors live in the dynamic stylesheet
/// (see `app::dynamic_css`) so Preferences can rewrite them. The dark
/// popover guards outrank the dynamic sheet by specificity, so the menu
/// stays Breeze-dark no matter what colors the notes use.
pub(crate) const STATIC_STYLE: &str = "
.zpad-note .note-bar {
    background-color: rgba(246, 239, 168, 0.94);
    color: #4a4400;
}
.zpad-note .note-bar button {
    min-width: 18px;
    min-height: 16px;
    padding: 0 3px;
    margin: 1px;
    color: #4a4400;
}
.zpad-note .note-bar button.danger {
    color: #d13b3b;
}
.zpad-note .note-bar separator.tool-sep {
    min-width: 1px;
    margin-top: 4px;
    margin-bottom: 4px;
    background-color: rgba(74, 68, 0, 0.35);
}
.zpad-note .find-bar {
    background-color: rgba(246, 239, 168, 0.97);
    border-radius: 6px;
}
popover.background.menu,
popover.background.menu > contents,
popover.background.menu scrolledwindow,
popover.background.menu viewport,
popover.background.menu stack,
popover.background > contents,
popover > contents,
popover scrolledwindow,
popover viewport,
popover stack {
    background-color: #202326;
    color: #eeeeec;
}
popover modelbutton,
popover.background.menu modelbutton {
    background-color: transparent;
    color: #eeeeec;
}
";

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();

    let state = ZpadState::new(app.clone());

    let weak_for_styles = Rc::downgrade(&state);
    app.connect_startup(move |_| {
        if let Some(state) = weak_for_styles.upgrade() {
            state.install_styles();
        }
    });

    // Logout and `systemctl --user stop zpad` deliver SIGTERM, which would
    // otherwise kill the process mid-debounce and lose the last second of
    // typing. Route it through the same path as Quit (flush + clean exit).
    // SIGINT (Ctrl+C) is covered by GTK's own handling; re-raising via the
    // default handler keeps `quit()` from running twice.
    {
        let weak_state = Rc::downgrade(&state);
        app.connect_startup(move |_| {
            let weak_state = weak_state.clone();
            glib::unix_signal_add_local_once(libc::SIGTERM, move || {
                if let Some(state) = weak_state.upgrade() {
                    state.quit();
                }
            });
        });
    }

    // activate fires on the primary instance for the very first launch AND
    // for every repeat invocation; ZpadState turns repeats into quick capture.
    {
        let weak_state = Rc::downgrade(&state);
        app.connect_activate(move |_| {
            if let Some(state) = weak_state.upgrade() {
                state.on_activated();
            }
        });
    }

    let new_note = gio::SimpleAction::new("new-note", None);
    {
        let weak_state = Rc::downgrade(&state);
        new_note.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.new_note();
            }
        });
    }
    app.add_action(&new_note);

    let show_notes = gio::SimpleAction::new("show-notes", None);
    {
        let weak_state = Rc::downgrade(&state);
        show_notes.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.show_all_notes();
            }
        });
    }
    app.add_action(&show_notes);

    let hide_notes = gio::SimpleAction::new("hide-notes", None);
    {
        let weak_state = Rc::downgrade(&state);
        hide_notes.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.hide_all_notes();
            }
        });
    }
    app.add_action(&hide_notes);

    // Parameterized: the tray menu's per-note entries pick one note by id.
    let show_note = gio::SimpleAction::new("show-note", Some(u64::static_variant_type().as_ref()));
    {
        let weak_state = Rc::downgrade(&state);
        show_note.connect_activate(move |_, parameter| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            if let Some(id) = parameter.and_then(|v| v.get::<u64>()) {
                state.show_note(id);
            }
        });
    }
    app.add_action(&show_note);

    let help = gio::SimpleAction::new("help", None);
    {
        let weak_state = Rc::downgrade(&state);
        help.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.show_about();
            }
        });
    }
    app.add_action(&help);

    let preferences = gio::SimpleAction::new("preferences", None);
    {
        let weak_state = Rc::downgrade(&state);
        preferences.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.show_preferences();
            }
        });
    }
    app.add_action(&preferences);

    let quit_action = gio::SimpleAction::new("quit", None);
    {
        let weak_state = Rc::downgrade(&state);
        quit_action.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.quit();
            }
        });
    }
    app.add_action(&quit_action);

    // Tray left-click, routed through config: "toggle" or "show-all".
    let tray_activate = gio::SimpleAction::new("tray-activate", None);
    {
        let weak_state = Rc::downgrade(&state);
        tray_activate.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.tray_activate();
            }
        });
    }
    app.add_action(&tray_activate);

    let toggle_read_only = gio::SimpleAction::new("toggle-read-only", None);
    {
        let weak_state = Rc::downgrade(&state);
        toggle_read_only.connect_activate(move |_, _| {
            if let Some(state) = weak_state.upgrade() {
                state.toggle_read_only();
            }
        });
    }
    app.add_action(&toggle_read_only);

    app.set_accels_for_action("app.new-note", &["<Primary>n"]);
    app.set_accels_for_action("win.close", &["<Primary>w"]);
    app.set_accels_for_action("app.toggle-read-only", &["<Primary>j"]);
    app.set_accels_for_action("app.preferences", &["<Primary>comma"]);
    app.set_accels_for_action("win.find", &["<Primary>f"]);

    if state.config().tray_enabled {
        match tray::spawn(&app, state.tray_menu_clone(), state.config().wait_systray) {
            Ok(handle) => state.set_tray(handle),
            Err(err) => {
                eprintln!("zpad: tray unavailable ({err}); closing the last note quits");
            }
        }
    } else {
        eprintln!("zpad: tray icon disabled in Preferences; closing the last note quits");
    }

    app.run()
}
