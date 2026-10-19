//! Desktop implementation of Android's in-memory TinyCC pipeline.
//! C strings go straight to libtcc; only a relocatable object is cached.
//! The replaceable LGPL library and its executable allocations stay alive
//! until the emulation thread has stopped using every block pointer.
use crate::{
    analyze::{Analysis, View},
    emit,
};
use gba_core::capi::RcgBlock;
use libloading::Library;
use std::{
    ffi::{c_char, c_int, c_void, CString},
    path::{Path, PathBuf},
};

type State = *mut c_void;
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub const REVISION: &str = "43c7708b85681a2fd4451c8a541af4494a8919b2";

pub fn library_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FFTA_TCC_LIBRARY") {
        return Some(path.into());
    }
    let name = if cfg!(windows) {
        "libtcc.dll"
    } else if cfg!(target_os = "macos") {
        "libtcc.dylib"
    } else {
        "libtcc.so"
    };
    let exe = std::env::current_exe().ok()?;
    let beside = exe.parent()?.join("tcc").join(name);
    if beside.is_file() {
        return Some(beside);
    }
    // macOS app bundle: Contents/MacOS/recomp + Contents/Resources/tcc.
    let bundled = crate::tcc::macos_bundle_resource_tcc(&exe)?.join(name);
    bundled.is_file().then_some(bundled)
}

