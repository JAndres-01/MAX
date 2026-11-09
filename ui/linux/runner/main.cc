#include "my_application.h"
#include <string.h>

int main(int argc, char** argv) {
  gboolean is_settings = FALSE;
  for (int i = 1; i < argc; i++) {
    if (argv[i] != nullptr && strcmp(argv[i], "--settings") == 0) {
      is_settings = TRUE;
      break;
    }
  }
  g_autoptr(MyApplication) app = my_application_new(is_settings);
  return g_application_run(G_APPLICATION(app), argc, argv);
}
