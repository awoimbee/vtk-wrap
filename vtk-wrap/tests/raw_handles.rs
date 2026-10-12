//! Focused tests with native preconditions explicitly established.

#[test]
fn reference_counting() {
    // SAFETY: vtkObject has no extra construction requirements, stays live on
    // this thread, and these metadata calls cannot invalidate its reference.
    unsafe {
        let object = vtk_wrap::raw::vtkObject::new();
        let before = object.get_reference_count();
        let clone = object.clone();
        assert_eq!(object.get_reference_count(), before + 1);
        drop(clone);
        assert_eq!(object.get_reference_count(), before);
    }
}

#[cfg(feature = "vtkIOCore")]
#[test]
fn smart_pointer_result_survives_native_temporary_and_original() {
    // SAFETY: URI parsing and cloning accept these NUL-free strings and live
    // objects without any other initialization. Each returned owned reference
    // must remain valid after the shim's temporary smart pointer is destroyed.
    unsafe {
        use vtk_wrap::raw::vtkURI;
        let uri = vtkURI::parse("https://example.com/a").expect("valid URI");
        assert_eq!(uri.get_reference_count(), 1);
        let copy = uri.clone_().expect("URI copy");
        assert_eq!(copy.get_reference_count(), 1);
        drop(uri);
        assert_eq!(copy.to_string(), "https://example.com/a");
    }
}
