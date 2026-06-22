//! [`View`] implementations for tuples, which join their elements in order.
//!
//! Tuples up to arity 128 implement [`View`]/[`ViewTemplate`]; wider sibling
//! lists are grouped through [`fragment`](super::fragment). The
//! `@impl_children` arm also threads dynamic-node children through
//! [`IntoViewChild`] so a tuple of mixed static views and dynamic nodes lowers
//! correctly.

use std::marker::PhantomData;

use crate::{DynamicValues, IntoDynNode};
use dioxus_core_template::TemplateRawTree;

use super::{IntoViewChild, View, ViewTemplate, dynamic_node};

struct StaticTupleViewChildMarker<T>(PhantomData<fn() -> T>);

macro_rules! impl_tuple_views {
    (($($name:ident $value:ident $marker:ident,)*) ;) => {};
    (($($name:ident $value:ident $marker:ident,)*) ; $next_name:ident $next_value:ident $next_marker:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl $($name $value $marker,)* $next_name $next_value $next_marker,);
        impl_tuple_views!(($($name $value $marker,)* $next_name $next_value $next_marker,) ; $($rest)*);
    };
    (@impl $first_name:ident $first_value:ident $first_marker:ident,) => {
        impl<$first_name: ViewTemplate> ViewTemplate for ($first_name,) {
            const TEMPLATE_TREE: &'static TemplateRawTree = $first_name::TEMPLATE_TREE;
        }

        impl<$first_name: View> View for ($first_name,) {
            #[inline]
            fn push(self, dynamic: &mut DynamicValues) {
                let ($first_value,) = self;
                $first_value.push(dynamic);
            }
        }

    };
    (@impl $first_name:ident $first_value:ident $first_marker:ident, $($name:ident $value:ident $marker:ident,)+) => {
        impl<$first_name: ViewTemplate, $($name: ViewTemplate),*> ViewTemplate for ($first_name, $($name,)*) {
            const TEMPLATE_TREE: &'static TemplateRawTree =
                &TemplateRawTree::Sequence(&[$first_name::TEMPLATE_TREE, $($name::TEMPLATE_TREE,)*]);
        }

        impl<$first_name: View, $($name: View),*> View for ($first_name, $($name,)*) {
            #[inline]
            fn push(self, dynamic: &mut DynamicValues) {
                let ($first_value, $($value,)*) = self;
                $first_value.push(dynamic);
                $($value.push(dynamic);)*
            }
        }

    };
    (@impl_children ($($before_name:ident $before_value:ident,)*) ;) => {};
    (@impl_children ($($before_name:ident $before_value:ident,)*) ; $dynamic_name:ident $dynamic_value:ident $dynamic_marker:ident, $($after_name:ident $after_value:ident $after_marker:ident,)*) => {
        impl_tuple_views!(
            @impl_child_at
            ($($before_name $before_value,)*)
            $dynamic_name $dynamic_value $dynamic_marker
            ($($after_name $after_value $after_marker,)*)
        );
        impl_tuple_views!(
            @impl_children
            ($($before_name $before_value,)* $dynamic_name $dynamic_value,)
            ;
            $($after_name $after_value $after_marker,)*
        );
    };
    (@impl_child_at ($($before_name:ident $before_value:ident,)*) $dynamic_name:ident $dynamic_value:ident $dynamic_marker:ident ($($after_name:ident $after_value:ident $after_marker:ident,)*)) => {
        impl<$($before_name,)* $dynamic_name, $dynamic_marker, $($after_name, $after_marker),*>
            IntoViewChild<($(
                StaticTupleViewChildMarker<$before_name>,
            )* dynamic_node::DynamicViewChildMarker<$dynamic_marker>, $($after_marker,)*)>
            for ($($before_name,)* $dynamic_name, $($after_name,)*)
        where
            $($before_name: View,)*
            $dynamic_name: IntoDynNode<$dynamic_marker>,
            $($after_name: IntoViewChild<$after_marker>),*
        {
            type Output = (
                $($before_name,)*
                dynamic_node::DynamicNodeBuilder<$dynamic_name, $dynamic_marker>,
                $(<$after_name as IntoViewChild<$after_marker>>::Output,)*
            );

            #[inline]
            fn into_child(self) -> Self::Output {
                let ($($before_value,)* $dynamic_value, $($after_value,)*) = self;
                (
                    $($before_value,)*
                    dynamic_node::dynamic_node_builder($dynamic_value),
                    $($after_value.into_child(),)*
                )
            }
        }
    };
}

macro_rules! impl_tuple_view_children {
    (($($name:ident $value:ident $marker:ident,)*) ;) => {};
    (($($name:ident $value:ident $marker:ident,)*) ; $next_name:ident $next_value:ident $next_marker:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl_children () ; $($name $value $marker,)* $next_name $next_value $next_marker,);
        impl_tuple_view_children!(($($name $value $marker,)* $next_name $next_value $next_marker,) ; $($rest)*);
    };
}

