use error::ErrorStack;
use ffi;
use libc::{c_int, c_char, c_void, c_long};
use std::fmt;
use std::ptr;
use std::cmp;

use bio::{MemBio, MemBioSlice};
use bn::BigNumRef;
use {cvt, cvt_p};
use types::OpenSslTypeRef;
use util::{CallbackState, invoke_passwd_cb_old};

type_!(Dsa, DsaRef, ffi::DSA, ffi::DSA_free);

impl DsaRef {
    private_key_to_pem!(ffi::PEM_write_bio_DSAPrivateKey);

    /// Encodes a DSA public key as PEM formatted data.
    pub fn public_key_to_pem(&self) -> Result<Vec<u8>, ErrorStack> {
        let mem_bio = try!(MemBio::new());
        unsafe {
            try!(cvt(ffi::PEM_write_bio_DSA_PUBKEY(mem_bio.as_ptr(), self.as_ptr())));
        }
        Ok(mem_bio.get_buf().to_owned())
    }

    /// Encodes a DSA private key as unencrypted DER formatted data.
    pub fn private_key_to_der(&self) -> Result<Vec<u8>, ErrorStack> {
        unsafe {
            let len = try!(cvt(ffi::i2d_DSAPrivateKey(self.as_ptr(), ptr::null_mut())));
            let mut buf = vec![0; len as usize];
            try!(cvt(ffi::i2d_DSAPrivateKey(self.as_ptr(), &mut buf.as_mut_ptr())));
            Ok(buf)
        }
    }

    /// Encodes a DSA public key as DER formatted data.
    pub fn public_key_to_der(&self) -> Result<Vec<u8>, ErrorStack> {
        unsafe {
            let len = try!(cvt(ffi::i2d_DSAPublicKey(self.as_ptr(), ptr::null_mut())));
            let mut buf = vec![0; len as usize];
            try!(cvt(ffi::i2d_DSAPublicKey(self.as_ptr(), &mut buf.as_mut_ptr())));
            Ok(buf)
        }
    }

    // FIXME should return u32
    pub fn size(&self) -> Option<u32> {
        if self.q().is_some() {
            unsafe { Some(ffi::DSA_size(self.as_ptr()) as u32) }
        } else {
            None
        }
    }

    pub fn p(&self) -> Option<&BigNumRef> {
        unsafe {
            let p = compat::pqg(self.as_ptr())[0];
            if p.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(p as *mut _))
            }
        }
    }

    pub fn q(&self) -> Option<&BigNumRef> {
        unsafe {
            let q = compat::pqg(self.as_ptr())[1];
            if q.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(q as *mut _))
            }
        }
    }

    pub fn g(&self) -> Option<&BigNumRef> {
        unsafe {
            let g = compat::pqg(self.as_ptr())[2];
            if g.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(g as *mut _))
            }
        }
    }

    pub fn has_public_key(&self) -> bool {
        unsafe { !compat::keys(self.as_ptr())[0].is_null() }
    }

    pub fn has_private_key(&self) -> bool {
        unsafe { !compat::keys(self.as_ptr())[1].is_null() }
    }
}

impl Dsa {
    /// Generate a DSA key pair.
    pub fn generate(bits: u32) -> Result<Dsa, ErrorStack> {
        unsafe {
            let dsa = Dsa(try!(cvt_p(ffi::DSA_new())));
            try!(cvt(ffi::DSA_generate_parameters_ex(dsa.0,
                                                     bits as c_int,
                                                     ptr::null(),
                                                     0,
                                                     ptr::null_mut(),
                                                     ptr::null_mut(),
                                                     ptr::null_mut())));
            try!(cvt(ffi::DSA_generate_key(dsa.0)));
            Ok(dsa)
        }
    }

    private_key_from_pem!(Dsa, ffi::PEM_read_bio_DSAPrivateKey);

    #[deprecated(since = "0.9.2", note = "use private_key_from_pem_callback")]
    pub fn private_key_from_pem_cb<F>(buf: &[u8], pass_cb: F) -> Result<Dsa, ErrorStack>
        where F: FnOnce(&mut [c_char]) -> usize
    {
        ffi::init();
        let mut cb = CallbackState::new(pass_cb);
        let mem_bio = try!(MemBioSlice::new(buf));

        unsafe {
            let cb_ptr = &mut cb as *mut _ as *mut c_void;
            let dsa = try!(cvt_p(ffi::PEM_read_bio_DSAPrivateKey(mem_bio.as_ptr(),
                                                                 ptr::null_mut(),
                                                                 Some(invoke_passwd_cb_old::<F>),
                                                                 cb_ptr)));
            Ok(Dsa(dsa))
        }
    }

