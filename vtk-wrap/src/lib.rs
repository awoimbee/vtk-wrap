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

#[allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    unused_imports,
    unused_unsafe,
    clippy::all
)]
#[path = "generated.rs"]
pub mod raw;