pub struct NativeCode {
    state: State,
    delete: unsafe extern "C" fn(State),
    library: Library,
    errors: Box<String>,
}
// Context access is serialized during compilation; execution has one owner.
unsafe impl Send for NativeCode {}
impl Drop for NativeCode {
    fn drop(&mut self) {
        if self.state.is_null() {
            return;
        }
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { (self.delete)(self.state) }
    }
}
unsafe extern "C" fn error_callback(opaque: *mut c_void, message: *const c_char) {
    if !opaque.is_null() && !message.is_null() {
        let errors = &mut *(opaque as *mut String);
        if errors.len() < 8192 {
            errors.push_str(&std::ffi::CStr::from_ptr(message).to_string_lossy());
            errors.push('\n');
        }
    }
}
fn path_string(path: &Path) -> Result<CString, String> {
    CString::new(path.to_str().ok_or("TinyCC path is not UTF-8")?).map_err(|e| e.to_string())
}
impl NativeCode {
    unsafe fn new(path: &Path, output_type: c_int) -> Result<Self, String> {
        let library =
            Library::new(path).map_err(|e| format!("Load TinyCC {}: {e}", path.display()))?;
        // ABI marker deliberately checks compatibility, not a file hash: users
        // can replace the DLL with a modified library implementing this ABI.
        let abi = *library
            .get::<unsafe extern "C" fn() -> c_int>(b"ffta_tcc_abi\0")
            .map_err(|e| format!("TinyCC needs the desktop ABI adapter: {e}"))?;
        if abi() != 1 {
            return Err("Incompatible TinyCC adapter ABI".into());
        }
        let new = *library
            .get::<unsafe extern "C" fn() -> State>(b"tcc_new\0")
            .map_err(|e| e.to_string())?;
        let delete = *library.get(b"tcc_delete\0").map_err(|e| e.to_string())?;
        let mut code = Self {
            state: std::ptr::null_mut(),
            delete,
            library,
            errors: Box::new(String::new()),
        };
        let _lock = LOCK.lock().map_err(|_| "TinyCC compiler lock poisoned")?;
        let state = new();
        if state.is_null() {
            return Err("TinyCC allocation failed".into());
        }
        code.state = state;
        let on_error = code
            .library
            .get::<unsafe extern "C" fn(
                State,
                *mut c_void,
                unsafe extern "C" fn(*mut c_void, *const c_char),
            )>(b"tcc_set_error_func\0")
            .map_err(|e| e.to_string())?;
        on_error(state, &mut *code.errors as *mut _ as *mut _, error_callback);
        let options = code
            .library
            .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(b"tcc_set_options\0")
            .map_err(|e| e.to_string())?;
        if options(state, b"-std=c11 -nostdinc -nostdlib\0".as_ptr().cast()) < 0 {
            return Err(format!("TinyCC options: {}", code.errors));
        }
        let output = code
            .library
            .get::<unsafe extern "C" fn(State, c_int) -> c_int>(b"tcc_set_output_type\0")
            .map_err(|e| e.to_string())?;
        if output(state, output_type) < 0 {
            return Err(format!("TinyCC output: {}", code.errors));
        }
        #[cfg(windows)]
        if output_type == 1 {
            // TinyCC's own helper has a special frame-building ABI. The OS
            // or MSVC __chkstk symbol is NOT an interchangeable substitute.
            let probe = *code
                .library
                .get::<unsafe extern "C" fn()>(b"__chkstk\0")
                .map_err(|e| e.to_string())?;
            let add = code
                .library
                .get::<unsafe extern "C" fn(State, *const c_char, *const c_void) -> c_int>(
                    b"tcc_add_symbol\0",
                )
                .map_err(|e| e.to_string())?;
            if add(state, b"__chkstk\0".as_ptr().cast(), probe as *const c_void) < 0 {
                return Err("TinyCC stack helper registration failed".into());
            }
        }
        Ok(code)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let library = library_path().ok_or("Bundled TinyCC is missing (tcc/libtcc)")?;
        let start = std::time::Instant::now();
        let code = unsafe { Self::new(&library, 1)? };
        let lock = LOCK.lock().map_err(|_| "TinyCC compiler lock poisoned")?;
        let result = (|| unsafe {
            let add = code
                .library
                .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(b"tcc_add_file\0")
                .map_err(|e| e.to_string())?;
            if add(code.state, path_string(path)?.as_ptr()) < 0 {
                return Err(format!("TinyCC cached object: {}", code.errors));
            }
            let relocate = code
                .library
                .get::<unsafe extern "C" fn(State) -> c_int>(b"tcc_relocate\0")
                .map_err(|e| e.to_string())?;
            if relocate(code.state) < 0 {
                return Err(format!("TinyCC relocation: {}", code.errors));
            }
            code.blocks()?;
            Ok(())
        })();
        drop(lock);
        result?;
        eprintln!("TCC_LOAD seconds={:.3}", start.elapsed().as_secs_f64());
        Ok(code)
    }
    pub fn blocks(&self) -> Result<&[RcgBlock], String> {
        unsafe {
            let symbol = self
                .library
                .get::<unsafe extern "C" fn(State, *const c_char) -> *const c_void>(
                    b"tcc_get_symbol\0",
                )
                .map_err(|e| e.to_string())?;
            let count = symbol(self.state, b"rcg_block_count\0".as_ptr().cast()) as *const u64;
            let blocks = symbol(self.state, b"rcg_blocks\0".as_ptr().cast()) as *const RcgBlock;
            if count.is_null() || blocks.is_null() || *count > 1_000_000 {
                return Err("TinyCC native block table missing or invalid".into());
            }
            Ok(std::slice::from_raw_parts(blocks, *count as usize))
        }
    }
}

