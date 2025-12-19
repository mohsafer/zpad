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
#include "xpad-app.h"
#include "xpad-preferences.h"
#include "xpad-settings.h"

G_DEFINE_TYPE(XpadPreferences, xpad_preferences, GTK_TYPE_DIALOG)
#define XPAD_PREFERENCES_GET_PRIVATE(object) (G_TYPE_INSTANCE_GET_PRIVATE ((object), XPAD_TYPE_PREFERENCES, XpadPreferencesPrivate))

struct XpadPreferencesPrivate 
{
	GtkWidget *fontcheck;
	GtkWidget *antifontcheck;
	GtkWidget *colorcheck;
	GtkWidget *anticolorcheck;
	GtkWidget *colorbox;
	
	GtkWidget *editcheck;
	GtkWidget *stickycheck;
	GtkWidget *confirmcheck;
	
	GtkWidget *textbutton;
	GtkWidget *backbutton;
	GtkWidget *fontbutton;
	
	guint notify_edit_handler;
	guint notify_sticky_handler;
	guint notify_confirm_handler;
	guint notify_font_handler;
	guint notify_back_handler;
	guint notify_text_handler;
	guint font_handler;
	guint back_handler;
	guint text_handler;
	guint colorcheck_handler;
	guint fontcheck_handler;
	guint editcheck_handler;
	guint stickycheck_handler;
	guint confirmcheck_handler;
};

static void change_edit_check (GtkToggleButton *button, XpadPreferences *pref);
static void change_sticky_check (GtkToggleButton *button, XpadPreferences *pref);
static void change_confirm_check (GtkToggleButton *button, XpadPreferences *pref);
static void change_color_check (GtkToggleButton *button, XpadPreferences *pref);
static void change_font_check (GtkToggleButton *button, XpadPreferences *pref);
static void change_text_color (GtkColorButton *button, XpadPreferences *pref);
static void change_back_color (GtkColorButton *button, XpadPreferences *pref);
static void change_font_face (GtkFontButton *button, XpadPreferences *pref);
static void notify_edit (XpadPreferences *pref);
static void notify_sticky (XpadPreferences *pref);
static void notify_confirm (XpadPreferences *pref);
static void notify_fontname (XpadPreferences *pref);
static void notify_text_color (XpadPreferences *pref);
static void notify_back_color (XpadPreferences *pref);
static void xpad_preferences_finalize (GObject *object);
static void xpad_preferences_response (GtkDialog *dialog, gint response);

static GtkWidget *_xpad_preferences = NULL;

void
xpad_preferences_open (void)
{
	if (_xpad_preferences)
	{
		gtk_window_present (GTK_WINDOW (_xpad_preferences));
	}
	else
	{
		_xpad_preferences = GTK_WIDGET (g_object_new (XPAD_TYPE_PREFERENCES, NULL));
		g_signal_connect_swapped (_xpad_preferences, "destroy", G_CALLBACK (g_nullify_pointer), &_xpad_preferences);
		gtk_widget_set_visible (_xpad_preferences, TRUE);
	}
}

static void
xpad_preferences_class_init (XpadPreferencesClass *klass)
{
	GObjectClass *gobject_class = G_OBJECT_CLASS (klass);
	
	gobject_class->finalize = xpad_preferences_finalize;
	
	g_type_class_add_private (gobject_class, sizeof (XpadPreferencesPrivate));
}

