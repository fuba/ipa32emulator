/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWebView`.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::msg;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg_class, msg_super, nil, objc_classes, release,
    ClassExports, NSZonePtr,
};
use crate::Environment;
use std::borrow::Cow;

#[derive(Default)]
struct UIWebViewHostObject {
    superclass: super::UIViewHostObject,
    delegate: id,
    pending_failure: id,
}
impl_HostObject_with_superclass!(UIWebViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWebView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UIWebViewHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    add_scroll_view(env, this);
    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    add_scroll_view(env, this);
    this
}

- (())setScalesPageToFit:(bool)_scales {
    // TODO
}
- (id)delegate {
    env.objc.borrow::<UIWebViewHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).delegate = delegate;
}
- (bool)isLoading {
    env.objc.borrow::<UIWebViewHostObject>(this).pending_failure != nil
}
- (())stopLoading {
    let pending = std::mem::replace(&mut env.objc.borrow_mut::<UIWebViewHostObject>(this).pending_failure, nil);
    release(env, pending);
}

- (())loadHTMLString:(id)_html baseURL:(id)_base_url {
    // This view has no HTML renderer. Report the load failure to its delegate.
    schedule_load_failure(env, this, -1002);
}

- (())loadRequest:(id)request { // NSURLRequest*
    let url_string = if request != nil {
        let url = msg![env; request URL];
        let url_desc = msg![env; url description];
        to_rust_string(env, url_desc)
    } else {
        Cow::default()
    };
    log!("TODO: [(UIWebView*) {:?} loadRequest:{:?} ({})]", this, request, url_string);
    schedule_load_failure(env, this, -1009);
}

- (())_touchHLE_webViewLoadFailed:(id)error {
    let host = env.objc.borrow_mut::<UIWebViewHostObject>(this);
    if host.pending_failure != error {
        return;
    }
    host.pending_failure = nil;
    let delegate = host.delegate;
    if delegate != nil && env.objc.object_has_method_named(&env.mem, delegate, "webView:didFailLoadWithError:") {
        () = msg![env; delegate webView:this didFailLoadWithError:error];
    }
    release(env, error);
}

- (())dealloc {
    () = msg![env; this stopLoading];
    msg_super![env; this dealloc]
}

@end

};

fn schedule_load_failure(env: &mut Environment, view: id, code: i32) {
    () = msg![env; view stopLoading];
    let domain = get_static_str(env, "NSURLErrorDomain");
    let error: id = msg_class![env; NSError alloc];
    let error: id = msg![env; error initWithDomain:domain code:code userInfo:nil];
    env.objc
        .borrow_mut::<UIWebViewHostObject>(view)
        .pending_failure = error;
    let callback = env
        .objc
        .lookup_selector("_touchHLE_webViewLoadFailed:")
        .unwrap();
    () = msg![env; view performSelectorOnMainThread:callback withObject:error waitUntilDone:false];
}

/// The page is shown in a scroll view, the web view's first subview.
/// Some games reach it through -subviews to hide its shadows.
fn add_scroll_view(env: &mut Environment, web_view: id) {
    let bounds: CGRect = msg![env; web_view bounds];
    let scroll_view: id = msg_class![env; UIScrollView alloc];
    let scroll_view: id = msg![env; scroll_view initWithFrame:bounds];
    () = msg![env; web_view addSubview:scroll_view];
    release(env, scroll_view);
}