pub fn build(
    analysis: &Analysis,
    view: &View,
    library: &Path,
    out: &Path,
    progress: &mut dyn FnMut(usize),
) -> Result<usize, String> {
    let start = std::time::Instant::now();
    // Context is created before the lock guard so every error releases the
    // guard before running the context destructor.
    let code = unsafe { NativeCode::new(library, 3)? };
    let lock = LOCK.lock().map_err(|_| "TinyCC compiler lock poisoned")?;
    let compile = unsafe {
        code.library
            .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(b"tcc_compile_string\0")
    }
    .map_err(|e| e.to_string())?;
    let units = emit::emit_c_chunked(analysis, view, 1 << 20, |source, done| {
        let source = CString::new(prepare_source(source)).map_err(|e| e.to_string())?;
        if unsafe { compile(code.state, source.as_ptr()) } < 0 {
            return Err(format!("TinyCC C compilation: {}", code.errors));
        }
        progress(done);
        Ok(())
    })?;
    let temporary = out.with_extension(format!("tcc-{}.tmp", std::process::id()));
    let write = unsafe {
        code.library
            .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(b"tcc_output_file\0")
    }
    .map_err(|e| e.to_string())?;
    if unsafe { write(code.state, path_string(&temporary)?.as_ptr()) } < 0 {
        return Err(format!("TinyCC write object: {}", code.errors));
    }
    std::fs::rename(&temporary, out).map_err(|e| e.to_string())?;
    drop(lock);
    eprintln!(
        "TCC_BUILD blocks={} units={units} seconds={:.3} c_files=0",
        analysis.blocks.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(units)
}

fn prepare_source(source: &str) -> String {
    source.replace("#include <stdint.h>",
        "typedef unsigned char uint8_t; typedef signed char int8_t; typedef unsigned short uint16_t; typedef signed short int16_t; typedef unsigned int uint32_t; typedef signed int int32_t; typedef unsigned long long uint64_t; typedef signed long long int64_t;")
        .replace("__declspec(dllexport)", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires FFTA_TCC_LIBRARY pointing at the desktop adapter"]
    fn compile_failure_releases_context_and_compiler_lock() {
        let library = library_path().expect("FFTA_TCC_LIBRARY required");
        {
            let code = unsafe { NativeCode::new(&library, 3).unwrap() };
            let _lock = LOCK.lock().unwrap();
            let compile = unsafe {
                code.library
                    .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(
                        b"tcc_compile_string\0",
                    )
                    .unwrap()
            };
            assert!(unsafe { compile(code.state, b"int broken( {\n\0".as_ptr().cast()) } < 0);
            assert!(!code.errors.is_empty());
        }
        // This would deadlock if the previous error retained the global lock.
        let code = unsafe { NativeCode::new(&library, 3).unwrap() };
        assert!(!code.state.is_null());
    }

    #[test]
    #[ignore = "requires FFTA_TCC_LIBRARY pointing at the desktop adapter"]
    fn cached_code_preserves_callback_abi_and_large_stack_frame() {
        let library = library_path().expect("FFTA_TCC_LIBRARY required");
        let path = std::env::temp_dir().join(format!("ffta-tcc-abi-{}.o", std::process::id()));
        {
            let code = unsafe { NativeCode::new(&library, 3).unwrap() };
            let _lock = LOCK.lock().unwrap();
            let source = CString::new(prepare_source(
                r#"
                #include <stdint.h>
                typedef uint32_t (*Read)(void*,uint32_t);
                typedef struct { Read read; } Api;
                typedef struct { uint32_t key; uint32_t (*fn)(const Api*,void*); } Block;
                uint32_t probe(const Api* a,void* m) {
                    volatile uint8_t scratch[16384];
                    scratch[0]=7; scratch[16383]=9;
                    return a->read(m,123)+scratch[0]+scratch[16383];
                }
                const Block rcg_blocks[]={{0x08000000,probe}};
                const uint64_t rcg_block_count=1;
            "#,
            ))
            .unwrap();
            unsafe {
                let compile = code
                    .library
                    .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(
                        b"tcc_compile_string\0",
                    )
                    .unwrap();
                assert_eq!(compile(code.state, source.as_ptr()), 0, "{}", code.errors);
                let write = code
                    .library
                    .get::<unsafe extern "C" fn(State, *const c_char) -> c_int>(
                        b"tcc_output_file\0",
                    )
                    .unwrap();
                assert_eq!(
                    write(code.state, path_string(&path).unwrap().as_ptr()),
                    0,
                    "{}",
                    code.errors
                );
            }
        }
        extern "C" fn read(_m: *mut c_void, address: u32) -> u32 {
            assert_eq!(address, 123);
            100
        }
        let code = NativeCode::load(&path).unwrap();
        let blocks = code.blocks().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].key, 0x08000000);
        #[repr(C)]
        struct One {
            read: extern "C" fn(*mut c_void, u32) -> u32,
        }
        let api = One { read };
        // The test C function accesses only its own one-pointer Api struct.
        let one = &api as *const One as *const gba_core::capi::RtApi;
        assert_eq!((blocks[0].func)(one, std::ptr::null_mut()), 116);
        drop(code);
        std::fs::remove_file(path).unwrap();
    }
}
