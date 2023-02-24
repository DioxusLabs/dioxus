// tests simple signal handling
use dioxus_signals::{with_rt, RuntimeOwner, Signal};
use std::sync::Arc;

#[test]
fn creation() {
    let rt = RuntimeOwner::new(Arc::new(|_| {}));
    let signal = Signal::new_in(*rt, 0);
    assert_eq!(signal.get(), 0);
    signal.update(|v| *v = 1);
    assert_eq!(signal.get(), 1);
}

#[test]
#[should_panic]
fn escape_runtime() {
    let rt = RuntimeOwner::new(Arc::new(|_| {}));
    let signal = Signal::new_in(*rt, 0);
    drop(rt);
    signal.get();
}

#[test]
fn drops() {
    let rt = RuntimeOwner::new(Arc::new(|_| {}));
    let signals = (0..10).map(|_| Signal::new_in(*rt, 0)).collect::<Vec<_>>();
    assert_eq!(with_rt(*rt, |rt| rt.size()), 10);
    with_rt(*rt, |rt| {
        for s in signals.iter() {
            rt.remove(s);
        }
    });
    assert_eq!(with_rt(*rt, |rt| rt.size()), 0);
    let signals = (0..1000)
        .map(|_| Signal::new_in(*rt, 0))
        .collect::<Vec<_>>();
    assert_eq!(with_rt(*rt, |rt| rt.size()), 1000);
    with_rt(*rt, |rt| {
        for s in signals.iter() {
            rt.remove(s);
        }
    });
    assert_eq!(with_rt(*rt, |rt| rt.size()), 0);
    drop(rt);
}
