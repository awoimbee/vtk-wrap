use vtk_wrap::raw::vtkObject;

fn read(object: &vtkObject) {
    let _ = object.is_a("vtkObject");
    let _ = object.get_class_name();
    object.modified();
    let _ = vtkObject::is_type_of("vtkObject");
}

fn main() {}
