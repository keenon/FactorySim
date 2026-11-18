//! Minimal, zero-dependency FFI wrapper around macOS/Linux system `libsqlite3`
//! for `data/users.sqlite` (accounts, single-use invites, password reset links, sessions, and game index).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

#[repr(C)]
pub struct sqlite3 {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sqlite3_stmt {
    _private: [u8; 0],
}

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_TRANSIENT: isize = -1;

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open(filename: *const c_char, ppDb: *mut *mut sqlite3) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        callback: Option<extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int>,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        zSql: *const c_char,
        nByte: c_int,
        ppStmt: *mut *mut sqlite3_stmt,
        pzTail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut sqlite3_stmt,
        idx: c_int,
        val: *const c_char,
        nByte: c_int,
        destructor: isize,
    ) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_count(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, iCol: c_int) -> *const u8;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
}

pub struct SqliteConn {
    db: *mut sqlite3,
}

unsafe impl Send for SqliteConn {}

impl SqliteConn {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let path_str = path.to_string_lossy();
        let c_path = CString::new(path_str.as_ref()).map_err(|e| e.to_string())?;
        let mut db: *mut sqlite3 = ptr::null_mut();
        let rc = unsafe { sqlite3_open(c_path.as_ptr(), &mut db) };
        if rc != SQLITE_OK || db.is_null() {
            return Err(format!("sqlite3_open failed with code {rc}"));
        }
        let conn = Self { db };
        conn.exec("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(conn)
    }

    pub fn exec(&self, sql: &str) -> Result<(), String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let rc = unsafe {
            sqlite3_exec(
                self.db,
                c_sql.as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if rc != SQLITE_OK {
            return Err(self.last_error());
        }
        Ok(())
    }

    pub fn execute_params(&self, sql: &str, params: &[&str]) -> Result<(), String> {
        let _ = self.query_rows(sql, params)?;
        Ok(())
    }

    pub fn query_rows(&self, sql: &str, params: &[&str]) -> Result<Vec<Vec<String>>, String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut stmt: *mut sqlite3_stmt = ptr::null_mut();
        let rc = unsafe {
            sqlite3_prepare_v2(
                self.db,
                c_sql.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            )
        };
        if rc != SQLITE_OK || stmt.is_null() {
            return Err(self.last_error());
        }

        for (i, p) in params.iter().enumerate() {
            let c_val = CString::new(*p).map_err(|e| e.to_string())?;
            let brc = unsafe {
                sqlite3_bind_text(
                    stmt,
                    (i + 1) as c_int,
                    c_val.as_ptr(),
                    -1,
                    SQLITE_TRANSIENT,
                )
            };
            if brc != SQLITE_OK {
                unsafe { sqlite3_finalize(stmt) };
                return Err(self.last_error());
            }
        }

        let mut rows = Vec::new();
        loop {
            let src = unsafe { sqlite3_step(stmt) };
            if src == SQLITE_ROW {
                let cols = unsafe { sqlite3_column_count(stmt) };
                let mut row = Vec::with_capacity(cols as usize);
                for c in 0..cols {
                    let ptr = unsafe { sqlite3_column_text(stmt, c) };
                    if ptr.is_null() {
                        row.push(String::new());
                    } else {
                        let s = unsafe { CStr::from_ptr(ptr as *const c_char) }
                            .to_string_lossy()
                            .into_owned();
                        row.push(s);
                    }
                }
                rows.push(row);
            } else if src == SQLITE_DONE {
                break;
            } else {
                let err = self.last_error();
                unsafe { sqlite3_finalize(stmt) };
                return Err(err);
            }
        }

        unsafe { sqlite3_finalize(stmt) };
        Ok(rows)
    }

    fn last_error(&self) -> String {
        unsafe {
            let ptr = sqlite3_errmsg(self.db);
            if ptr.is_null() {
                "unknown sqlite error".to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for SqliteConn {
    fn drop(&mut self) {
        if !self.db.is_null() {
            unsafe {
                sqlite3_close(self.db);
            }
        }
    }
}
