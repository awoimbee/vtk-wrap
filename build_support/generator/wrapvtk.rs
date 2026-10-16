//! Building vendored WrapVTK and producing the XML API description.
//!
//! The bindings are generated from VTK's own wrapping metadata.  That metadata
//! is not shipped with VTK: it is produced by [WrapVTK]'s `vtkWrapXML`, a tool
//! that reads the installed VTK headers and writes one directory of `*.xml`
//! per module.
//!
//! This module builds the bundled `vtkWrapXML` against VTK and hands its XML
//! output to the generator. Sources are read-only; all build files are cached.
//!
//! [WrapVTK]: https://github.com/dgobbi/WrapVTK

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Separate CMake project selecting the requested modules without patching
/// the upstream sources.
const WRAPVTK_CMAKE: &str = include_str!("../support/wrapvtk-CMakeLists.txt");

/// Everything the XML step needs to know.
pub struct Request {
    /// Package root, containing vendor/WrapVTK.
    pub repo: PathBuf,
    /// VTK CMake package directory (or install prefix).  `None` means "find it".
    pub vtk_dir: Option<PathBuf>,
    /// Local source override. `None` uses the bundled vendor/WrapVTK tree.
    pub wrapvtk_dir: Option<PathBuf>,
    /// Out-of-source build tree, separate for each module set.
    pub build_dir: Option<PathBuf>,
    /// VTK modules to wrap, in VTK's spelling (`vtkCommonCore`).  Empty means all.
    pub modules: Vec<String>,
    /// Value for `cmake --build --parallel`.
    pub jobs: Option<String>,
}

/// Build WrapVTK as needed and return the directory holding the XML.
pub fn ensure_xml(req: &Request) -> Result<PathBuf> {
    let vtk_dir = resolve_vtk_dir(req.vtk_dir.as_deref(), &req.repo)?;
    println!("Using VTK at {}", vtk_dir.display());
    let wrapvtk_dir = source_dir(&req.repo, req.wrapvtk_dir.as_deref())?;
    let build = req
        .build_dir
        .clone()
        .unwrap_or_else(|| req.repo.join("target/wrapvtk/build"));
    build_wrapvtk(
        &wrapvtk_dir,
        &build,
        &vtk_dir,
        &req.modules,
        req.jobs.as_deref(),
    )?;

    let xml = build.join("xml");
    if !xml.is_dir() {
        bail!(
            "WrapVTK did not produce an XML directory at {}.\n\
             Check the output above for the module list it actually wrapped.",
            xml.display()
        );
    }
    Ok(xml)
}

// ---------------------------------------------------------------------------
// VTK
// ---------------------------------------------------------------------------

/// Locate a VTK CMake package directory (the one containing `vtk-config.cmake`).
///
/// The pinned VTK built by this package is preferred over system installations.
/// The lookup order is:
///
/// 1. an explicit `--vtk-dir`,
/// 2. `VTK_DIR`,
/// 3. the pinned build under `VTK_WRAP_CACHE_DIR` or `<repo>/target/vtk-wrap-vtk`,
/// 4. a system install (Homebrew, `/usr/local`, `/usr`).
pub fn resolve_vtk_dir(explicit: Option<&Path>, repo: &Path) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return check_vtk_dir(dir);
    }
    if let Some(dir) = std::env::var_os("VTK_DIR") {
        return check_vtk_dir(Path::new(&dir));
    }
    if let Some(dir) = pinned_vtk_dir(repo) {
        return Ok(dir);
    }

    let mut prefixes: Vec<PathBuf> = Vec::new();
    // `brew --prefix vtk` knows the real location; the literal Homebrew
    // prefixes are the fallback when `brew` cannot be run (locked-down CI).
    if let Some(prefix) = brew_prefix() {
        prefixes.push(prefix);
    }
    prefixes.push(PathBuf::from("/opt/homebrew"));
    prefixes.push(PathBuf::from("/usr/local"));
    prefixes.push(PathBuf::from("/usr"));

    for prefix in prefixes {
        if let Some(dir) = find_vtk_under(&prefix) {
            return Ok(dir);
        }
    }
    bail!(
        "could not find VTK.\n\
         WrapVTK needs a VTK that includes the `WrappingTools` component.  Build\
         the pinned one first (`cargo build`), or set VTK_DIR\
         to the directory that contains `vtk-config.cmake`, \
         usually `<prefix>/lib/cmake/vtk-<version>`."
    )
}

