//! An unkeyed virtual list whose index path is later taken by a non-virtual element must not
//! leave stale [`VirtualList`] metadata behind at that path.
use std::borrow::Cow;

use rune::{Cx, El, Env, GrowthSpec, Runtime, ScrollSpec, col, el, text};

struct Toggle {
    list: bool,
}

fn unkeyed_replacement_view(s: &Toggle, _: &mut Cx<Toggle>) -> El {
    let body = if s.list {
        col()
            .w_full()
            .min_h(0.0)
            .virtual_rows(
                1,
                20.0,
                |_: &Toggle, i| Cow::Owned(format!("row-{i}")),
                |_: &Toggle, _| text("row"),
            )
            .growth(GrowthSpec::default())
    } else {
        col().child(text("a")).child(text("b"))
    };
    col()
        .key("pane")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: true })
        .child(col())
        .child(body)
}

#[test]
fn unkeyed_virtual_list_replaced_by_unkeyed_children_does_not_panic() {
    let mut rt = Runtime::new(
        Toggle { list: true },
        unkeyed_replacement_view,
        (400.0, 300.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    rt.frame(0.1);
    rt.update(|s, _| s.list = false);
    rt.frame(0.1);
    while rt.frame(1.0 / 60.0) {}

    let present: Vec<_> = rt
        .elements()
        .into_iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| e.text)
        .collect();
    assert_eq!(present, vec!["a", "b"]);

    // Switching back to the virtual list at the same path mounts cleanly again.
    rt.update(|s, _| s.list = true);
    rt.frame(0.1);
    while rt.frame(1.0 / 60.0) {}
    let present: Vec<_> = rt
        .elements()
        .into_iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| e.text)
        .collect();
    assert_eq!(present, vec!["row"]);
}

fn scrolled_keyed_replacement_view(s: &Toggle, _: &mut Cx<Toggle>) -> El {
    let body = if s.list {
        col()
            .w_full()
            .growth(GrowthSpec::default())
            .virtual_rows(
                50,
                40.0,
                |_: &Toggle, i| Cow::Owned(format!("row-{i}")),
                |_: &Toggle, i| el().key(format!("row-{i}")).w_full().h(40.0),
            )
    } else {
        col().w_full().child(el().key("empty-state").w_full().h(60.0))
    };
    col()
        .key("pane")
        .size(300.0, 200.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(body)
}

#[test]
fn scrolled_unkeyed_virtual_list_replaced_by_keyed_child_clears_culled_offset() {
    let mut rt = Runtime::new(
        Toggle { list: true },
        scrolled_keyed_replacement_view,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    rt.update(|_, tx| tx.scroll_to_instant("pane", 0.0, 1200.0));
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }

    rt.update(|s, _| s.list = false);
    for _ in 0..90 {
        rt.frame(1.0 / 60.0);
    }

    let elems = rt.elements();
    let empty = elems
        .iter()
        .find(|e| e.key.as_deref() == Some("empty-state") && !e.exiting)
        .expect("replacement child is mounted");
    assert!(
        empty.rect[1].abs() < 0.5,
        "replacement child rect.y was shifted by stale virtual list origin: {:?}",
        empty.rect
    );
    assert!(
        elems.iter().filter(|e| !e.exiting).all(|e| !e.virtual_extent),
        "replacement container must not report virtual_extent"
    );
}

fn scrolled_growth_replacement_view(s: &Toggle, _: &mut Cx<Toggle>) -> El {
    let body = if s.list {
        col()
            .w_full()
            .growth(GrowthSpec::default())
            .virtual_rows(
                50,
                40.0,
                |_: &Toggle, i| Cow::Owned(format!("row-{i}")),
                |_: &Toggle, i| el().key(format!("row-{i}")).w_full().h(40.0),
            )
    } else {
        col()
            .w_full()
            .growth(GrowthSpec::default())
            .child(el().key("banner").w_full().h(50.0))
    };
    col()
        .key("pane")
        .size(300.0, 200.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(body)
}

#[test]
fn scrolled_unkeyed_virtual_list_replaced_by_growth_stack_resets_stack_origin() {
    let mut rt = Runtime::new(
        Toggle { list: true },
        scrolled_growth_replacement_view,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    rt.update(|_, tx| tx.scroll_to_instant("pane", 0.0, 1200.0));
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }

    rt.update(|s, _| s.list = false);
    for _ in 0..90 {
        rt.frame(1.0 / 60.0);
    }

    let banner = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("banner") && !e.exiting)
        .expect("replacement growth row is mounted");
    assert!(
        banner.rect[1].abs() < 0.5 && (banner.rect[3] - 50.0).abs() < 0.5,
        "replacement growth row rect was corrupted by stale virtual stack state: {:?}",
        banner.rect
    );
}
