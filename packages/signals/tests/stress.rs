use dioxus_signals::{with_rt, RuntimeOwner, Signal};
use std::sync::Arc;

#[test]
fn stress() {
    let rt = RuntimeOwner::new(Arc::new(|_| {}));
    let mut signals = Vec::new();
    for _ in 0..100000 {
        match rand::random::<u8>() % 3 {
            // add signals
            0 => {
                signals.push(Signal::new_in(*rt, 0));
                assert_eq!(with_rt(*rt, |rt| rt.size()), signals.len());
            }
            // remove signals
            1 => {
                if let Some(s) = signals.pop() {
                    with_rt(*rt, |rt| rt.remove(&s));
                    assert_eq!(with_rt(*rt, |rt| rt.size()), signals.len());
                }
            }
            // update signals
            2 => {
                if let Some(s) = signals.last_mut() {
                    let prev: i32 = s.get();
                    s.update(|v| *v = v.wrapping_add(1));
                    let new = prev.wrapping_add(1);
                    assert_eq!(s.get(), new);
                }
            }
            _ => unreachable!(),
        }
    }
}
