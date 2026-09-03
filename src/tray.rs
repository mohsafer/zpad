//! System tray item (StatusNotifier) so zPad can live in the Plasma panel.
//!
//! Clicking the icon opens a menu listing every note by title; picking one
//! surfaces just that note. The menu also offers *Show all notes*,
//! *New note*, and *Quit*.
//!
//! Runs on its own ksni thread. GTK objects are main-thread-only, so the
//! tray holds a `SendWeakRef` to the application (for actions, handed to the
//! main loop via `MainContext::invoke`) and a shared snapshot of the note
//! list that the GTK side rewrites whenever notes change.

use std::sync::{Arc, Mutex};

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

/// One note as seen by the tray menu.
#[derive(Clone, Debug)]
pub struct TrayNote {
    pub id: u64,
    pub title: String,
    pub visible: bool,
    /// File modification time, used to sort newest-first in the menu.
    pub mtime: u64,
}

/// Shared between the GTK main thread (writer) and the ksni thread (reader
/// when the menu is opened). Rewritten wholesale on every note change.
pub type SharedNotes = Arc<Mutex<Vec<TrayNote>>>;

pub fn shared_notes() -> SharedNotes {
    Arc::new(Mutex::new(Vec::new()))
}

pub struct ZpadTray {
    ctx: glib::MainContext,
    app: glib::SendWeakRef<gtk::Application>,
    notes: SharedNotes,
}

impl ZpadTray {
    pub fn new(app: &gtk::Application, notes: SharedNotes) -> Self {
        ZpadTray {
            ctx: glib::MainContext::default(),
            app: glib::SendWeakRef::from(app.downgrade()),
            notes,
        }
    }

    fn run_action(&self, action: &'static str) {
        let app = self.app.clone();
        self.ctx.invoke(move || {
            if let Some(app) = app.upgrade() {
                app.activate_action(action, None);
            }
        });
    }

    fn run_action_with_id(&self, action: &'static str, id: u64) {
        let app = self.app.clone();
        self.ctx.invoke(move || {
            if let Some(app) = app.upgrade() {
                app.activate_action(action, Some(&id.to_variant()));
            }
        });
    }
}

impl ksni::Tray for ZpadTray {
    // Left click is handled by the app's `tray-activate` action, which
    // follows the "Tray left mouse click behavior" preference (toggle
    // show-all, or always show). Right-click still opens this menu.
    const MENU_ON_ACTIVATE: bool = false;

    fn id(&self) -> String {
        "zpad".into()
    }

    fn title(&self) -> String {
        "zPad".into()
    }