static void
xpad_preferences_init (XpadPreferences *pref)
{
	GtkWidget *hbox, *font_hbox, *vbox;
	const GdkRGBA *color;
	const gchar *fontname;
	GtkWidget *label, *appearance_frame, *appearance_vbox;
	GtkWidget *options_frame, *options_vbox, *global_vbox;
	gchar *text;
	GtkSizeGroup *size_group_labels = gtk_size_group_new (GTK_SIZE_GROUP_HORIZONTAL);
	
	pref->priv = XPAD_PREFERENCES_GET_PRIVATE (pref);
	
	text = g_strconcat ("<b>", _("Appearance"), "</b>", NULL);
	label = gtk_label_new (NULL);
	gtk_label_set_markup (GTK_LABEL (label), text);
	gtk_widget_set_halign (label, GTK_ALIGN_START);
	g_free (text);
	
	appearance_vbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 18);
	appearance_frame = gtk_frame_new (NULL);
	gtk_frame_set_label_widget (GTK_FRAME (appearance_frame), label);
	gtk_frame_set_child (GTK_FRAME (appearance_frame), appearance_vbox);
	gtk_widget_set_margin_start (appearance_vbox, 12);
	gtk_widget_set_margin_top (appearance_vbox, 12);
	
	pref->priv->textbutton = gtk_color_button_new ();
	pref->priv->backbutton = gtk_color_button_new ();
	pref->priv->fontbutton = gtk_font_button_new ();
	
	pref->priv->antifontcheck = gtk_check_button_new_with_mnemonic (_("Use font from theme"));
	pref->priv->fontcheck = gtk_check_button_new_with_mnemonic (_("Use this font:"));
	gtk_check_button_set_group (GTK_CHECK_BUTTON (pref->priv->fontcheck), GTK_CHECK_BUTTON (pref->priv->antifontcheck));
	
	pref->priv->anticolorcheck = gtk_check_button_new_with_mnemonic (_("Use colors from theme"));
	pref->priv->colorcheck = gtk_check_button_new_with_mnemonic (_("Use these colors:"));
	gtk_check_button_set_group (GTK_CHECK_BUTTON (pref->priv->colorcheck), GTK_CHECK_BUTTON (pref->priv->anticolorcheck));
	
	font_hbox = gtk_box_new (GTK_ORIENTATION_HORIZONTAL, 6);
	gtk_box_append (GTK_BOX (font_hbox), pref->priv->fontcheck);
	gtk_box_append (GTK_BOX (font_hbox), pref->priv->fontbutton);
	gtk_widget_set_hexpand (pref->priv->fontbutton, TRUE);
	
	pref->priv->colorbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 6);
	hbox = gtk_box_new (GTK_ORIENTATION_HORIZONTAL, 12);
	label = gtk_label_new_with_mnemonic (_("Background:"));
	gtk_widget_set_halign (label, GTK_ALIGN_START);
	gtk_size_group_add_widget (size_group_labels, label);
	gtk_box_append (GTK_BOX (hbox), label);
	gtk_box_append (GTK_BOX (hbox), pref->priv->backbutton);
	gtk_widget_set_hexpand (pref->priv->backbutton, TRUE);
	gtk_box_append (GTK_BOX (pref->priv->colorbox), hbox);
	
	hbox = gtk_box_new (GTK_ORIENTATION_HORIZONTAL, 12);
	label = gtk_label_new_with_mnemonic (_("Foreground:"));
	gtk_widget_set_halign (label, GTK_ALIGN_START);
	gtk_size_group_add_widget (size_group_labels, label);
	gtk_box_append (GTK_BOX (hbox), label);
	gtk_box_append (GTK_BOX (hbox), pref->priv->textbutton);
	gtk_widget_set_hexpand (pref->priv->textbutton, TRUE);
	gtk_box_append (GTK_BOX (pref->priv->colorbox), hbox);
	gtk_widget_set_margin_start (pref->priv->colorbox, 12);
	
	pref->priv->editcheck = gtk_check_button_new_with_mnemonic (_("_Edit lock"));
	pref->priv->stickycheck = gtk_check_button_new_with_mnemonic (_("_Pads start on all workspaces"));
	pref->priv->confirmcheck = gtk_check_button_new_with_mnemonic (_("_Confirm pad deletion"));
	
	gtk_dialog_add_button (GTK_DIALOG (pref), "gtk-close", GTK_RESPONSE_CLOSE);
	gtk_dialog_set_default_response (GTK_DIALOG (pref), GTK_RESPONSE_CLOSE);
	g_signal_connect (pref, "response", G_CALLBACK (xpad_preferences_response), NULL);
	gtk_window_set_title (GTK_WINDOW (pref), _("Xpad Preferences"));
	
	gtk_color_button_set_use_alpha (GTK_COLOR_BUTTON (pref->priv->textbutton), FALSE);
	gtk_color_button_set_use_alpha (GTK_COLOR_BUTTON (pref->priv->backbutton), xpad_app_get_translucent ());
	
	gtk_color_button_set_title (GTK_COLOR_BUTTON (pref->priv->textbutton), _("Set Foreground Color"));
	gtk_color_button_set_title (GTK_COLOR_BUTTON (pref->priv->backbutton), _("Set Background Color"));
	gtk_font_button_set_title (GTK_FONT_BUTTON (pref->priv->fontbutton), _("Set Font"));
	
	/* Set current state */
	color = xpad_settings_get_back_color (xpad_settings ());
	if (color)
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->colorcheck), TRUE);
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->backbutton), color);
	}
	else
	{
		GdkRGBA default_color = { 1.0, 1.0, 1.0, 1.0 };
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->anticolorcheck), TRUE);
		gtk_widget_set_sensitive (pref->priv->colorbox, FALSE);
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->backbutton), &default_color);
	}
	
	color = xpad_settings_get_text_color (xpad_settings ());
	if (color)
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->textbutton), color);
	else
	{
		GdkRGBA default_color = { 0.0, 0.0, 0.0, 1.0 };
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->textbutton), &default_color);
	}
	
	fontname = xpad_settings_get_fontname (xpad_settings ());
	if (fontname)
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->fontcheck), TRUE);
		gtk_font_button_set_font_name (GTK_FONT_BUTTON (pref->priv->fontbutton), fontname);
	}
	else
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->antifontcheck), TRUE);
		gtk_widget_set_sensitive (pref->priv->fontbutton, FALSE);
		/* Use default system font */
		gtk_font_button_set_font_name (GTK_FONT_BUTTON (pref->priv->fontbutton), "Sans 10");
	}
	
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->editcheck), xpad_settings_get_edit_lock (xpad_settings ()));
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->stickycheck), xpad_settings_get_sticky (xpad_settings ()));
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->confirmcheck), xpad_settings_get_confirm_destroy (xpad_settings ()));
	
	vbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 6);
	gtk_box_append (GTK_BOX (vbox), pref->priv->antifontcheck);
	gtk_box_append (GTK_BOX (vbox), font_hbox);
	gtk_box_append (GTK_BOX (appearance_vbox), vbox);
	
	vbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 6);
	gtk_box_append (GTK_BOX (vbox), pref->priv->anticolorcheck);
	gtk_box_append (GTK_BOX (vbox), pref->priv->colorcheck);
	gtk_box_append (GTK_BOX (vbox), pref->priv->colorbox);
	gtk_box_append (GTK_BOX (appearance_vbox), vbox);
	
	
	text = g_strconcat ("<b>", _("Options"), "</b>", NULL);
	label = GTK_WIDGET (g_object_new (GTK_TYPE_LABEL,
		"label", text,
		"use-markup", TRUE,
		"xalign", 0.0,
		NULL));
	g_free (text);
	options_vbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 6);
	gtk_widget_set_margin_start (options_vbox, 12);
	gtk_widget_set_margin_top (options_vbox, 12);
	options_frame = gtk_frame_new (NULL);
	gtk_frame_set_label_widget (GTK_FRAME (options_frame), label);
	gtk_frame_set_child (GTK_FRAME (options_frame), options_vbox);
	
	
	gtk_box_append (GTK_BOX (options_vbox), pref->priv->editcheck);
	gtk_box_append (GTK_BOX (options_vbox), pref->priv->stickycheck);
	gtk_box_append (GTK_BOX (options_vbox), pref->priv->confirmcheck);	
	
	global_vbox = gtk_box_new (GTK_ORIENTATION_VERTICAL, 18);
	gtk_widget_set_margin_start (global_vbox, 6);
	gtk_widget_set_margin_end (global_vbox, 6);
	gtk_widget_set_margin_top (global_vbox, 6);
	gtk_widget_set_margin_bottom (global_vbox, 6);
	gtk_box_append (GTK_BOX (global_vbox), appearance_frame);
	gtk_box_append (GTK_BOX (global_vbox), options_frame);
	
	gtk_box_append (GTK_BOX (gtk_dialog_get_content_area (GTK_DIALOG (pref))), global_vbox);
	
	pref->priv->editcheck_handler = g_signal_connect (pref->priv->editcheck, "toggled", G_CALLBACK (change_edit_check), pref);
	pref->priv->stickycheck_handler = g_signal_connect (pref->priv->stickycheck, "toggled", G_CALLBACK (change_sticky_check), pref);
	pref->priv->confirmcheck_handler = g_signal_connect (pref->priv->confirmcheck, "toggled", G_CALLBACK (change_confirm_check), pref);
	pref->priv->colorcheck_handler = g_signal_connect (pref->priv->colorcheck, "toggled", G_CALLBACK (change_color_check), pref);
	pref->priv->fontcheck_handler = g_signal_connect (pref->priv->fontcheck, "toggled", G_CALLBACK (change_font_check), pref);
	pref->priv->text_handler = g_signal_connect (pref->priv->textbutton, "color-set", G_CALLBACK (change_text_color), pref);
	pref->priv->back_handler = g_signal_connect (pref->priv->backbutton, "color-set", G_CALLBACK (change_back_color), pref);
	pref->priv->font_handler = g_signal_connect (pref->priv->fontbutton, "font-set", G_CALLBACK (change_font_face), pref);
	pref->priv->notify_font_handler = g_signal_connect_swapped (xpad_settings (), "notify::fontname", G_CALLBACK (notify_fontname), pref);
	pref->priv->notify_text_handler = g_signal_connect_swapped (xpad_settings (), "notify::text-color", G_CALLBACK (notify_text_color), pref);
	pref->priv->notify_back_handler = g_signal_connect_swapped (xpad_settings (), "notify::back-color", G_CALLBACK (notify_back_color), pref);
	pref->priv->notify_sticky_handler = g_signal_connect_swapped (xpad_settings (), "notify::sticky", G_CALLBACK (notify_sticky), pref);
	pref->priv->notify_edit_handler = g_signal_connect_swapped (xpad_settings (), "notify::edit-lock", G_CALLBACK (notify_edit), pref);
	pref->priv->notify_confirm_handler = g_signal_connect_swapped (xpad_settings (), "notify::confirm-destroy", G_CALLBACK (notify_confirm), pref);
	
	g_object_unref (size_group_labels);

