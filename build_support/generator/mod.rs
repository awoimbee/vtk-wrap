//! Generate C++ shims, Rust FFI declarations, unsafe handles and diagnostics
//! from the wrapping metadata of a configured VTK installation.

#![allow(dead_code)]

mod gen_cpp;
mod gen_rust;
mod model;
mod parse;
pub mod wrapvtk;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const SUPPORT: &[(&str, &str)] = &[
    (
        "vtk_wrap_shim.h",
        include_str!("../support/vtk_wrap_shim.h"),
    ),
    (
        "vtk_wrap_shim.cpp",
        include_str!("../support/vtk_wrap_shim.cpp"),
    ),
    ("link_probe.cpp", include_str!("../support/link_probe.cpp")),
];

/// Generate a self-contained output tree. `output` need not be a repository;
/// it contains `shim`, `ffi`, `raw` and `smoke` directories.
/// Only selected modules are parsed and emitted.
pub fn generate(
    xml_dir: &Path,
    vtk_include: Option<&Path>,
    modules: &[String],
    output: &Path,
) -> Result<String> {
    let api = parse::build_api(
        xml_dir,
        &parse::BuildOptions {
            modules: modules.to_vec(),
            vtk_include: vtk_include.map(Path::to_path_buf),
        },
    )?;
    let shim = output.join("shim");
    let sources = shim.join("src");
    fs::create_dir_all(&sources)?;
    prune(&sources, "cpp", |stem| api.classes.contains_key(stem))?;
    for class in api.classes.values() {
        write_if_changed(
            &sources.join(format!("{}.cpp", class.name)),
            gen_cpp::emit_class_cpp(class, &api.headers).as_bytes(),
        )?;
    }
    write_if_changed(
        &shim.join("CMakeLists.txt"),
        gen_cpp::emit_cmake(&api).as_bytes(),
    )?;
    for (name, contents) in SUPPORT {
        write_if_changed(&shim.join("support").join(name), contents.as_bytes())?;
    }
    let modules = &api.modules;
    write_modules(&output.join("ffi/generated"), modules, |m| {
        gen_rust::emit_ffi_module(&api, m)
    })?;
    write_if_changed(
        &output.join("ffi/generated.rs"),
        gen_rust::emit_ffi_index(modules).as_bytes(),
    )?;
    write_modules(&output.join("raw/generated"), modules, |m| {
        gen_rust::emit_wrappers_module(&api, m)
    })?;
    write_if_changed(
        &output.join("raw/generated.rs"),
        gen_rust::emit_wrappers_index(modules).as_bytes(),
    )?;
    write_modules(&output.join("smoke/smoke"), modules, |m| {
        gen_rust::emit_smoke_module(&api, m)
    })?;
    write_if_changed(
        &output.join("smoke/smoke.rs"),
        gen_rust::emit_smoke_index(modules).as_bytes(),
    )?;
    Ok(gen_rust::summary(&api))
}

/// A content fingerprint of the generator and its embedded native support.
/// Native caches must be invalidated when any generation input changes, even
/// during development without a package version bump.
pub fn fingerprint() -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for source in [
        env!("CARGO_PKG_VERSION"),
        include_str!("mod.rs"),
        include_str!("gen_cpp.rs"),
        include_str!("gen_rust.rs"),
        include_str!("model.rs"),
        include_str!("parse.rs"),
        include_str!("wrapvtk.rs"),
        include_str!("../support/wrapvtk-CMakeLists.txt"),
    ] {
        source.hash(&mut hash);
    }
    SUPPORT.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

/// Preserve timestamps when content has not changed, so native incremental
/// builds and Cargo don't recompile otherwise identical generated code.
pub fn write_if_changed(path: &Path, contents: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|old| old == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

/// Copy generated sources into a crate's OUT_DIR without touching its checkout.
pub fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(destination)? {
        let path = entry?.path();
        if !source.join(path.file_name().unwrap()).exists() {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            write_if_changed(&target, &fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

fn write_modules<F>(dir: &Path, modules: &[String], emit: F) -> Result<()>
where
    F: Fn(&str) -> String,
{
    fs::create_dir_all(dir)?;
    prune(dir, "rs", |stem| modules.iter().any(|m| m == stem))?;
    for module in modules {
        write_if_changed(&dir.join(format!("{module}.rs")), emit(module).as_bytes())?;
    }
    Ok(())
}

fn prune(dir: &Path, extension: &str, keep: impl Fn(&str) -> bool) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == extension)
            && !keep(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default(),
            )
        {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_self_contained_incremental_and_feature_scoped() {
        let root = std::env::temp_dir().join(format!("vtk-wrap-generate-{}", std::process::id()));
        let xml = root.join("xml");
        let output = root.join("output");
        for (module, class) in [
            ("vtkCommonCore", "vtkAlpha"),
            ("vtkCommonDataModel", "vtkBeta"),
        ] {
            fs::create_dir_all(xml.join(module)).unwrap();
            fs::write(xml.join(module).join(format!("{class}.xml")), format!(
                "<file name=\"{class}.h\"><class name=\"{class}\"><base name=\"vtkObjectBase\"/><method name=\"GetValue\"><return type=\"int\"/></method></class></file>"
            )).unwrap();
        }
        let core = vec!["vtkCommonCore".into()];
        generate(&xml, None, &[], &output).unwrap();
        let alpha = output.join("shim/src/vtkAlpha.cpp");
        let before = fs::metadata(&alpha).unwrap().modified().unwrap();
        generate(&xml, None, &core, &output).unwrap();
        assert_eq!(before, fs::metadata(alpha).unwrap().modified().unwrap());
        assert!(!output.join("shim/src/vtkBeta.cpp").exists());
        for dir in ["ffi/generated", "raw/generated", "smoke/smoke"] {
            assert!(output.join(dir).join("vtkCommonCore.rs").is_file());
            assert!(!output.join(dir).join("vtkCommonDataModel.rs").exists());
        }
        assert!(output.join("shim/support/vtk_wrap_shim.cpp").is_file());
        assert!(output.join("shim/support/link_probe.cpp").is_file());
        let wrappers = fs::read_to_string(output.join("raw/generated.rs")).unwrap();
        let smoke = fs::read_to_string(output.join("smoke/smoke.rs")).unwrap();
        assert!(!wrappers.contains("//!"));
        assert!(!smoke.contains("#!["));
        assert!(!smoke.contains("//!"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copying_preserves_unchanged_files_and_removes_stale_modules() {
        let root = std::env::temp_dir().join(format!("vtk-wrap-copy-{}", std::process::id()));
        let source = root.join("source");
        let dest = root.join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("current.rs"), "current").unwrap();
        copy_tree(&source, &dest).unwrap();
        let before = fs::metadata(dest.join("current.rs"))
            .unwrap()
            .modified()
            .unwrap();
        fs::write(dest.join("stale.rs"), "stale").unwrap();
        copy_tree(&source, &dest).unwrap();
        assert!(!dest.join("stale.rs").exists());
        assert_eq!(
            before,
            fs::metadata(dest.join("current.rs"))
                .unwrap()
                .modified()
                .unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
