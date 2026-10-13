# Building and regenerating

[Back to the README](../README.md)

The default build compiles the pinned VTK release and the C++ shim
statically, so a resulting executable needs no VTK shared libraries.

## Pipeline

1. [`vtk-wrap-sys/build.rs`](../vtk-wrap-sys/build.rs) downloads the pinned
   release archive, verifies its SHA-256, and unpacks it in the native cache.
2. CMake builds and installs VTK with `BUILD_SHARED_LIBS=OFF` and position
   independent code. Cargo features select the modules and their dependencies.
3. The shim is built against that installation, and a link probe turns CMake's
   link line into Rust linker directives.

VTK is installed under `<cache>/vtk-<version>/build/`; the default cache root
is `target/vtk-wrap-vtk/`. The shim is cached per feature set, so an unchanged
shim is not rebuilt.

| Environment variable | Purpose |
| --- | --- |
| `VTK_WRAP_CACHE_DIR` | Relocate the VTK and shim cache. |
| `VTK_WRAP_VTK_SOURCE_DIR` | Build from a local VTK source tree instead of downloading. |
| `VTK_WRAP_VTK_PREBUILT_DIR` | Use an existing VTK install prefix instead of building VTK. |
| `VTK_WRAP_VTK_URL` | Override the download URL for the pinned release archive. |
| `DOCS_RS` | Skip native compilation for documentation builds. |

`VTK_WRAP_VTK_SOURCE_DIR` is still built locally, and must be the pinned
release the committed bindings target. `VTK_WRAP_VTK_PREBUILT_DIR` must match
that version, contain the enabled modules, and — for static linking — have
been configured with `BUILD_SHARED_LIBS=OFF` and
`CMAKE_POSITION_INDEPENDENT_CODE=ON`.

## Regenerating bindings

The committed bindings target the version pinned in
[`vtk-wrap-sys/build.rs`](../vtk-wrap-sys/build.rs). Users do not need the
generator or WrapVTK.

```sh
cargo build -p vtk-wrap-sys
./scripts/regenerate.sh
```

[`vtk-wrap-gen`](../vtk-wrap-gen/src/main.rs) clones or reuses
[WrapVTK](https://github.com/dgobbi/WrapVTK), builds its `vtkWrapXML` tool,
and emits the C++ shim, raw FFI declarations, Rust wrappers, and smoke tests.
Generated files are overwritten; make binding changes in the generator.

By default the module list comes from the `vtk*` features declared in
[`vtk-wrap-sys/Cargo.toml`](../vtk-wrap-sys/Cargo.toml), not from the current
build. Select modules explicitly with:

```sh
MODULES=vtkCommonCore,vtkFiltersSources ./scripts/regenerate.sh
```

With a prebuilt VTK, point `VTK_DIR` at its CMake package directory; the
installation must include the `WrappingTools` component. List generator
options with `cargo run -p vtk-wrap-gen -- --help`.

## Platform limitations

The [CI workflow](../.github/workflows/ci.yml) builds and tests the
non-rendering modules on Linux and macOS; both jobs block PRs. Integer array
signatures preserve their native C++ element types while exposing fixed-width
Rust integers. Scalar arguments use their native types to select integer
overloads. Compile-time size and alignment checks enforce the integer ABI.

The committed sources were generated from macOS VTK headers, so some
platform-specific work remains:

- Cocoa-specific classes and headers in the rendering modules prevent those
  modules from building on Linux or Windows.
- Native link reconstruction depends on CMake's `link.txt`, which the default
  Windows Visual Studio generator does not produce.

The Windows CI job remains non-blocking until these are resolved.
