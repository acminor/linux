/*! file_mmu.rs: ramfs MMU-based file operations
 * - ported from file-mmu.c
 *
 * Resizable simple ram filesystem for Linux.
 *
 * Copyright (C) 2000 Linus Torvalds.
 *               2000 Transmeta Corp.
 *
 * Usage limits added by David Gibson, Linuxcare Australia.
 * This file is released under the GPL.
 *
 * NOTE! This filesystem is probably most useful
 * not as a real filesystem, but as an example of
 * how virtual filesystems can be written.
 *
 * It doesn't get much simpler than this. Consider
 * that this file implements the full semantics of
 * a POSIX-compliant read-write filesystem.
 *
 * Note in particular how the filesystem does not
 * need to implement any data structures of its own
 * to keep track of the virtual data: using the VFS
 * caches is sufficient.
 */

#![no_std]
#![feature(allocator_api, global_asm)]
#![allow(missing_docs)]

use kernel::c_default_struct;
use kernel::bindings::{file, file_operations, generic_file_llseek, generic_file_mmap,
                       generic_file_read_iter, generic_file_splice_read, generic_file_write_iter,
                       inode_operations, iter_file_splice_write, noop_fsync, simple_getattr,
                       simple_setattr};
use kernel::c_types::c_ulong;
use kernel::task::Task;

#[no_mangle]
pub static mut ramfs_file_operations: file_operations = file_operations {
    read_iter: Some(generic_file_read_iter),
    write_iter: Some(generic_file_write_iter),
    mmap: Some(generic_file_mmap),
    fsync: Some(noop_fsync),
    splice_read: Some(generic_file_splice_read),
    splice_write: Some(iter_file_splice_write),
    llseek: Some(generic_file_llseek),
    get_unmapped_area: Some(ramfs_mmu_get_unmapped_area),
    ..c_default_struct!(file_operations)
};

#[no_mangle]
pub static ramfs_file_inode_operations: inode_operations = inode_operations {
    setattr: Some(simple_setattr),
    getattr: Some(simple_getattr),
    ..c_default_struct!(inode_operations)
};

#[no_mangle]
pub unsafe extern "C" fn ramfs_mmu_get_unmapped_area(
    file: *mut file,
    addr: c_ulong,
    len: c_ulong,
    pgoff: c_ulong,
    flags: c_ulong,
) -> c_ulong {
    // could potentially fix this __bindgen_anon_1 with a C-preprocessor
    // definition that is only set during C-bindgen
    //
    // Without this we are blocked by https://github.com/rust-lang/rust-bindgen/issues/1971
    // and https://github.com/rust-lang/rust-bindgen/issues/2000 in-terms of getting better names from bindgen.
    //
    // Luckily their is only one outer anonymous struct used for struct layout randomization
    // `__randomize_layout`, TODO not sure how Rust-for-Linux handles this here and in task_struct
    //
    // Safety: original ramfs code assumed that mm was not null, we do the same here
    let mm = unsafe { Task::current().as_task_ptr().mm.as_ref().unwrap() };
    let get_unmapped_area = mm.__bindgen_anon_1.get_unmapped_area.unwrap();

    unsafe { get_unmapped_area(file, addr, len, pgoff, flags) }
}