static void
xpad_preferences_finalize (GObject *object)
{
	XpadPreferences *pref = XPAD_PREFERENCES (object);
	
	g_signal_handlers_disconnect_matched (xpad_settings (), G_SIGNAL_MATCH_DATA, 0, 0, NULL, NULL, pref);
	
	G_OBJECT_CLASS (xpad_preferences_parent_class)->finalize (object);
}

static void
xpad_preferences_response (GtkDialog *dialog, gint response)
{
	if (response == GTK_RESPONSE_CLOSE)
		gtk_window_destroy (GTK_WINDOW (dialog));
}

static void
change_color_check (GtkToggleButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_back_handler);
	g_signal_handler_block (xpad_settings (), pref->priv->notify_text_handler);
	
	if (!gtk_toggle_button_get_active (button))
	{
		xpad_settings_set_text_color (xpad_settings (), NULL);
		xpad_settings_set_back_color (xpad_settings (), NULL);
	}
	else
	{
		GdkRGBA color;
		gtk_color_chooser_get_rgba (GTK_COLOR_CHOOSER (pref->priv->textbutton), &color);
		xpad_settings_set_text_color (xpad_settings (), &color);
		gtk_color_chooser_get_rgba (GTK_COLOR_CHOOSER (pref->priv->backbutton), &color);
		xpad_settings_set_back_color (xpad_settings (), &color);
	}
	
	gtk_widget_set_sensitive (pref->priv->colorbox, gtk_toggle_button_get_active (button));
	
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_back_handler);
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_text_handler);
}

