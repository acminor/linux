#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;

use kernel::bindings::{
    file_operations,
    generic_file_read_iter,
    generic_file_write_iter,
    generic_file_mmap,
    noop_fsync,
    generic_file_splice_read,
    iter_file_splice_write,
    generic_file_llseek,

};

/*
const struct file_operations ramfs_file_operations = {
.read_iter	= generic_file_read_iter,
.write_iter	= generic_file_write_iter,
.mmap		= generic_file_mmap,
.fsync		= noop_fsync,
.splice_read	= generic_file_splice_read,
.splice_write	= iter_file_splice_write,
.llseek		= generic_file_llseek,
.get_unmapped_area	= ramfs_mmu_get_unmapped_area,
};
*/

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
    // To my knowledge must list these manually because
    // Default::default is not compile time able
    // so cannot do ..Default::default as I have seen on StackOverflow
    read: None,
    write: None,
    iopoll: None,
    iterate: None,
    iterate_shared: None,
    poll: None,
    unlocked_ioctl: None,
    compat_ioctl: None,
    mmap_supported_flags: 0,
    open: None,
    flush: None,
    release: None,
    fasync: None,
    owner: core::ptr::null_mut(),
    lock: None,
    sendpage: None,
    check_flags: None,
    flock: None,
    setlease: None,
    fallocate: None,
    show_fdinfo: None,
    #[cfg(not(CONFIG_MMU))]
    mmap_capabilities: None,
    copy_file_range: None,
    remap_file_range: None,
    fadvise: None,
};

extern "C" {
    pub fn ramfs_mmu_get_unmapped_area(file: *mut kernel::bindings::file,
            addr: kernel::c_types::c_ulong, len: kernel::c_types::c_ulong, pgoff: kernel::c_types::c_ulong,
            flags: kernel::c_types::c_ulong) -> kernel::c_types::c_ulong;
}
