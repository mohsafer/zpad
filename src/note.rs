//! One sticky note window: a slim header (`+` new, trash delete) over a
//! plain-text view. Light yellow, no formatting, autosaves one second after
//! the last keystroke.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::app::ZpadState;
use crate::config::Config;

const AUTOSAVE_DELAY_MS: u64 = 1000;
const TITLE_MAX_CHARS: usize = 40;

pub struct NoteWindow {
    id: u64,
    window: gtk::ApplicationWindow,
    buffer: gtk::TextBuffer,
    view: gtk::TextView,
    scrolled: gtk::ScrolledWindow,
    revealer: gtk::Revealer,
    find_revealer: gtk::Revealer,
    search_entry: gtk::SearchEntry,
    autohide: Cell<bool>,
    /// Poll timer for the bar-zone reveal (autohide mode only).
    hover_poll: RefCell<Option<glib::SourceId>>,
    /// Sticky hysteresis: once revealed via the strip, the bar stays up
    /// while the pointer remains anywhere on it, even mid-animation when
    /// the strip is briefly covered.
    pointer_in_bar: Cell<bool>,
    /// True once the user has typed anything in this note (even if they
    /// then deleted it — clearing an existing note must persist the clear).
    ever_dirty: Cell<bool>,
    pending_save: RefCell<Option<glib::SourceId>>,
}

