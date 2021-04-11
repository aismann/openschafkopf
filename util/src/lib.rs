#[macro_use]
pub mod verify;
pub use self::verify::*;
pub mod iteratorext;
pub use self::iteratorext::*;
pub mod via_out_param;
pub use self::via_out_param::*;
#[macro_use]
pub mod mutate_return;
pub use mutate_return::*;
pub mod array_into_iter;
pub use array_into_iter::*;
#[macro_use]
pub mod box_clone;
#[macro_use]
pub mod staticvalue;
pub mod assign;
pub mod parser;
pub use self::{assign::*, box_clone::*, staticvalue::*};

// TODORUST static_assert not available in rust
#[macro_export]
macro_rules! static_assert{($assert_name:ident($($args:tt)*)) => {
    $assert_name!($($args)*)
}}

// TODORUST return impl
#[macro_export]
macro_rules! return_impl {
    ($t:ty) => {
        $t
    };
}

// TODORUST Objects should be upcastable to supertraits: https://github.com/rust-lang/rust/issues/5665
#[macro_export]
macro_rules! make_upcastable {
    ($upcasttrait:ident, $trait:ident) => {
        pub trait $upcasttrait {
            fn upcast(&self) -> &dyn $trait;
            fn upcast_box(self: Box<Self>) -> Box<dyn $trait>
            where
                Self: 'static;
        }
        impl<T: $trait> $upcasttrait for T {
            fn upcast(&self) -> &dyn $trait {
                self
            }
            fn upcast_box(self: Box<Self>) -> Box<dyn $trait>
            where
                Self: 'static,
            {
                self as Box<dyn $trait>
            }
        }
    };
}

#[macro_export]
macro_rules! if_then_some {
    ($cond: expr, $val: expr) => {
        if $cond {
            Some($val)
        } else {
            None
        }
    };
    (let $pattern:pat = $expr: expr, $val: expr) => {
        if let $pattern = $expr {
            Some($val)
        } else {
            None
        }
    };
}

pub fn tpl_flip_if<T>(b: bool, (t0, t1): (T, T)) -> (T, T) {
    if b {
        (t1, t0)
    } else {
        (t0, t1)
    }
}

#[macro_export]
macro_rules! cartesian_match(
    (
        $macro_callback: ident,
        $(match ($e: expr) {
            $($x: pat $(| $xs: pat)* => $y: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (),
            $(match ($e) {
                $($x $(| $xs)* => $y,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
        match ($e: expr) {
            $($x: pat $(| $xs: pat)* => $y: tt,)*
        },
        $(match ($e2: expr) {
            $($x2: pat $(| $xs2: pat)* => $y2: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (
                match ($e) {
                    $($x $(| $xs)* => $y,)*
                },
                $rest_packed,
            ),
            $(match ($e2) {
                $($x2 $(| $xs2)* => $y2,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
    ) => {
        cartesian_match!(@p1,
            $macro_callback,
            @matched{()},
            $rest_packed,
        )
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (
            match ($e: expr) {
                $($x: pat $(| $xs: pat)* => $y: tt,)*
            },
            $rest_packed: tt,
        ),
    ) => {
        match $e {
            $($x $(| $xs)* => cartesian_match!(@p1,
                $macro_callback,
                @matched{ ($matched_packed, $y,) },
                $rest_packed,
            ),)*
        }
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked(),
            $matched_packed,
        )
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (
            $rest_packed: tt,
            $y: tt,
        ),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked($($u,)* $y,),
            $rest_packed,
        )
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (),
    ) => {
        $macro_callback!($($u,)*)
    };
);

#[macro_export]
macro_rules! type_dispatch_enum{(pub enum $e: ident {$($v: ident ($t: ty),)+}) => {
    pub enum $e {
        $($v($t),)+
    }
    $(
        impl From<$t> for $e {
            fn from(t: $t) -> Self {
                $e::$v(t)
            }
        }
    )+
}}

// TODORUST some types should not implement copy (e.g. array), but do.
// In these cases, clippy warns about using clone on a copy type.
// To avoid this warning, use explicit_clone.
pub trait TExplicitClone: Clone {
    fn explicit_clone(&self) -> Self {
        self.clone()
    }
}
impl<T: Clone> TExplicitClone for T {}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_dbg)*
}}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_else)*
}}
