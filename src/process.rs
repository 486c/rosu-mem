use std::path::PathBuf;

use super::{error::ProcessError, signature::Signature};
use paste::paste;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize,
}

macro_rules! prim_read_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t>]<T: TryInto<usize>>(
                &self,
                addr: T
            ) -> Result<$t, ProcessError> {
                let mut bytes = [0u8; std::mem::size_of::<$t>()];
                self.read(addr, std::mem::size_of::<$t>(), &mut bytes)?;

                Ok($t::from_le_bytes(bytes))
            }
        }
    };
}

macro_rules! prim_read_array_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t _array>]<T: TryInto<usize>>(
                &self,
                addr: T,
                buff: &mut Vec<$t>
            ) -> Result<(), ProcessError> {
                let addr: usize = addr.try_into()
                    .map_err(|_| ProcessError::AddressConvertError)?;

                let items_ptr = self.read_i32(addr + 4)?;
                let size = self.read_i32(addr + 12)? as usize;

                buff.resize(size, 0 as $t);

                let byte_buff = unsafe { std::slice::from_raw_parts_mut(
                    buff.as_mut_ptr() as *mut u8,
                    buff.len() * std::mem::size_of::<$t>()
                ) };


                self.read(
                    items_ptr + 8,
                    size * std::mem::size_of::<$t>(),
                    byte_buff
                )?;

                Ok(())
            }
        }
    };
}

pub struct Process {
    #[cfg(target_os = "linux")]
    pub pid: i32,

    #[cfg(target_os = "windows")]
    pub pid: u32,

    #[cfg(target_os = "windows")]
    pub handle: HANDLE,

    pub maps: Vec<MemoryRegion>,

    pub executable_dir: Option<PathBuf>,
}

pub trait ProcessTraits
where
    Self: Sized,
{
    /// Initialize a `Process` struct
    ///
    /// * `proc_name` - Name of the process or key words
    /// * `exclude` - Key words to avoid when searching for process name
    ///
    /// Notes
    /// For more details of searching the process name see [`find_process`]
    /// method
    fn initialize(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Self, ProcessError>;

    /// Attemp to find a process
    ///
    /// * `proc_name` - Name of the process or key words
    /// * `exclude` - Key words to avoid when searching for process name
    ///
    /// # Notes
    /// It's going try to search process name by using `contains` function
    /// with `proc_name` argument on process name. Same applies to `exclude`
    fn find_process(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Self, ProcessError>;

    fn read_regions(self) -> Result<Self, ProcessError>;

    fn read_signature<T: TryFrom<usize>>(
        &self,
        sign: &Signature,
    ) -> Result<T, ProcessError>;

    fn read<T: TryInto<usize>>(
        &self,
        addr: T,
        len: usize,
        buff: &mut [u8],
    ) -> Result<(), ProcessError>;

    fn read_uleb128<T: TryInto<usize>>(
        &self,
        addr: T,
    ) -> Result<u64, ProcessError> {
        let mut addr: usize = addr
            .try_into()
            .map_err(|_| ProcessError::AddressConvertError)?;

        let mut value: u64 = 0;
        let mut bytes_read = 0;

        loop {
            let byte = self.read_u8(addr)?;
            addr += 1;

            let byte_value = (byte & 0b0111_1111) as u64;
            value |= byte_value << (7 * bytes_read);

            bytes_read += 1;

            if (byte & !0b0111_1111) == 0 {
                break;
            }
        }

        Ok(value)
    }

    /// Reads a C# string based on C# string structure
    /// Assumes passed `addr` is a pointer, so it's gonna make
    /// additional pointer read.
    fn read_string_from_ptr<T: TryInto<usize>>(
        &self,
        addr: T,
    ) -> Result<String, ProcessError> {
        let addr = self.read_i32(addr)?;

        self.read_string(addr)
    }

    /// Reads a C# string based on C# string structure
    /// Assumes passed `addr` is not a pointer, so no additional
    /// pointer reads is gonna be made.
    ///
    /// If you have a pointer to string either read that pointer youself
    /// or use [`read_string_from_ptr()`]
    fn read_string<T: TryInto<usize>>(
        &self,
        addr: T,
    ) -> Result<String, ProcessError> {
        let mut addr: usize = addr
            .try_into()
            .map_err(|_| ProcessError::AddressConvertError)?;

        // C# string structure: 4B obj header, 4B str len, str itself
        addr += 0x4; // Skipping 4B obj header
        let len = self.read_u32(addr)? as usize; // Reading 4B str len
        addr += 0x4; // Since we read length skipping it too

        let mut buff = vec![0u16; len];

        let byte_buff = unsafe {
            std::slice::from_raw_parts_mut(
                buff.as_mut_ptr() as *mut u8,
                buff.len() * 2,
            )
        };

        self.read(addr, byte_buff.len(), byte_buff)?;

        Ok(String::from_utf16_lossy(&buff))
    }

    prim_read_impl!(i8);
    prim_read_impl!(i16);
    prim_read_impl!(i32);
    prim_read_impl!(i64);
    prim_read_impl!(i128);

    prim_read_impl!(u8);
    prim_read_impl!(u16);
    prim_read_impl!(u32);
    prim_read_impl!(u64);
    prim_read_impl!(u128);

    prim_read_impl!(f32);
    prim_read_impl!(f64);

    prim_read_array_impl!(i8);
    prim_read_array_impl!(i16);
    prim_read_array_impl!(i32);
    prim_read_array_impl!(i64);
    prim_read_array_impl!(i128);

    prim_read_array_impl!(u8);
    prim_read_array_impl!(u16);
    prim_read_array_impl!(u32);
    prim_read_array_impl!(u64);
    prim_read_array_impl!(u128);

    prim_read_array_impl!(f32);
    prim_read_array_impl!(f64);
}
