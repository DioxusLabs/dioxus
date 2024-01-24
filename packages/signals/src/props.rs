use crate::Signal;
use dioxus_core::prelude::*;

#[doc(hidden)]
pub struct SignalFromMarker<M>(std::marker::PhantomData<M>);

impl<T, O, M> SuperFrom<T, SignalFromMarker<M>> for Signal<O>
where
    O: SuperFrom<T, M>,
{
    fn super_from(input: T) -> Self {
        Signal::new(O::super_from(input))
    }
}

#[test]
#[allow(unused)]
fn into_signal_compiles() {
    fn takes_signal_string<M>(_: impl SuperInto<Signal<String>, M>) {}

    fn takes_option_signal_string<M>(_: impl SuperInto<Signal<Option<String>>, M>) {}

    fn don_t_run() {
        takes_signal_string("hello world");
        takes_signal_string(Signal::new(String::from("hello world")));
        takes_option_signal_string("hello world");
    }
}
