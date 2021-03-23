use numpy::{npyffi::NPY_TYPES, Element, PyArrayDescr};
use pyo3::{ffi, Py, Python};
use std::str::from_utf8_unchecked;
use widestring::{U16String, U32String};
#[derive(Clone)]
#[repr(transparent)]
pub struct PyString(Py<pyo3::types::PyString>);

// In order to put it into a numpy array
impl Element for PyString {
    const DATA_TYPE: numpy::DataType = numpy::DataType::Object;
    fn is_same_type(dtype: &PyArrayDescr) -> bool {
        unsafe { *dtype.as_dtype_ptr() }.type_num == NPY_TYPES::NPY_OBJECT as i32
    }
}

// impl PyString {
//     pub fn new(py: Python, val: &[u8]) -> Self {
//         PyString(pyo3::types::PyString::new(py, unsafe { from_utf8_unchecked(val) }).into())
//     }
// }

impl PyString {
    pub fn new(py: Python, val: &[u8]) -> Self {
        let val = unsafe { from_utf8_unchecked(val) };
        let maxchar = val.chars().map(|c| c as u32).max().unwrap_or(0);
        let (maxchar, length) = if maxchar <= 0x7F {
            (0x7F, val.len())
        } else if maxchar <= 0xFF {
            (0xFF, val.chars().count())
        } else if maxchar <= 0xFFFF {
            (0xFFFF, val.chars().count())
        } else {
            (0x10FFFF, val.chars().count())
        };

        let objptr = unsafe { ffi::PyUnicode_New(length as ffi::Py_ssize_t, maxchar) };

        let s: &pyo3::types::PyString = unsafe { py.from_owned_ptr(objptr) };
        PyString(s.into())
    }

    // the val should be same as the val used for new
    pub unsafe fn write(&mut self, val: &[u8]) {
        let ascii = PyASCIIObject::from_owned(self.0.clone());
        let is_ascii = (ascii.state & 0x00000040) >> 6;
        if is_ascii == 1 {
            let buf = std::slice::from_raw_parts_mut(
                (ascii as *mut PyASCIIObject).offset(1) as *mut u8,
                ascii.length as usize,
            );
            buf.copy_from_slice(val);
        } else {
            let kind = (ascii.state & 0x0000001C) >> 2;
            let compact = PyCompactUnicodeObject::from_owned(self.0.clone());
            let val = from_utf8_unchecked(val);
            if kind == 1 {
                let chars: Vec<u8> = val.chars().map(|c| c as u8).collect();
                let buf = std::slice::from_raw_parts_mut(
                    (compact as *mut PyCompactUnicodeObject).offset(1) as *mut u8,
                    chars.len(),
                );
                buf.copy_from_slice(chars.as_slice());
            } else if kind == 2 {
                let ucs_string = U16String::from_str(val);
                let buf = std::slice::from_raw_parts_mut(
                    (compact as *mut PyCompactUnicodeObject).offset(1) as *mut u16,
                    ucs_string.len(),
                );
                buf.copy_from_slice(ucs_string.as_slice());
            } else {
                let ucs_string = U32String::from_str(val);
                let buf = std::slice::from_raw_parts_mut(
                    (compact as *mut PyCompactUnicodeObject).offset(1) as *mut u32,
                    ucs_string.len(),
                );
                buf.copy_from_slice(ucs_string.as_slice());
            }
        }
    }
}

#[repr(C)]
pub struct PyASCIIObject {
    obj: ffi::PyObject,
    length: ffi::Py_ssize_t,
    hash: ffi::Py_hash_t,
    state: u32,
    wstr: *mut u8,
    // python string stores data right after all the fields
}

impl PyASCIIObject {
    pub unsafe fn from_owned<'a>(obj: Py<pyo3::types::PyString>) -> &'a mut Self {
        let ascii: &mut PyASCIIObject = std::mem::transmute(obj);
        ascii
    }
}

#[repr(C)]
pub struct PyCompactUnicodeObject {
    base: PyASCIIObject,
    utf8_length: ffi::Py_ssize_t,
    utf8: *mut u8,
    wstr_length: ffi::Py_ssize_t,
    // python string stores data right after all the fields
}

impl PyCompactUnicodeObject {
    pub unsafe fn from_owned<'a>(obj: Py<pyo3::types::PyString>) -> &'a mut Self {
        let utf8: &mut PyCompactUnicodeObject = std::mem::transmute(obj);
        utf8
    }
}