static void
change_font_check (GtkToggleButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_font_handler);
	
	if (!gtk_toggle_button_get_active (button))
		xpad_settings_set_fontname (xpad_settings (), NULL);
	else
		xpad_settings_set_fontname (xpad_settings (), gtk_font_button_get_font_name (GTK_FONT_BUTTON (pref->priv->fontbutton)));
	
	gtk_widget_set_sensitive (pref->priv->fontbutton, gtk_toggle_button_get_active (button));
	
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_font_handler);
}

static void
change_edit_check (GtkToggleButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_edit_handler);
	xpad_settings_set_edit_lock (xpad_settings (), gtk_toggle_button_get_active (button));
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_edit_handler);
}

static void
change_sticky_check (GtkToggleButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_sticky_handler);
	xpad_settings_set_sticky (xpad_settings (), gtk_toggle_button_get_active (button));
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_sticky_handler);
}

static void
change_confirm_check (GtkToggleButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_confirm_handler);
	xpad_settings_set_confirm_destroy (xpad_settings (), gtk_toggle_button_get_active (button));
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_confirm_handler);
}

static void
change_text_color (GtkColorButton *button, XpadPreferences *pref)
{
	GdkRGBA color;
	gtk_color_chooser_get_rgba (GTK_COLOR_CHOOSER (button), &color);
	g_signal_handler_block (xpad_settings (), pref->priv->notify_text_handler);
	xpad_settings_set_text_color (xpad_settings (), &color);
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_text_handler);
}