    /// Reads a DSA public key from PEM formatted data.
    pub fn public_key_from_pem(buf: &[u8]) -> Result<Dsa, ErrorStack> {
        ffi::init();

        let mem_bio = try!(MemBioSlice::new(buf));
        unsafe {
            let dsa = try!(cvt_p(ffi::PEM_read_bio_DSA_PUBKEY(mem_bio.as_ptr(),
                                                              ptr::null_mut(),
                                                              None,
                                                              ptr::null_mut())));
            Ok(Dsa(dsa))
        }
    }

    /// Reads a DSA private key from DER formatted data.
    pub fn private_key_from_der(buf: &[u8]) -> Result<Dsa, ErrorStack> {
        unsafe {
            ffi::init();
            let len = cmp::min(buf.len(), c_long::max_value() as usize) as c_long;
            let dsa = try!(cvt_p(ffi::d2i_DSAPrivateKey(ptr::null_mut(), &mut buf.as_ptr(), len)));
            Ok(Dsa(dsa))
        }
    }

    /// Reads a DSA public key from DER formatted data.
    pub fn public_key_from_der(buf: &[u8]) -> Result<Dsa, ErrorStack> {
        unsafe {
            ffi::init();
            let len = cmp::min(buf.len(), c_long::max_value() as usize) as c_long;
            let dsa = try!(cvt_p(ffi::d2i_DSAPublicKey(ptr::null_mut(), &mut buf.as_ptr(), len)));
            Ok(Dsa(dsa))
        }
    }
}

impl fmt::Debug for Dsa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DSA")
    }
}

#[cfg(ossl110)]
mod compat {
    use std::ptr;
    use ffi::{self, BIGNUM, DSA};

    pub unsafe fn pqg(d: *const DSA) -> [*const BIGNUM; 3] {
        let (mut p, mut q, mut g) = (ptr::null(), ptr::null(), ptr::null());
        ffi::DSA_get0_pqg(d, &mut p, &mut q, &mut g);
        [p, q, g]
    }

    pub unsafe fn keys(d: *const DSA) -> [*const BIGNUM; 2] {
        let (mut pub_key, mut priv_key) = (ptr::null(), ptr::null());
        ffi::DSA_get0_key(d, &mut pub_key, &mut priv_key);
        [pub_key, priv_key]
    }
}

#[cfg(ossl10x)]
mod compat {
    use ffi::{BIGNUM, DSA};

    pub unsafe fn pqg(d: *const DSA) -> [*const BIGNUM; 3] {
        [(*d).p, (*d).q, (*d).g]
    }

    pub unsafe fn keys(d: *const DSA) -> [*const BIGNUM; 2] {
        [(*d).pub_key, (*d).priv_key]
    }
}

#[cfg(test)]
mod test {
    use symm::Cipher;

    use super::*;

    #[test]
    pub fn test_generate() {
        Dsa::generate(1024).unwrap();
    }

    #[test]
    pub fn test_password() {
        let key = include_bytes!("../test/dsa-encrypted.pem");
        Dsa::private_key_from_pem_passphrase(key, b"mypass").unwrap();
    }

    #[test]
    fn test_to_password() {
        let key = Dsa::generate(2048).unwrap();
        let pem = key.private_key_to_pem_passphrase(Cipher::aes_128_cbc(), b"foobar").unwrap();
        Dsa::private_key_from_pem_passphrase(&pem, b"foobar").unwrap();
        assert!(Dsa::private_key_from_pem_passphrase(&pem, b"fizzbuzz").is_err());
    }

    #[test]
    pub fn test_password_callback() {
        let mut password_queried = false;
        let key = include_bytes!("../test/dsa-encrypted.pem");
        Dsa::private_key_from_pem_callback(key, |password| {
                password_queried = true;
                password[..6].copy_from_slice(b"mypass");
                6
            })
            .unwrap();

        assert!(password_queried);
    }
}
