#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include "flutter_window.h"
#include "utils.h"

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  flutter::DartProject project(L"data");

  std::vector<std::string> command_line_arguments =
      GetCommandLineArguments();

  bool is_settings = false;
  for (const auto& arg : command_line_arguments) {
    if (arg == "--settings") {
      is_settings = true;
      break;
    }
  }

  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project, is_settings);
  int screen_width = ::GetSystemMetrics(SM_CXSCREEN);
  int screen_height = ::GetSystemMetrics(SM_CYSCREEN);

  Win32Window::Point origin = is_settings
      ? Win32Window::Point((screen_width - 820) / 2, (screen_height - 580) / 2)
      : Win32Window::Point((screen_width - 500) / 2, 0);

  Win32Window::Size size = is_settings
      ? Win32Window::Size(820, 580)
      : Win32Window::Size(500, 240);

  const wchar_t* title = is_settings ? L"MAX - Configuración" : L"MAX";

  if (!window.Create(title, origin, size)) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }

  ::CoUninitialize();
  return EXIT_SUCCESS;
}
