//! End-to-end tests for the unsafe generated bindings.

use vtk_wrap::raw::vtkSphereSource;

#[cfg(feature = "vtkFiltersSources")]
#[test]
fn sphere_source_pipeline() {
    // SAFETY: an input-free `vtkSphereSource` has no extra construction
    // requirements, every setter receives a valid value, and each borrowed
    // result is used while the source that owns it is still alive.
    unsafe {
        let sphere = vtkSphereSource::new();
        sphere.set_radius(2.0);
        sphere.set_theta_resolution(16);
        sphere.set_phi_resolution(8);
        sphere.set_center([1.0, -2.0, 0.5]);

        // `update` comes from `vtkAlgorithm`.
        sphere.update();

        // `get_output` comes from `vtkPolyDataAlgorithm`, `get_points` from
        // `vtkPointSet`, `get_number_of_points` from `vtkPoints`; all reached
        // through `Deref`.
        let polydata = sphere.get_output().expect("output");
        let points = polydata.get_points().expect("points");
        let n = points.get_number_of_points();
        assert!(n > 0, "sphere produced no points");

        // `get_class_name` comes from `vtkObjectBase`.
        assert_eq!(sphere.get_class_name(), "vtkSphereSource");
        assert_eq!(sphere.get_radius(), 2.0);
        assert_eq!(sphere.get_theta_resolution(), 16);
    }
}

#[cfg(feature = "vtkFiltersSources")]
#[test]
fn reference_counting() {
    // SAFETY: the handle owns one reference; cloning retains and dropping
    // releases it, so the count round-trips.
    unsafe {
        let sphere = vtkSphereSource::new();
        let before = sphere.get_reference_count();
        {
            let clone = sphere.clone();
            assert_eq!(clone.get_reference_count(), before + 1);
        }
        assert_eq!(sphere.get_reference_count(), before);
    }
}
