//! Exercise the same private generator modules used by build.rs.

#[path = "../build_support/generator/mod.rs"]
mod generator;

/// Developer-only inspection/export. Normal builds do not write snapshots.
#[test]
#[ignore = "explicit inspection or documentation snapshot refresh"]
fn export_bindings() {
    use std::{env, fs, path::PathBuf};
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let update_docs = env::var("UPDATE_DOCS").as_deref() == Ok("1");
    let modules = if update_docs {
        generator::wrapvtk::modules_from_features(&package)
    } else {
        let mut modules: Vec<String> =
            fs::read_dir(PathBuf::from(env!("OUT_DIR")).join("ffi/generated"))
                .unwrap()
                .map(|entry| {
                    entry
                        .unwrap()
                        .path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
        modules.sort();
        modules
    };
    let explicit = env::var_os("VTK_WRAP_VTK_PREBUILT_DIR").map(PathBuf::from);
    let vtk = generator::wrapvtk::resolve_vtk_dir(explicit.as_deref(), &package).unwrap();
    let include = generator::wrapvtk::vtk_include_dir(&vtk).unwrap();
    let cache = env::var_os("VTK_WRAP_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| package.join("target/vtk-wrap-vtk"));
    let xml = match env::var_os("VTK_WRAP_XML_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => generator::wrapvtk::ensure_xml(&generator::wrapvtk::Request {
            repo: package.clone(),
            vtk_dir: Some(vtk),
            wrapvtk_dir: env::var_os("VTK_WRAP_WRAPVTK_SOURCE_DIR")
                .map(PathBuf::from)
                .or_else(|| {
                    Some(cache.join(format!(
                        "WrapVTK-{}",
                        generator::wrapvtk::DEFAULT_WRAPVTK_REVISION
                    )))
                }),
            build_dir: Some(cache.join("inspection-wrapxml")),
            wrapvtk_url: generator::wrapvtk::DEFAULT_WRAPVTK_URL.into(),
            modules: modules.clone(),
            jobs: None,
        })
        .unwrap(),
    };
    for module in &modules {
        assert!(
            xml.join(module).is_dir(),
            "XML has no {module}; documentation snapshots require every module"
        );
    }
    let output = env::var_os("VTK_WRAP_GENERATED_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| package.join("target/vtk-wrap-generated"));
    println!(
        "{}",
        generator::generate(&xml, Some(&include), &modules, &output).unwrap()
    );
    if update_docs {
        for layer in ["ffi", "raw"] {
            generator::copy_tree(
                &output.join(layer),
                &package.join("doc_snapshots").join(layer),
            )
            .unwrap();
        }
    }
    println!("Generated files: {}", output.display());
}
