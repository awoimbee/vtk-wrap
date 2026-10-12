use vtk_wrap::raw::vtkObject;

fn main() {
    // SAFETY: an ordinary vtkObject is live on this thread; these metadata
    // queries require no additional initialization and preserve its reference.
    unsafe {
        let object = vtkObject::new();
        assert_eq!(object.is_a("vtkObject"), 1);
        let count = object.get_reference_count();
        let clone = object.clone();
        assert_eq!(object.get_reference_count(), count + 1);
        drop(clone);
        assert_eq!(object.get_reference_count(), count);
    }
}
