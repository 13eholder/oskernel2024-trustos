cfg_if::cfg_if! {
    if #[cfg(feature="board_qemu")]{
        mod qemu;
        pub use qemu::*;
    }else if #[cfg(feature="board_vf2")]{
        mod vf2;
        pub use vf2::*;
    }else if #[cfg(feature="board_ramdisk")]{
        mod vf2;
        pub use vf2::*;
    }
}
