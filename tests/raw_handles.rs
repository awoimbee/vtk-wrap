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

#[cfg(feature = "vtkCommonCore")]
#[test]
fn native_64_bit_integer_array_ranges() {
    // SAFETY: each array has one component and two initialized values. The
    // range buffers hold two elements, and component zero exists.
    unsafe {
        use vtk_wrap::raw::{vtkIdTypeArray, vtkLongLongArray, vtkUnsignedLongLongArray};

        let values = [-(1_i64 << 40), 1_i64 << 42];
        let ids = vtkIdTypeArray::new();
        let signed = vtkLongLongArray::new();
        for value in values {
            ids.insert_next_value(value);
            signed.insert_next_value(value);
        }
        let mut range = [0; 2];
        ids.get_value_range(&mut range, 0);
        assert_eq!(range, values);
        signed.get_value_range(&mut range, 0);
        assert_eq!(range, values);

        let values = [1_u64 << 40, (1_u64 << 63) + 17];
        let unsigned = vtkUnsignedLongLongArray::new();
        for value in values {
            unsigned.insert_next_value(value);
        }
        let mut range = [0; 2];
        unsigned.get_value_range(&mut range, 0);
        assert_eq!(range, values);
    }
}

#[cfg(feature = "vtkCommonDataModel")]
#[test]
fn native_64_bit_integer_attribute_overloads() {
    // SAFETY: node zero is the assembly's root, and the attribute names are
    // NUL-free strings. VTK copies the scalar values into the assembly.
    unsafe {
        use vtk_wrap::raw::vtkDataAssembly;
        let assembly = vtkDataAssembly::new();
        let value = (1_i64 << 42) + 17;
        let default = -(1_i64 << 40);
        assembly.set_attribute_v4(0, "wide", value);
        assert_eq!(
            assembly.get_attribute_or_default_v4(0, "wide", default),
            value
        );
        assert_eq!(
            assembly.get_attribute_or_default_v4(0, "missing", default),
            default
        );
    }
}

#[cfg(feature = "vtkCommonDataModel")]
#[test]
fn native_id_slice_cell_connectivity() {
    // SAFETY: all three point IDs exist, the polydata has allocated cell
    // storage, and VTK copies the three IDs from the slice during insertion.
    unsafe {
        use vtk_wrap::raw::{vtkIdList, vtkPoints, vtkPolyData};
        let points = vtkPoints::new();
        points.insert_next_point([0.0, 0.0, 0.0]);
        points.insert_next_point([1.0, 0.0, 0.0]);
        points.insert_next_point([0.0, 1.0, 0.0]);
        let polydata = vtkPolyData::new();
        polydata.set_points(&points);
        assert!(polydata.allocate_estimate(1, 3));
        let ids = [0, 1, 2];
        let cell = polydata.insert_next_cell_v2(5, 3, &ids); // VTK_TRIANGLE
        assert_eq!(cell, 0);
        assert_eq!(polydata.get_number_of_cells(), 1);
        let result = vtkIdList::new();
        polydata.get_cell_points(cell, &result);
        assert_eq!(result.get_number_of_ids(), 3);
        for (index, expected) in ids.into_iter().enumerate() {
            assert_eq!(result.get_id(index as i64), expected);
        }
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
