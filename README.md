# vtk-wrap — Rust bindings for VTK

Rust bindings for the native C++
[Visualization Toolkit (VTK)](https://vtk.org/): a complete, generated
**unsafe API** in `vtk_wrap::raw`.

The committed bindings target **VTK 9.7.1** and cover **1,582 classes and
~35,600 methods**. Modules are enabled through Cargo features.

## Quick start

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

By default, `vtk-wrap-sys` downloads the pinned VTK release and builds VTK
and the C++ shim statically. Cold builds take several minutes; later builds
reuse the native cache. See [Building and
regenerating](docs/building.md) for the pipeline, cache and source overrides,
and platform limitations.

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

## Workspace

| Package | Rust library / executable | Purpose |
| --- | --- | --- |
| `vtk-wrap` | `vtk_wrap` | Rust wrappers |
| `vtk-wrap-sys` | `vtk_wrap_sys` | Raw FFI and native shim |
| `vtk-wrap-gen` | `vtk-wrap-gen` | Binding generator |

## Safety boundary

Generated `vtk*` types live under `vtk_wrap::raw`. Every native method,
including constructors and inherited methods, requires `unsafe`; raw handles
have no `Default` implementation. The caller upholds native initialization,
index bounds, buffer sizes, pointer lifetimes, ownership, aliasing, and
thread-affinity requirements.

- Raw `Clone` retains a reference; `Drop` releases it.
- Raw base-class methods are available through `Deref`. Overloads use suffixes
  such as `_v2`; generated docs include aliases for the original C++ names.
- Borrowed object results take an extra reference; owned raw-pointer results
  can still leak because ownership-transfer conventions are not fully modeled.
- Value types such as `vtkVector` and `vtkVariant`, most templates, reference
  output parameters, multi-dimensional arrays, and callbacks are unsupported
  and omitted.

See [Binding design](docs/design.md) for the handle contract and type mapping.

## Development

```sh
prek install          # formatting, spelling and workflow hooks
prek run --all-files
cargo test --workspace
```

The generated accessor sweep deliberately violates native preconditions and
can crash. Run it only in a disposable process, with explicit opt-in:

```sh
cargo test -p vtk-wrap --test smoke --features crash-hunt
# Rendering uses the raw API and needs a macOS window server/main thread:
cargo test -p vtk-wrap --test rendering --features vtkRenderingOpenGL2,raw-rendering-tests
```

Generated sources are committed; users do not need the wrapping tools.

## Acknowledgements

This project uses [VTK](https://vtk.org/) and David Gobbi's
[WrapVTK](https://github.com/dgobbi/WrapVTK).
[vtk-pure-rs](https://github.com/henriksson-lab/vtk-pure-rs) reimplements VTK
in Rust instead of wrapping the C++ library.

Licensed under [BSD-3-Clause](LICENSE), the same license as VTK.
