//! On a virtual or manual clock, `Script::poll` advances the scheduler's clock even when no
//! task is due, so a later `onPress` or `Script::call` that invokes `task.wait` or `task.delay`
//! counts from the current frame rather than from the last task that ran (#1154).

use rune::{Cx, Env, Runtime};
use rune_luau::{Data, Script, Sources};

#[test]
fn on_press_task_wait_counts_from_latest_idle_poll_without_entering_vm() {
    let src = r#"
local ui = require("@rune/ui")
local step = 0
local waited = -1

return {
    render = function(view)
        return ui.col({ w = 200, h = 100 }, {
            ui.row({
                key = "btn",
                w = 100,
                h = 40,
                onPress = function()
                    step = 1
                    waited = task.wait(0.2)
                    step = 2
                end,
            }, { ui.text(`step={step}`) }),
        })
    end,
    state = function()
        return { step = step, waited = waited }
    end,
}
"#;
    let script = Script::new("virtual-clock", Sources::single(src));
    let mut rt = Runtime::new(
        script,
        |s: &Script, cx: &mut Cx<Script>| s.view(cx),
        (200.0, 100.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);

    let calls_before_idle = rt.state.metrics().vm_calls;
    for t in [1.0, 2.5, 5.0] {
        let mut ran = false;
        rt.update_if(|s, tx| {
            ran = s.poll(t, tx);
            ran
        });
        assert!(!ran, "idle poll at {t} should not run script code");
    }
    assert_eq!(
        rt.state.metrics().vm_calls,
        calls_before_idle,
        "idle polls must not enter the Luau VM"
    );

    // Click the button at virtual time t = 5.0.
    rt.pointer_move(20.0, 20.0);
    rt.pointer_down(20.0, 20.0);
    rt.pointer_up(20.0, 20.0);
    rt.frame(5.0);

    assert_eq!(rt.state.query("state", []).unwrap().get("step").as_i64(), Some(1));

    // 50 ms and 190 ms after the press (5.05 s and 5.19 s): the 0.2 s wait is not due yet.
    for t in [5.05, 5.19] {
        let mut ran = false;
        rt.update_if(|s, tx| {
            ran = s.poll(t, tx);
            ran
        });
        assert!(!ran, "task.wait(0.2) started at 5.0 must not wake early at {t}");
    }
    assert_eq!(rt.state.query("state", []).unwrap().get("step").as_i64(), Some(1));

    // At 5.25 s (>= 5.0 + 0.2 + any sub-ms wall elapsed), the task resumes.
    let mut woke = false;
    rt.update_if(|s, tx| {
        woke = s.poll(5.25, tx);
        woke
    });
    assert!(woke, "task.wait(0.2) should wake once virtual clock passes 5.2 s");
    let st = rt.state.query("state", []).unwrap();
    assert_eq!(st.get("step").as_i64(), Some(2));
    let waited = st.get("waited").as_f64().unwrap();
    assert!(
        (0.2..0.35).contains(&waited),
        "resumed task should receive ~0.25 s elapsed on virtual clock, got {waited}"
    );
}

#[test]
fn task_delay_after_idle_polls_counts_from_virtual_clock() {
    let src = r#"
local fired = false
return {
    arm = function()
        task.delay(0.3, function()
            fired = true
        end)
    end,
    fired = function()
        return fired
    end,
}
"#;
    let script = Script::new("delay-clock", Sources::single(src));
    let mut rt = Runtime::new(
        (),
        |_: &(), _: &mut Cx<()>| rune::el(),
        (10.0, 10.0),
        Env::new(true, 1.0),
    );

    rt.update_if(|_, tx| script.poll(10.0, tx));
    script.call("arm", []).unwrap();

    let mut early = false;
    rt.update_if(|_, tx| {
        early = script.poll(10.1, tx);
        early
    });
    assert!(!early, "task.delay(0.3) armed at 10.0 must not fire at 10.1");
    assert_eq!(script.query("fired", []).unwrap(), Data::Bool(false));

    let mut due = false;
    rt.update_if(|_, tx| {
        due = script.poll(10.35, tx);
        due
    });
    assert!(due, "task.delay(0.3) armed at 10.0 must fire at 10.35");
    assert_eq!(script.query("fired", []).unwrap(), Data::Bool(true));
}
