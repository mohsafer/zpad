/*

Copyright (c) 2001-2007 Michael Terry

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
#include <string.h>
#include <gtk/gtk.h>
#include "help.h"

GtkWidget *help_window = NULL;

static void help_close (void)
{
	help_window = NULL;
}


static void show_help_at_page (gint page);

static GtkWidget *create_help (gint page)
{
	GtkWidget *dialog, *helptext, *button, *content, *actions;
	gchar *helptextbuf;
	
	/* Create the widgets using GtkWindow instead of deprecated GtkDialog */
	
	dialog = gtk_window_new ();
	helptext = gtk_label_new ("");
	
	/* we use g_strdup_printf because C89 has size limits on static strings */
	helptextbuf = g_strdup_printf ("%s\n\n%s\n\n%s\n\n%s\n\n%s\n\n%s",
_("Each xpad session consists of one or more open pads.  "
"These pads are basically sticky notes on your desktop in which "
"you can write memos."),
_("<b>To move a pad</b>, left drag on the toolbar, right drag "
"on the resizer in the bottom right, or hold down CTRL "
"while left dragging anywhere on the pad."),
_("<b>To resize a pad</b>, left drag on the resizer or hold down "
"CTRL while right dragging anywhere on the pad."),
_("<b>To change color settings</b>, right click on a pad "
"and choose Edit->Preferences."),
_("Most actions are available throught the popup menu "
"that appears when you right click on a pad.  Try it out and "
"enjoy."),
_("Please send comments or bug reports to "
"xpad-devel@lists.sourceforge.net"));
	
	gtk_label_set_markup (GTK_LABEL (helptext), helptextbuf);
	
	g_free (helptextbuf);
	
	gtk_widget_set_margin_start (helptext, 12);
	gtk_widget_set_margin_end (helptext, 12);
	gtk_widget_set_margin_top (helptext, 12);
	gtk_widget_set_margin_bottom (helptext, 12);
	gtk_widget_set_halign (helptext, GTK_ALIGN_START);
	gtk_widget_set_valign (helptext, GTK_ALIGN_START);
	gtk_label_set_wrap (GTK_LABEL (helptext), TRUE);
	
	gtk_window_set_title (GTK_WINDOW (dialog), _("Help"));
	
	/* Build content box with help text and close button */
	content = gtk_box_new (GTK_ORIENTATION_VERTICAL, 6);
	gtk_box_append (GTK_BOX (content), helptext);
	
	actions = gtk_box_new (GTK_ORIENTATION_HORIZONTAL, 6);
	gtk_widget_set_halign (actions, GTK_ALIGN_END);
	gtk_widget_set_margin_start (actions, 12);
	gtk_widget_set_margin_end (actions, 12);
	gtk_widget_set_margin_bottom (actions, 12);
	
	button = gtk_button_new_with_label (_("Close"));
	gtk_box_append (GTK_BOX (actions), button);
	gtk_box_append (GTK_BOX (content), actions);
	
	gtk_window_set_child (GTK_WINDOW (dialog), content);
	
	g_signal_connect (dialog, "destroy", 
		G_CALLBACK (help_close), NULL);
	g_signal_connect_swapped (button, "clicked", 
		G_CALLBACK (gtk_window_destroy), dialog);
	
	gtk_window_set_resizable (GTK_WINDOW (dialog), FALSE);
	gtk_widget_set_visible (dialog, TRUE);
	
	return dialog;
}

void show_help (void)
{
	show_help_at_page (0);
}

static void show_help_at_page (gint page)
{
	if (help_window == NULL)
		help_window = create_help (page);
	else
		gtk_window_present (GTK_WINDOW (help_window));
}
