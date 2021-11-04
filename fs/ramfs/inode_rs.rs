//!
//! Rust transition file from C to Rust
//! - use generated inode_rs.h to include struct/function declarations in C
//!

#![no_std]
#![feature(allocator_api, global_asm, new_uninit)]

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(missing_docs)]

use core::ptr;
use kernel::prelude::*;
use kernel::c_str;
use kernel::bindings::{
    user_namespace,
    inode,
    dentry,
    umode_t,
    dev_t,
    super_block,
    d_make_root,
    d_instantiate,
    d_tmpfile,
    current_time,
    fs_context,
    S_IFREG,
    S_IFDIR,
    inc_nlink,
    init_user_ns,
    ENOSPC,
    ENOMEM,
    ENOPARAM,
    super_operations,
    loff_t,
    fs_parameter_spec,
    fs_parameter,
    S_IALLUGO,
    S_IFLNK,
    S_IRWXUGO,
    strlen,
    iput,
    page_symlink,
    kill_litter_super,
    seq_file,
};
use kernel::c_types::{
    c_int,
    c_uint,
    c_ulong,
    c_uchar,
    c_char,
};

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
#[rustfmt::skip]
mod __anon__ {
    struct user_namespace;
    struct inode;
    struct dentry;
    struct fs_context;
    struct super_block;
    struct fs_parameter;
    struct seq_file;
}

#[repr(C)]
/// Ported C ramfs_mount_opts struct
pub struct ramfs_mount_opts {
    mode: umode_t,
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
        -(ENOSPC as c_int)
    }
}

/*
 * The following should provide a version
 * of fs_parse_result as bindgen bindings do not have
 * a version. To my knowledge, this version should match
 * the C-version. Relevant info on repr(C) on Rust unions
 * and their matching of C-unions can be found here
 * https://github.com/rust-lang/unsafe-code-guidelines/issues/13#issuecomment-417413059
 */

#[repr(C)]
/// cbindgen:ignore
union fs_parse_result_inner {
    boolean: bool,
    int_32: c_int,
    uint_32: c_uint,
    uint_64: u64,
}

#[repr(C)]
/// cbindgen:ignore
struct fs_parse_result {
    negated: bool,
    result: fs_parse_result_inner,
}

impl Default for fs_parse_result {
    fn default() -> Self {
        fs_parse_result { negated: false, result: fs_parse_result_inner { uint_64: 0 } }
    }
}

/*
 * Not an issue to represent this enum as
 * a Rust enum as it is not being used to
 * represent C flags.
 */
#[repr(C)]
enum ramfs_param {
    Opt_mode
}

