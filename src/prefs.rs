//! The Preferences window: a tabbed, xpad-style settings dialog whose every
//! switch live-applies and persists to `~/.config/zpad/config.toml`.
//! Tabs: View · Layout · Startup · Tray · Other.
// ComboBoxText/FontButton/ColorButton are deprecated in favor of the
// libadwaita replacements we deliberately don't depend on; on KDE they are
// still the pragmatic, fully supported choice.
#![allow(deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::app::ZpadState;
use crate::config::Config;

/// Single-slot storage for the prefs window, cleared automatically when the
/// user closes it (so reopening shows the freshly saved state).
#[derive(Default)]
pub struct Slot {
    inner: RefCell<glib::WeakRef<gtk::Window>>,
}

impl Slot {
    pub fn new() -> Self {
        Slot::default()
    }

    fn upgrade(&self) -> Option<gtk::Window> {
        self.inner.borrow().upgrade()
    }

    fn set(&self, window: Option<&gtk::Window>) {
        self.inner.borrow_mut().set(window);
    }
}

pub fn open(state: &Rc<ZpadState>, slot: &Slot) {
    let window = match slot.upgrade() {
        Some(window) => window,
        None => {
            let window = build(state);
            slot.set(Some(&window));
            window
        }
    };
    window.present();
}

fn build(state: &Rc<ZpadState>) -> gtk::Window {
    let cfg = state.config();

    let window = gtk::Window::builder()
        .title("zPad Preferences")
        .default_width(420)
        .build();

    let notebook = gtk::Notebook::new();
    notebook.append_page(
        &view_page(state, &cfg),
        Some(&gtk::Label::new(Some("View"))),
    );
    notebook.append_page(
        &layout_page(state, &cfg),
        Some(&gtk::Label::new(Some("Layout"))),
    );
    notebook.append_page(
        &startup_page(state, &cfg),
        Some(&gtk::Label::new(Some("Startup"))),
    );
    notebook.append_page(
        &tray_page(state, &cfg),
        Some(&gtk::Label::new(Some("Tray"))),
    );
    notebook.append_page(
        &other_page(state, &cfg),
        Some(&gtk::Label::new(Some("Other"))),
    );

    let outer = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();
    outer.append(&notebook);

    let close = gtk::Button::with_label("Close");
    {
        let weak_window = window.downgrade();
        close.connect_clicked(move |_| {
            if let Some(window) = weak_window.upgrade() {
                window.close();
            }
        });
    }
    let close_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    close_row.set_halign(gtk::Align::End);
    close_row.append(&close);
    outer.append(&close_row);

    window.set_child(Some(&outer));
    window
}

fn page_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn check(label: &str, active: bool) -> gtk::CheckButton {
    let button = gtk::CheckButton::with_label(label);
    button.set_active(active);
    button
}

fn wire_check(state: &Rc<ZpadState>, button: &gtk::CheckButton, set: fn(&mut Config, bool)) {
    let weak_state = Rc::downgrade(state);
    button.connect_toggled(move |button| {
        let Some(state) = weak_state.upgrade() else {
            return;
        };
        let value = button.is_active();
        state.update_config(|config| set(config, value));
    });
}

fn combo(options: &[&str], active: usize) -> gtk::ComboBoxText {
    assert!(
        !options.is_empty(),
        "combo() called with no options — caller bug"
    );
    let combo = gtk::ComboBoxText::new();
    for option in options {
        combo.append_text(option);
    }
    combo.set_active(Some((active as u32).min(options.len() as u32 - 1)));
    combo
}

fn wire_combo(
    state: &Rc<ZpadState>,
    combo: &gtk::ComboBoxText,
    values: &'static [&'static str],
    set: fn(&mut Config, &str),
) {
    let weak_state = Rc::downgrade(state);
    combo.connect_changed(move |combo| {
        let Some(state) = weak_state.upgrade() else {
            return;
        };
        if let Some(i) = combo
            .active()
            .map(|i| i as usize)
            .filter(|&i| i < values.len())
        {
            let value = values[i].to_string();
            state.update_config(|config| set(config, &value));
        }
    });
}

