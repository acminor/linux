// SPDX-License-Identifier: GPL-2.0

//! Files system.
//!
//! C headers: [`include/linux/fs_parser.h`](../../../../include/linux/fs_parser.h) and
//! [`include/linux/fs_parser.h`](../../../../include/linux/fs_parser.h)

/// Corresponds to the __fsparam macro in C
#[doc(hidden)]
#[macro_export]
macro_rules! __fsparam {
    /* type: path, name: value, opt: value/path, flags: value/path, data: value/path */
    /* danielkeep little book of rust macros */
	  ($type_:expr, $name:expr, $opt:expr, $flags:expr, $data:expr) => {
        ::kernel::bindings::fs_parameter_spec {
            name: $name,
            opt: $opt,
            type_: $type_,
            flags: $flags,
            data: $data,
        }
	  };
}

/// Corresponds to the fsparam_u32oct macro in C
#[macro_export]
macro_rules! fsparam_u32oct {
	  ($name:literal, $opt:expr) => {
        $crate::__fsparam!(
            Some(::kernel::bindings::fs_param_is_u32),
            ::kernel::c_str!($name).as_char_ptr(),
            $opt as _,
            0,
            8 as _
        )
	  };
}
