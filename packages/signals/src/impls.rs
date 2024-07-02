/// This macro is used to generate a `impl Default` block for any type with the function new_maybe_sync that takes a generic `T`
///
/// # Example
/// ```rust
/// use generational_box::*;
/// use dioxus::prelude::*;
///
/// struct MyCopyValue<T: 'static, S: Storage<T>> {
///     value: CopyValue<T, S>,
/// }
///
/// impl<T: 'static, S: Storage<T>> MyCopyValue<T, S> {
///     fn new_maybe_sync(value: T) -> Self {
///         Self { value: CopyValue::new_maybe_sync(value) }
///     }
/// }
///
/// impl<T: 'static, S: Storage<T>> Readable for MyCopyValue<T, S> {
///     type Target = T;
///     type Storage = S;
///
///     fn try_read_unchecked(
///         &self,
///     ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
///         self.value.try_read_unchecked()
///     }
///
///     fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
///         self.value.read_unchecked()
///     }
/// }
///
/// default_impl!(MyCopyValue<T, S: Storage<T>>);
/// ```
#[macro_export]
macro_rules! default_impl {
    (
        $ty:ident
        // Accept generics
        < T $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?
    ) => {
        impl<T: Default + 'static
            $(, $gen $(: $gen_bound)?)*
        > Default for $ty <T $(, $gen)*>
        $(
            where
                $(
                    $extra_bound_ty: $extra_bound
                ),+
        )?
        {
            #[track_caller]
            fn default() -> Self {
                Self::new_maybe_sync(Default::default())
            }
        }
    }
}

/// This macro is used to generate `impl Display`, `impl Debug`, `impl PartialEq`, and `impl Eq` blocks for any Readable type that takes a generic `T`
///
/// # Example
/// ```rust
/// use generational_box::*;
/// use dioxus::prelude::*;
///
/// struct MyCopyValue<T: 'static, S: Storage<T>> {
///     value: CopyValue<T, S>,
/// }
///
/// impl<T: 'static, S: Storage<T>> Readable for MyCopyValue<T, S> {
///     type Target = T;
///     type Storage = S;
///
///     fn try_read_unchecked(
///         &self,
///     ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
///         self.value.try_read_unchecked()
///     }
///
///     fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
///         self.value.read_unchecked()
///     }
/// }
///
/// read_impls!(MyCopyValue<T, S: Storage<T>>);
/// ```
#[macro_export]
macro_rules! read_impls {
    (
        $ty:ident
        // Accept generics
        < T $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?
    ) => {
        $crate::fmt_impls!{
            $ty<
                T
                $(
                    , $gen
                    $(: $gen_bound)?
                )*
            >
            $(
                where
                    $($extra_bound_ty: $extra_bound),*
            )?
        }
        $crate::eq_impls!{
            $ty<
                T
                $(
                    , $gen
                    $(: $gen_bound)?
                )*
            >
            $(
                where
                    $($extra_bound_ty: $extra_bound),*
            )?
        }
    };
}

/// This macro is used to generate `impl Display`, and `impl Debug` blocks for any Readable type that takes a generic `T`
///
/// # Example
/// ```rust
/// use generational_box::*;
/// use dioxus::prelude::*;
///
/// struct MyCopyValue<T: 'static, S: Storage<T>> {
///     value: CopyValue<T, S>,
/// }
///
/// impl<T: 'static, S: Storage<T>> Readable for MyCopyValue<T, S> {
///     type Target = T;
///     type Storage = S;
///
///     fn try_read_unchecked(
///         &self,
///     ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
///         self.value.try_read_unchecked()
///     }
///
///     fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
///         self.value.read_unchecked()
///     }
/// }
///
/// fmt_impls!(MyCopyValue<T, S: Storage<T>>);
/// ```
#[macro_export]
macro_rules! fmt_impls {
    (
        $ty:ident
        // Accept generics
        < T $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?
    ) => {
    impl<
        T: std::fmt::Display + 'static
        $(, $gen $(: $gen_bound)?)*
    > std::fmt::Display for $ty<T $(, $gen)*>
        $(
            where
                $($extra_bound_ty: $extra_bound,)*
        )?
    {
        #[track_caller]
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.with(|v| std::fmt::Display::fmt(v, f))
        }
    }

    impl<
        T: std::fmt::Debug + 'static
        $(, $gen $(: $gen_bound)?)*
    > std::fmt::Debug for $ty<T $(, $gen)*>
        $(
            where
                $($extra_bound_ty: $extra_bound,)*
        )?
    {
        #[track_caller]
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.with(|v| std::fmt::Debug::fmt(v, f))
        }
    }
};
    }

