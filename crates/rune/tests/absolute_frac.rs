//! `w_frac` on an absolute child resolves against its own parent's padding box: not a padded
//! ancestor's (which made it 12 pt too wide under a 6 pt padded grandparent), not the parent's
//! content box, and with no scrollbar gutter in a scroll viewport.
use rune::runtime::ElementInfo;
use rune::{Color, Cx, El, Env, Runtime, ScrollSpec, col, el, row};

struct S;

fn bar(key: &str) -> El {
    el().key(key).absolute(0.0, 0.0).w_frac(1.0).h(4.0)
}

fn view(_: &S, _: &mut Cx<S>) -> El {
    col().w_full().h_full().padding(6.0).gap(10.0)
        // Rooms' case: a padded card inside a padded column; the bar spans the card.
        .child(col().key("card").w_full().h(60.0).padding(6.0).border(1.0, Color::BLACK).child(bar("card-bar")))
        // Half and full widths in a row cell with its own padding.
        .child(row().w_full().h(40.0).child(
            col().key("cell").w(200.0).h_full().px(12.0)
                .child(bar("cell-bar"))
                .child(el().key("cell-half").absolute(0.0, 10.0).w_frac(0.5).h(4.0)),
        ))
        // Inside a padded scroll viewport's content: no gutter, no viewport padding added.
        .child(col().key("scroll").w(240.0).h(100.0).padding(6.0).scroll(ScrollSpec { stick_bottom: false })
            .child(col().key("content").w_full().h(300.0).child(bar("content-bar"))))
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    let e: ElementInfo = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("{key}"));
    e.rect
}

#[test]
fn absolute_fractions_span_the_parents_padding_box() {
    let mut rt = Runtime::new(S, view, (400.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let card = rect(&rt, "card");
    assert_eq!(card[2], 388.0);
    assert_eq!(rect(&rt, "card-bar"), [card[0], card[1], card[2], 4.0], "the card's padding box (its border paints inside)");

    let cell = rect(&rt, "cell");
    assert_eq!(rect(&rt, "cell-bar"), [cell[0], cell[1], 200.0, 4.0], "padding box, not the 176 pt content box");
    assert_eq!(rect(&rt, "cell-half"), [cell[0], cell[1] + 10.0, 100.0, 4.0]);

    let (scroll, content) = (rect(&rt, "scroll"), rect(&rt, "content"));
    assert_eq!(content[2], 228.0, "content fills the viewport inside its padding, with no gutter");
    assert_eq!(rect(&rt, "content-bar"), [content[0], content[1], 228.0, 4.0]);
    assert_eq!(content[0], scroll[0] + 6.0);
}

#[test]
fn h_frac_sizes_flow_and_absolute_children_by_parent_height() {
    fn view_h(_: &S, _: &mut Cx<S>) -> El {
        let mut quarter = el().key("flow-quarter").w(20.0);
        quarter.set_h_frac(0.25);

        col().w_full().h_full().padding(6.0).gap(10.0)
            // Flow children in a padded row resolve against the parent's content height (100 - 2*10 = 80).
            .child(
                row().key("flow-box").w(200.0).h(100.0).py(10.0).gap(8.0)
                    .child(el().key("flow-forty").w(20.0).h_frac(0.4))
                    .child(quarter),
            )
            // Absolute children in a padded/bordered card resolve against the card's padding box (60 pt).
            .child(
                col().key("abs-card").w(120.0).h(60.0).padding(6.0).border(1.0, Color::BLACK)
                    .child(el().key("abs-full").absolute(0.0, 0.0).w(4.0).h_frac(1.0))
                    .child(el().key("abs-half").absolute(10.0, 0.0).w(4.0).h_frac(0.5)),
            )
    }

    let mut rt = Runtime::new(S, view_h, (400.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);

    let flow_box = rect(&rt, "flow-box");
    assert_eq!(rect(&rt, "flow-forty"), [flow_box[0], flow_box[1] + 10.0, 20.0, 32.0]);
    assert_eq!(rect(&rt, "flow-quarter"), [flow_box[0] + 28.0, flow_box[1] + 10.0, 20.0, 20.0]);

    let abs_card = rect(&rt, "abs-card");
    assert_eq!(rect(&rt, "abs-full"), [abs_card[0], abs_card[1], 4.0, 60.0], "padding box height, not 48 pt content box");
    assert_eq!(rect(&rt, "abs-half"), [abs_card[0] + 10.0, abs_card[1], 4.0, 30.0]);
}