fn rgba_hex(color: &gdk::RGBA) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (color.red().clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.green().clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.blue().clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

// ---------------------------------------------------------------- View tab

fn view_page(state: &Rc<ZpadState>, cfg: &Config) -> gtk::Widget {
    let box_ = page_box();

    let show_toolbar = check("Show toolbar", cfg.show_toolbar);
    let autohide = check("Autohide toolbar", cfg.autohide_toolbar);
    autohide.set_sensitive(cfg.show_toolbar);
    let show_scrollbar = check("Show scrollbar", cfg.show_scrollbar);
    let decorations = check("Show window decorations", cfg.show_decorations);
    box_.append(&show_toolbar);
    box_.append(&autohide);
    box_.append(&show_scrollbar);
    box_.append(&decorations);

    {
        let weak_state = Rc::downgrade(state);
        let weak_autohide = autohide.downgrade();
        show_toolbar.connect_toggled(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let value = button.is_active();
            state.update_config(|config| config.show_toolbar = value);
            if let Some(autohide) = weak_autohide.upgrade() {
                autohide.set_sensitive(value);
            }
        });
    }
    wire_check(state, &autohide, |config, v| config.autohide_toolbar = v);
    wire_check(state, &show_scrollbar, |config, v| {
        config.show_scrollbar = v
    });
    wire_check(state, &decorations, |config, v| config.show_decorations = v);

    box_.upcast()
}

// -------------------------------------------------------------- Layout tab

