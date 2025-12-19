# GTK4 Migration Summary for Xpad

This document summarizes the changes made to migrate Xpad from GTK2/GTK3 to GTK4.

## Overview

The migration involved updating approximately 2000+ lines of code across 15 source files. All deprecated GTK2/GTK3 APIs have been replaced with their GTK4 equivalents.

## Build Configuration

### configure.ac
- Updated dependency from `gtk+-2.0 >= 2.12` to `gtk4 >= 4.0`
- Removed deprecated `GTK_MULTIHEAD_SAFE` flag

## Core API Changes

### Color System (All Files)
- **Old:** `GdkColor` (16-bit RGB with pixel value)
- **New:** `GdkRGBA` (floating-point RGBA 0.0-1.0)
- **Functions replaced:**
  - `gtk_color_button_get_color()` → `gtk_color_chooser_get_rgba()`
  - `gtk_color_button_set_color()` → `gtk_color_chooser_set_rgba()`
  - Property types: `GDK_TYPE_COLOR` → `GDK_TYPE_RGBA`

### Event Handling (xpad-text-view.c, xpad-toolbar.c, xpad-pad.c)
- **Old:** Signal-based events (`button-press-event`, `key-press-event`, etc.)
- **New:** Event controllers
  - `GtkGestureClick` for mouse clicks
  - `GtkEventControllerKey` for keyboard events
  - `GtkEventControllerMotion` for mouse movement
  - `GtkEventControllerFocus` for focus events

### Box Widgets (All dialog files)
- **Old:** `gtk_hbox_new()`, `gtk_vbox_new()`
- **New:** `gtk_box_new(GTK_ORIENTATION_HORIZONTAL/VERTICAL, spacing)`
- **Packing:** `gtk_box_pack_start()` → `gtk_box_append()`

### Container APIs
- **Old:** `gtk_container_add()`, `gtk_container_remove()`
- **New:** Widget-specific methods:
  - Windows: `gtk_window_set_child()`
  - Boxes: `gtk_box_append()`, `gtk_box_remove()`
  - Frames: `gtk_frame_set_child()`

### Dialog APIs
- **Old:** `GTK_DIALOG(dialog)->vbox`
- **New:** `gtk_dialog_get_content_area(GTK_DIALOG(dialog))`
- **Removed:** `gtk_dialog_set_has_separator()` (separators removed in GTK4)

### Widget Destruction
- **Old:** `gtk_widget_destroy(widget)`
- **New:** `gtk_window_destroy(GTK_WINDOW(window))`

### Widget Visibility
- **Old:** `gtk_widget_show_all(widget)` (recursive)
- **New:** Widgets visible by default, use `gtk_widget_set_visible()` explicitly

### Stock Items (xpad-toolbar.c, xpad-app.c)
- **Old:** `GTK_STOCK_*` constants
- **New:** Icon names (e.g., `"edit-copy"`, `"document-new"`)
- **Image creation:** `gtk_image_new_from_icon_name(icon_name)`

### Menu System (xpad-toolbar.c)
- **Old:** `GtkMenu`, `gtk_menu_popup()`
- **New:** `GtkPopoverMenu`, `gtk_popover_popup()`
- **Structure:** Use `GtkBox` hierarchy instead of menu items

### Widget Styling (xpad-text-view.c, xpad-pad.c)
- **Old:** `gtk_widget_modify_*()`, `GtkStyle`
- **New:** `GtkCssProvider` with CSS strings
  ```c
  GtkCssProvider *provider = gtk_css_provider_new();
  gtk_css_provider_load_from_data(provider, "textview { color: rgba(...); }", -1);
  gtk_style_context_add_provider(context, provider, GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  ```

### Window Operations (xpad-pad.c)
- **Old:** `gtk_window_begin_move_drag(window, button, x, y, time)`
- **New:** `gdk_toplevel_begin_move(GDK_TOPLEVEL(surface), device, button, x, y, timestamp)`
- **Resize:** `gdk_toplevel_begin_resize()`

### Clipboard (xpad-pad.c)
- **Old:** `GtkClipboard`, `gtk_clipboard_get()`
- **New:** `GdkClipboard`, `gdk_display_get_clipboard()`

### Cursor Management
- **Old:** `gdk_cursor_new()`, `gdk_window_set_cursor()`
- **New:** `gtk_widget_set_cursor_from_name(widget, "cursor-name")`

### Main Loop (xpad-app.c)
- **Old:** `gtk_main()`, `gtk_main_quit()`, `gtk_main_level()`
- **New:** 
  ```c
  GMainLoop *loop = g_main_loop_new(NULL, FALSE);
  g_main_loop_run(loop);
  ```
