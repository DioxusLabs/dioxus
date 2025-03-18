use super::APP_JUMP_TABLE;

pub trait HotFunction<Args, Marker> {
    type Return;
    type Real;

    // rust-call isnt' stable, so we wrap the underyling call with our own, giving it a stable vtable entry
    fn call_it(&mut self, args: Args) -> Self::Return;

    // call this as if it were a real function pointer. This is very unsafe
    unsafe fn call_as_ptr(&mut self, _args: Args) -> Self::Return;
}

macro_rules! impl_hot_function {
    (
        $(
            ($marker:ident, $($arg:ident),*)
        ),*
    ) => {
        $(
            pub struct $marker;
            impl<T, $($arg,)* R> HotFunction<($($arg,)*), $marker> for T
            where
                T: FnMut($($arg),*) -> R,
            {
                type Return = R;
                type Real = fn($($arg),*) -> R;

                fn call_it(&mut self, args: ($($arg,)*)) -> Self::Return {
                    #[allow(non_snake_case)]
                    let ( $($arg,)* ) = args;
                    self($($arg),*)
                }

                unsafe fn call_as_ptr(&mut self, args: ($($arg,)*)) -> Self::Return {
                    unsafe {
                        if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                            let real = std::mem::transmute_copy::<Self, Self::Real>(&self);
                            let real = real as *const ();
                            if let Some(ptr) = jump_table.map.get(&(real as u64)).cloned() {
                                let detoured = std::mem::transmute::<*const (), Self::Real>(ptr as *const ());
                                #[allow(non_snake_case)]
                                let ( $($arg,)* ) = args;
                                return detoured($($arg),*);
                            }
                        }

                        self.call_it(args)
                    }
                }
            }
        )*
    };
}

impl_hot_function!(
    (Fn0Marker,),
    (Fn1Marker, A),
    (Fn2BMarker, A, B),
    (Fn3BCMarker, A, B, C)
);
