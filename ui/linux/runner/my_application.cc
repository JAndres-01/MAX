#include "my_application.h"

#include <cairo/cairo.h>
#include <flutter_linux/flutter_linux.h>
#include <gtk-layer-shell/gtk-layer-shell.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "flutter/generated_plugin_registrant.h"

static constexpr int kCanvasWidth = 500;
static constexpr int kCanvasHeight = 240;

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
  GtkWindow* window;
  FlView* view;
  FlMethodChannel* window_channel;
  int active_input_width;
  int active_input_height;
  bool is_settings_mode;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

static gboolean has_settings_arg(char** args) {
  if (args == nullptr) return FALSE;
  for (int i = 0; args[i] != nullptr; i++) {
    if (g_strcmp0(args[i], "--settings") == 0) {
      return TRUE;
    }
  }
  return FALSE;
}

static void update_input_region(MyApplication* self, int width, int height) {
  if (self->is_settings_mode) {
    return;
  }
  self->active_input_width = width;
  self->active_input_height = height;

  if (self->window == nullptr) {
    return;
  }

  GdkWindow* gdk_window = gtk_widget_get_window(GTK_WIDGET(self->window));
  if (gdk_window == nullptr) {
    return;
  }

  int clamped_w = width > kCanvasWidth ? kCanvasWidth : (width < 40 ? 40 : width);
  int clamped_h = height > kCanvasHeight ? kCanvasHeight : (height < 8 ? 8 : height);
  int offset_x = (kCanvasWidth - clamped_w) / 2;

  cairo_rectangle_int_t rect = {offset_x, 0, clamped_w, clamped_h};
  cairo_region_t* region = cairo_region_create_rectangle(&rect);
  gdk_window_input_shape_combine_region(gdk_window, region, 0, 0);

  if (self->view != nullptr) {
    GdkWindow* view_gdk_window = gtk_widget_get_window(GTK_WIDGET(self->view));
    if (view_gdk_window != nullptr) {
      gdk_window_input_shape_combine_region(view_gdk_window, region, 0, 0);
    }
  }

  cairo_region_destroy(region);
}

static void method_call_cb(FlMethodChannel* channel,
                           FlMethodCall* method_call,
                           gpointer user_data) {
  MyApplication* self = MY_APPLICATION(user_data);
  const gchar* method = fl_method_call_get_name(method_call);

  if (g_strcmp0(method, "setInputRegion") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    int width = 220;
    int height = 18;
    if (fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      FlValue* w_val = fl_value_lookup_string(args, "width");
      FlValue* h_val = fl_value_lookup_string(args, "height");
      if (w_val != nullptr && fl_value_get_type(w_val) == FL_VALUE_TYPE_INT) {
        width = static_cast<int>(fl_value_get_int(w_val));
      }
      if (h_val != nullptr && fl_value_get_type(h_val) == FL_VALUE_TYPE_INT) {
        height = static_cast<int>(fl_value_get_int(h_val));
      }
    }
    update_input_region(self, width, height);
    g_autoptr(FlMethodResponse) response =
        FL_METHOD_RESPONSE(fl_method_success_response_new(nullptr));
    fl_method_call_respond(method_call, response, nullptr);
  } else {
    g_autoptr(FlMethodResponse) response =
        FL_METHOD_RESPONSE(fl_method_not_implemented_response_new());
    fl_method_call_respond(method_call, response, nullptr);
  }
}

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  GtkWidget* toplevel = gtk_widget_get_toplevel(GTK_WIDGET(view));
  gtk_widget_show(toplevel);
  if (!self->is_settings_mode) {
    update_input_region(self, self->active_input_width, self->active_input_height);
  }
}

