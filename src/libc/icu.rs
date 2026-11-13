// SPDX-License-Identifier: MIT
//! Bridges the legacy ICU regex ABI to the Apple platform's ICU.

use crate::dyld::{export_c_func, HostDylib};
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};

const ILLEGAL_ARGUMENT: i32 = 1;
const REGEX_INVALID_STATE: i32 = 0x10302;

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct ParseError {
    line: i32,
    offset: i32,
    pre_context: [u16; 16],
    post_context: [u16; 16],
}
unsafe impl SafeRead for ParseError {}

type Regex = *mut c_void;

#[link(name = "icucore")]
extern "C" {
    #[link_name = "u_errorName"]
    fn error_name(code: i32) -> *const c_char;
    #[link_name = "uregex_open"]
    fn open(
        pattern: *const u16,
        len: i32,
        flags: u32,
        error: *mut ParseError,
        status: *mut i32,
    ) -> Regex;
    #[link_name = "uregex_close"]
    fn close(regex: Regex);
    #[link_name = "uregex_setText"]
    fn set_text(regex: Regex, text: *const u16, len: i32, status: *mut i32);
    #[link_name = "uregex_find"]
    fn find(regex: Regex, start: i32, status: *mut i32) -> i8;
    #[link_name = "uregex_findNext"]
    fn find_next(regex: Regex, status: *mut i32) -> i8;
    #[link_name = "uregex_groupCount"]
    fn group_count(regex: Regex, status: *mut i32) -> i32;
    #[link_name = "uregex_start"]
    fn start(regex: Regex, group: i32, status: *mut i32) -> i32;
    #[link_name = "uregex_end"]
    fn end(regex: Regex, group: i32, status: *mut i32) -> i32;
}

struct Entry {
    regex: Regex,
    // ICU borrows this buffer until the next successful setText or close.
    text: Vec<u16>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        unsafe { close(self.regex) };
    }
}

#[derive(Default)]
pub struct State {
    regexes: HashMap<MutVoidPtr, Entry>,
    error_names: HashMap<i32, ConstPtr<u8>>,
}

fn u_errorName(env: &mut Environment, code: i32) -> ConstPtr<u8> {
    if let Some(&name) = env.libc_state.icu.error_names.get(&code) {
        return name;
    }
    let name = unsafe { CStr::from_ptr(error_name(code)) };
    let name = env.mem.alloc_and_write_cstr(name.to_bytes()).cast_const();
    env.libc_state.icu.error_names.insert(code, name);
    name
}

fn u_strlen(env: &mut Environment, text: ConstPtr<u16>) -> i32 {
    let mut len = 0;
    while env.mem.read(text + len) != 0 {
        len += 1;
    }
    len as i32
}

fn guest_text(env: &mut Environment, text: ConstPtr<u16>, len: i32) -> Option<Vec<u16>> {
    if text.is_null() || len < -1 {
        return None;
    }
    let len = if len == -1 { u_strlen(env, text) } else { len };
    Some((0..len as u32).map(|i| env.mem.read(text + i)).collect())
}

fn uregex_open(
    env: &mut Environment,
    pattern: ConstPtr<u16>,
    len: i32,
    flags: u32,
    parse_error: MutPtr<ParseError>,
    status: MutPtr<i32>,
) -> MutVoidPtr {
    let mut code = env.mem.read(status);
    if code > 0 {
        return Ptr::null();
    }
    let Some(pattern) = guest_text(env, pattern, len) else {
        env.mem.write(status, ILLEGAL_ARGUMENT);
        return Ptr::null();
    };
    let mut error = ParseError::default();
    let regex = unsafe {
        open(
            pattern.as_ptr(),
            pattern.len() as i32,
            flags,
            &mut error,
            &mut code,
        )
    };
    if !parse_error.is_null() {
        env.mem.write(parse_error, error);
    }
    env.mem.write(status, code);
    if regex.is_null() {
        return Ptr::null();
    }
    let handle = env.mem.alloc(1);
    env.libc_state.icu.regexes.insert(
        handle,
        Entry {
            regex,
            text: Vec::new(),
        },
    );
    handle
}

fn uregex_close(env: &mut Environment, handle: MutVoidPtr) {
    if env.libc_state.icu.regexes.remove(&handle).is_some() {
        env.mem.free(handle);
    }
}

