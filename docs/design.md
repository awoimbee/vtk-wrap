# Binding design

[Back to the README](../README.md)

## Two layers

1. The private `ffi` module declares unsafe C ABI functions; the C++ shim
   casts opaque pointers and forwards calls.
2. `vtk-wrap::raw` provides generated reference-counted handles mirroring VTK.
   Every native method and constructor is `unsafe`, including static type
   queries, downcasts, and inherited methods. The generator makes no safety
   decisions based on names, constness, or parameter counts.

Both layers are generated in one build of the single `vtk-wrap` crate. FFI
and the generator are implementation details; only `raw` is public. There is
no safe layer. Review the C++ contract before calling anything in an `unsafe` block.

## Raw handle contract

Each handle owns one valid VTK reference. `Clone` retains it; `Drop` releases
it. Construction is unsafe because native factories and destruction can have
thread and environment requirements. There is no raw `Default`; `as_ptr()`
only exposes a borrowed pointer.

Generated string arguments and by-value arrays can use call-local storage. If
a native method retains such a pointer, do not use its call-local adapter.
It needs a dedicated binding with explicitly owned storage; a `&'static str`
does not extend the lifetime of the adapter's temporary `CString`.

Every unsafe call must satisfy native initialization, index bounds, buffer
sizes, pointer lifetimes, ownership, aliasing, and thread-affinity
requirements. Native code may retain input pointers beyond the call and must
not invalidate references owned by any Rust handle.

Handles contain `NonNull<c_void>` and are neither `Send` nor `Sync`. Clones
share native state; `&mut` on one clone would not establish exclusive access.

Borrowed object results take an extra reference. Raw-pointer results that
transfer ownership may still leak, since ownership metadata is incomplete — a
leak, not undefined behavior. For `vtkSmartPointer<T>` results the shim
retains the object while the temporary smart pointer is alive, and Rust adopts
that reference. All C++ entry points are `noexcept`: a native exception
terminates the process rather than unwinding across the Rust C ABI.

## Inheritance, overloads, and types

Raw wrappers are `#[repr(transparent)]` pointer handles. `Deref` targets the
nearest wrapped base class, but does not make inherited native calls safe.

Methods use snake_case; the overload with the fewest parameters keeps the
unsuffixed name and others get `_v2`, `_v3`, …. Generated docs include the
original C++ name as a search alias.

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
