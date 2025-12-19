/*

Copyright (c) 2002 Jamis Buck
Copyright (c) 2003-2007 Michael Terry

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program; if not, write to the Free Software
Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA

*/

#include "../config.h"
#include <glib/gi18n.h>
#include <gtk/gtk.h>
#include "fio.h"
#include "xpad-app.h"
#include "xpad-pad.h"
#include "xpad-pad-group.h"
#include "xpad-preferences.h"
#include "xpad-tray.h"

/* 
 * NOTE: GtkStatusIcon was removed in GTK4.
 * This is a stub implementation that disables tray functionality.
 * To restore tray support, consider using:
 * - libayatana-appindicator (for Ubuntu/Ayatana indicators)
 * - org.freedesktop.StatusNotifierItem D-Bus protocol
 * - Native platform implementations (Windows/macOS system tray APIs)
 */

static gboolean tray_available = FALSE;

void
xpad_tray_open (void)
{
	/* GtkStatusIcon removed in GTK4 - tray functionality disabled */
	tray_available = FALSE;
}

void
xpad_tray_close (void)
{
	tray_available = FALSE;
}

gboolean
xpad_tray_is_open (void)
{
	return tray_available;
}
