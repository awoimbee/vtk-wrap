# Building and generating bindings

[Back to the README](../README.md)

`vtk-wrap` is one crate. Its build script generates the API from the configured
headers of the VTK installation it actually links. The pinned release is VTK
9.7.1. VTK and the C++ shim are static, so an executable needs no VTK shared
libraries.

## Pipeline

1. [`build.rs`](../build.rs) downloads the pinned VTK archive, verifies its
   SHA-256, and builds/installs VTK. Cargo features select modules and their
   dependencies; the `WrappingTools` component is also enabled.
2. Private [`build_support/generator/`](../build_support/generator/) code
   builds the [vendored WrapVTK](../vendor/WrapVTK/VENDORED.md) tool and produces
   XML from that installation's headers and module metadata. No Git command or
   wrapping-tool download is involved.
3. The generator emits the C++ shim, private FFI declarations, public unsafe
   Rust handles, and opt-in diagnostics for the selected modules/platform.
4. The same build script compiles the shim and emits native link directives
   from a CMake link probe. Generated `ffi/`, `raw/`, and `smoke/` Rust files
   are copied from the persistent generation cache into this crate's `OUT_DIR`.
5. [`src/lib.rs`](../src/lib.rs) includes the private `ffi` module and public
   `raw` module. There is no dependency-metadata handoff or second build script.

Normal builds never write generated sources into the checkout. Feature sets
have separate generation/shim trees, and unchanged files retain their timestamps.
Cache fingerprints include the target, modules, VTK headers/CMake metadata,
parser library, generator/build executable, and wrapping-tool sources. A cache
lock serializes builds sharing the same cache.

VTK is installed under `<cache>/vtk-<version>/build/`; the default cache root
is `target/vtk-wrap-vtk/`. Wrapping-tool binaries, XML, generated sources and
shim builds live alongside it. The bundled source tree is read-only; its CMake
adapter is written inside each wrapping-tool build directory. Use separate
native caches for different target architectures.

| Environment variable | Purpose |
| --- | --- |
| `VTK_WRAP_CACHE_DIR` | Relocate all native, wrapping-tool and generation caches. |
| `VTK_WRAP_VTK_SOURCE_DIR` | Build a local VTK source tree instead of downloading. |
| `VTK_WRAP_VTK_PREBUILT_DIR` | Generate/link against an existing VTK install prefix. |
| `VTK_WRAP_VTK_URL` | Override the download URL for the pinned VTK archive. |
| `VTK_WRAP_WRAPVTK_SOURCE_DIR` | Override the vendored WrapVTK sources for development. |
| `DOCS_RS` | Use Rust documentation snapshots and skip all native work. |

Local VTK sources and prebuilt installations must match the pinned release.
A prebuilt must contain the enabled modules **and `WrappingTools`**, and be
configured with `BUILD_SHARED_LIBS=OFF` and `CMAKE_POSITION_INDEPENDENT_CODE=ON`.
The package includes all WrapVTK sources and its BSD-3-Clause copyright notice.
With local VTK and an available Cargo dependency cache, a cold native build
needs neither Git nor network access. An invalid wrapping-source override is
an error, never a request to clone or download sources.

## Generator development and documentation snapshots

Generator changes automatically invalidate build-time generation; manual
regeneration is not needed before building or testing. Generator unit tests
are compiled through [`tests/generator.rs`](../tests/generator.rs), using the
same private modules as `build.rs`:

```sh
cargo test --test generator
```

There is no standalone generator crate or CLI. To export an inspection tree
under `target/vtk-wrap-generated/`, run:

```sh
./scripts/regenerate.sh
MODULES=vtkCommonCore,vtkCommonDataModel ./scripts/regenerate.sh
```

`UPDATE_DOCS=1 ./scripts/regenerate.sh` enables every `vtk*` module and also
refreshes [`doc_snapshots/`](../doc_snapshots/). Those Rust-only snapshots let
docs.rs build without native VTK, Git, or network access. They are never used
by native builds. Platform-specific classes in snapshots describe the machine
used to refresh them, not every target.

For existing XML, set `VTK_WRAP_XML_DIR`. The developer export test also accepts
`VTK_WRAP_GENERATED_DIR` to override its output directory. These variables
only affect explicit exports, not the normal build pipeline. A snapshot export
requires XML for every module declared in [`Cargo.toml`](../Cargo.toml).

## Platform support

Linux and macOS generate their own class lists and integer signatures. Native
array element types are preserved, and scalar casts select the intended
integer overloads. Compile-time size/alignment checks enforce the Rust integer
ABI. CI runs fast tests on both platforms and compiles Linux OpenGL bindings
without opening a window.

Native link reconstruction still depends on CMake's `link.txt`, which the
default Windows Visual Studio generator does not produce. Windows CI remains
non-blocking.

Cross-compilation is explicitly rejected rather than generating host bindings
for a target VTK. Supporting it requires host-runnable wrapping tools configured
to parse target headers and compiler definitions.