static void
change_back_color (GtkColorButton *button, XpadPreferences *pref)
{
	GdkRGBA color;
	gtk_color_chooser_get_rgba (GTK_COLOR_CHOOSER (button), &color);
	g_signal_handler_block (xpad_settings (), pref->priv->notify_back_handler);
	xpad_settings_set_back_color (xpad_settings (), &color);
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_back_handler);
}

static void
change_font_face (GtkFontButton *button, XpadPreferences *pref)
{
	g_signal_handler_block (xpad_settings (), pref->priv->notify_font_handler);
	xpad_settings_set_fontname (xpad_settings (), gtk_font_button_get_font_name (button));
	g_signal_handler_unblock (xpad_settings (), pref->priv->notify_font_handler);
}

static void
notify_back_color (XpadPreferences *pref)
{
	const GdkRGBA *color = xpad_settings_get_back_color (xpad_settings ());
	
	g_signal_handler_block (pref->priv->backbutton, pref->priv->back_handler);
	g_signal_handler_block (pref->priv->colorcheck, pref->priv->colorcheck_handler);
	
	if (color)
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->colorcheck), TRUE);
		gtk_widget_set_sensitive (pref->priv->colorbox, TRUE);
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->backbutton), color);
	}
	else
	{
		gtk_widget_set_sensitive (pref->priv->colorbox, FALSE);
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->anticolorcheck), TRUE);
	}
	
	g_signal_handler_unblock (pref->priv->colorcheck, pref->priv->colorcheck_handler);
	g_signal_handler_unblock (pref->priv->backbutton, pref->priv->back_handler);
}

static void
notify_text_color (XpadPreferences *pref)
{
	const GdkRGBA *color = xpad_settings_get_text_color (xpad_settings ());
	
	g_signal_handler_block (pref->priv->textbutton, pref->priv->text_handler);
	g_signal_handler_block (pref->priv->colorcheck, pref->priv->colorcheck_handler);
	
	if (color)
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->colorcheck), TRUE);
		gtk_widget_set_sensitive (pref->priv->colorbox, TRUE);
		gtk_color_chooser_set_rgba (GTK_COLOR_CHOOSER (pref->priv->textbutton), color);
	}
	else
	{
		gtk_widget_set_sensitive (pref->priv->colorbox, FALSE);
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->anticolorcheck), TRUE);
	}
	
	g_signal_handler_unblock (pref->priv->colorcheck, pref->priv->colorcheck_handler);
	g_signal_handler_unblock (pref->priv->textbutton, pref->priv->text_handler);
}

static void
notify_fontname (XpadPreferences *pref)
{
	const gchar *fontname = xpad_settings_get_fontname (xpad_settings ());
	
	g_signal_handler_block (pref->priv->fontbutton, pref->priv->font_handler);
	g_signal_handler_block (pref->priv->fontcheck, pref->priv->fontcheck_handler);
	
	if (fontname)
	{
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->fontcheck), TRUE);
		gtk_widget_set_sensitive (pref->priv->fontbutton, TRUE);
		gtk_font_button_set_font_name (GTK_FONT_BUTTON (pref->priv->fontbutton), fontname);
	}
	else
	{
		gtk_widget_set_sensitive (pref->priv->fontbutton, FALSE);
		gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->antifontcheck), TRUE);
	}
	
	g_signal_handler_unblock (pref->priv->fontcheck, pref->priv->fontcheck_handler);
	g_signal_handler_unblock (pref->priv->fontbutton, pref->priv->font_handler);
}

static void
notify_edit (XpadPreferences *pref)
{
	g_signal_handler_block (pref->priv->editcheck, pref->priv->editcheck_handler);
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->editcheck), xpad_settings_get_edit_lock (xpad_settings ()));
	g_signal_handler_unblock (pref->priv->editcheck, pref->priv->editcheck_handler);
}

static void
notify_sticky (XpadPreferences *pref)
{
	g_signal_handler_block (pref->priv->stickycheck, pref->priv->stickycheck_handler);
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->stickycheck), xpad_settings_get_sticky (xpad_settings ()));
	g_signal_handler_unblock (pref->priv->stickycheck, pref->priv->stickycheck_handler);
}

static void
notify_confirm (XpadPreferences *pref)
{
	g_signal_handler_block (pref->priv->confirmcheck, pref->priv->confirmcheck_handler);
	gtk_toggle_button_set_active (GTK_TOGGLE_BUTTON (pref->priv->confirmcheck), xpad_settings_get_confirm_destroy (xpad_settings ()));
	g_signal_handler_unblock (pref->priv->confirmcheck, pref->priv->confirmcheck_handler);
}
