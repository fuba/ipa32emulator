/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationController`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_array, NSUInteger};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr, SEL,
};

// TODO: navigation bar and toolbar
// TODO: animations

#[derive(Default)]
struct UINavigationControllerHostObject {
    superclass: super::UIViewControllerHostObject,
    /// something implementing UINavigationControllerDelegate
    delegate: id,
    /// Navigation stack of view controllers, non-retaining
    /// (we explicitly retain/release on push/pop messages)
    navigation_stack: Vec<id>,
    navigation_bar: id,
    navigation_bar_hidden: bool,
}
impl_HostObject_with_superclass!(UINavigationControllerHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithRootViewController:(id)root_vc { // UIViewController *
    () = msg![env; this pushViewController:root_vc animated:false];
    this
}

// weak/non-retaining
- (())setDelegate:(id)delegate { // something implementing UINavigationControllerDelegate
    log_dbg!("[(UINavigationController*){:?} setDelegate:{:?}]", this, delegate);
    let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UINavigationControllerHostObject>(this).delegate
}

- (())pushViewController:(id)view_controller // UIViewController *
                animated:(bool)_animated {
    let stack = &mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack;
    assert!(!stack.contains(&view_controller));
    stack.push(view_controller);
    retain(env, view_controller);

    let delegate = env.objc.borrow::<UINavigationControllerHostObject>(this).delegate;
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:willShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this willShowViewController:view_controller animated:false];
    }
    let self_view: id = msg![env; this view];
    let vc_view: id = msg![env; view_controller view];
    // TODO: animations
    () = msg![env; view_controller viewWillAppear:false];
    () = msg![env; self_view addSubview:vc_view];
    () = msg![env; view_controller viewDidAppear:false];
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:didShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool  = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this didShowViewController:view_controller animated:false];
    }
}

- (id)topViewController {
    if let Some(top_vc) = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.last() {
        *top_vc
    } else {
        nil
    }
}

- (id)viewControllers {
    let vcs = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.to_vec();
    for vc in &vcs {
        retain(env, *vc);
    }
    let res = ns_array::from_vec(env, vcs);
    autorelease(env, res)
}
- (())setViewControllers:(id)controllers { // NSArray *
    msg![env; this setViewControllers:controllers animated:false]
}

- (())setViewControllers:(id)controllers // NSArray *
                animated:(bool)animated {
    assert!(!animated);

    // Clean existing view controllers
    let self_view = env.objc.borrow::<UINavigationControllerHostObject>(this).superclass.view;
    let mut stack = std::mem::take(&mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack);
    // TODO: shall we drain in reverse order? does it matter?
    for controller in stack.drain(..) {
        let vc_view = env.objc.borrow::<super::UIViewControllerHostObject>(controller).view;
        let vc_view_superview = msg![env; vc_view superview];
        assert_eq!(self_view, vc_view_superview);
        // TODO: view{Will,Did}Disappear: messages for vc?
        () = msg![env; vc_view removeFromSuperview];

        release(env, controller);
    }

    let mut tmp_stack: Vec<id> = Vec::new();
    let count: NSUInteger = msg![env; controllers count];
    // TODO: zero count
    assert!(count > 0);
    for i in 0..(count - 1) {
        let next: id = msg![env; controllers objectAtIndex:i];
        tmp_stack.push(next);
        retain(env, next);
    }
    env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack = tmp_stack;

    // The n-1 element in the controllers array is special and need to be pushed
    // TODO: double check this behavior
    let last_vc: id = msg![env; controllers objectAtIndex:(count - 1)];
    () = msg![env; this pushViewController:last_vc animated:animated];
}

- (id)navigationBar {
    let host = env.objc.borrow::<UINavigationControllerHostObject>(this);
    if host.navigation_bar != nil {
        return host.navigation_bar;
    }
    let hidden = host.navigation_bar_hidden;
    let bar: id = msg_class![env; UINavigationBar alloc];
    let bar: id = msg![env; bar initWithFrame:(CGRect::default())];
    env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_bar = bar;
    let view: id = msg![env; this view];
    let bounds: CGRect = msg![env; view bounds];
    let size: CGSize = msg![env; bar sizeThatFits:(bounds.size)];
    let frame = CGRect { origin: CGPoint::default(), size };
    () = msg![env; bar setFrame:frame];
    () = msg![env; bar setHidden:hidden];
    () = msg![env; view addSubview:bar];
    bar
}
- (bool)isNavigationBarHidden {
    env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_bar_hidden
}
- (())setNavigationBarHidden:(bool)hidden {
    msg![env; this setNavigationBarHidden:hidden animated:false]
}
- (())setNavigationBarHidden:(bool)hidden animated:(bool)_animated {
    let host = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host.navigation_bar_hidden = hidden;
    let bar = host.navigation_bar;
    msg![env; bar setHidden:hidden]
}
- (())dealloc {
    let host = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    let bar = std::mem::replace(&mut host.navigation_bar, nil);
    let controllers = std::mem::take(&mut host.navigation_stack);
    release(env, bar);
    for controller in controllers {
        release(env, controller);
    }
    msg_super![env; this dealloc]
}

@end

};
