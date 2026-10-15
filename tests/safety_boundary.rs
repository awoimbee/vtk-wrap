#[cfg(feature = "vtkCommonCore")]
#[test]
fn raw_api_requires_explicit_unsafe() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/raw_constructor.rs");
    tests.compile_fail("tests/ui/raw_methods.rs");
    tests.compile_fail("tests/ui/raw_default.rs");
    tests.compile_fail("tests/ui/private_ffi.rs");
    #[cfg(feature = "vtkCommonDataModel")]
    tests.compile_fail("tests/ui/uninitialized_iterator.rs");
    tests.pass("tests/ui/explicit_raw.rs");
}