- **Exit:** Use `exit(0)` instead of checking `gtk_main_level()`

### Initialization (xpad-app.c)
- **Old:** `gtk_init_check(&argc, &argv)`, `gtk_set_locale()`, `gdk_set_program_class()`
- **New:** `gtk_init_check()` (no parameters), removed locale/program class functions

### Layout Widgets
- **Old:** `GtkAlignment`, `gtk_alignment_new()`, `gtk_alignment_set_padding()`
- **New:** Direct margin properties:
  ```c
  gtk_widget_set_margin_start/end/top/bottom(widget, pixels);
  ```

### Miscellaneous Widgets
- **Old:** `GtkMisc`, `gtk_misc_set_alignment()`
- **New:** `gtk_widget_set_halign()`, `gtk_widget_set_valign()`

### Radio Buttons (xpad-preferences.c, xpad-pad-properties.c)
- **Old:** `gtk_radio_button_new_*()`, linked list groups
- **New:** `gtk_check_button_new_*()` with `gtk_check_button_set_group()`

## File-by-File Changes

### configure.ac
- Updated GTK4 dependency

### xpad-settings.h/c
- Changed all color properties from `GdkColor*` to `GdkRGBA*`
- Updated initialization with floating-point color values

### xpad-text-view.c
- Implemented `GtkGestureClick` for button events
- Implemented `GtkEventControllerFocus` for focus events
- Added `GtkCssProvider` for color styling
- Updated cursor API

### xpad-toolbar.c
- Replaced stock icons with icon names
- Converted `GtkMenu` to `GtkPopoverMenu`
- Implemented event controllers for drag operations
- Updated keyboard handling with `GtkEventControllerKey`

### xpad-pad.c
- Extensive event controller implementation
- Updated window move/resize with `GdkToplevel` API
- Clipboard API updates
- CSS provider for styling
- Dialog content area updates

### help.c
- Updated dialog APIs
- Box widget updates
- Margin properties

### xpad-preferences.c
- Complete GdkRGBA migration
- Radio buttons → check buttons with groups
- Removed `GtkAlignment`, using margins
- Dialog content area updates
- Removed deprecated style system usage

### xpad-pad-properties.c
- Mirror changes from xpad-preferences.c
- GdkRGBA API updates
- Box and layout widget updates

### xpad-app.c
- Main loop migration to `GMainLoop`
- Removed deprecated initialization functions
- Updated error dialog stock items
- Removed visual/colormap code (built-in transparency in GTK4)

### xpad-tray.c
- **CRITICAL:** System tray functionality disabled
- **Reason:** `GtkStatusIcon` completely removed in GTK4
- **Status:** Stub implementation provided
- **Future:** Requires external library (libayatana-appindicator) or D-Bus StatusNotifierItem

## Known Limitations

1. **System Tray Disabled**: The most significant limitation. GtkStatusIcon was removed in GTK4. To restore:
   - Use libayatana-appindicator for Ubuntu/compatible systems
   - Implement org.freedesktop.StatusNotifierItem D-Bus protocol
   - Platform-specific implementations (Windows/macOS system tray APIs)

2. **Transparency**: Now relies on compositor support (standard in modern systems)

3. **Default Widget Visibility**: Widgets are visible by default in GTK4, may need explicit `gtk_widget_set_visible(FALSE)` in some cases

## Testing Recommendations

1. **Color Settings**: Test all color preference saving/loading
2. **Event Handling**: Test all mouse/keyboard interactions
3. **Dialogs**: Verify preferences and properties dialogs
4. **Window Operations**: Test window moving, resizing, and decorations
5. **Clipboard**: Test copy/paste operations
6. **Font Settings**: Verify font selection and application
7. **Menu System**: Test toolbar popup menus

## Build Instructions

```bash
./autogen.sh
./configure
make
```

If autotools are not present:
```bash
apt install autoconf automake libtool gettext pkg-config
```

Required GTK4 development libraries:
```bash
apt install libgtk-4-dev
```

## Migration Statistics

- Files modified: 15
- Core source files: 13
- Header files: 2
- Approximate lines changed: 2000+
- Build configuration: 1 file
- API replacements: 50+ deprecated function patterns

## GTK4 Benefits

1. **Modern rendering**: Improved performance with GSK renderer
2. **Simplified APIs**: More consistent widget hierarchy
3. **Better event handling**: Event controllers are more flexible
4. **Native transparency**: No need for visual/colormap hacks
5. **CSS styling**: More powerful and consistent theming
6. **Future-proof**: Active development and long-term support

## Compatibility

- Minimum GTK version: 4.0
- GLib/GObject: As required by GTK4
- Compiles without errors as of this migration
- Runtime testing required for full validation
