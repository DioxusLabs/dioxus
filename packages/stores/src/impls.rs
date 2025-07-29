#[macro_export]
macro_rules! store_impls {
    (
        $raw_ty:ident
        // Accept generics
       $(
            <$($raw_gen:ident $(: $raw_gen_bound:path)?),*>
       )?

        =>

        $ty:ident
        // Accept generics
        <$write:ident $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?) => {
        impl<$write $(, $gen $(: $gen_bound)?)*>
        PartialEq for $ty<$write $(, $gen)*>
        where
            $write: PartialEq,
            $($($extra_bound_ty: $extra_bound),*)?
        {
            fn eq(&self, other: &Self) -> bool {
                self.selector == other.selector
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        Clone for $ty<$write $(, $gen)*>
        where
            $write: Clone,
            $($($extra_bound_ty: $extra_bound),*)?
        {
            fn clone(&self) -> Self {
                Self {
                    selector: self.selector.clone(),
                    _phantom: PhantomData,
                }
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*> Copy for $ty<$write $(, $gen)*>
        where
            $write: Copy,
            $($($extra_bound_ty: $extra_bound),*)?
        {}

        impl<__F, __FMut, $write $(, $gen $(: $gen_bound)?)*>
        ::std::convert::From<$ty<MappedMutSignal<$raw_ty $(< $($raw_gen),*>)?, $write, __F, __FMut> $(, $gen)*>>
        for $ty<WriteSignal<$raw_ty $(< $($raw_gen),*>)?> $(, $gen)*>
        where
            $write: $crate::macro_helpers::dioxus_signals::Writable<Storage = UnsyncStorage> + 'static,
            __F: Fn(&$write::Target) -> &$raw_ty $(< $($raw_gen),*>)? + 'static,
            __FMut: Fn(&mut $write::Target) -> &mut $raw_ty $(< $($raw_gen),*>)? + 'static,
        {
            fn from(value: $ty<MappedMutSignal<$raw_ty $(< $($raw_gen),*>)?, $write, __F, __FMut> $(, $gen)*>) -> Self {
                $ty {
                    selector: value.selector.map(::std::convert::Into::into),
                    _phantom: PhantomData,
                }
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        $crate::macro_helpers::dioxus_signals::Readable for $ty<$write $(, $gen)*>
        where
            $write: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?> + 'static,
            $raw_ty $(< $($raw_gen),*>)?: 'static,
        {
            type Storage = $write::Storage;
            type Target = $raw_ty $(< $($raw_gen),*>)?;

            fn try_read_unchecked(
                &self,
            ) -> Result<
                $crate::macro_helpers::dioxus_signals::ReadableRef<'static, Self>,
                $crate::macro_helpers::dioxus_signals::BorrowError,
            > {
                self.selector.try_read_unchecked()
            }

            fn try_peek_unchecked(
                &self,
            ) -> Result<
                $crate::macro_helpers::dioxus_signals::ReadableRef<'static, Self>,
                $crate::macro_helpers::dioxus_signals::BorrowError,
            > {
                self.selector.try_peek_unchecked()
            }

            fn subscribers(&self) -> Option<$crate::macro_helpers::dioxus_core::Subscribers> {
                self.selector.subscribers()
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        $crate::macro_helpers::dioxus_signals::Writable for $ty<$write $(, $gen)*>
        where
            $write: $crate::macro_helpers::dioxus_signals::Writable<Target = $raw_ty $(< $($raw_gen),*>)?> + 'static,
            $raw_ty $(< $($raw_gen),*>)?: 'static,
        {
            type WriteMetadata = $write::WriteMetadata;

            fn try_write_unchecked(
                &self,
            ) -> Result<
                $crate::macro_helpers::dioxus_signals::WritableRef<'static, Self>,
                $crate::macro_helpers::dioxus_signals::BorrowMutError,
            > {
                self.selector.try_write_unchecked()
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        ::std::fmt::Debug for $ty<$write $(, $gen)*>
        where
            Self: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?>,
            $raw_ty $(< $($raw_gen),*>)?: ::std::fmt::Debug + 'static,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.read().fmt(f)
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        ::std::fmt::Display for $ty<$write $(, $gen)*>
        where
            Self: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?>,
            $raw_ty $(< $($raw_gen),*>)?: ::std::fmt::Display + 'static,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.read().fmt(f)
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        $crate::macro_helpers::dioxus_core::IntoAttributeValue for $ty<$write $(, $gen)*>
        where
            Self: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?>,
            $raw_ty $(< $($raw_gen),*>)?: ::std::clone::Clone + $crate::macro_helpers::dioxus_core::IntoAttributeValue + 'static,
        {
            fn into_value(self) -> $crate::macro_helpers::dioxus_core::AttributeValue {
                self.cloned().into_value()
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        $crate::macro_helpers::dioxus_core::IntoDynNode for $ty<$write $(, $gen)*>
        where
            Self: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?>,
            $raw_ty $(< $($raw_gen),*>)?: ::std::clone::Clone + $crate::macro_helpers::dioxus_core::IntoDynNode + 'static,
        {
            fn into_dyn_node(self) -> $crate::macro_helpers::dioxus_core::DynamicNode {
                self.cloned().into_dyn_node()
            }
        }

        impl<$write $(, $gen $(: $gen_bound)?)*>
        ::std::ops::Deref for $ty<$write $(, $gen)*>
        where
            Self: $crate::macro_helpers::dioxus_signals::Readable<Target = $raw_ty $(< $($raw_gen),*>)?> + 'static,
            $raw_ty $(< $($raw_gen),*>)?: ::std::clone::Clone + 'static,
        {
            type Target = dyn Fn() -> $raw_ty $(< $($raw_gen),*>)?;

            fn deref(&self) -> &Self::Target {
                unsafe { $crate::macro_helpers::dioxus_signals::ReadableExt::deref_impl(self) }
            }
        }
    };
}
