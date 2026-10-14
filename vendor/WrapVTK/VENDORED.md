# Vendored WrapVTK

- Upstream: <https://github.com/dgobbi/WrapVTK>
- Revision: `202e8146496bb969732dad520025d91a67f91b61`
- License: BSD-3-Clause; see [Copyright.txt](Copyright.txt).

All upstream tracked files at this revision are included unchanged. Git
metadata, build outputs, and generated XML are not included.

The build uses a separate CMake project from
[`build_support/support/wrapvtk-CMakeLists.txt`](../../build_support/support/wrapvtk-CMakeLists.txt),
which selects Cargo's enabled VTK modules. It never patches this source tree.

To update, export the desired reviewed upstream commit with `git archive`,
replace the upstream files here, and update the revision above. Preserve
upstream copyright/license notices. Run the generator tests, native builds,
and package verification after updating.
