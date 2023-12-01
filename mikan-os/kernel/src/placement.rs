#![allow(unused)]

use core::mem::size_of_val;

/// 保持しておきたいオブジェクトと、それを保存するバッファを受け取り、オブジェクトへの参照を返す。
/// ただし、バッファがオブジェクトを保存するのに不足していた場合は、必要なバイト数をエラーとして返す。
pub(crate) fn new_with_buf<'a, T: Sized>(item: T, buf: &'a [u8]) -> Result<&'a T, usize> {
    if size_of_val(&item) > buf.len() {
        return Err(size_of_val(&item));
    }
    let buf = unsafe {
        *(buf.as_ptr() as *mut T) = item;
        &*(buf.as_ptr() as *const T)
    };
    Ok(buf)
}

/// 保持しておきたいオブジェクトと、それを保存するバッファを受け取り、オブジェクトへの可変参照を返す。
/// ただし、バッファがオブジェクトを保存するのに不足していた場合は、必要なバイト数をエラーとして返す。
pub(crate) fn new_mut_with_buf<'a, T>(item: T, buf: &'a [u8]) -> Result<&'a mut T, usize> {
    if size_of_val(&item) > buf.len() {
        return Err(size_of_val(&item));
    }
    let buf = unsafe {
        *(buf.as_ptr() as *mut T) = item;
        &mut *(buf.as_ptr() as *mut T)
    };
    Ok(buf)
}
