// SPDX-License-Identifier: GPL-2.0

//! Sequence file printing facilities.
//!
//! C header: [`include/linux/seq_file.h`](../../../../include/linux/seq_file.h)
//!
//! Reference: <https://www.kernel.org/doc/html/latest/core-api/printk-basics.html>

use core::fmt;

use crate::bindings;
use crate::c_types::c_void;

/// Contains format strings for seq_printf files
pub mod format_strings {
    /// Default format string for seq_printf files
    pub const FORMAT_STRING: &str = "%pA\0";
}

/// Prints a message via the kernel's [`seq_printf`].
///
/// Public but hidden since it should only be used from public macros.
///
/// # Safety
///
/// The format string must be one of the ones in [`format_strings`]
///
/// [`seq_printf`]: ../../../../include/linux/seq_file.h
#[doc(hidden)]
pub unsafe fn call_seq_printf(
    seq_file: &mut bindings::seq_file,
    args: fmt::Arguments<'_>,
) {
    unsafe {
        bindings::seq_printf(
            seq_file as *mut _,
            format_strings::FORMAT_STRING.as_ptr() as _,
            &args as *const _ as *const c_void,
        );
    }
}

/// Stub for doctests
#[cfg(testlib)]
#[macro_export]
macro_rules! seq_printf {
	  ($file:expr, $($arg:tt)*) => {
        ()
	  };
}

/// Prints to a sequence file.
///
/// Mimics the interface of [`std::print!`]. See [`core::fmt`] and
/// [`alloc::format!`] for information about the formatting syntax.
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::seq_printf;
/// # let seq_file = 0;
/// seq_printf!(seq_file, "hello {}\n", "there");
/// ```
#[cfg(not(testlib))]
#[macro_export]
macro_rules! seq_printf (
    ($file:expr, $($arg:tt)*) => (
        // If this was not outside of an unsafe block code like the
        // following would be marked as unnecessary unsafe.
        //
        // seq_printf!(unsafe{ seq_file.as_mut().unwrap() }, "{}", 0);
        let file = $file;
        unsafe {
            $crate::seq_print::call_seq_printf(
                file,
                format_args!($($arg)+),
            );
        }
    )
);
