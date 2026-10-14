use vtk_wrap::raw::vtkCellArrayIterator;

fn read(iterator: &vtkCellArrayIterator) {
    let _ = iterator.get_current_cell();
}

fn main() {}
