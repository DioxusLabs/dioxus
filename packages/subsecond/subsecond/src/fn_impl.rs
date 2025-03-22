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
                            let nibble /*   */ = real as u64 & 0xFF000_000_0000_0000;
                            let canonical_addr = real as u64 & 0x00FFF_FFF_FFFF_FFFF;
                            // let canonical_addr = real as u64 & 0x00FFFFFFFFFFFFFF;
                            // let canonical_addr = real as u64 & 0x00FFFFFFFFFFFFFF;
                            if let Some(ptr) = jump_table.map.get(&canonical_addr).cloned() {
                                println!("Detouring fat pointer ({canonical_addr:?}) {:#x} -> {:#x}", canonical_addr, ptr as u64);
                                println!("its nibble is {:#x}", nibble);
                                // apply the nibble
                                let ptr = ptr | nibble;
                                // align the ptr to 16 bytes
                                #[repr(C, align(8))]
                                struct AlignedPtr<T>(*const T);
                                let ptr = AlignedPtr(ptr as *const ());

                                let detoured = std::mem::transmute::<AlignedPtr<_>, Self::Real>(ptr);
                                #[allow(non_snake_case)]
                                let ( $($arg,)* ) = args;
                                return detoured($($arg),*);
                            } else {
                                let weare = std::any::type_name::<Self>();
                                println!("Could not find detour for {:?} while calling {weare}", real);
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
