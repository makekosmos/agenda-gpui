# Local patch vs crates.io gpui-pre 0.3.5

Companion half of the WM_NCHITTEST fix; see
vendor/gpui-pre-windows-0.3.5-patched/PATCH.md for the rationale.

Files changed:
- src/platform.rs — adds `PlatformWindow::hit_test_window_control_position`
  with a `None` default. The existing `on_hit_test_window_control` callback
  signature remains upstream-compatible, so non-Windows backend crates do not
  need local patches.
- src/window.rs — the closure hit-tests `rendered_frame` at the backend's
  in-flight hit-test position, falling back to `window.mouse_position`,
  instead of using the stale `window.mouse_hit_test` snapshot.
- src/window.rs — `bounds_changed` only marks the window dirty when
  `scale_factor`, `viewport_size` or `display_id` actually changed. Upstream
  refreshes on every move event, so each `WM_MOVE` during a titlebar drag
  triggered a full scene re-render inside the modal move loop and made the
  drag visibly judder; a pure position change produces an identical scene.
- src/window.rs — `dispatch_event` records `mouse_position` from
  `MouseExited`. Upstream leaves it at the last in-window point, so the
  per-frame `mouse_hit_test` recomputation resurrects hover on the element
  under the stale position after the cursor has already left.
- src/elements/div.rs — `update_hover` notifies the element's view after
  invoking the `on_hover` listener. Upstream leaves repainting to the
  listener, so `on_hover` callbacks that only mutate state (the common case)
  produce no redraw: hover transitions on elements without a `hover` style
  are never painted and the last hover visibly sticks.

The same local patch also prevents a duplicate Windows manifest:
- Cargo.toml — removes `windows-manifest` from the default feature list. The
  feature is still accepted for compatibility and can be force-enabled by
  `gpui-pre-platform`.
- build.rs — makes the `windows-manifest` embed a no-op. agenda-gpui embeds
  its own `windows/app.manifest` (Common Controls v6 + PerMonitorV2, a
  superset of gpui.manifest.xml) from the top-level build script; embedding
  a second RT_MANIFEST id=1 resource fails to link with CVTRES CVT1100 /
  LNK1123.