    fn icon_name(&self) -> String {
        // Resolved from the theme once `install.sh` has registered zpad.svg.
        "zpad".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        // Procedural fallback so the tray is never blank before installation.
        vec![sticky_icon(48)]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: "zpad".into(),
            icon_pixmap: vec![],
            title: "zPad".into(),
            description: "Quick plain-text sticky notes".into(),
        }
    }

    // Left click follows the tray preference via `tray-activate`.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.run_action("tray-activate");
    }

    fn menu(&self) -> Vec<ksni::menu::MenuItem<Self>> {
        use ksni::menu::{MenuItem, StandardItem, SubMenu};

        let mut items = vec![StandardItem {
            label: "New note".into(),
            activate: Box::new(|tray: &mut Self| tray.run_action("new-note")),
            ..Default::default()
        }
        .into()];

        let mut snapshot: Vec<TrayNote> = match self.notes.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        // Newest first: the notes the user touched most recently are the
        // ones they want to reach without digging.
        snapshot.sort_by_key(|note| std::cmp::Reverse(note.mtime));

        // The flat list stays short — at most MAX_FLAT entries; everything
        // older folds into an "Older notes" submenu.
        const MAX_FLAT: usize = 10;
        if !snapshot.is_empty() {
            items.push(MenuItem::Separator);
            for note in snapshot.iter().take(MAX_FLAT) {
                items.push(Self::note_item(note.clone()));
            }
            if snapshot.len() > MAX_FLAT {
                let older: Vec<MenuItem<Self>> = snapshot
                    .iter()
                    .skip(MAX_FLAT)
                    .cloned()
                    .map(Self::note_item)
                    .collect();
                items.push(
                    SubMenu {
                        label: format!("Older notes ({})", snapshot.len() - MAX_FLAT),
                        submenu: older,
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(MenuItem::Separator);
        }

        items.push(
            StandardItem {
                label: "Show all".into(),
                activate: Box::new(|tray: &mut Self| tray.run_action("show-notes")),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Close all".into(),
                activate: Box::new(|tray: &mut Self| tray.run_action("hide-notes")),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Help".into(),
                activate: Box::new(|tray: &mut Self| tray.run_action("help")),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Preferences".into(),
                activate: Box::new(|tray: &mut Self| tray.run_action("preferences")),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut Self| tray.run_action("quit")),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

impl ZpadTray {
    /// One clickable entry that surfaces just that note. Takes ownership of
    /// a clone because the activate closure must live independently of the
    /// caller's snapshot.
    fn note_item(note: TrayNote) -> ksni::menu::MenuItem<Self> {
        use ksni::menu::StandardItem;
        let marker = if note.visible { "● " } else { "○ " };
        StandardItem {
            label: format!("{}{}", marker, note.title.replace('_', "__")),
            activate: Box::new(move |tray: &mut Self| {
                tray.run_action_with_id("show-note", note.id)
            }),
            ..Default::default()
        }
        .into()
    }
}

pub fn spawn(
    app: &gtk::Application,
    notes: SharedNotes,
    wait_systray: bool,
) -> Result<ksni::blocking::Handle<ZpadTray>, ksni::Error> {
    use ksni::blocking::TrayMethods;
    if wait_systray {
        // "Wait for systray (if possible)": tolerate starting before the
        // desktop shell has published its StatusNotifierWatcher.
        ZpadTray::new(app, notes).assume_sni_available(true).spawn()
    } else {
        ZpadTray::new(app, notes).spawn()
    }
}

/// A small sticky note drawn procedurally as ARGB32 in network byte order
/// (alpha first, then R, G, B), matching the SNI pixmap format: pale yellow
/// paper, ruled lines, and a cut top-right corner with a curled flap —
/// readable even at Plasma's ~22 px tray size.
fn sticky_icon(size: i32) -> ksni::Icon {
    let s = size as usize;
    let edge = (s / 24).max(2);
    let fold = s / 4;
    let flap = (s / 16).max(3);
    let line_th = (s / 16).max(2);
    let l1 = s * 3 / 8;
    let l2 = s * 2 / 4;
    let l3 = s * 5 / 8;
    let line_x0 = s / 6;
    let line_x1 = s - s / 6;
    let line_x1_short = s / 2;

    const PAPER: [u8; 4] = [255, 255, 249, 196];
    const OUTLINE: [u8; 4] = [255, 214, 200, 110];
    const FLAP: [u8; 4] = [255, 255, 232, 111];
    const LINE: [u8; 4] = [255, 227, 217, 141];

    let mut data = Vec::with_capacity(s * s * 4);
    for y in 0..s {
        for x in 0..s {
            let dx = s - 1 - x; // distance from the right edge
            let dy = y; // distance from the top edge
            let (px, on_line) = if dx + dy < fold - 1 {
                ([0, 0, 0, 0], false) // corner cut away — transparent
            } else if dx + dy < fold - 1 + flap {
                (FLAP, false) // curled flap along the cut
            } else if x < edge || y < edge || x >= s - edge || y >= s - edge {
                (OUTLINE, false)
            } else {
                (PAPER, true)
            };
            let in_line = on_line
                && ((y >= l1 && y < l1 + line_th)
                    || (y >= l2 && y < l2 + line_th)
                    || (y >= l3 && y < l3 + line_th && x < line_x1_short))
                && x >= line_x0
                && x < line_x1;
            let px = if in_line { LINE } else { px };
            data.extend_from_slice(&px);
        }
    }

    ksni::Icon {
        width: size,
        height: size,
        data,
    }
}