/// The pinned VTK downloaded and built by `build.rs`, if it is present.
///
/// The native build installs it at
/// `<cache>/vtk-<version>/build`, where `<cache>` is `VTK_WRAP_CACHE_DIR` or
/// `<repo>/target/vtk-wrap-vtk`.  Reading it from there means regeneration uses
/// the same release as the native build, no matter what is installed system-wide.
fn pinned_vtk_dir(repo: &Path) -> Option<PathBuf> {
    let cache = match std::env::var_os("VTK_WRAP_CACHE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => repo.join("target/vtk-wrap-vtk"),
    };
    find_pinned_vtk_under(&cache)
}

fn find_pinned_vtk_under(cache: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(cache).ok()?;
    // The native build installs each release at `<cache>/vtk-<version>/build`;
    // turn that into the `.../lib/cmake/vtk-<version>` package directory, which
    // is what the rest of this module works with.
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| find_vtk_under(&entry.path().join("build")))
        .collect();
    // Newest pinned release wins, so a stray older cache cannot shadow it.
    candidates.sort_by_key(|dir| version_key(dir));
    candidates.pop()
}

fn check_vtk_dir(dir: &Path) -> Result<PathBuf> {
    if dir.join("vtk-config.cmake").is_file() {
        return Ok(dir.to_path_buf());
    }
    // A bare install prefix was passed; look for the CMake package below it.
    if let Some(found) = find_vtk_under(dir) {
        return Ok(found);
    }
    bail!(
        "{} does not look like VTK: no `vtk-config.cmake` there or under \
         `lib/cmake`.",
        dir.display()
    )
}

/// Newest `lib/cmake/vtk-*` below an install prefix.
fn find_vtk_under(prefix: &Path) -> Option<PathBuf> {
    let base = prefix.join("lib/cmake");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&base)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("vtk-"))
                && path.join("vtk-config.cmake").is_file()
        })
        .collect();
    // `vtk-9.9` must sort after `vtk-9.10`, hence the numeric comparison.
    candidates.sort_by_key(|path| version_key(path));
    candidates.pop()
}

