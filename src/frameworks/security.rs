// SPDX-License-Identifier: MIT
//! Internet-password storage isolated to the emulated application's filesystem.

use crate::dyld::{export_c_func, HostConstant, HostDylib};
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::mem::{ConstVoidPtr, MutPtr, Ptr};
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;
use plist::{Dictionary, Value};

const PARAM: i32 = -50;
const IO: i32 = -36;
const DUPLICATE_ITEM: i32 = -25299;
const ITEM_NOT_FOUND: i32 = -25300;
const ATTRIBUTES: &[&str] = &["acct", "srvr", "sdmn", "atyp", "type"];

fn query_value(env: &mut Environment, query: id, key: &'static str) -> id {
    let key = get_static_str(env, key);
    msg![env; query objectForKey:key]
}

fn attributes(env: &mut Environment, query: id) -> Option<Dictionary> {
    let class = query_value(env, query, "class");
    let internet_password = get_static_str(env, "inet");
    if !msg![env; class isEqualToString:internet_password] {
        return None;
    }
    let mut attrs = Dictionary::new();
    let string_class = env.objc.get_known_class("NSString", &mut env.mem);
    let number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    for &key in ATTRIBUTES {
        let value = query_value(env, query, key);
        if value == nil {
            continue;
        }
        let value = if msg![env; value isKindOfClass:string_class] {
            Value::String(to_rust_string(env, value).into_owned())
        } else if msg![env; value isKindOfClass:number_class] {
            let number: i64 = msg![env; value longLongValue];
            Value::Integer(number.into())
        } else {
            return None;
        };
        attrs.insert(key.into(), value);
    }
    Some(attrs)
}

fn load(env: &Environment) -> Result<Vec<Value>, i32> {
    let file = env
        .fs
        .home_directory()
        .join("Library")
        .join("Keychain.plist");
    if !env.fs.exists(file.as_ref()) {
        return Ok(Vec::new());
    }
    let bytes = env.fs.read(&file).map_err(|_| IO)?;
    let value = Value::from_reader(std::io::Cursor::new(bytes)).map_err(|_| IO)?;
    value.into_array().ok_or(IO)
}

fn save(env: &mut Environment, items: Vec<Value>) -> Result<(), i32> {
    let library = env.fs.home_directory().join("Library");
    env.fs.create_dir_all(library.clone()).map_err(|_| IO)?;
    let file = library.join("Keychain.plist");
    let staged = library.join("Keychain.plist.tmp");
    let mut bytes = Vec::new();
    Value::Array(items)
        .to_writer_binary(&mut bytes)
        .map_err(|_| IO)?;
    env.fs.write(&staged, &bytes).map_err(|_| IO)?;
    env.fs.rename(&staged, &file).map_err(|_| IO)
}

fn matches(item: &Value, attrs: &Dictionary) -> bool {
    item.as_dictionary().is_some_and(|item| {
        attrs
            .iter()
            .all(|(key, value)| item.get(key) == Some(value))
    })
}

fn SecItemCopyMatching(env: &mut Environment, query: id, result: MutPtr<id>) -> i32 {
    let Some(attrs) = attributes(env, query) else {
        return PARAM;
    };
    let items = match load(env) {
        Ok(items) => items,
        Err(error) => return error,
    };
    let Some(item) = items.iter().find(|item| matches(item, &attrs)) else {
        return ITEM_NOT_FOUND;
    };
    let return_data = query_value(env, query, "r_Data");
    if !result.is_null() && msg![env; return_data boolValue] {
        let Some(bytes) = item
            .as_dictionary()
            .and_then(|item| item.get("v_Data"))
            .and_then(Value::as_data)
        else {
            return IO;
        };
        let length = bytes.len().try_into().unwrap();
        let ptr: ConstVoidPtr = if bytes.is_empty() {
            Ptr::null()
        } else {
            let ptr = env.mem.alloc(length);
            env.mem
                .bytes_at_mut(ptr.cast(), length)
                .copy_from_slice(bytes);
            ptr.cast_const()
        };
        let data: id = msg_class![env; NSData alloc];
        let data: id = msg![env; data initWithBytes:ptr length:length];
        if !ptr.is_null() {
            env.mem.free(ptr.cast_mut());
        }
        env.mem.write(result, data);
    }
    0
}

fn SecItemAdd(env: &mut Environment, query: id, result: MutPtr<id>) -> i32 {
    let Some(mut attrs) = attributes(env, query) else {
        return PARAM;
    };
    let data = query_value(env, query, "v_Data");
    let data_class = env.objc.get_known_class("NSData", &mut env.mem);
    if !msg![env; data isKindOfClass:data_class] {
        return PARAM;
    }
    let len: u32 = msg![env; data length];
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let bytes = if len == 0 {
        Vec::new()
    } else {
        env.mem.bytes_at(bytes.cast(), len).to_vec()
    };
    let mut items = match load(env) {
        Ok(items) => items,
        Err(error) => return error,
    };
    if items.iter().any(|item| {
        item.as_dictionary().is_some_and(|item| {
            ATTRIBUTES
                .iter()
                .all(|key| item.get(*key) == attrs.get(*key))
        })
    }) {
        return DUPLICATE_ITEM;
    }
    attrs.insert("v_Data".into(), Value::Data(bytes));
    items.push(Value::Dictionary(attrs));
    if !result.is_null() {
        env.mem.write(result, nil);
    }
    save(env, items).err().unwrap_or(0)
}

fn SecItemDelete(env: &mut Environment, query: id) -> i32 {
    let Some(attrs) = attributes(env, query) else {
        return PARAM;
    };
    let mut items = match load(env) {
        Ok(items) => items,
        Err(error) => return error,
    };
    let count = items.len();
    items.retain(|item| !matches(item, &attrs));
    if items.len() == count {
        return ITEM_NOT_FOUND;
    }
    save(env, items).err().unwrap_or(0)
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Security.framework/Security",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[&[
        ("_kSecClass", HostConstant::NSString("class")),
        ("_kSecClassInternetPassword", HostConstant::NSString("inet")),
        ("_kSecAttrAccount", HostConstant::NSString("acct")),
        ("_kSecAttrServer", HostConstant::NSString("srvr")),
        ("_kSecAttrSecurityDomain", HostConstant::NSString("sdmn")),
        ("_kSecAttrType", HostConstant::NSString("type")),
        (
            "_kSecAttrAuthenticationType",
            HostConstant::NSString("atyp"),
        ),
        (
            "_kSecAttrAuthenticationTypeDefault",
            HostConstant::NSString("dflt"),
        ),
        ("_kSecMatchLimit", HostConstant::NSString("m_Limit")),
        ("_kSecMatchLimitOne", HostConstant::NSString("m_LimitOne")),
        ("_kSecReturnData", HostConstant::NSString("r_Data")),
        ("_kSecValueData", HostConstant::NSString("v_Data")),
    ]],
    function_exports: &[&[
        export_c_func!(SecItemCopyMatching(_, _)),
        export_c_func!(SecItemAdd(_, _)),
        export_c_func!(SecItemDelete(_)),
    ]],
};
