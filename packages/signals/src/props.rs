use crate::{ReadSignal, Signal};
use dioxus_core::SuperFrom;

#[doc(hidden)]
pub struct ReadFromMarker<M>(std::marker::PhantomData<M>);

impl<T, O, M> SuperFrom<T, ReadFromMarker<M>> for ReadSignal<O>
where
    O: SuperFrom<T, M> + 'static,
    T: 'static,
{
    fn super_from(input: T) -> Self {
        ReadSignal::new(Signal::new(O::super_from(input)))
    }
}

#[test]
#[allow(unused)]
fn into_signal_compiles() {
    use dioxus_core::SuperInto;
    fn takes_signal_string<M>(_: impl SuperInto<ReadSignal<String>, M>) {}

    fn takes_option_signal_string<M>(_: impl SuperInto<ReadSignal<Option<String>>, M>) {}

    fn don_t_run() {
        takes_signal_string("hello world");
        takes_signal_string(Signal::new(String::from("hello world")));
        takes_option_signal_string("hello world");
    }
}
