//! Provides utilities for working with named pipes
use libc::{c_int, mkfifo, mode_t};
use rand::{self, Rng};
use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::str;

// Taken from tempfile crate
fn tmpname(prefix: &OsStr, suffix: &OsStr, rand_len: usize) -> OsString {
    let mut buf = OsString::with_capacity(prefix.len() + suffix.len() + rand_len);
    buf.push(prefix);

    // Push each character in one-by-one. Unfortunately, this is the only
    // safe(ish) simple way to do this without allocating a temporary
    // String/Vec.
    unsafe {
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(rand_len)
            .for_each(|b| buf.push(str::from_utf8_unchecked(&[b as u8])))
    }
    buf.push(suffix);
    buf
}

// Returns the name of the pipe
pub fn create() -> io::Result<PathBuf> {
    let path = tmpname(OsStr::new("nu-"), OsStr::new(".tmp"), 16usize);
    let path = CString::new(path.into_string().unwrap())?;

    let result: c_int = unsafe { mkfifo(path.as_ptr(), 0o644 as mode_t) };
    if result == 0i32 {
        Ok(path.into_string().unwrap().into())
    } else {
        let error = errno::errno();
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("could not create named pipe {:?}: {}", path, error),
        ))
    }
}

pub fn open_read<P: AsRef<Path>>(path: P) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        //.custom_flags(libc::O_NONBLOCK)
        .open(path)
}

pub fn open_write<P: AsRef<Path>>(path: P) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .append(true)
        //.custom_flags(libc::O_NONBLOCK)
        .open(path)
}
