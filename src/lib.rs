//! Unsafe Rust bindings to the Visualization Toolkit (VTK).
//!
//! The complete generated API lives in [`raw`]. Every native method and
//! constructor is `unsafe`; the caller upholds VTK's initialization, index,
//! lifetime, ownership, aliasing and thread requirements.
//!
//! With the `vtkFiltersSources` feature:
//!
//! ```no_run
//! use vtk_wrap::raw::vtkSphereSource;
//!
//! // SAFETY: constructing a sphere source has no extra requirements, the
//! // handle owns one reference, and the calls below preserve it.
//! unsafe {
//!     let sphere = vtkSphereSource::new();
//!     sphere.set_radius(2.0);
//!     sphere.update();
//! }
//! ```

#![deny(unsafe_op_in_unsafe_fn)]

#[allow(non_snake_case, dead_code, improper_ctypes)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/ffi/generated.rs"));

    extern "C" {
        pub fn vtk_wrap_register(obj: *mut ::core::ffi::c_void);
        pub fn vtk_wrap_delete(obj: *mut ::core::ffi::c_void);
        pub fn vtk_wrap_strdup(s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
        pub fn vtk_wrap_free(p: *mut ::core::ffi::c_void);
    }
}

#[allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    unused_imports,
    unused_unsafe,
    clippy::all
)]
pub mod raw {
    //! Unaudited VTK handles. All native methods and constructors are unsafe.
    //!
    //! Handles own a reference: clone retains it and drop releases it. Native
    //! calls must preserve those references and obey VTK lifetime and thread
    //! requirements. Handles are neither Send nor Sync and have no Default.

    include!(concat!(env!("OUT_DIR"), "/raw/generated.rs"));
}
