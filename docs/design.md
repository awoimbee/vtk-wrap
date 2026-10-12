# Binding design

[Back to the README](../README.md)

## Two layers

1. `vtk-wrap-sys::ffi` declares unsafe C ABI functions. The C++ shim casts
   opaque pointers and forwards calls.
2. `vtk-wrap::raw` provides generated reference-counted handles mirroring VTK.
   **Every native method and constructor is unsafe**, including static type
   queries, downcasts, and inherited methods. The generator makes no safety
   decisions based on names, constness, or parameter counts.

There is no audited safe layer. Use `vtk_wrap::raw::vtkFoo` for native access
and review the C++ contract before calling it in an unsafe block.

## Raw handle contract

Each handle owns one valid VTK reference. `Clone` retains it; `Drop` releases
it. Construction is unsafe because native factories and destruction can have
thread and environment requirements. There is no raw `Default`.
`as_ptr()` only exposes a borrowed pointer; dereferencing it or using it in
FFI requires unsafe.

Generated string arguments and by-value arrays can use call-local storage.
If a native method retains such a pointer, use `vtk-wrap-sys::ffi` with
explicitly owned storage instead; even a Rust `&'static str` does not extend
the lifetime of a temporary `CString` created by the adapter.

Every unsafe call must satisfy native initialization, index bounds, buffer
sizes, pointer lifetimes, ownership, aliasing, and thread-affinity requirements.
Native code may retain input pointers beyond the call. It must not invalidate
references owned by any Rust handle: calling a native `Delete` or changing a
reference count arbitrarily can make a later `Clone` or `Drop` invalid.

Handles contain `NonNull<c_void>` and are neither `Send` nor `Sync`. Clones
share native state; `&mut` on one clone would not establish exclusive access.

Borrowed object results use `from_borrowed` to take an extra reference.
Raw-pointer methods that transfer ownership may still leak, since ownership
metadata is incomplete. A leak is distinct from undefined behavior.
For `vtkSmartPointer<T>` results, the shim retains the object while the
temporary smart pointer is still alive, and Rust adopts that owned reference.
All C++ entry points are `noexcept`: a native exception terminates the process
rather than unwinding across the non-unwinding Rust C ABI.

## Inheritance, overloads, and types

Raw wrappers are `#[repr(transparent)]` pointer handles. `Deref` targets the
nearest wrapped base class, but does not make inherited native calls safe.

Methods use snake_case. The overload with the fewest parameters keeps the
unsuffixed name; others receive `_v2`, `_v3`, etc. Generated documentation
includes the original C++ name as a search alias.

| C++ | Raw Rust wrapper |
| --- | --- |
| `int`, `double`, … | `i32`, `f64`, … |
| `long` / `unsigned long` | `i64` / `u64` (FFI uses `c_long` / `c_ulong`) |
| `vtkIdType` | `i64` |
| `bool` | `bool` |
| `const char *`, `std::string` | `&str` parameters, `String` results |
| `T *GetX()` where `T : vtkObjectBase` | `Option<T>` |
| `const double x[3]` | `[f64; 3]` |
| `double x[3]` parameter | `&mut [f64; 3]` |
| `const double x[]` parameter | `&[f64]` |
| `double x[]` parameter | `&mut [f64]` |

These conversions do not establish semantic preconditions or retained-buffer
lifetimes. Unsupported value types, templates, callbacks, and other signatures
are omitted and reported by the generator.

## Tests and regeneration

Default `cargo test` excludes unchecked crash diagnostics. Compile-fail tests
protect the unsafe boundary: raw constructors, methods, static queries and
handle access must be called inside `unsafe`, and raw handles cannot be
cloned.

The generated smoke accessor sweep is opt-in through `crash-hunt`. It
deliberately calls methods on uninitialized state and may invoke undefined
behavior; it belongs in a disposable process, not the ordinary test suite.
Raw rendering tests require `raw-rendering-tests` and a window server.

Generated bindings and diagnostics are split by VTK module. All binding changes
must be made in `vtk-wrap-gen`, then regenerated. See
[Regenerating bindings](generating.md).