impl_tuple_views! {
    ();
    T00 t00 M00,
    T01 t01 M01,
    T02 t02 M02,
    T03 t03 M03,
    T04 t04 M04,
    T05 t05 M05,
    T06 t06 M06,
    T07 t07 M07,
    T08 t08 M08,
    T09 t09 M09,
    T10 t10 M10,
    T11 t11 M11,
    T12 t12 M12,
    T13 t13 M13,
    T14 t14 M14,
    T15 t15 M15,
    T16 t16 M16,
    T17 t17 M17,
    T18 t18 M18,
    T19 t19 M19,
    T20 t20 M20,
    T21 t21 M21,
    T22 t22 M22,
    T23 t23 M23,
    T24 t24 M24,
    T25 t25 M25,
    T26 t26 M26,
    T27 t27 M27,
    T28 t28 M28,
    T29 t29 M29,
    T30 t30 M30,
    T31 t31 M31,
    T32 t32 M32,
    T33 t33 M33,
    T34 t34 M34,
    T35 t35 M35,
    T36 t36 M36,
    T37 t37 M37,
    T38 t38 M38,
    T39 t39 M39,
    T40 t40 M40,
    T41 t41 M41,
    T42 t42 M42,
    T43 t43 M43,
    T44 t44 M44,
    T45 t45 M45,
    T46 t46 M46,
    T47 t47 M47,
    T48 t48 M48,
    T49 t49 M49,
    T50 t50 M50,
    T51 t51 M51,
    T52 t52 M52,
    T53 t53 M53,
    T54 t54 M54,
    T55 t55 M55,
    T56 t56 M56,
    T57 t57 M57,
    T58 t58 M58,
    T59 t59 M59,
    T60 t60 M60,
    T61 t61 M61,
    T62 t62 M62,
    T63 t63 M63,
    T64 t64 M64,
    T65 t65 M65,
    T66 t66 M66,
    T67 t67 M67,
    T68 t68 M68,
    T69 t69 M69,
    T70 t70 M70,
    T71 t71 M71,
    T72 t72 M72,
    T73 t73 M73,
    T74 t74 M74,
    T75 t75 M75,
    T76 t76 M76,
    T77 t77 M77,
    T78 t78 M78,
    T79 t79 M79,
    T80 t80 M80,
    T81 t81 M81,
    T82 t82 M82,
    T83 t83 M83,
    T84 t84 M84,
    T85 t85 M85,
    T86 t86 M86,
    T87 t87 M87,
    T88 t88 M88,
    T89 t89 M89,
    T90 t90 M90,
    T91 t91 M91,
    T92 t92 M92,
    T93 t93 M93,
    T94 t94 M94,
    T95 t95 M95,
    T96 t96 M96,
    T97 t97 M97,
    T98 t98 M98,
    T99 t99 M99,
    T100 t100 M100,
    T101 t101 M101,
    T102 t102 M102,
    T103 t103 M103,
    T104 t104 M104,
    T105 t105 M105,
    T106 t106 M106,
    T107 t107 M107,
    T108 t108 M108,
    T109 t109 M109,
    T110 t110 M110,
    T111 t111 M111,
    T112 t112 M112,
    T113 t113 M113,
    T114 t114 M114,
    T115 t115 M115,
    T116 t116 M116,
    T117 t117 M117,
    T118 t118 M118,
    T119 t119 M119,
    T120 t120 M120,
    T121 t121 M121,
    T122 t122 M122,
    T123 t123 M123,
    T124 t124 M124,
    T125 t125 M125,
    T126 t126 M126,
    T127 t127 M127,
}

impl_tuple_view_children! {
    ();
    T00 t00 M00,
    T01 t01 M01,
    T02 t02 M02,
    T03 t03 M03,
    T04 t04 M04,
    T05 t05 M05,
    T06 t06 M06,
    T07 t07 M07,
    T08 t08 M08,
    T09 t09 M09,
    T10 t10 M10,
    T11 t11 M11,
    T12 t12 M12,
    T13 t13 M13,
    T14 t14 M14,
    T15 t15 M15,
    T16 t16 M16,
    T17 t17 M17,
    T18 t18 M18,
    T19 t19 M19,
    T20 t20 M20,
    T21 t21 M21,
    T22 t22 M22,
    T23 t23 M23,
    T24 t24 M24,
    T25 t25 M25,
    T26 t26 M26,
    T27 t27 M27,
    T28 t28 M28,
    T29 t29 M29,
    T30 t30 M30,
    T31 t31 M31,
}
