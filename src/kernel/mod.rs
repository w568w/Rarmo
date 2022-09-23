pub mod init;
pub mod mem;
pub mod rust_allocator;
pub mod proc;
pub mod cpu;
pub mod sched;

#[macro_export]
macro_rules! define_early_init {
    ($func:ident) => {
        // This is a hack to get the linker to include the function pointer in the
        // early init section.
        //
        // Also notice that we use `paste!` to generate a unique name for the
        // function pointer.
        // `pub` is required to tell the compiler not to optimize the variable away.
        // You can use `#[used]` attribute to achieve the same effect too.
        paste::paste! {
            #[link_section = ".init.early"]
            #[no_mangle]
            pub static mut [<__early_init_ $func>] : *const () = $func as *const ();
        }
    };
}

#[macro_export]
macro_rules! define_init {
    ($func:ident) => {
        paste::paste! {
            #[link_section = ".init"]
            #[no_mangle]
            pub static mut [<__init_ $func>] : *const () = $func as *const ();
        }
    };
}