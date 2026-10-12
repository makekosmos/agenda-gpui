//! wasm stand-in for `mundus-gpui-kit` (lib name `mundus_gpui_kit`): Agenda
//! uses only `engine_error` — typed wire codes + Russian user text. Engine
//! transports (HTTP/WS) are native-only; the wasm build runs the in-memory
//! demo store and never reaches Engine code.
pub mod engine_error;
