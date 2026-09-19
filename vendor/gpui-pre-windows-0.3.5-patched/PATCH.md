# Local patch vs crates.io gpui-pre-windows 0.3.5

Same fix that used to live in vendor/gpui-0.2.2-patched; the backend moved
from the monolithic gpui crate to `gpui-pre-windows`.

`WM_NCHITTEST` resolves `window_control_area` hitboxes at the cursor position
carried by the message (`lparam` -> `ScreenToClient` -> logical px). The
backend stores that position while invoking the upstream-compatible no-arg
`PlatformWindow::on_hit_test_window_control` callback, and exposes it through
`PlatformWindow::hit_test_window_control_position`. Upstream instead asks
`Window` to match `mouse_hit_test`, which is only refreshed on dispatched
mouse events — one event stale — so caption/drag areas flapped between client
and non-client hit codes and hover stopped tracking the cursor.

Files changed:
- src/events.rs — `handle_hit_test_msg` computes the client point first,
  converts to logical pixels, and publishes it for the duration of the
  callback. Upstream ordering kept: `is_movable`/`is_resizable`/
  `is_minimizable` gates on control areas and the top-edge `HTTOP*` resize
  zone checked before `drag_area`.
- src/window.rs — adds `Callbacks::hit_test_window_control_position`,
  implements `PlatformWindow::hit_test_window_control_position`, keeps
  the upstream `on_hit_test_window_control` callback signature, and adds
  the `size_move_loop`/`size_move_resized` state cells used by events.rs.
- src/events.rs — `handle_mouse_leave_msg` now also dispatches
  `PlatformInput::MouseExited` (with the live cursor position) through the
  input callback. Upstream only toggles `hovered_status_change`, so GPUI
  never delivered `hovered=false` when the cursor left the client area and
  the last hovered element stayed highlighted forever.
- src/events.rs — `draw_window` skips painting inside a modal
  `WM_ENTERSIZEMOVE` loop until a `WM_SIZE` arrives. A pure position change
  leaves the scene identical (DWM translates the composited surface), so
  every timer/vsync-driven repaint just stalled the modal message pump and
  made titlebar dragging judder. `WM_ENTERSIZEMOVE`/`WM_EXITSIZEMOVE` are
  now tracked via `WindowsWindowState::size_move_loop`/`size_move_resized`
  (`WM_ENTERMENULOOP` keeps the timer but does not suppress painting), and
  `handle_size_msg` marks real resizes so live-resize still paints.

Mirrored in `vendor/gpui-pre-0.3.5-patched` (default trait method in
platform.rs and the `Window` closure in window.rs). The trait keeps the
upstream callback signature and only adds a provided method, so the
macOS/Linux/web backend crates continue to compile without local mirrors.

### set_background_appearance: reset the Mica system backdrop on leave

`set_background_appearance` set `DWMWA_SYSTEMBACKDROP_TYPE` for
Mica/MicaAlt but never cleared it, so switching back to Opaque/Blurred
left Mica rendering behind the window. The non-Mica arms now call
`dwm_set_window_composition_attribute(hwnd, 1)` (DWMSBT_NONE) before
restoring the legacy accent policy.

Files changed:
- src/window.rs — `Opaque`, `Transparent`, and `Blurred` arms of
  `set_background_appearance` reset the system backdrop type first.

### set_background_appearance: actually show the system backdrop

Upstream set `DWMWA_SYSTEMBACKDROP_TYPE` but never extended the glass
frame, so DWM had no frame region to draw the material into and Mica
rendered nothing. `dwm_set_window_composition_attribute` now calls
`DwmExtendFrameIntoClientArea` with `-1` margins while a backdrop is
active (and restores `0` when cleared), and returns success so the
`Blurred` arm can prefer the documented `DWMSBT_TRANSIENTWINDOW` acrylic
backdrop, falling back to the undocumented accent policy only on
pre-22621 builds where it silently did nothing anyway.

Files changed:
- src/window.rs — backdrop arms use `DWMSBT_*` constants, the helper
  extends/restores the glass frame and returns `bool`.

### set_window_appearance: app-level light/dark override

The trait already declares `set_window_appearance` (default no-op, used by
the macOS backend); the Windows backend ignored it, so window appearance
always followed the OS. Apps that offer their own light/dark theme
(Zed-style) need the window — and especially the DWM-drawn Mica/MicaAlt
material tint — to follow the app theme instead.

Files changed:
- src/platform.rs — `WindowsPlatform` gains a shared
  `appearance_override: Arc<Cell<Option<WindowAppearance>>>`, passes it to
  every new window through `WindowCreationInfo`, reports it from
  `window_appearance`, and implements `set_window_appearance`: stores the
  override, applies `configure_dwm_dark_mode` (which also tints the Mica
  backdrop) to every tracked hwnd, and fires `appearance_changed` so
  windows repaint.
- src/window.rs — `WindowsWindowState` stores the shared
  `appearance_override`; window creation resolves the effective
  appearance through it.
- src/events.rs — `ImmersiveColorSet` handling applies the override on
  top of the fresh system appearance, so OS theme flips do not clobber
  the app-pinned mode.
