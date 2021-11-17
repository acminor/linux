//!
//! Rust transition file from C to Rust
//! - use generated inode_rs.h to include struct/function declarations in C
//!

#![no_std]
#![feature(allocator_api, global_asm, new_uninit)]

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(missing_docs)]
#![allow(non_upper_case_globals)]

use core::ptr;
use kernel::prelude::*;
use kernel::{
    c_str,
    fsparam_u32oct,
    c_default_struct,
};
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
    register_filesystem,
    file_system_type,
    get_next_ino,
    inode_init_owner,
    init_special_inode,
    inode_nohighmem,
    address_space,
    gfp_t,
    S_IFMT,
    simple_dir_operations,
    page_symlink_inode_operations,
    inode_operations,
    ram_aops,
    file_operations,
    new_inode,
    generic_delete_inode,
    simple_link,
    simple_lookup,
    simple_rename,
    simple_rmdir,
    simple_statfs,
    simple_unlink,
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

/* The original FS_USERNS_MOUNT is a macro defined in include/linux/fs.h.
 * We need this definition in order to initialize the 'ramfs_fs_type' struct
 * in this file as compile-time. Since Rust cannot see C macros, this is our
 * best current solution.
 */
const RAMFS_RUST_FS_USERNS_MOUNT: c_int = 8;

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
    struct file_system_type;
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
pub unsafe extern "C" fn ramfs_get_inode(sb: *mut super_block, dir: *const inode,
                                         mode: umode_t, dev: dev_t) -> *mut inode {
    let inode = unsafe { new_inode(sb) };

    if !ptr::eq(inode, ptr::null_mut()) {
        let inode = unsafe{ inode.as_mut().unwrap() };
        inode.i_ino = unsafe{ get_next_ino() } as c_ulong;
        unsafe { inode_init_owner(&mut init_user_ns, inode, dir, mode); }
        unsafe { inode.i_mapping.as_mut().unwrap().a_ops = &ram_aops; }
        unsafe { ramfs_mapping_set_gfp_mask(inode.i_mapping, ramfs_get_gfp_highuser()); }
        unsafe { ramfs_mapping_set_unevictable(inode.i_mapping); }

        let cur_time = unsafe{ current_time(inode) };
        inode.i_atime = cur_time;
        inode.i_mtime = cur_time;
        inode.i_ctime = cur_time;

        match mode as c_uint & S_IFMT {
            S_IFREG => {
                inode.i_op = unsafe { &ramfs_file_inode_operations };
                inode.__bindgen_anon_3.i_fop = unsafe { &ramfs_file_operations };
            }
            S_IFDIR => {
                inode.i_op = &ramfs_dir_inode_operations;
                inode.__bindgen_anon_3.i_fop = unsafe { &simple_dir_operations };

                /* directory inodes start off with i_nlink == 2 (for "." entry) */
                unsafe{ inc_nlink(inode); }
            }
            S_IFLNK => {
                inode.i_op = unsafe { &page_symlink_inode_operations };
                unsafe { inode_nohighmem(inode); }
            }
            _ => {
                unsafe { init_special_inode(inode, mode, dev); }
            }
        }
    }

    inode
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
pub enum ramfs_param {
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
pub extern "C" fn ramfs_get_tree(fc: *mut fs_context) -> c_int
{
    unsafe { ramfs_rust_get_tree_nodev(fc, ramfs_fill_super) }
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

// static in original C file
static ramfs_dir_inode_operations: inode_operations = inode_operations {
    create: Some(ramfs_create),
    lookup: Some(simple_lookup),
    link: Some(simple_link),
    unlink: Some(simple_unlink),
    symlink: Some(ramfs_symlink),
    mkdir: Some(ramfs_mkdir),
    rmdir: Some(simple_rmdir),
    mknod: Some(ramfs_mknod),
    rename: Some(simple_rename),
    tmpfile: Some(ramfs_tmpfile),
    ..c_default_struct!(inode_operations)
};

// static in original C file
static ramfs_ops: super_operations = super_operations {
    statfs: Some(simple_statfs),
    drop_inode: Some(generic_delete_inode),
    show_options: Some(ramfs_show_options),
    ..c_default_struct!(super_operations)
};

// not static in original C file
pub const ramfs_fs_parameters: [fs_parameter_spec; 2] = [
    fsparam_u32oct!("mode", ramfs_param::Opt_mode),
    c_default_struct!(fs_parameter_spec),
];

// static in original C file
#[no_mangle]
static mut ramfs_fs_type: file_system_type = file_system_type {
    name: c_str!("ramfs").as_char_ptr(),
    init_fs_context: Some(ramfs_init_fs_context),
    parameters: ramfs_fs_parameters.as_ptr(),
    kill_sb: Some(ramfs_kill_sb),
    fs_flags: RAMFS_RUST_FS_USERNS_MOUNT,
    ..c_default_struct!(file_system_type)
};

/* The original C source uses the '__init' macro (defined in include/linux/init.h)
 * to apply a few attributes to this init function. '__init' expands to:
 *      __section(".init.text") __cold  __latent_entropy __noinitretpoline __nocfi
 * We can replicate these modifiers with Rust attributes:
 *      __section(".init.text")     -->     link_section
 *      __cold                      -->     cold
 *      __latent_entropy            -->     (NOT AVAILABLE FOR RUST COMPILER)
 *      __noinitretpoline           -->     (NOT AVAILABLE FOR RUST COMPILER)
 *      __nocfi                     -->     N/A (see below)
 * A few of these don't have rustc equivalents, so we can't perfectly recreate
 * how this function is compiled. Despite this, our kernel compiles and our
 * ramfs seem to have no issues. We'll leave this issue to future research.
 * __nocfi disables a Clang Control Flow Integrity feature. We shouldn't need
 * to worry about it in our Rust code (we're not using Clang, we're using rustc).
 * Sources:
 *  - https://doc.rust-lang.org/reference/abi.html#the-link_section-attribute
 *  - https://doc.rust-lang.org/reference/attributes/codegen.html#the-cold-attribute
 *  - https://clang.llvm.org/docs/ControlFlowIntegrity.html
 */
#[no_mangle]
#[link_section = ".init.text"]  /* __section(".init.text") */
#[cold]                         /* __cold */
pub extern "C" fn init_ramfs_fs() -> c_int
{
    unsafe { register_filesystem(&mut ramfs_fs_type) }
}

#[no_mangle]
#[allow(non_snake_case)]
/// dummy function to make sure struct ramfs_mount_opts and ramfs_fs_info is exported and ramfs_param
pub extern "C" fn __dummy_rust__ramfs_fs_info(_dummy: ramfs_fs_info, _dummy_2: ramfs_param) {}

#[repr(C)]
struct fs_context_operations {
    /* same thing that bindgen generates for seemingly opaque types */
    _unused: [u8; 0],
}

/// cbindgen:ignore
extern "C" {
    static ramfs_context_ops: fs_context_operations;
    #[allow(improper_ctypes)]
    static ramfs_file_operations: file_operations;
    #[allow(improper_ctypes)]
    static ramfs_file_inode_operations: inode_operations;

    /* NOTE allow(improper_ctypes)
    something about vm_userfaultfd_ctx causing this to fail
    - I believe this is due to that being zero-sized struct
    but it is repr(C) in the bindings_generated.rs file
    so not sure. For now, I assume that is is safe to ignore */

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
    #[allow(improper_ctypes)]
    fn ramfs_rust_get_tree_nodev(fc: *mut fs_context,
                                 fill_super: extern "C" fn(*mut super_block, *mut fs_context) -> c_int) -> c_int;
    #[allow(improper_ctypes)]
    fn ramfs_mapping_set_gfp_mask(m: *mut address_space, mask: gfp_t);
    #[allow(improper_ctypes)]
    fn ramfs_mapping_set_unevictable(mapping: *mut address_space);
    #[allow(improper_ctypes)]
    fn ramfs_get_gfp_highuser() -> gfp_t;
}
