# vtk-wrap — Rust bindings for VTK

Rust bindings for the native C++
[Visualization Toolkit (VTK)](https://vtk.org/): a complete, generated
**unsafe API** in `vtk_wrap::raw`.

The committed bindings target **VTK 9.7.1** and cover **1,582 classes and
~35,600 methods**. Modules are enabled through Cargo features.

## Quick start

From a checkout:

```sh
cargo build -p vtk-wrap
cargo run -p vtk-wrap --example sphere_source --features vtkFiltersSources
```

With the `vtkFiltersSources` feature enabled:

```rust
use vtk_wrap::raw::vtkSphereSource;

// SAFETY: constructing a sphere source has no extra requirements, the handle
// owns one reference, and the calls below preserve it.
unsafe {
    let sphere = vtkSphereSource::new();
    sphere.set_radius(2.0);
    sphere.set_theta_resolution(32);
    sphere.update();
}
```

## Requirements and building

- Rust/Cargo, a C++ compiler, and CMake ≥ 3.12.
- Network access on the first build, unless you supply VTK sources or a
  prebuilt installation.
- The committed bindings were generated on macOS; Linux and Windows have
  known native-build limitations.

By default, `vtk-wrap-sys` downloads the pinned VTK release and builds VTK
and the C++ shim statically. No system VTK installation is required.
Cold builds take several minutes; subsequent builds reuse the native cache.

See [Native builds](docs/building.md) for the build pipeline and platform
limitations.

## Feature flags

Each wrapped VTK module has a Cargo feature with the same name. Features
select both the native libraries to build and the Rust bindings to expose;
module dependencies are enabled automatically.

| Default feature | Purpose |
| --- | --- |
| `vtkCommonCore` | Object model |
| `vtkCommonDataModel` | Datasets |
| `vtkFiltersCore` | Core pipeline filters |

Enable additional modules as needed:

```sh
cargo build -p vtk-wrap --features vtkFiltersSources,vtkRenderingCore
```

The full feature list is in [`vtk-wrap/Cargo.toml`](vtk-wrap/Cargo.toml).

## Build cache and overrides

The default cache is `target/vtk-wrap-vtk/`. Remove it to force a native
rebuild, or set `VTK_WRAP_CACHE_DIR` to share a cache across checkouts.

| Environment variable | Purpose |
| --- | --- |
| `VTK_WRAP_CACHE_DIR` | Relocate the VTK and shim cache. |
| `VTK_WRAP_VTK_SOURCE_DIR` | Build from a local VTK source tree instead of downloading. |
| `VTK_WRAP_VTK_PREBUILT_DIR` | Use an existing VTK install prefix instead of building VTK. |
| `VTK_WRAP_VTK_URL` | Override the download URL for the pinned release archive. |
| `DOCS_RS` | Skip native compilation for documentation builds. |

A prebuilt installation must match the pinned VTK version and contain the
enabled modules. To retain static linking, build it with
`BUILD_SHARED_LIBS=OFF` and `CMAKE_POSITION_INDEPENDENT_CODE=ON`.

```sh
VTK_WRAP_VTK_PREBUILT_DIR=/path/to/vtk-install cargo build -p vtk-wrap-sys
```

## Workspace

| Package | Rust library / executable | Purpose |
| --- | --- | --- |
| `vtk-wrap` | `vtk_wrap` | Rust wrappers |
| `vtk-wrap-sys` | `vtk_wrap_sys` | Raw FFI and native shim |
| `vtk-wrap-gen` | `vtk-wrap-gen` | Binding generator |

## Safety boundary

Generated `vtk*` types live under `vtk_wrap::raw`. Every generated native
method, including constructors and inherited methods, requires `unsafe`; raw
handles have no `Default` implementation. The caller upholds native
initialization, index bounds, buffer sizes, pointer lifetimes, ownership,
aliasing, and thread-affinity requirements — the Rust types alone do not
establish them.

- Raw `Clone` retains a reference; `Drop` releases it. Unsafe calls must not
  invalidate references owned by Rust handles.
- Raw base-class methods are available through `Deref`. Overloads use suffixes
  such as `_v2`; generated docs include aliases for the original C++ names.
- Borrowed object results take an extra reference. Owned raw-pointer results
  can still leak because ownership-transfer conventions are not fully modeled.
  Smart-pointer results retain a reference before crossing the C ABI.
- Value types such as `vtkVector` and `vtkVariant`, most templates,
  reference output parameters, multi-dimensional arrays, and callbacks are
  not supported. Unsupported methods are omitted.

VTK itself remains trusted native code. There is no audited safe layer; the
crate exposes the native API directly and leaves every call's contract to the
caller.

See [Binding design](docs/design.md) for the raw handle contract and type
mapping.

## Development

Install and run the formatting, spelling, and workflow checks with
[prek](https://github.com/j178/prek):

```sh
prek install
prek run --all-files
cargo test --workspace
cargo test --workspace --features vtk-wrap/vtkFiltersSources,vtk-wrap/vtkImagingCore,vtk-wrap/vtkIOLegacy,vtk-wrap/vtkIOPLY
```

Default tests exclude unchecked native diagnostics. Compile-fail tests verify
that raw constructors, methods and handle access require explicit `unsafe`
and that raw handles cannot be aliased.

The generated accessor sweep deliberately violates some native preconditions
and can crash. Run it only in a disposable process, with explicit opt-in:

```sh
cargo test -p vtk-wrap --test smoke --features crash-hunt
# Rendering uses the raw API and needs a macOS window server/main thread:
cargo test -p vtk-wrap --test rendering --features vtkRenderingOpenGL2,raw-rendering-tests
```

These diagnostics find bad calls and glue bugs; they do not prove soundness.

### Regenerating bindings

Generated sources are committed; users do not need the wrapping tools.
To regenerate, first build the pinned VTK with its wrapping tools, then run:

```sh
cargo build -p vtk-wrap-sys
./scripts/regenerate.sh
```

See [Regenerating bindings](docs/generating.md) for custom VTK installations
and module selection. List generator options with
`cargo run -p vtk-wrap-gen -- --help`.

## Related projects and acknowledgements

[vtk-pure-rs](https://github.com/henriksson-lab/vtk-pure-rs) reimplements VTK
in Rust rather than wrapping the C++ library. It does not require a C++
toolchain or native VTK.

This project uses [VTK](https://vtk.org/) and David Gobbi's
[WrapVTK](https://github.com/dgobbi/WrapVTK). The earlier
[vtk-rs](https://github.com/jonaspleyer/vtk-rs) project and
[VTK forum discussion](https://discourse.vtk.org/t/rust-bindings/15700)
established the WrapVTK-based approach.

Licensed under [BSD-3-Clause](LICENSE), the same license as VTK.
