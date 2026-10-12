use vtk_wrap::raw::vtkObject;

fn main() {
    let _ = vtkObject::new();
    let _ = vtkObject::try_new();
}