fn uregex_setText(
    env: &mut Environment,
    handle: MutVoidPtr,
    text: ConstPtr<u16>,
    len: i32,
    status: MutPtr<i32>,
) {
    let mut code = env.mem.read(status);
    if code > 0 {
        return;
    }
    let Some(text) = guest_text(env, text, len) else {
        env.mem.write(status, ILLEGAL_ARGUMENT);
        return;
    };
    if let Some(entry) = env.libc_state.icu.regexes.get_mut(&handle) {
        unsafe { set_text(entry.regex, text.as_ptr(), text.len() as i32, &mut code) };
        if code <= 0 {
            entry.text = text;
        }
    } else {
        code = REGEX_INVALID_STATE;
    }
    env.mem.write(status, code);
}

fn call<T>(
    env: &mut Environment,
    handle: MutVoidPtr,
    status: MutPtr<i32>,
    default: T,
    f: impl FnOnce(Regex, &mut i32) -> T,
) -> T {
    let mut code = env.mem.read(status);
    if code > 0 {
        return default;
    }
    let result = if let Some(entry) = env.libc_state.icu.regexes.get(&handle) {
        f(entry.regex, &mut code)
    } else {
        code = REGEX_INVALID_STATE;
        default
    };
    env.mem.write(status, code);
    result
}

fn uregex_find(env: &mut Environment, handle: MutVoidPtr, index: i32, status: MutPtr<i32>) -> bool {
    call(env, handle, status, false, |regex, code| unsafe {
        find(regex, index, code) != 0
    })
}

fn uregex_findNext(env: &mut Environment, handle: MutVoidPtr, status: MutPtr<i32>) -> bool {
    call(env, handle, status, false, |regex, code| unsafe {
        find_next(regex, code) != 0
    })
}

fn uregex_groupCount(env: &mut Environment, handle: MutVoidPtr, status: MutPtr<i32>) -> i32 {
    call(env, handle, status, 0, |regex, code| unsafe {
        group_count(regex, code)
    })
}

fn uregex_start(env: &mut Environment, handle: MutVoidPtr, group: i32, status: MutPtr<i32>) -> i32 {
    call(env, handle, status, -1, |regex, code| unsafe {
        start(regex, group, code)
    })
}

fn uregex_end(env: &mut Environment, handle: MutVoidPtr, group: i32, status: MutPtr<i32>) -> i32 {
    call(env, handle, status, -1, |regex, code| unsafe {
        end(regex, group, code)
    })
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libicucore.A.dylib",
    aliases: &["/usr/lib/libicucore.dylib"],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[&[
        export_c_func!(u_errorName(_)),
        export_c_func!(u_strlen(_)),
        export_c_func!(uregex_open(_, _, _, _, _)),
        export_c_func!(uregex_close(_)),
        export_c_func!(uregex_setText(_, _, _, _)),
        export_c_func!(uregex_find(_, _, _)),
        export_c_func!(uregex_findNext(_, _)),
        export_c_func!(uregex_groupCount(_, _)),
        export_c_func!(uregex_start(_, _, _)),
        export_c_func!(uregex_end(_, _, _)),
    ]],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_in_utf16_code_units_and_keeps_subject_alive() {
        let mut env = Environment::new_fake_for_test();
        let pattern = env.mem.alloc(8).cast::<u16>();
        for (index, unit) in "(x)".encode_utf16().chain([0]).enumerate() {
            env.mem.write(pattern + index as u32, unit);
        }
        let subject = env.mem.alloc(8).cast::<u16>();
        for (index, unit) in "😀xx".encode_utf16().enumerate() {
            env.mem.write(subject + index as u32, unit);
        }
        let status = env.mem.alloc_and_write(0);
        let regex = uregex_open(&mut env, pattern.cast_const(), -1, 0, Ptr::null(), status);
        assert!(!regex.is_null());
        uregex_setText(&mut env, regex, subject.cast_const(), 4, status);
        env.mem.free(subject.cast());
        assert!(uregex_find(&mut env, regex, 0, status));
        assert_eq!(uregex_groupCount(&mut env, regex, status), 1);
        assert_eq!(uregex_start(&mut env, regex, 1, status), 2);
        assert_eq!(uregex_end(&mut env, regex, 1, status), 3);
        assert!(uregex_findNext(&mut env, regex, status));
        assert_eq!(uregex_start(&mut env, regex, 1, status), 3);
        assert_eq!(env.mem.read(status), 0);
        uregex_close(&mut env, regex);
    }
}
