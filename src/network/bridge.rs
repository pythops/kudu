use std::{
    ffi::{CString, c_char},
    io::Error,
};

use anyhow::Result;

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/sockios.h

const SIOCBRADDBR: libc::c_ulong = 0x89a0;

pub struct Bridge;

impl Bridge {
    pub fn add(name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            let fd = libc::socket(libc::AF_LOCAL, libc::SOCK_STREAM, 0);
            if fd < 0 {
                return Err(Error::last_os_error().into());
            }

            let ret = libc::ioctl(fd, SIOCBRADDBR, c_name.as_ptr());

            if ret < 0 {
                let err = Error::last_os_error();
                libc::close(fd);

                if err.raw_os_error() == Some(libc::EEXIST) {
                    return Ok(());
                };
                return Err(err.into());
            };

            libc::close(fd);
            Ok(())
        }
    }

    pub fn up(name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        let name_bytes = c_name.as_bytes_with_nul();

        unsafe {
            let fd = libc::socket(libc::AF_LOCAL, libc::SOCK_STREAM, 0);
            if fd < 0 {
                return Err(Error::last_os_error().into());
            }

            let mut ifr: libc::ifreq = std::mem::zeroed();
            for (i, &b) in name_bytes.iter().enumerate() {
                ifr.ifr_name[i] = b as c_char;
            }

            if libc::ioctl(fd, libc::SIOCGIFFLAGS, &mut ifr) < 0 {
                let err = Error::last_os_error();
                libc::close(fd);
                return Err(err.into());
            }

            ifr.ifr_ifru.ifru_flags |= libc::IFF_UP as i16;

            if libc::ioctl(fd, libc::SIOCSIFFLAGS, &ifr) < 0 {
                let err = Error::last_os_error();
                libc::close(fd);
                return Err(err.into());
            }

            libc::close(fd);
            Ok(())
        }
    }
}
