// SPDX-License-Identifier: GPL-2.0

//! File system.
//!
//! C headers: [`include/linux/fs_parser.h`](../../../../include/linux/fs_parser.h) and
//! [`include/linux/fs_parser.h`](../../../../include/linux/fs_parser.h)

/// TODO: Unsure of the total implications of this
/// - It behaved safely on the C side being sent to and fro
/// - Rust might do some different compiler tricks on this (unsure)
/// - This is necessary for Rust RamFS to have fs_parameter_spec info
///   as a static
/// - https://doc.rust-lang.org/nomicon/send-and-sync.html
unsafe impl Sync for crate::bindings::fs_parameter_spec {}

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
