//! User preferences, persisted to `~/.config/zpad/config.toml`.
//! Every field is live-applied; see `prefs.rs` for the UI.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    // view
    pub show_toolbar: bool,
    pub autohide_toolbar: bool,
    pub show_scrollbar: bool,
    pub show_decorations: bool,
    /// Stored for future X11 support; inert on Wayland (compositor-owned).
    pub show_on_all_workspaces: bool,
    pub hide_from_taskbar: bool,
    pub hide_from_pager: bool,

    // layout
    /// When false, `font_family` (a Pango description like "Sans 9") applies.
    pub font_from_theme: bool,
    pub font_family: String,
    /// When false, notes use the system theme colors.
    pub use_custom_colors: bool,
    pub custom_text: String,
    pub custom_background: String,
    pub new_pad_width: i32,
    pub new_pad_height: i32,

    // startup
    pub autostart: bool,
    pub wait_systray: bool,
    pub startup_delay_secs: u32,

    // tray
    pub tray_enabled: bool,
    /// "toggle" (default) or "show-all".
    pub tray_click: String,

    // other
    pub read_only: bool,
    pub confirm_delete: bool,
    /// Stored for a future release; GTK needs a custom gutter widget for it.
    pub line_numbering: bool,

    // legacy fields kept so old config files still parse cleanly
    pub notes_on_all_workspaces: bool,
}

impl Config {
    pub fn load() -> Self {
        match std::fs::read_to_string(config_path()) {
            Ok(raw) => match toml::from_str(&raw) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("zpad: config.toml unreadable ({err}); using defaults");
                    Config::defaults_with_colors()
                }
            },
            Err(_) => Config::defaults_with_colors(),
        }
    }

    /// Field defaults via `Default` keep booleans false; the notes themselves
    /// should start with the classic sticky look, so colors default on.
    pub fn defaults_with_colors() -> Self {
        Config {
            show_toolbar: true,
            autohide_toolbar: true,
            show_scrollbar: true,
            show_decorations: true,
            use_custom_colors: true,
            custom_text: "#1f1d00".into(),
            custom_background: "#fff9c4".into(),
            new_pad_width: 260,
            new_pad_height: 220,
            autostart: false,
            wait_systray: true,
            tray_enabled: true,
            tray_click: "toggle".into(),
            confirm_delete: true,
            ..Default::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let raw = toml::to_string(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        // Atomic like the notes themselves: a power cut mid-write must not
        // leave a truncated config behind.
        crate::storage::atomic_write(&path, raw.as_bytes())
    }
}

fn config_path() -> PathBuf {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() && PathBuf::from(&v).is_absolute() => PathBuf::from(v),
        _ => {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        }
    };
    base.join("zpad").join("config.toml")
}

/// The three window-management switches the compositor owns on Wayland.
/// They stay visible (greyed) so the layout matches expectations, but the
/// tooltip explains why they cannot work here.
pub fn wayland_window_controls() -> bool {
    matches!(std::env::var("XDG_SESSION_TYPE").as_deref(), Ok("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