impl NoteWindow {
    /// Builds the widget tree and wires all signals. Returns the window as
    /// an `Rc` because every signal handler keeps only a `Weak` back-reference;
    /// the caller (the windows map in `ZpadState`) holds the strong count.
    pub fn new(state: &Rc<ZpadState>, id: u64, text: String, width: i32, height: i32) -> Rc<Self> {
        let cfg = state.config();
        let window = gtk::ApplicationWindow::builder()
            .application(state.app())
            .title("New note")
            .build();
        window.set_default_size(width, height);
        window.add_css_class("zpad-note");
        // System decorations: KDE's own title bar, not a custom one.

        let tool_button = |icon: &str, tooltip: &str| {
            let button = gtk::Button::from_icon_name(icon);
            button.add_css_class("flat");
            button.set_has_frame(false);
            button.set_tooltip_text(Some(tooltip));
            button
        };
        let new_button = tool_button("list-add-symbolic", "New note (Ctrl+N)");
        let cut_button = tool_button("edit-cut-symbolic", "Cut (Ctrl+X)");
        let copy_button = tool_button("edit-copy-symbolic", "Copy (Ctrl+C)");
        let paste_button = tool_button("edit-paste-symbolic", "Paste (Ctrl+V)");
        let undo_button = tool_button("edit-undo-symbolic", "Undo (Ctrl+Z)");
        let redo_button = tool_button("edit-redo-symbolic", "Redo (Ctrl+Shift+Z)");
        let find_button = tool_button("system-search-symbolic", "Find (Ctrl+F)");
        let delete_button = tool_button("user-trash-symbolic", "Delete note");
        delete_button.add_css_class("danger");

        let tool_sep = || {
            let separator = gtk::Separator::new(gtk::Orientation::Vertical);
            separator.add_css_class("tool-sep");
            separator
        };

        let view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::Word)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Thin tool bar floating over the bottom edge; hidden until the
        // pointer enters the window, slid away again when it leaves.
        let bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(1)
            .build();
        bar.add_css_class("note-bar");
        bar.set_hexpand(true);
        bar.append(&new_button);
        bar.append(&cut_button);
        bar.append(&copy_button);
        bar.append(&paste_button);
        bar.append(&tool_sep());
        bar.append(&undo_button);
        bar.append(&redo_button);
        bar.append(&tool_sep());
        bar.append(&find_button);
        let bar_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar_spacer.set_hexpand(true);
        bar.append(&bar_spacer);
        bar.append(&delete_button);

        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideUp);
        revealer.set_transition_duration(150);
        revealer.set_child(Some(&bar));
        revealer.set_valign(gtk::Align::End);
        revealer.set_halign(gtk::Align::Fill);

        // Find bar, floating over the top edge.
        let search_entry = gtk::SearchEntry::new();
        let search_prev = tool_button("go-up-symbolic", "Previous match (Enter)");
        let search_next = tool_button("go-down-symbolic", "Next match (Enter)");
        let search_close = tool_button("window-close-symbolic", "Close (Esc)");
        let find_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(2)
            .margin_top(3)
            .margin_bottom(3)
            .margin_start(4)
            .margin_end(4)
            .build();
        find_box.add_css_class("find-bar");
        find_box.append(&search_entry);
        find_box.append(&search_prev);
        find_box.append(&search_next);
        find_box.append(&search_close);
        let find_revealer = gtk::Revealer::new();
        find_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        find_revealer.set_transition_duration(150);
        find_revealer.set_child(Some(&find_box));
        find_revealer.set_valign(gtk::Align::Start);
        find_revealer.set_halign(gtk::Align::Fill);

        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scrolled));
        overlay.add_overlay(&find_revealer);
        overlay.set_measure_overlay(&find_revealer, false);
        overlay.add_overlay(&revealer);
        overlay.set_measure_overlay(&revealer, false);
        window.set_child(Some(&overlay));

        let autohide = Cell::new(cfg.autohide_toolbar);
        revealer.set_visible(cfg.show_toolbar);
        revealer.set_reveal_child(cfg.show_toolbar && !cfg.autohide_toolbar);
        window.set_decorated(cfg.show_decorations);
        Self::apply_scroll_policy(&scrolled, cfg.show_scrollbar);

        let buffer = view.buffer();
        let this = Rc::new(NoteWindow {
            id,
            window,
            buffer,
            view,
            scrolled,
            revealer,
            find_revealer,
            search_entry,
            autohide,
            hover_poll: RefCell::new(None),
            pointer_in_bar: Cell::new(false),
            ever_dirty: Cell::new(false),
            pending_save: RefCell::new(None),
        });

        // Bar reveal: while autohide is on, poll the pointer every 150 ms.
        // The bar exists only while the pointer sits in the bottom strip
        // (or on the bar itself); hovering the note body hides it. Polling
        // is deliberate — enter/leave events misbehave around the overlay
        // animation, and 6 wake-ups/second per open note is negligible.
        {
            let weak_this = Rc::downgrade(&this);
            let source =
                glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                    let Some(this) = weak_this.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    if !this.autohide.get() || !this.revealer.is_visible() {
                        return glib::ControlFlow::Continue;
                    }
                    let (_px, py) = match this.window.surface() {
                        Some(surface) => {
                            let seat = gtk::prelude::RootExt::display(&this.window).default_seat();
                            match seat
                                .and_then(|seat| surface.device_position(seat.pointer().as_ref()?))
                            {
                                Some((x, y, _)) => (x, y),
                                None => return glib::ControlFlow::Continue,
                            }
                        }
                        None => return glib::ControlFlow::Continue,
                    };
                    let bar_height = this.revealer.height() as f64;
                    let zone_height = 10.0_f64;
                    let inside = py >= this.window.height() as f64 - zone_height
                        || (this.revealer.reveals_child()
                            && py >= this.window.height() as f64 - bar_height);
                    if inside != this.pointer_in_bar.get() {
                        this.pointer_in_bar.set(inside);
                        this.revealer.set_reveal_child(inside);
                    }
                    glib::ControlFlow::Continue
                });
            *this.hover_poll.borrow_mut() = Some(source);
        }

        // Tool bar actions — all operate on this note's own buffer. Cut,
        // paste, undo, and redo mutate the buffer directly, so unlike
        // Ctrl+X/Ctrl+V (which respect the TextView's editable flag) they
        // must honor read-only mode themselves.
        {
            let weak_this = Rc::downgrade(&this);
            cut_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    if !this.view.is_editable() {
                        return;
                    }
                    let clipboard = this.view.display().clipboard();
                    this.buffer.cut_clipboard(&clipboard, true);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            copy_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    let clipboard = this.view.display().clipboard();
                    this.buffer.copy_clipboard(&clipboard);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            paste_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    if !this.view.is_editable() {
                        return;
                    }
                    let clipboard = this.view.display().clipboard();
                    this.buffer
                        .paste_clipboard(&clipboard, None::<&gtk::TextIter>, true);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            undo_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    if !this.view.is_editable() {
                        return;
                    }
                    this.buffer.undo();
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            redo_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    if !this.view.is_editable() {
                        return;
                    }
                    this.buffer.redo();
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            find_button.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.toggle_find();
                }
            });
        }

        // Find bar wiring.
        {
            let weak_this = Rc::downgrade(&this);
            this.search_entry.connect_search_changed(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.do_search(false);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            this.search_entry.connect_activate(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.do_search(false);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            this.search_entry.connect_stop_search(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.close_find();
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            search_prev.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.do_search(true);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            search_next.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.do_search(false);
                }
            });
        }
        {
            let weak_this = Rc::downgrade(&this);
            search_close.connect_clicked(move |_| {
                if let Some(this) = weak_this.upgrade() {
                    this.close_find();
                }
            });
        }

        // The initial text is set before signals are connected (below), so
        // loading a note never triggers a redundant save.
        this.buffer.set_text(&text);
        this.update_title();

        Self::connect(&this, state, &new_button, &delete_button);
        this
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn present(&self) {
        // present() alone maps the window asynchronously; set the flag now
        // so visibility checks (tray menu) are correct in the same tick.
        self.window.set_visible(true);
        self.window.present();
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    /// Record the window's current size in the app state (used on close and
    /// on "Close all").
    pub fn remember_size_into(&self, state: &ZpadState) {
        state.remember_size(self.id, (self.window.width(), self.window.height()));
    }

    fn toggle_find(&self) {
        let show = !self.find_revealer.reveals_child();
        self.find_revealer.set_reveal_child(show);
        if show {
            self.search_entry.grab_focus();
        }
    }

    fn close_find(&self) {
        self.find_revealer.set_reveal_child(false);
        self.view.grab_focus();
    }

    /// Search for the find-bar text, selecting the match and scrolling to
    /// it; wraps around at the buffer edges.
    fn do_search(&self, backwards: bool) {
        let query = self.search_entry.text().to_string();
        if query.is_empty() {
            return;
        }
        let flags = gtk::TextSearchFlags::CASE_INSENSITIVE | gtk::TextSearchFlags::TEXT_ONLY;
        let (search_from, wrap_from) = if backwards {
            match self.buffer.selection_bounds() {
                Some((start, _)) => (start, self.buffer.end_iter()),
                None => (self.buffer.end_iter(), self.buffer.end_iter()),
            }
        } else {
            match self.buffer.selection_bounds() {
                Some((_, end)) => (end, self.buffer.start_iter()),
                None => (self.buffer.start_iter(), self.buffer.start_iter()),
            }
        };
        let primary = if backwards {
            search_from.backward_search(&query, flags, None)
        } else {
            search_from.forward_search(&query, flags, None)
        };
        let found = primary.or_else(|| {
            if backwards {
                wrap_from.backward_search(&query, flags, None)
            } else {
                wrap_from.forward_search(&query, flags, None)
            }
        });
        if let Some((mut start, end)) = found {
            self.buffer.select_range(&start, &end);
            self.view.scroll_to_iter(&mut start, 0.0, true, 0.0, 0.5);
        }
    }

    pub fn cancel_save(&self) {
        if let Some(source) = self.pending_save.borrow_mut().take() {
            source.remove();
        }
    }

    /// Save now, cancelling any pending debounced save. A note that has
    /// never been typed in and is still empty leaves no file behind —
    /// accidental Ctrl+N/Ctrl+W must not litter the notes folder.
    pub fn flush_save(&self, state: &ZpadState) {
        self.cancel_save();
        let text = self
            .buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false);
        if !self.ever_dirty.get() && text.is_empty() {
            return;
        }
        if let Err(err) = state.store().save_note(self.id, &text) {
            eprintln!("zpad: saving note {}: {err}", self.id);
        }
    }

    pub fn destroy(&self) {
        self.cancel_save();
        self.window.destroy();
    }

    fn apply_scroll_policy(scrolled: &gtk::ScrolledWindow, show: bool) {
        scrolled.set_policy(
            gtk::PolicyType::Never,
            if show {
                gtk::PolicyType::Automatic
            } else {
                gtk::PolicyType::Never
            },
        );
    }

    /// Live-apply preferences to this note (called from `update_config`).
    pub fn apply_settings(&self, cfg: &Config) {
        self.autohide.set(cfg.autohide_toolbar);
        self.revealer.set_visible(cfg.show_toolbar);
        self.revealer
            .set_reveal_child(cfg.show_toolbar && !cfg.autohide_toolbar);
        self.window.set_decorated(cfg.show_decorations);
        self.view.set_editable(!cfg.read_only);
        Self::apply_scroll_policy(&self.scrolled, cfg.show_scrollbar);
    }

    fn schedule_save(self: &Rc<Self>, state: &Rc<ZpadState>) {
        let mut pending = self.pending_save.borrow_mut();
        if let Some(source) = pending.take() {
            source.remove();
        }
        let weak_this = Rc::downgrade(self);
        let weak_state = Rc::downgrade(state);
        let source = glib::timeout_add_local(
            std::time::Duration::from_millis(AUTOSAVE_DELAY_MS),
            move || {
                if let (Some(this), Some(state)) = (weak_this.upgrade(), weak_state.upgrade()) {
                    this.flush_save(&state);
                }
                glib::ControlFlow::Break
            },
        );
        *pending = Some(source);
    }

    fn update_title(&self) {
        let text = self
            .buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false);
        let title = Self::title_from_text(&text);
        self.window.set_title(Some(&title));
    }

    /// The (id, menu label, visibility) triple the tray menu shows for this
    /// note.
    pub fn tray_entry(&self) -> (u64, String, bool) {
        let text = self
            .buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false);
        (self.id, Self::title_from_text(&text), self.is_visible())
    }

    fn title_from_text(text: &str) -> String {
        let first_line = text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("New note");
        let mut title = String::new();
        for (i, ch) in first_line.chars().enumerate() {
            if i == TITLE_MAX_CHARS {
                title.push('…');
                break;
            }
            title.push(ch);
        }
        title
    }

    fn connect(
        this: &Rc<Self>,
        state: &Rc<ZpadState>,
        new_button: &gtk::Button,
        delete_button: &gtk::Button,
    ) {
        // Typing: live title + debounced autosave.
        {
            let weak_this = Rc::downgrade(this);
            let weak_state = Rc::downgrade(state);
            this.buffer.connect_changed(move |_| {
                let (Some(this), Some(state)) = (weak_this.upgrade(), weak_state.upgrade()) else {
                    return;
                };
                this.ever_dirty.set(true);
                this.update_title();
                this.schedule_save(&state);
            });
        }

        // Focus lost: flush immediately so the file is never more than a
        // second behind, even if the user switches away mid-thought.
        {
            let weak_this = Rc::downgrade(this);
            let weak_state = Rc::downgrade(state);
            let focus = gtk::EventControllerFocus::new();
            focus.connect_leave(move |_| {
                if let (Some(this), Some(state)) = (weak_this.upgrade(), weak_state.upgrade()) {
                    state.flush_window(&this);
                }
            });
            this.window.add_controller(focus);
        }

        // User close (X button or Ctrl+W): save, remember the size, then
        // hide — the app keeps living in the tray.
        {
            let weak_this = Rc::downgrade(this);
            let weak_state = Rc::downgrade(state);
            this.window.connect_close_request(move |_| {
                let (Some(this), Some(state)) = (weak_this.upgrade(), weak_state.upgrade()) else {
                    return glib::Propagation::Proceed;
                };
                this.flush_save(&state);
                state.remember_size(this.id(), (this.window.width(), this.window.height()));
                this.window.set_visible(false);
                state.sync_tray_menu_now();
                state.check_idle_after_hide();
                glib::Propagation::Stop
            });
        }

        // Window action group for accelerators (win.close, win.find).
        {
            let actions = gio::SimpleActionGroup::new();
            let close = gio::SimpleAction::new("close", None);
            let weak_window = this.window.downgrade();
            close.connect_activate(move |_, _| {
                if let Some(window) = weak_window.upgrade() {
                    // Same path as the X button: save + hide.
                    window.close();
                }
            });
            actions.add_action(&close);
            let find_action = gio::SimpleAction::new("find", None);
            let weak_this = Rc::downgrade(this);
            find_action.connect_activate(move |_, _| {
                if let Some(this) = weak_this.upgrade() {
                    this.toggle_find();
                }
            });
            actions.add_action(&find_action);
            this.window.insert_action_group("win", Some(&actions));
        }

        // Header: new note.
        {
            let weak_state = Rc::downgrade(state);
            new_button.connect_clicked(move |_| {
                if let Some(state) = weak_state.upgrade() {
                    state.new_note();
                }
            });
        }

        // Header: delete — with confirmation unless the preference says no.
        // The confirmation exists because this removes the file.
        {
            let weak_this = Rc::downgrade(this);
            let weak_state = Rc::downgrade(state);
            delete_button.connect_clicked(move |_| {
                let (Some(this), Some(state)) = (weak_this.upgrade(), weak_state.upgrade()) else {
                    return;
                };
                if state.config().confirm_delete {
                    let alert = gtk::AlertDialog::builder()
                        .modal(true)
                        .message("Delete this note?")
                        .detail("The text will be removed from disk.")
                        .buttons(vec!["Cancel", "Delete"])
                        .cancel_button(0)
                        .default_button(1)
                        .build();
                    let weak_this = Rc::downgrade(&this);
                    let weak_state = Rc::downgrade(&state);
                    alert.choose(Some(&this.window), gio::Cancellable::NONE, move |result| {
                        if matches!(result, Ok(1)) {
                            if let (Some(this), Some(state)) =
                                (weak_this.upgrade(), weak_state.upgrade())
                            {
                                state.delete_note(this.id());
                            }
                        }
                    });
                } else {
                    state.delete_note(this.id());
                }
            });
        }
    }
}
