//! AppKit crashes on window close: `_NSTouchBarFinderObservation` removes a
//! `nextResponder` observer that is already gone (`NSRangeException`). Catch
//! that throw so the traffic-light close and Command-Q can finish.

use objc::runtime::{Class, Object, Sel};
use std::os::raw::c_char;

type Invalidate = unsafe extern "C" fn(*mut Object, Sel);

static mut ORIGINAL: Option<Invalidate> = None;

extern "C" {
    fn class_getInstanceMethod(cls: *const Class, sel: Sel) -> *mut objc::runtime::Method;
    fn method_getImplementation(method: *const objc::runtime::Method) -> Invalidate;
    fn method_setImplementation(method: *mut objc::runtime::Method, imp: Invalidate) -> Invalidate;
    fn sel_registerName(name: *const c_char) -> Sel;
}

unsafe extern "C" fn invalidate_guard(this: *mut Object, sel: Sel) {
    let Some(original) = ORIGINAL else {
        return;
    };
    let _ = objc_exception::r#try(|| original(this, sel));
}

pub fn install() {
    unsafe {
        let Some(class) = Class::get("_NSTouchBarFinderObservation") else {
            return;
        };
        let sel = sel_registerName(c"invalidate".as_ptr());
        let method = class_getInstanceMethod(class, sel);
        if method.is_null() {
            return;
        }
        ORIGINAL = Some(method_getImplementation(method));
        method_setImplementation(method, invalidate_guard);
    }
}