fn version_key(path: &Path) -> Vec<u32> {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .trim_start_matches("vtk-")
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

fn brew_prefix() -> Option<PathBuf> {
    let output = Command::new("brew").args(["prefix", "vtk"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// The include directory that goes with a VTK CMake package.
///
/// Follows the layout CMake installs: `<prefix>/lib/cmake/vtk-<x.y>` next to
/// `<prefix>/include/vtk-<x.y>`.
pub fn vtk_include_dir(vtk_dir: &Path) -> Option<PathBuf> {
    let version = vtk_dir.file_name()?.to_str()?.trim_start_matches("vtk-");
    // <prefix>/lib/cmake/vtk-<version>
    let prefix = vtk_dir.parent()?.parent()?.parent()?;
    let candidate = prefix.join("include").join(format!("vtk-{version}"));
    if candidate.is_dir() {
        return Some(candidate);
    }
    // Some installs keep the headers directly in `include/vtk`.
    let fallback = prefix.join("include/vtk");
    fallback.is_dir().then_some(fallback)
}

// ---------------------------------------------------------------------------
// WrapVTK
// ---------------------------------------------------------------------------

/// Resolve a bundled or explicitly supplied local source tree. Missing sources
/// are an error, never a request to fetch anything from the network.
pub fn source_dir(package: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    let dir = explicit
        .map(Path::to_path_buf)
        .unwrap_or_else(|| package.join("vendor/WrapVTK"));
    let dir = super::canonicalize(&dir)
        .with_context(|| format!("locating WrapVTK sources at {}", dir.display()))?;
    for file in [
        "Source/CMakeLists.txt",
        "Source/vtkWrapXML.c",
        "Source/vtkParseProperties.c",
        "Source/vtkParseProperties.h.in",
        "CMake/vtkModuleWrapXML.cmake",
    ] {
        if !dir.join(file).is_file() {
            bail!(
                "{} is not a WrapVTK source tree: missing {file}",
                dir.display()
            );
        }
    }
    Ok(dir)
}

/// Write our CMake adapter inside the build tree, never in the source tree.
fn prepare_project(build: &Path) -> Result<PathBuf> {
    let project = build.join("project");
    super::write_if_changed(&project.join("CMakeLists.txt"), WRAPVTK_CMAKE.as_bytes())?;
    Ok(project)
}

fn build_wrapvtk(
    dir: &Path,
    build: &Path,
    vtk_dir: &Path,
    modules: &[String],
    jobs: Option<&str>,
) -> Result<()> {
    let project = prepare_project(build)?;
    let mut configure = Command::new("cmake");
    configure.arg("-S").arg(&project).arg("-B").arg(&build);
    // Without this CMake picks its default generator (Visual Studio / NMake on
    // Windows), which does not match the MinGW toolchain VTK is built with and
    // fails with `CMAKE_C_COMPILER not set`.  Mirror the generator the `cmake`
    // crate selects for the VTK build.
    if let Some(generator) = cmake_generator() {
        configure.arg("-G").arg(generator);
    }
    configure
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DVTK_DIR={}", vtk_dir.display()))
        .arg(format!("-DWRAPVTK_SOURCE_DIR={}", dir.display()));
    if !modules.is_empty() {
        // WrapVTK wants the component names, i.e. without the `vtk` prefix.
        let components = modules
            .iter()
            .map(|module| module.trim().trim_start_matches("vtk"))
            .collect::<Vec<_>>()
            .join(";");
        configure.arg(format!("-DWRAPVTK_MODULES={components}"));
    }
    run(&mut configure, "cmake configure")?;

    let mut compile = Command::new("cmake");
    compile.arg("--build").arg(&build);
    // Multi-config generators default to Debug; match the Release VTK this tool
    // links against (and rustc's release C runtime).
    compile.arg("--config").arg("Release");
    match jobs {
        Some(jobs) => {
            compile.arg("--parallel").arg(jobs);
        }
        None => {
            compile.arg("--parallel");
        }
    }
    run(&mut compile, "cmake build")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// module list
// ---------------------------------------------------------------------------

/// The `vtk*` features declared by this package's Cargo.toml.
///
/// Those features are the single source of truth for which modules this crate
/// supports, so they also drive which modules WrapVTK wraps.  Deriving the list
/// here means the generated XML, the generated bindings and the Cargo features
/// cannot drift apart.
///
/// Returns an empty list when the manifest has no `vtk*` features, which the
/// caller reads as "wrap everything".
pub fn modules_from_features(repo: &Path) -> Vec<String> {
    modules_from_manifest(&repo.join("Cargo.toml"))
}

/// Module features from a package manifest, also usable outside a workspace.
pub fn modules_from_manifest(manifest: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return Vec::new();
    };

    let mut modules = Vec::new();
    let mut in_features = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            // Leave `[features]` at the next table.
            in_features = line == "[features]";
            continue;
        }
        if !in_features || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.starts_with("vtk") && key.chars().all(|c| c.is_ascii_alphanumeric()) {
            modules.push(key.to_owned());
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The CMake generator to use for the WrapVTK build.
///
/// `CMAKE_GENERATOR` wins; otherwise, on a MinGW target, use the generator the
/// `cmake` crate uses for the VTK build.  Other platforms keep CMake's default.
fn cmake_generator() -> Option<String> {
    if let Ok(generator) = env::var("CMAKE_GENERATOR") {
        if !generator.trim().is_empty() {
            return Some(generator);
        }
    }
    (env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") && cfg!(windows))
        .then(|| "MinGW Makefiles".to_owned())
}

fn run(command: &mut Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .map_err(|error| anyhow::anyhow!("running {what}: {error}"))?;
    if !status.success() {
        bail!("{what} failed with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_key_orders_numerically() {
        let mut dirs = vec![
            PathBuf::from("/x/lib/cmake/vtk-9.9"),
            PathBuf::from("/x/lib/cmake/vtk-9.10"),
            PathBuf::from("/x/lib/cmake/vtk-9.7"),
        ];
        dirs.sort_by_key(|path| version_key(path));
        assert_eq!(dirs.last().unwrap().file_name().unwrap(), "vtk-9.10");
    }

    #[test]
    fn include_dir_follows_cmake_layout() {
        let base = std::env::temp_dir().join(format!("vtk-wrap-inc-{}", std::process::id()));
        let cmake_dir = base.join("lib/cmake/vtk-9.7");
        let include_dir = base.join("include/vtk-9.7");
        std::fs::create_dir_all(&cmake_dir).unwrap();
        std::fs::create_dir_all(&include_dir).unwrap();
        assert_eq!(
            vtk_include_dir(&cmake_dir).as_deref(),
            Some(include_dir.as_path())
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn cmake_adapter_uses_external_sources_and_build_directory() {
        assert!(WRAPVTK_CMAKE.contains("WRAPVTK_MODULES"));
        assert!(WRAPVTK_CMAKE.contains("${WRAPVTK_SOURCE_DIR}/Source"));
        let root = std::env::temp_dir().join(format!("vtk-wrap-project-{}", std::process::id()));
        let project = prepare_project(&root).unwrap();
        assert_eq!(project, root.join("project"));
        assert_eq!(
            std::fs::read_to_string(project.join("CMakeLists.txt")).unwrap(),
            WRAPVTK_CMAKE
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn vendored_sources_and_license_are_shipped() {
        let package = Path::new(env!("CARGO_MANIFEST_DIR"));
        let bundled = source_dir(package, None).unwrap();
        assert_eq!(
            bundled,
            crate::generator::canonicalize(&package.join("vendor/WrapVTK")).unwrap()
        );
        assert!(bundled.join("Copyright.txt").is_file());
        assert!(bundled.join("VENDORED.md").is_file());
    }

    #[test]
    fn invalid_source_override_does_not_fetch_or_create_sources() {
        let root = std::env::temp_dir().join(format!("vtk-wrap-missing-{}", std::process::id()));
        assert!(!root.exists());
        assert!(source_dir(Path::new(env!("CARGO_MANIFEST_DIR")), Some(&root)).is_err());
        assert!(!root.exists());
    }

    #[test]
    fn modules_come_from_the_cargo_features() {
        let base = std::env::temp_dir().join(format!("vtk-wrap-feat-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("Cargo.toml"),
            "[package]\nname = \"vtk-wrap\"\n\n[features]\ndefault = [\"vtkCommonCore\"]\n\
             vtkCommonCore = []\nvtkFiltersCore = [\"vtkCommonCore\"]\n\n[dependencies]\ncmake = \"0.1\"\n",
        )
        .unwrap();
        assert_eq!(
            modules_from_features(&base),
            vec!["vtkCommonCore".to_owned(), "vtkFiltersCore".to_owned()]
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn no_features_means_wrap_everything() {
        let base = std::env::temp_dir().join(format!("vtk-wrap-none-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        assert!(modules_from_features(&base).is_empty());
        std::fs::remove_dir_all(&base).ok();
    }

    /// The cached pinned VTK is reported as a CMake package directory.
    #[test]
    fn pinned_vtk_is_found_in_the_cache() {
        let repo = std::env::temp_dir().join(format!("vtk-wrap-pin-{}", std::process::id()));
        let cache = repo.join("target/vtk-wrap-vtk");
        let build = cache.join("vtk-9.7.1/build");
        std::fs::create_dir_all(build.join("lib/cmake/vtk-9.7")).unwrap();
        std::fs::write(build.join("lib/cmake/vtk-9.7/vtk-config.cmake"), "").unwrap();
        std::fs::create_dir_all(build.join("include/vtk-9.7")).unwrap();

        let found = find_pinned_vtk_under(&cache).expect("pinned VTK not found");
        assert_eq!(found, build.join("lib/cmake/vtk-9.7"));
        assert_eq!(
            vtk_include_dir(&found).as_deref(),
            Some(build.join("include/vtk-9.7").as_path())
        );
        std::fs::remove_dir_all(&repo).ok();
    }
}
