use std::ffi::{c_char, c_int, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::ptr;
use stow_core::{Error, Reader, Writer};

pub const STOW_OK: c_int = 0;
pub const STOW_ERR_INVALID_ARGUMENT: c_int = 1;
pub const STOW_ERR_IO: c_int = 2;
pub const STOW_ERR_INTERNAL: c_int = 3;

pub struct StowWriter {
    inner: Writer,
}

pub struct StowReader {
    inner: Reader,
    names: Vec<CString>,
}

fn guard<F: FnOnce() -> c_int>(f: F) -> c_int {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(STOW_ERR_INTERNAL)
}

fn code(e: Error) -> c_int {
    e.code()
}

unsafe fn c_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

unsafe fn give_buffer(data: Vec<u8>, out: *mut *mut u8, out_len: *mut usize) {
    let boxed = data.into_boxed_slice();
    *out_len = boxed.len();
    *out = Box::into_raw(boxed) as *mut u8;
}

#[no_mangle]
pub extern "C" fn stow_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn stow_error_message(err: c_int) -> *const c_char {
    let msg: &'static [u8] = match err {
        STOW_OK => b"ok\0",
        STOW_ERR_INVALID_ARGUMENT => b"invalid argument\0",
        STOW_ERR_IO => b"i/o error\0",
        STOW_ERR_INTERNAL => b"internal error\0",
        other => match Error::from_code(other) {
            Some(Error::InvalidMagic) => b"not a STOW archive\0",
            Some(Error::UnsupportedVersion) => b"unsupported STOW version\0",
            Some(Error::Truncated) => b"archive is truncated\0",
            Some(Error::Corrupt) => b"archive is corrupt\0",
            Some(Error::ChecksumMismatch) => b"checksum mismatch\0",
            Some(Error::UnknownCodec) => b"unknown codec\0",
            Some(Error::InvalidName) => b"invalid entry name\0",
            Some(Error::DuplicateName) => b"duplicate entry name\0",
            Some(Error::TooLarge) => b"data too large\0",
            Some(Error::IndexOutOfRange) => b"entry index out of range\0",
            None => b"unknown error\0",
        },
    };
    msg.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn stow_buffer_free(data: *mut u8, len: usize) {
    if data.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(Box::from_raw(std::slice::from_raw_parts_mut(data, len)));
    }));
}

#[no_mangle]
pub extern "C" fn stow_writer_new() -> *mut StowWriter {
    catch_unwind(|| Box::into_raw(Box::new(StowWriter { inner: Writer::new() })))
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn stow_writer_free(writer: *mut StowWriter) {
    if writer.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(writer))));
}

#[no_mangle]
pub unsafe extern "C" fn stow_writer_add_mem(
    writer: *mut StowWriter,
    name: *const c_char,
    data: *const u8,
    len: usize,
) -> c_int {
    guard(|| {
        let (Some(w), Some(name)) = (writer.as_mut(), c_str(name)) else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        if data.is_null() && len != 0 {
            return STOW_ERR_INVALID_ARGUMENT;
        }
        let slice: &[u8] = if len == 0 { &[] } else { std::slice::from_raw_parts(data, len) };
        match w.inner.add(name, slice) {
            Ok(()) => STOW_OK,
            Err(e) => code(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn stow_writer_add_file(
    writer: *mut StowWriter,
    path: *const c_char,
    name: *const c_char,
) -> c_int {
    guard(|| {
        let (Some(w), Some(path)) = (writer.as_mut(), c_str(path)) else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        let entry_name = if name.is_null() {
            match Path::new(path).file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_owned(),
                None => return STOW_ERR_INVALID_ARGUMENT,
            }
        } else {
            match c_str(name) {
                Some(n) => n.to_owned(),
                None => return STOW_ERR_INVALID_ARGUMENT,
            }
        };
        let Ok(data) = std::fs::read(path) else {
            return STOW_ERR_IO;
        };
        match w.inner.add(&entry_name, &data) {
            Ok(()) => STOW_OK,
            Err(e) => code(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn stow_writer_finish(
    writer: *mut StowWriter,
    out: *mut *mut u8,
    out_len: *mut usize,
) -> c_int {
    guard(|| {
        let Some(w) = writer.as_ref() else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        if out.is_null() || out_len.is_null() {
            return STOW_ERR_INVALID_ARGUMENT;
        }
        give_buffer(w.inner.finish(), out, out_len);
        STOW_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn stow_writer_save(writer: *mut StowWriter, out_path: *const c_char) -> c_int {
    guard(|| {
        let (Some(w), Some(path)) = (writer.as_ref(), c_str(out_path)) else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        match std::fs::write(path, w.inner.finish()) {
            Ok(()) => STOW_OK,
            Err(_) => STOW_ERR_IO,
        }
    })
}

fn make_reader(bytes: Vec<u8>, out: *mut *mut StowReader) -> c_int {
    match Reader::new(bytes) {
        Ok(inner) => {
            let names = inner
                .entries()
                .iter()
                .map(|e| CString::new(e.name.as_str()).unwrap_or_default())
                .collect();
            unsafe { *out = Box::into_raw(Box::new(StowReader { inner, names })) };
            STOW_OK
        }
        Err(e) => code(e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_open_mem(
    data: *const u8,
    len: usize,
    out: *mut *mut StowReader,
) -> c_int {
    guard(|| {
        if out.is_null() || (data.is_null() && len != 0) {
            return STOW_ERR_INVALID_ARGUMENT;
        }
        *out = ptr::null_mut();
        let bytes = if len == 0 { Vec::new() } else { std::slice::from_raw_parts(data, len).to_vec() };
        make_reader(bytes, out)
    })
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_open_file(path: *const c_char, out: *mut *mut StowReader) -> c_int {
    guard(|| {
        if out.is_null() {
            return STOW_ERR_INVALID_ARGUMENT;
        }
        *out = ptr::null_mut();
        let Some(path) = c_str(path) else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        match std::fs::read(path) {
            Ok(bytes) => make_reader(bytes, out),
            Err(_) => STOW_ERR_IO,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_free(reader: *mut StowReader) {
    if reader.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(reader))));
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_count(reader: *const StowReader) -> usize {
    reader.as_ref().map(|r| r.inner.len()).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_name(reader: *const StowReader, index: usize) -> *const c_char {
    match reader.as_ref().and_then(|r| r.names.get(index)) {
        Some(n) => n.as_ptr(),
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_size(reader: *const StowReader, index: usize) -> u64 {
    reader
        .as_ref()
        .and_then(|r| r.inner.entries().get(index))
        .map(|e| e.raw_len)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn stow_reader_read(
    reader: *const StowReader,
    index: usize,
    out: *mut *mut u8,
    out_len: *mut usize,
) -> c_int {
    guard(|| {
        let Some(r) = reader.as_ref() else {
            return STOW_ERR_INVALID_ARGUMENT;
        };
        if out.is_null() || out_len.is_null() {
            return STOW_ERR_INVALID_ARGUMENT;
        }
        match r.inner.read(index) {
            Ok(data) => {
                give_buffer(data, out, out_len);
                STOW_OK
            }
            Err(e) => code(e),
        }
    })
}
