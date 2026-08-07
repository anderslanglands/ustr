use ustr::Ustr;

#[no_mangle]
pub extern "C" fn ustr(chars: *const std::os::raw::c_char) -> Ustr {
    let cs = unsafe { std::ffi::CStr::from_ptr(chars) };
    let utf8 = cs
        .to_str()
        .unwrap_or_else(|e| {
            eprintln!("ustr() received non-UTF8 input: {e}");
            std::process::abort();
        });
    Ustr::from(utf8)
}

#[no_mangle]
pub extern "C" fn ustr_len(u: Ustr) -> usize {
    u.len()
}

#[no_mangle]
pub extern "C" fn ustr_hash(u: Ustr) -> u64 {
    u.precomputed_hash()
}
