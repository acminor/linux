//!
//! Rust transition file from C to Rust
//! - use generated inode_rs.h to include struct/function declarations in C
//!

#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;

#[allow(non_camel_case_types)]
#[repr(C)]
/// Ported C ramfs_mount_opts struct
pub struct ramfs_mount_opts {
     mode: kernel::bindings::umode_t,
}

#[no_mangle]
#[allow(non_snake_case)]
/// dummy function to make sure struct ramfs_mount_opts is exported
pub extern "C" fn __dummy_rust__ramfs_mount_opts(_dummy: ramfs_mount_opts) {
}