/// This macro is used to generate `impl PartialEq` blocks for any Readable type that takes a generic `T`
///
/// # Example
/// ```rust
/// use generational_box::*;
/// use dioxus::prelude::*;
///
/// struct MyCopyValue<T: 'static, S: Storage<T>> {
///     value: CopyValue<T, S>,
/// }
///
/// impl<T: 'static, S: Storage<T>> Readable for MyCopyValue<T, S> {
///     type Target = T;
///     type Storage = S;
///
///     fn try_read_unchecked(
///         &self,
///     ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
///         self.value.try_read_unchecked()
///     }
///
///     fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
///         self.value.read_unchecked()
///     }
/// }
///
/// eq_impls!(MyCopyValue<T, S: Storage<T>>);
/// ```
#[macro_export]
macro_rules! eq_impls {
    (
        $ty:ident
        // Accept generics
        < T $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?
    ) => {
        impl<
            T: PartialEq + 'static
            $(, $gen $(: $gen_bound)?)*
        > PartialEq<T> for $ty<T $(, $gen)*>
            $(
                where
                    $($extra_bound_ty: $extra_bound,)*
            )?
        {
            #[track_caller]
            fn eq(&self, other: &T) -> bool {
                self.with(|v| *v == *other)
            }
        }
    };
}

/// This macro is used to generate `impl Add`, `impl AddAssign`, `impl Sub`, `impl SubAssign`, `impl Mul`, `impl MulAssign`, `impl Div`, and `impl DivAssign` blocks for any Writable type that takes a generic `T`
///
/// # Example
/// ```rust, ignore
/// use generational_box::*;
/// use dioxus::prelude::*;
///
/// struct MyCopyValue<T: 'static, S: Storage<T>> {
///     value: CopyValue<T, S>,
/// }
///
/// impl<T: 'static, S: Storage<T>> Readable for MyCopyValue<T, S> {
///     type Target = T;
///     type Storage = S;
///
///     fn try_read_unchecked(
///         &self,
///     ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
///         self.value.try_read_unchecked()
///     }
///
///     fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
///         self.value.read_unchecked()
///     }
/// }
///
/// impl<T: 'static, S: Storage<T>> Writable for MyCopyValue<T, S> {
///     fn try_write_unchecked(
///         &self,
///     ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
///         self.value.try_write_unchecked()
///
///      }
///
///     //...
/// }
///
/// write_impls!(MyCopyValue<T, S: Storage<T>>);
/// ```
#[macro_export]
macro_rules! write_impls {
    (
        $ty:ident
        // Accept generics
        < T $(, $gen:ident $(: $gen_bound:path)?)* $(,)?>
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?) => {
        impl<T: std::ops::Add<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::Add<T>
            for $ty<T $(, $gen)*>
        {
            type Output = T;

            #[track_caller]
            fn add(self, rhs: T) -> Self::Output {
                self.with(|v| *v + rhs)
            }
        }

        impl<T: std::ops::Add<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::AddAssign<T>
            for $ty<T $(, $gen)*>
        {
            #[track_caller]
            fn add_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v + rhs)
            }
        }

        impl<T: std::ops::Sub<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::SubAssign<T>
            for $ty<T $(, $gen)*>
        {
            #[track_caller]
            fn sub_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v - rhs)
            }
        }

        impl<T: std::ops::Sub<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::Sub<T>
            for $ty<T $(, $gen)*>
        {
            type Output = T;

            #[track_caller]
            fn sub(self, rhs: T) -> Self::Output {
                self.with(|v| *v - rhs)
            }
        }

        impl<T: std::ops::Mul<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::MulAssign<T>
            for $ty<T $(, $gen)*>
        {
            #[track_caller]
            fn mul_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v * rhs)
            }
        }

        impl<T: std::ops::Mul<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::Mul<T>
            for $ty<T $(, $gen)*>
        {
            type Output = T;

            #[track_caller]
            fn mul(self, rhs: T) -> Self::Output {
                self.with(|v| *v * rhs)
            }
        }

        impl<T: std::ops::Div<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::DivAssign<T>
            for $ty<T $(, $gen)*>
        {
            #[track_caller]
            fn div_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v / rhs)
            }
        }

        impl<T: std::ops::Div<Output = T> + Copy + 'static
        $(, $gen $(: $gen_bound)?)*
        > std::ops::Div<T>
            for $ty<T $(, $gen)*>
        {
            type Output = T;

            #[track_caller]
            fn div(self, rhs: T) -> Self::Output {
                self.with(|v| *v / rhs)
            }
        }
    };
}
