//!
//! Rust transition file from C to Rust
//! - use generated inode_rs.h to include struct/function declarations in C
//!

#![no_std]
#![feature(allocator_api, global_asm, new_uninit)]

#![allow(non_camel_case_types)]
#![allow(missing_docs)]

use core::ptr;
use kernel::prelude::*;
use kernel::bindings::{
    user_namespace,
    inode,
    dentry,
    umode_t,
    dev_t,
    super_block,
    d_instantiate,
    current_time,
    fs_context,
};
use kernel::c_types::c_int;

/*
 * Learning experience, 0755 in C is octal
 * so we need to prefix 755 in Rust with 0o755
 */
const RAMFS_DEFAULT_MODE: umode_t = 0o755;

/* Predeclaration as required by cbindgen. Without this, cbindgen
   would not know what type of variable these are as we do not have
   proper cargo metadata parsing setup.

   This is a hack that relies on cbindgen undefined behavior for v0.20.0
   https://github.com/eqrion/cbindgen/blob/master/docs.md (section Writing Your C API)
   - Essentially, it cannot find the type so we give it a name in another namespace
     which it then finds. This causes the proper struct tag, etc. to be added to the type
     name on export. Plus, Rust (which understands module namespacing) reads the correct one
     from kernel and not from our fake module.*/
#[allow(unused)]
mod __anon__ {
    struct user_namespace;
    struct inode;
    struct dentry;
    struct fs_context;
}

#[repr(C)]
/// Ported C ramfs_mount_opts struct
pub struct ramfs_mount_opts {
    mode: kernel::bindings::umode_t,
}

#[repr(C)]
/// Ported C ramfs_fs_info struct
pub struct ramfs_fs_info {
    mount_opts: ramfs_mount_opts,
}

#[no_mangle]
pub unsafe extern "C" fn ramfs_mknod(_mnt_userns: *mut user_namespace, dir: *mut inode,
                                     dentry: *mut dentry, mode: umode_t, dev: dev_t) -> c_int
{
    let inode = unsafe {
        ramfs_get_inode((*dir).i_sb, dir, mode, dev)
    };

    /* safe way to make sure the pointer is not null */
    if !ptr::eq(inode, ptr::null_mut()) {
        unsafe {
            d_instantiate(dentry, inode);
            ramfs_rust_dget(dentry); /* Extra count - pin the dentry in core */

            /* in C-code they should have the same time */
            let ctime = current_time(dir);
            (*dir).i_mtime = ctime;
            (*dir).i_ctime = ctime;
        }
        0
    } else {
        /* type cast required b/c ENOSPC is u32 and cannot be negated by default in Rust
           - should be safe, as this is what is done in C code implicitly
             (if I know my casts correctly) */
        -(kernel::bindings::ENOSPC as kernel::c_types::c_int)
    }
}

#[no_mangle]
/*
 * Not sure how to test this. The best way forward for now is to test
 * that the mount point (by default) has RAMFS_DEFAULT_MODE permissions
 */
pub unsafe extern "C" fn ramfs_init_fs_context(fc: *mut fs_context) -> c_int {
    /* Looking at the default allocator code in rust/kernel/allocator.rs
     * - if uses GFP_KERNEL, so we are fine here
     * - the kzalloc docs state that the memory is zeroed
     */
    let fsi = Box::<ramfs_fs_info>::try_new_zeroed();
    match fsi {
        Ok(fsi) => {
            /* this should be safe b/c the C struct is valid initialized as all zeros */
            let mut fsi = unsafe { fsi.assume_init() };
            (*fsi).mount_opts.mode = RAMFS_DEFAULT_MODE;
            unsafe {
                /* Unsure of the borrow checker safety of taking
                 * a reference to this as using as a pointer in C-land
                 * - should be fine as ramfs_context_ops has a static lifetime
                 * - might need different semantics if we need a mut and const version of this
                 *   at the same time later
                 */
                ramfs_rust_fs_context_set_s_fs_info(fc, Box::into_raw(fsi));
                ramfs_rust_fs_context_set_ops(fc, &ramfs_context_ops);
            }
            0
        }
        Err(_) => {
            -(kernel::bindings::ENOMEM as c_int)
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
/// dummy function to make sure struct ramfs_mount_opts and ramfs_fs_info is exported
pub extern "C" fn __dummy_rust__ramfs_fs_info(_dummy: ramfs_fs_info) {}

#[repr(C)]
struct fs_context_operations {
    /* same thing that bindgen generates for seemingly opaque types */
    _unused: [u8; 0],
}

/// cbindgen:ignore
extern "C" {
    static ramfs_context_ops: fs_context_operations;

    /* something about vm_userfaultfd_ctx causing this to fail
       - I believe this is due to that being zero-sized struct
         but it is repr(C) in the bindings_generated.rs file
         so not sure. For now, I assume that is is safe to ignore */
    #[allow(improper_ctypes)]
    fn ramfs_get_inode(sb: *mut super_block, dir: *const inode, mode: umode_t, dev: dev_t) -> *mut inode;

    #[allow(improper_ctypes)]
    fn ramfs_rust_dget(dentry: *mut dentry) -> *mut dentry;

    #[allow(improper_ctypes)]
    fn ramfs_rust_fs_context_set_ops(fc: *mut fs_context,
                                     ops: *const fs_context_operations);
    #[allow(improper_ctypes)]
    fn ramfs_rust_fs_context_set_s_fs_info(fc: *mut fs_context,
                                           fsi: *mut ramfs_fs_info);
}