#[no_mangle]
pub unsafe extern "C" fn ramfs_parse_param(fc: *mut fs_context, param: *mut fs_parameter) -> c_int {
    let fsi = unsafe { ramfs_rust_fs_context_get_s_fs_info(fc) };

    let mut result = fs_parse_result::default();
    let opt = unsafe { rust_fs_parse(fc, ramfs_fs_parameters.as_ptr(), param, &mut result) };

    /*
     * Match on int becaues Rust enum's are not like C enum's.
     * - We do not want to cast the opt to the ramfs_param enum
     *   and opt not be a valid value for ramfs_param enum.
     */
    let Opt_mode = ramfs_param::Opt_mode as c_int;
    let enoparam = -(ENOPARAM as c_int);
    match opt {
        opt if opt == Opt_mode => {
            unsafe {
                (*fsi).mount_opts.mode = (result.result.uint_32 & S_IALLUGO) as umode_t;
            }
        }
        /*
		 * We might like to report bad mount options here;
		 * but traditionally ramfs has ignored all mount options,
		 * and as it is used as a !CONFIG_SHMEM simple substitute
		 * for tmpfs, better continue to ignore other mount options.
		 */
        opt if opt == enoparam => {}
        opt if opt < 0 => { return opt; }
        _ => {}
    };

    0
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
            -(ENOMEM as c_int)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ramfs_mkdir(_mnt_userns: *mut user_namespace, dir: *mut inode,
                                     dentry: *mut dentry, mode: umode_t) -> c_int
{
    unsafe {
        let retval = ramfs_mknod(&mut init_user_ns, dir, dentry, mode | S_IFDIR as umode_t, 0);
        if retval == 0 {
            /* increment link counter for directory (fs/inode.c) */
            inc_nlink(dir);
        }
        retval
    }
}

#[no_mangle]
pub unsafe extern "C" fn ramfs_create(_mnt_userns: *mut user_namespace, dir: *mut inode,
                                      dentry: *mut dentry, mode: umode_t, _excl: bool) -> c_int
{
    unsafe {
        ramfs_mknod(&mut init_user_ns, dir, dentry, mode | S_IFREG as umode_t, 0)
    }
}

#[no_mangle]
pub unsafe extern "C" fn ramfs_symlink(_mnt_userns: *mut user_namespace, dir: *mut inode,
                                       dentry: *mut dentry, symname: *const c_char) -> c_int
{
    let inode = unsafe {
        ramfs_get_inode((*dir).i_sb, dir, (S_IFLNK | S_IRWXUGO) as umode_t, 0)
    };
    if ptr::eq(inode, ptr::null_mut()) {
        return -(ENOSPC as c_int);
    }

    /* Grab symbol name length and attempt linkage. On linkage failure, we'll
       iput(inode) to decrement the usage count, ultimately destroying it. */
    let l = unsafe { strlen(symname) } + 1;
    let err = unsafe { page_symlink(inode, symname, l as c_int) };
    if err != 0 {
        unsafe { iput(inode) };
        err
    } else {
        /* On successful linkage, we'll instantiate, increment the reference
        count, and update the inode's modification time. */
        unsafe {
            d_instantiate(dentry, inode);
            ramfs_rust_dget(dentry);

            let ct = current_time(dir);
            (*dir).i_mtime = ct;
            (*dir).i_ctime = ct;
        }
        0
    }
}

#[no_mangle]
pub extern "C" fn ramfs_tmpfile(_mnt_userns: *mut user_namespace, dir: *mut inode,
                                dentry: *mut dentry, mode: umode_t) -> c_int {
    let inode = unsafe {
        ramfs_get_inode((*dir).i_sb, dir, mode, 0)
    };

    /*
     * It is interesting to see how early return C patterns are reduced
     * to if/else return patterns in Rust, could also do early return in
     * Rust if you wanted to.
     */
    if !ptr::eq(inode, ptr::null_mut()) {
        unsafe { d_tmpfile(dentry, inode); }
        0
    } else {
        -(ENOSPC as c_int)
    }
}

#[no_mangle]
pub extern "C" fn ramfs_show_options(m: *mut seq_file, root: *mut dentry) -> c_int
{
    let sb = unsafe { (*root).d_sb };
    let fsi = unsafe { (*sb).s_fs_info as *mut ramfs_fs_info };
    let mode = unsafe { (*fsi).mount_opts.mode };
    if mode != RAMFS_DEFAULT_MODE {
        /* Invoke our C-wrapper for seq_printf().
           (We're not exporting seq_printf() yet) */
        unsafe {
            ramfs_rust_seq_puts_mode(m, c_str!(",mode=%o").as_char_ptr(), mode);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn ramfs_kill_sb(sb: *mut super_block) {
    unsafe { Box::from_raw((*sb).s_fs_info as *mut ramfs_fs_info); }
    unsafe { kill_litter_super(sb); }
}

#[no_mangle]
pub extern "C" fn ramfs_fill_super(sb: *mut super_block, _fc: *mut fs_context) -> c_int {
    let fsi = unsafe { (*sb).s_fs_info as *mut ramfs_fs_info };

    unsafe {
        (*sb).s_maxbytes = ramfs_get_max_lfs_filesize();
        (*sb).s_blocksize = ramfs_get_page_size();
        (*sb).s_blocksize_bits = ramfs_get_page_shift();
        (*sb).s_magic = ramfs_get_ramfs_magic();
        (*sb).s_op = &ramfs_ops;
        (*sb).s_time_gran = 1;
    }

    let inode = unsafe { ramfs_get_inode(sb, ptr::null_mut(), S_IFDIR as umode_t | (*fsi).mount_opts.mode, 0) };
    unsafe {
        (*sb).s_root = d_make_root(inode);
    }

    let s_root = unsafe { (*sb).s_root };
    if ptr::eq(s_root, ptr::null_mut()) {
        -(ENOMEM as c_int)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ramfs_free_fc(fc: *mut fs_context)
{
    let fsi = unsafe { ramfs_rust_fs_context_get_s_fs_info(fc) };

    /*
     * RAII drop should be safe if fsi is valid coming from C-land
     * - however the spec does state the following,
     *   "For this to be safe, the memory must have been allocated
     *    in accordance with the memory layout used by Box."
     *    - https://doc.rust-lang.org/std/boxed/struct.Box.html#method.from_raw
     *    - this should be fine because we define this struct as C-typed
     * - also could have an issue with it being allocated with different settings
     *   than the default allocator we have. However, we do not have to use from_raw_into
     *   because kfree can handle the different kmalloc memtypes just fine :)
     */
    unsafe { Box::from_raw(fsi); }
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
    static ramfs_fs_parameters: [fs_parameter_spec; 2];

    #[allow(improper_ctypes)]
    static ramfs_ops: super_operations;

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
    fn rust_fs_parse(fc: *mut fs_context, desc: *const fs_parameter_spec,
                     param: *mut fs_parameter, result: *mut fs_parse_result) -> c_int;
    #[allow(improper_ctypes)]
    fn ramfs_rust_fs_context_get_s_fs_info(fc: *mut fs_context) -> *mut ramfs_fs_info;
    #[allow(improper_ctypes)]
    fn ramfs_rust_fs_context_set_s_fs_info(fc: *mut fs_context,
                                           fsi: *mut ramfs_fs_info);
    #[allow(improper_ctypes)]
    fn ramfs_rust_seq_puts_mode(m: *mut seq_file, string: *const c_char, mode: umode_t);
    #[allow(improper_ctypes)]
    fn ramfs_get_max_lfs_filesize() -> loff_t;
    #[allow(improper_ctypes)]
    fn ramfs_get_page_size() -> c_ulong;
    #[allow(improper_ctypes)]
    fn ramfs_get_page_shift() -> c_uchar;
    #[allow(improper_ctypes)]
    fn ramfs_get_ramfs_magic() -> c_ulong;
}