// Implements GApplication::activate.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  self->is_settings_mode = has_settings_arg(self->dart_entrypoint_arguments);

  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));
  self->window = window;

  if (!self->is_settings_mode) {
    g_application_hold(application);
  }

  if (self->is_settings_mode) {
    gtk_window_set_title(window, "MAX - Configuración");
    gtk_window_set_decorated(window, TRUE);
    gtk_window_set_resizable(window, TRUE);
    gtk_window_set_default_size(window, 820, 580);
    gtk_widget_set_size_request(GTK_WIDGET(window), 680, 460);
    gtk_window_set_position(window, GTK_WIN_POS_CENTER);

    GtkCssProvider* css_provider = gtk_css_provider_new();
    gtk_css_provider_load_from_data(
        css_provider,
        "window, .background { background-color: #0D0D10; color: #FFFFFF; }",
        -1, nullptr);
    GdkScreen* screen = gtk_window_get_screen(window);
    gtk_style_context_add_provider_for_screen(
        screen, GTK_STYLE_PROVIDER(css_provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    g_object_unref(css_provider);

    GdkVisual* visual = gdk_screen_get_rgba_visual(screen);
    if (visual != nullptr) {
      gtk_widget_set_visual(GTK_WIDGET(window), visual);
    }
  } else {
    self->active_input_width = 220;
    self->active_input_height = 18;

    // Wayland Layer Shell setup: true top-center floating hardware notch
    if (gtk_layer_is_supported()) {
      gtk_layer_init_for_window(window);
      gtk_layer_set_layer(window, GTK_LAYER_SHELL_LAYER_OVERLAY);
      gtk_layer_set_namespace(window, "max-notch");
      gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_TOP, TRUE);
      gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_LEFT, FALSE);
      gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_RIGHT, FALSE);
      gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_BOTTOM, FALSE);
      gtk_layer_set_exclusive_zone(window, -1);
      gtk_layer_set_margin(window, GTK_LAYER_SHELL_EDGE_TOP, 0);
      gtk_layer_set_keyboard_mode(window, GTK_LAYER_SHELL_KEYBOARD_MODE_NONE);
    } else {
      gtk_window_set_type_hint(window, GDK_WINDOW_TYPE_HINT_DOCK);
      gtk_window_set_keep_above(window, TRUE);
      gtk_window_set_skip_taskbar_hint(window, TRUE);
      gtk_window_set_skip_pager_hint(window, TRUE);
    }

    gtk_window_set_title(window, "MAX");
    gtk_window_set_decorated(window, FALSE);
    gtk_window_set_resizable(window, FALSE);
    gtk_window_set_default_size(window, kCanvasWidth, kCanvasHeight);
    gtk_widget_set_size_request(GTK_WIDGET(window), kCanvasWidth, kCanvasHeight);

    // Force 100% transparent GTK background via CSS so no window box appears
    GtkCssProvider* css_provider = gtk_css_provider_new();
    gtk_css_provider_load_from_data(
        css_provider,
        "window, .background { background-color: rgba(0, 0, 0, 0); "
        "background: transparent; border: none; box-shadow: none; }",
        -1, nullptr);
    GdkScreen* rgba_screen = gtk_window_get_screen(window);
    gtk_style_context_add_provider_for_screen(
        rgba_screen, GTK_STYLE_PROVIDER(css_provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    g_object_unref(css_provider);

    GdkVisual* visual = gdk_screen_get_rgba_visual(rgba_screen);
    if (visual != nullptr) {
      gtk_widget_set_visual(GTK_WIDGET(window), visual);
    }
  }

  gtk_widget_set_app_paintable(GTK_WIDGET(window), TRUE);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  self->view = view;
  GdkRGBA background_color;
  if (self->is_settings_mode) {
    gdk_rgba_parse(&background_color, "#0D0D10");
  } else {
    gdk_rgba_parse(&background_color, "#00000000");
  }
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(window));
  gtk_widget_realize(GTK_WIDGET(view));

  fl_register_plugins(FL_PLUGIN_REGISTRY(view));

  // Register MethodChannel for dynamic click-through input region shaping
  FlEngine* engine = fl_view_get_engine(view);
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  self->window_channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(engine), "max/window",
      FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      self->window_channel, method_call_cb, self, nullptr);

  if (!self->is_settings_mode) {
    update_input_region(self, self->active_input_width, self->active_input_height);
  }
  gtk_widget_grab_focus(GTK_WIDGET(view));
}

// Implements GApplication::local_command_line.
static gboolean my_application_local_command_line(GApplication* application,
                                                  gchar*** arguments,
                                                  int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);

  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Failed to register: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }

  g_application_activate(application);
  *exit_status = 0;

  return TRUE;
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  g_clear_object(&self->window_channel);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line =
      my_application_local_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new(gboolean is_settings) {
  const gchar* app_id = is_settings ? "com.example.max_drop.settings" : "com.example.max_drop.notch";
  g_set_prgname(is_settings ? "max_settings" : "max_drop");

  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", app_id, "flags",
                                     G_APPLICATION_NON_UNIQUE, nullptr));
}
