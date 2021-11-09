#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;

use kernel::bindings::{
    file, file_operations, generic_file_llseek, generic_file_mmap, generic_file_read_iter,
    generic_file_splice_read, generic_file_write_iter, iter_file_splice_write, noop_fsync,
};
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