fn layout_page(state: &Rc<ZpadState>, cfg: &Config) -> gtk::Widget {
    let box_ = page_box();

    // Font
    let font_theme = check("Use font from theme", cfg.font_from_theme);
    let font_custom = check("Use this font", !cfg.font_from_theme);
    font_custom.set_group(Some(&font_theme));
    let font_button = gtk::FontButton::new();
    font_button.set_font(&cfg.font_family);
    font_button.set_sensitive(!cfg.font_from_theme && !cfg.font_family.is_empty());
    box_.append(&font_theme);
    {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.append(&font_custom);
        row.append(&font_button);
        box_.append(&row);
    }

    box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // Colors
    let colors_theme = check("Use colors from theme", !cfg.use_custom_colors);
    let colors_custom = check("Use these colors", cfg.use_custom_colors);
    colors_custom.set_group(Some(&colors_theme));
    box_.append(&colors_theme);
    box_.append(&colors_custom);

    let text_color = gtk::ColorButton::with_rgba(
        &gdk::RGBA::parse(&cfg.custom_text)
            .unwrap_or_else(|_| gdk::RGBA::new(0.12, 0.11, 0.0, 1.0)),
    );
    let bg_color = gtk::ColorButton::with_rgba(
        &gdk::RGBA::parse(&cfg.custom_background)
            .unwrap_or_else(|_| gdk::RGBA::new(1.0, 0.98, 0.77, 1.0)),
    );
    let colors_grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .margin_start(24)
        .build();
    colors_grid.attach(&gtk::Label::new(Some("Text")), 0, 0, 1, 1);
    colors_grid.attach(&text_color, 1, 0, 1, 1);
    colors_grid.attach(&gtk::Label::new(Some("Background")), 0, 1, 1, 1);
    colors_grid.attach(&bg_color, 1, 1, 1, 1);
    box_.append(&colors_grid);

    box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // New pad size
    let width_spin = gtk::SpinButton::with_range(100.0, 2000.0, 10.0);
    width_spin.set_value(cfg.new_pad_width as f64);
    let height_spin = gtk::SpinButton::with_range(100.0, 2000.0, 10.0);
    height_spin.set_value(cfg.new_pad_height as f64);
    let size_grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .build();
    size_grid.attach(&gtk::Label::new(Some("Width new pad")), 0, 0, 1, 1);
    size_grid.attach(&width_spin, 1, 0, 1, 1);
    size_grid.attach(&gtk::Label::new(Some("Height new pad")), 0, 1, 1, 1);
    size_grid.attach(&height_spin, 1, 1, 1, 1);
    box_.append(&size_grid);

    // Wiring
    {
        let weak_state = Rc::downgrade(state);
        let weak_button = font_button.downgrade();
        font_theme.connect_toggled(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let from_theme = button.is_active();
            state.update_config(|config| config.font_from_theme = from_theme);
            if let Some(button) = weak_button.upgrade() {
                button.set_sensitive(!from_theme);
            }
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        font_button.connect_font_set(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let font = button.font().map(|f| f.to_string()).unwrap_or_default();
            state.update_config(|config| config.font_family = font);
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        colors_theme.connect_toggled(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let from_theme = button.is_active();
            state.update_config(|config| config.use_custom_colors = !from_theme);
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        text_color.connect_color_set(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let hex = rgba_hex(&button.rgba());
            state.update_config(|config| config.custom_text = hex);
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        bg_color.connect_color_set(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let hex = rgba_hex(&button.rgba());
            state.update_config(|config| config.custom_background = hex);
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        width_spin.connect_value_changed(move |spin| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let value = spin.value() as i32;
            state.update_config(|config| config.new_pad_width = value);
        });
    }
    {
        let weak_state = Rc::downgrade(state);
        height_spin.connect_value_changed(move |spin| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let value = spin.value() as i32;
            state.update_config(|config| config.new_pad_height = value);
        });
    }

    box_.upcast()
}

// ------------------------------------------------------------- Startup tab

fn startup_page(state: &Rc<ZpadState>, cfg: &Config) -> gtk::Widget {
    let box_ = page_box();

    let autostart = check("Start zPad automatically after login", cfg.autostart);
    let wait_systray = check("Wait for systray (if possible)", cfg.wait_systray);
    box_.append(&autostart);
    box_.append(&wait_systray);
    box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let note = gtk::Label::builder()
        .label("zPad always starts with one empty note. Your saved notes stay\non disk — reach them any time from the tray menu (newest first).")
        .xalign(0.0)
        .wrap(true)
        .build();
    note.add_css_class("dim-label");
    box_.append(&note);

    let delay_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    delay_row.append(&gtk::Label::new(Some("Delay in seconds")));
    let delay_options = ["0", "1", "2", "3", "5", "10", "30"];
    let delay_active = delay_options
        .iter()
        .position(|o| o.parse::<u32>() == Ok(cfg.startup_delay_secs))
        .unwrap_or(0);
    let delay = combo(&delay_options, delay_active);
    delay_row.append(&delay);
    box_.append(&delay_row);

    let display_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .visible(false)
        .build();
    let _ = &display_row;
    // (Display-pads preference removed: startup is always one empty note.)

    wait_systray.set_sensitive(cfg.autostart);
    delay.set_sensitive(cfg.autostart);

    wire_check(state, &autostart, |config, v| config.autostart = v);
    {
        let weak_state = Rc::downgrade(state);
        let weak_delay = delay.downgrade();
        autostart.connect_toggled(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let value = button.is_active();
            state.update_config(|config| config.autostart = value);
            if let Some(delay) = weak_delay.upgrade() {
                delay.set_sensitive(value);
            }
        });
    }
    wire_check(state, &wait_systray, |config, v| config.wait_systray = v);
    wire_combo(
        state,
        &delay,
        &["0", "1", "2", "3", "5", "10", "30"],
        |config, v| config.startup_delay_secs = v.parse().unwrap_or(0),
    );

    box_.upcast()
}

// ----------------------------------------------------------------- Tray tab

fn tray_page(state: &Rc<ZpadState>, cfg: &Config) -> gtk::Widget {
    let box_ = page_box();

    let tray_enabled = check("Enable tray icon", cfg.tray_enabled);
    box_.append(&tray_enabled);

    let click_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    click_row.append(&gtk::Label::new(Some("Tray left mouse click behavior")));
    let click = combo(
        &["Toggle Show All", "Show All Notes"],
        if cfg.tray_click == "show-all" { 1 } else { 0 },
    );
    click.set_sensitive(cfg.tray_enabled);
    click_row.append(&click);
    box_.append(&click_row);

    box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let note = gtk::Label::builder()
        .label("With the tray icon disabled, closing the last note quits zPad.\nPreferences stay reachable with Ctrl+Comma.")
        .xalign(0.0)
        .wrap(true)
        .build();
    note.add_css_class("dim-label");
    box_.append(&note);

    {
        let weak_state = Rc::downgrade(state);
        let weak_click = click.downgrade();
        tray_enabled.connect_toggled(move |button| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            let value = button.is_active();
            state.update_config(|config| config.tray_enabled = value);
            if let Some(click) = weak_click.upgrade() {
                click.set_sensitive(value);
            }
        });
    }
    wire_combo(state, &click, &["toggle", "show-all"], |config, v| {
        config.tray_click = v.to_string()
    });

    box_.upcast()
}

// ---------------------------------------------------------------- Other tab

fn other_page(state: &Rc<ZpadState>, cfg: &Config) -> gtk::Widget {
    let box_ = page_box();

    let read_only = check("Make pads read-only (Ctrl+J)", cfg.read_only);
    let confirm_delete = check("Confirm pad deletion", cfg.confirm_delete);
    box_.append(&read_only);
    box_.append(&confirm_delete);

    let line_numbers = check("Enable line numbering", cfg.line_numbering);
    line_numbers.set_sensitive(false);
    line_numbers.set_tooltip_text(Some("Planned — needs a custom text gutter widget."));
    box_.append(&line_numbers);

    wire_check(state, &read_only, |config, v| config.read_only = v);
    wire_check(state, &confirm_delete, |config, v| {
        config.confirm_delete = v
    });

    box_.upcast()
}
