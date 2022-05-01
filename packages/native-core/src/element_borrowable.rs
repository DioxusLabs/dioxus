pub trait ElementBorrowable {
    type Borrowed<'a>
    where
        Self: 'a;
    fn borrow_elements<'a>(&'a self) -> Self::Borrowed<'a>;
}

impl ElementBorrowable for () {
    type Borrowed<'a> = ()
    where
        Self: 'a;
    fn borrow_elements<'a>(&'a self) -> Self::Borrowed<'a> {
        ()
    }
}

macro_rules! impl_element_borrowable {
    ( $( ($x:tt, $i:ident) ),+ ) => {
        impl< $($x),+ > ElementBorrowable for ($($x,)+) {
            type Borrowed<'a> = ($(&'a $x,)+) where Self: 'a;
            fn borrow_elements<'a>(&'a self) -> Self::Borrowed<'a>{
                let ($($i,)+) = self;
                ($($i,)+)
            }
        }
    };
}

impl_element_borrowable! {(T1, t1)}
impl_element_borrowable! {(T1, t1), (T2, t2)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5), (T6, t6)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5), (T6, t6), (T7, t7)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5), (T6, t6), (T7, t7), (T8, t8)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5), (T6, t6), (T7, t7), (T8, t8), (T9, t9)}
impl_element_borrowable! {(T1, t1), (T2, t2), (T3, t3), (T4, t4), (T5, t5), (T6, t6), (T7, t7), (T8, t8), (T9, t9), (T10, t10)}
