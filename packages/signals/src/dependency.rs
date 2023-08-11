/// A dependency is a trait that can be used to determine if a effect or selector should be re-run.
pub trait Dependency: Sized + Clone {
    /// The output of the dependency
    type Out: Clone + PartialEq;
    /// Returns the output of the dependency.
    fn out(&self) -> Self::Out;
    /// Returns true if the dependency has changed.
    fn changed(&self, other: &Self::Out) -> bool {
        self.out() != *other
    }
}

impl Dependency for () {
    type Out = ();
    fn out(&self) -> Self::Out {}
}

/// A dependency is a trait that can be used to determine if a effect or selector should be re-run.
pub trait Dep: 'static + PartialEq + Clone {}
impl<T> Dep for T where T: 'static + PartialEq + Clone {}

impl<A: Dep> Dependency for &A {
    type Out = A;
    fn out(&self) -> Self::Out {
        (*self).clone()
    }
}

macro_rules! impl_dep {
    (
        $($el:ident=$name:ident $other:ident,)*
    ) => {
        impl< $($el),* > Dependency for ($(&$el,)*)
        where
            $(
                $el: Dep
            ),*
        {
            type Out = ($($el,)*);

            fn out(&self) -> Self::Out {
                let ($($name,)*) = self;
                ($((*$name).clone(),)*)
            }

            fn changed(&self, other: &Self::Out) -> bool {
                let ($($name,)*) = self;
                let ($($other,)*) = other;
                $(
                    if *$name != $other {
                        return true;
                    }
                )*
                false
            }
        }
    };
}

impl_dep!(A = a1 a2,);
impl_dep!(A = a1 a2, B = b1 b2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2, G = g1 g2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2, G = g1 g2, H = h1 h2,);
