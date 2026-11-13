/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSMethodSignature`.
//!
//! Resources:
//! - [Type encodings](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtTypeEncodings.html#//apple_ref/doc/uid/TP40008048-CH100)
//! - [Method signatures](https://gcc.gnu.org/onlinedocs/gcc-4.9.0/gcc/Method-signatures.html)

use crate::dyld::{export_c_func, FunctionExports};
use crate::environment::Environment;
use crate::frameworks::foundation::NSUInteger;
use crate::libc::string::strncpy;
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};

struct NSMethodSignatureHostObject {
    return_type: ConstPtr<u8>,
    arg_types: Vec<(ConstPtr<u8>, GuestUSize)>,
}
impl HostObject for NSMethodSignatureHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSMethodSignature: NSObject

+ (id)signatureWithObjCTypes:(ConstPtr<u8>)types {
    let (return_type, arg_types) = parse_signature(env, types);
    let host_object = Box::new(NSMethodSignatureHostObject {
        return_type,
        arg_types,
    });
    let res = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, res)
}

- (NSUInteger)numberOfArguments {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).arg_types.len().try_into().unwrap()
}

- (ConstPtr<u8>)getArgumentTypeAtIndex:(NSUInteger)idx {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).arg_types[idx as usize].0
}

- (ConstPtr<u8>)methodReturnType {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).return_type
}

- (NSUInteger)methodReturnLength {
    let ty = env.objc.borrow::<NSMethodSignatureHostObject>(this).return_type;
    type_layout(env.mem.cstr_at(ty), 0).unwrap().1
}

- (())dealloc {
    let host_obj = env.objc.borrow::<NSMethodSignatureHostObject>(this);
    env.mem.free(host_obj.return_type.cast_mut().cast());
    for (arg_type, _) in &host_obj.arg_types {
        env.mem.free((*arg_type).cast_mut().cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

fn parse_signature(
    env: &mut Environment,
    sig: ConstPtr<u8>,
) -> (ConstPtr<u8>, Vec<(ConstPtr<u8>, GuestUSize)>) {
    // first parse return type and total size in bytes
    let (scanned, return_type_idx, total_size) = parse_signature_inner(env, sig);
    let return_type_str: ConstPtr<u8> = env.mem.calloc(return_type_idx + 1).cast().cast_const();
    _ = strncpy(env, return_type_str.cast_mut(), sig, return_type_idx);
    log_dbg!(
        "Return type: '{}'; Total size {} bytes",
        env.mem.cstr_at_utf8(return_type_str).unwrap(),
        total_size
    );
    let mut scan_idx = scanned;
    let mut res = Vec::new();
    loop {
        let c = env.mem.read(sig + scan_idx);
        if c == b'\0' {
            break;
        }
        let (scanned, arg_type_idx, offset) = parse_signature_inner(env, sig + scan_idx);
        let arg_type_str: ConstPtr<u8> = env.mem.calloc(arg_type_idx + 1).cast().cast_const();
        _ = strncpy(env, arg_type_str.cast_mut(), sig + scan_idx, arg_type_idx);
        log_dbg!(
            "Arg {} type '{}' at offset {}",
            res.len(),
            env.mem.cstr_at_utf8(arg_type_str).unwrap(),
            offset
        );
        res.push((arg_type_str, offset));
        scan_idx += scanned;
    }
    (return_type_str, res)
}

fn parse_signature_inner(env: &mut Environment, curr: ConstPtr<u8>) -> (GuestUSize, u32, u32) {
    let mut idx = 0;
    let c = env.mem.read(curr);
    match c {
        // pointer, or a type qualifier such as `r` for const
        b'^' | b'r' | b'n' | b'N' | b'o' | b'O' | b'R' | b'V' => {
            let (scanned, read, size) = parse_signature_inner(env, curr + 1);
            (scanned + 1, read + 1, size)
        }
        b'v' | b'@' | b'#' | b':' | b'*' | b'c' | b'C' | b's' | b'S' | b'i' | b'I' | b'l'
        | b'L' | b'q' | b'Q' | b'f' | b'd' | b'B' => {
            idx += 1;
            let mut size = 0;
            while let cc @ b'0'..=b'9' = env.mem.read(curr + idx) {
                size = size * 10 + (cc - b'0') as u32;
                idx += 1;
            }
            (idx, 1, size)
        }
        _ => unimplemented!("parse_signature_inner: {}", c as char),
    }
}

fn NSGetSizeAndAlignment(
    env: &mut Environment,
    encoding: ConstPtr<u8>,
    size: MutPtr<NSUInteger>,
    alignment: MutPtr<NSUInteger>,
) -> ConstPtr<u8> {
    let (end, bytes, align) = type_layout(env.mem.cstr_at(encoding), 0).unwrap_or_else(|| {
        panic!(
            "Unsupported Objective-C type encoding {:?}",
            env.mem.cstr_at_utf8(encoding)
        )
    });
    if !size.is_null() {
        env.mem.write(size, bytes);
    }
    if !alignment.is_null() {
        env.mem.write(alignment, align);
    }
    encoding + end as u32
}

fn type_layout(mut encoding: &[u8], depth: u32) -> Option<(usize, u32, u32)> {
    if depth > 64 {
        return None;
    }
    let original_length = encoding.len();
    while encoding
        .first()
        .is_some_and(|code| b"rnNoORV".contains(code))
    {
        encoding = &encoding[1..];
    }
    let qualifiers = original_length - encoding.len();
    let (&code, tail) = encoding.split_first()?;
    let (len, size, align) = match code {
        b'v' => (1, 0, 1),
        b'B' | b'c' | b'C' => (1, 1, 1),
        b's' | b'S' => (1, 2, 2),
        b'i' | b'I' | b'l' | b'L' | b'f' | b'*' | b'#' | b':' => (1, 4, 4),
        b'q' | b'Q' | b'd' => (1, 8, 8),
        b'@' if tail.first() == Some(&b'?') => (2, 4, 4),
        b'@' if tail.first() == Some(&b'"') => {
            (3 + tail[1..].iter().position(|&c| c == b'"')?, 4, 4)
        }
        b'@' => (1, 4, 4),
        b'^' if tail.first() == Some(&b'?') => (2, 4, 4),
        b'^' => (1 + type_layout(tail, depth + 1)?.0, 4, 4),
        b'[' => {
            let count_len = tail.iter().take_while(|c| c.is_ascii_digit()).count();
            let count: u32 = std::str::from_utf8(&tail[..count_len]).ok()?.parse().ok()?;
            let (len, size, align) = type_layout(&tail[count_len..], depth + 1)?;
            if tail.get(count_len + len) != Some(&b']') {
                return None;
            }
            (2 + count_len + len, size.checked_mul(count)?, align)
        }
        b'{' | b'(' => {
            let close = if code == b'{' { b'}' } else { b')' };
            let separator = tail.iter().position(|&c| c == b'=' || c == close)?;
            let mut index = separator + 1;
            let mut size: u32 = 0;
            let mut alignment = 1;
            if tail[separator] == b'=' {
                while *tail.get(index)? != close {
                    if tail[index] == b'"' {
                        index += 2 + tail[index + 1..].iter().position(|&c| c == b'"')?;
                    }
                    let (len, field_size, field_align) = type_layout(&tail[index..], depth + 1)?;
                    // The 32-bit Darwin ARM aggregate ABI caps member alignment at four.
                    let field_align = field_align.min(4);
                    size = if code == b'{' {
                        size.checked_next_multiple_of(field_align)?
                            .checked_add(field_size)?
                    } else {
                        size.max(field_size)
                    };
                    alignment = alignment.max(field_align);
                    index += len;
                }
                index += 1;
            }
            (
                1 + index,
                size.checked_next_multiple_of(alignment)?,
                alignment,
            )
        }
        _ => return None,
    };
    Some((qualifiers + len, size, align))
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSGetSizeAndAlignment(_, _, _))];

#[cfg(test)]
mod tests {
    use super::type_layout;

    #[test]
    fn uses_guest_pointer_and_aggregate_alignment() {
        assert_eq!(type_layout(b"r^@12", 0), Some((3, 4, 4)));
        assert_eq!(type_layout(b"d", 0), Some((1, 8, 8)));
        assert_eq!(type_layout(b"{S=cd}", 0), Some((6, 12, 4)));
        assert_eq!(type_layout(b"[3{S=cd}]", 0), Some((9, 36, 4)));
    }
}
