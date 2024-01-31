use crate::{ReadOnlySignal, Signal};
use dioxus_core::prelude::*;

#[doc(hidden)]
pub struct SignalFromMarker<M>(std::marker::PhantomData<M>);

impl<T, O, M> SuperFrom<T, SignalFromMarker<M>> for ReadOnlySignal<O>
where
    O: SuperFrom<T, M>,
{
    fn super_from(input: T) -> Self {
        ReadOnlySignal::new(Signal::new(O::super_from(input)))
    }
}

#[test]
#[allow(unused)]
fn into_signal_compiles() {
    fn takes_signal_string<M>(_: impl SuperInto<ReadOnlySignal<String>, M>) {}

    fn takes_option_signal_string<M>(_: impl SuperInto<ReadOnlySignal<Option<String>>, M>) {}

    fn don_t_run() {
        takes_signal_string("hello world");
        takes_signal_string(Signal::new(String::from("hello world")));
        takes_option_signal_string("hello world");
    }
}
