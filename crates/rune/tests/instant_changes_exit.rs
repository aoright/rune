//! An element removed in the commit that puts `instant_changes` on an ancestor leaves at once,
//! unless the element itself stopped `instant_changes` with `animated_changes` (#1144).

use rune::{Cx, El, Env, Runtime, col, el};

struct S {
    lands: bool,
    child: bool,
    own: bool,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    let mut parent = col().key("parent").size(200.0, 100.0);
    if s.lands {
        parent = parent.instant_changes();
    }
    if s.child {
        let child = el().key("child").size(80.0, 24.0);
        parent = parent.child(if s.own { child.animated_changes() } else { child });
    }
    col().w_full().h_full().child(parent)
}

fn shows<T: 'static>(rt: &Runtime<T>, key: &str) -> bool {
    rt.elements().iter().any(|e| e.key.as_deref() == Some(key))
}

fn probe(before: bool, after: bool, own: bool) -> (bool, bool) {
    let mut rt = Runtime::new(S { lands: before, child: true, own }, view, (200.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(move |s, tx| {
        (s.lands, s.child) = (after, false);
        tx.animated();
    });
    let animating = rt.frame(1.0 / 60.0);
    (shows(&rt, "child"), animating)
}

#[test]
fn child_removed_when_parent_gains_instant_changes_leaves_at_once() {
    assert_eq!(probe(false, true, false), (false, false), "parent gains instant_changes in this commit");
    assert_eq!(probe(true, true, false), (false, false), "parent already had instant_changes");
    assert_eq!(probe(false, false, false), (true, true), "parent never has instant_changes");
    assert_eq!(probe(true, true, true), (true, true), "parent already had it, child has animated_changes");
    assert_eq!(probe(false, true, true), (true, true), "parent gains it, child already had animated_changes");
}

#[test]
fn grandchild_removed_when_grandparent_gains_instant_changes_leaves_at_once() {
    struct Deep {
        lands: bool,
        child: bool,
    }
    fn deep_view(s: &Deep, _: &mut Cx<Deep>) -> El {
        let mut middle = col().key("middle").size(200.0, 100.0);
        if s.child {
            middle = middle.child(el().key("leaf").size(80.0, 24.0));
        }
        let mut outer = col().key("outer").w_full().h_full().child(middle);
        if s.lands {
            outer = outer.instant_changes();
        }
        outer
    }
    let mut rt = Runtime::new(Deep { lands: false, child: true }, deep_view, (200.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|s, tx| {
        s.lands = true;
        s.child = false;
        tx.animated();
    });
    let animating = rt.frame(1.0 / 60.0);
    assert_eq!((shows(&rt, "leaf"), animating), (false, false));
}

#[test]
fn instant_changes_element_keeps_its_own_exit_under_normal_parent() {
    fn own_exit(present: &bool, _: &mut Cx<bool>) -> El {
        let mut root = col().w_full().h_full();
        if *present {
            root = root.child(el().key("popup").size(80.0, 24.0).instant_changes());
        }
        root
    }
    let mut rt = Runtime::new(true, own_exit, (200.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|present, tx| {
        *present = false;
        tx.animated();
    });
    let animating = rt.frame(1.0 / 60.0);
    assert_eq!((shows(&rt, "popup"), animating), (true, true));
}
