#include "flutter_window.h"

#include <dwmapi.h>
#include <flutter_windows.h>
#include <optional>

#include "flutter/generated_plugin_registrant.h"

FlutterWindow::FlutterWindow(const flutter::DartProject& project, bool is_settings)
    : project_(project), is_settings_(is_settings) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  HWND hwnd = GetHandle();

  // Enable true DWM transparency so the transparent canvas areas render seamlessly
  if (!is_settings_ && hwnd) {
    MARGINS margins = {-1, -1, -1, -1};
    ::DwmExtendFrameIntoClientArea(hwnd, &margins);
  }

  RECT frame = GetClientArea();

  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  // Register max/window channel for dynamic click-through input region shaping
  window_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "max/window",
      &flutter::StandardMethodCodec::GetInstance());

  window_channel_->SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
        if (call.method_name().compare("setInputRegion") == 0) {
          const auto* arguments = std::get_if<flutter::EncodableMap>(call.arguments());
          if (arguments) {
            int width = 240;
            int height = 24;
            auto width_it = arguments->find(flutter::EncodableValue("width"));
            if (width_it != arguments->end()) {
              if (std::holds_alternative<int>(width_it->second)) {
                width = std::get<int>(width_it->second);
              }
            }
            auto height_it = arguments->find(flutter::EncodableValue("height"));
            if (height_it != arguments->end()) {
              if (std::holds_alternative<int>(height_it->second)) {
                height = std::get<int>(height_it->second);
              }
            }
            SetInputRegion(width, height);
          }
          result->Success();
        } else {
          result->NotImplemented();
        }
      });

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    this->Show();
    if (!is_settings_) {
      SetInputRegion(240, 24);
      ::SetWindowPos(GetHandle(), HWND_TOPMOST, 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
  });

  if (!is_settings_ && hwnd) {
    ::SetTimer(hwnd, 1001, 1500, nullptr);
  }

  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::SetInputRegion(int width, int height) {
  if (is_settings_) {
    return;
  }
  HWND hwnd = GetHandle();
  if (!hwnd) {
    return;
  }

  // Calculate physical DPI scaling factor
  HMONITOR monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
  UINT dpi = FlutterDesktopGetDpiForMonitor(monitor);
  double scale_factor = dpi / 96.0;

  RECT client_rect{};
  GetClientRect(hwnd, &client_rect);
  int canvas_w = client_rect.right - client_rect.left;
  if (canvas_w <= 0) {
    canvas_w = static_cast<int>(500 * scale_factor);
  }

  int clamped_w = static_cast<int>((width > 500 ? 500 : (width < 40 ? 40 : width)) * scale_factor);
  int clamped_h = static_cast<int>((height > 240 ? 240 : (height < 8 ? 8 : height)) * scale_factor);
  if (clamped_w > canvas_w) {
    clamped_w = canvas_w;
  }

  int offset_x = (canvas_w - clamped_w) / 2;

  // Clip the window region so only the Notch bar receives mouse clicks;
  // all other areas of the 500x240 canvas pass clicks through to windows underneath.
  int radius = static_cast<int>(18 * scale_factor);
  HRGN rgn = CreateRoundRectRgn(offset_x, 0, offset_x + clamped_w, clamped_h, radius, radius);
  SetWindowRgn(hwnd, rgn, TRUE);

  // Keep topmost active after region adjustment
  ::SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0,
                 SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
}

void FlutterWindow::OnDestroy() {
  if (!is_settings_ && GetHandle()) {
    ::KillTimer(GetHandle(), 1001);
  }

  window_channel_ = nullptr;
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
    case WM_TIMER:
      if (wparam == 1001 && !is_settings_) {
        ::SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0,
                       SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
      }
      break;
    case WM_ACTIVATE:
    case WM_ACTIVATEAPP:
      if (!is_settings_) {
        ::SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0,
                       SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
      }
      break;
    case WM_WINDOWPOSCHANGING:
      if (!is_settings_) {
        WINDOWPOS* wp = reinterpret_cast<WINDOWPOS*>(lparam);
        if (wp) {
          wp->hwndInsertAfter = HWND_TOPMOST;
          wp->flags &= ~SWP_NOZORDER;
        }
      }
      break;
    case WM_DPICHANGED:
      if (!is_settings_) {
        HMONITOR monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        UINT dpi = FlutterDesktopGetDpiForMonitor(monitor);
        double scale_factor = dpi / 96.0;
        MONITORINFO mi = { sizeof(mi) };
        GetMonitorInfo(monitor, &mi);
        int mon_w = mi.rcMonitor.right - mi.rcMonitor.left;
        int win_w = static_cast<int>(500 * scale_factor);
        int win_h = static_cast<int>(240 * scale_factor);
        int win_x = mi.rcMonitor.left + (mon_w - win_w) / 2;
        int win_y = mi.rcMonitor.top;
        SetWindowPos(hwnd, HWND_TOPMOST, win_x, win_y, win_w, win_h, SWP_NOACTIVATE);
        SetInputRegion(240, 24);
        return 0;
      }
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}
