use std::{fs, io::IoSliceMut, path::PathBuf};

use nix::{
    errno::Errno,
    sys::uio::{process_vm_readv, RemoteIoVec},
    unistd::Pid,
};

use crate::{
    error::ProcessError,
    process::{MemoryRegion, Process, ProcessTraits},
};

use super::signature::{find_signature, Signature};

impl ProcessTraits for Process {
    fn initialize(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Process, super::error::ProcessError> {
        let process = Process::find_process(proc_name, exclude)?;
        process.read_regions()
    }

    fn find_process(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Process, ProcessError> {
        let paths = fs::read_dir("/proc")?;

        'path_loop: for path in paths {
            let p = path?.path();

            if !p.is_dir() {
                continue;
            }

            let cmd_line = p.join("cmdline");

            if !cmd_line.exists() {
                continue;
            }

            let mut cmd_buff = fs::read_to_string(cmd_line)?;

            let line = cmd_buff.split(' ').next().unwrap();

            if line.contains(proc_name) {
                for exclude_word in exclude {
                    if line.contains(exclude_word) {
                        continue 'path_loop;
                    }
                }

                let stat = p.join("stat");
                let buff = fs::read_to_string(stat)?;

                // Formatting path
                cmd_buff.retain(|c| c != '\0');
                cmd_buff = cmd_buff.replace('\\', "/");

                cmd_buff.remove(0);
                cmd_buff.remove(0);

                let executable_path = PathBuf::from(cmd_buff);
                let executable_dir =
                    executable_path.parent().map(|v| v.to_path_buf());

                let pid_str = buff.split(' ').next().unwrap();

                let pid = pid_str.parse()?;

                return Ok(Self {
                    pid,
                    maps: Vec::new(),
                    executable_dir,
                });
            }
        }

        Err(ProcessError::ProcessNotFound)
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        let path = format!("/proc/{}/maps", &self.pid);
        let mut v = Vec::new();

        let buff = fs::read_to_string(path)?;

        for line in buff.split('\n') {
            if line.is_empty() {
                break;
            }

            let mut split = line.split_whitespace();
            let range_raw = split.next().unwrap();
            let mut permissions_raw_chars = split.next().unwrap().chars();

            let mut range_split = range_raw.split('-');

            let from_str = range_split.next().unwrap();
            let to_str = range_split.next().unwrap();

            let from = usize::from_str_radix(from_str, 16)?;
            let to = usize::from_str_radix(to_str, 16)?;

            let read = permissions_raw_chars.next().unwrap();
            let write = permissions_raw_chars.next().unwrap();

            if read == 'r' && write == 'w' {
                v.push(MemoryRegion {
                    from,
                    size: to - from,
                });
            }
        }

        self.maps = v;
        Ok(self)
    }

    fn read_signature<T: TryFrom<usize>>(
        &self,
        sign: &Signature,
    ) -> Result<T, ProcessError> {
        let mut buff = Vec::new();

        for region in &self.maps {
            let remote = RemoteIoVec {
                base: region.from,
                len: region.size,
            };

            buff.resize(region.size, 0);

            let slice = IoSliceMut::new(buff.as_mut_slice());

            let res = process_vm_readv(
                Pid::from_raw(self.pid),
                &mut [slice],
                &[remote],
            );

            if let Err(e) = res {
                match e {
                    Errno::EPERM | Errno::ESRCH => return Err(e.into()),
                    _ => continue,
                }
            }

            if let Some(offset) = find_signature(buff.as_slice(), sign) {
                return (remote.base + offset)
                    .try_into()
                    .map_err(|_| ProcessError::AddressConvertError);
            }
        }

        Err(ProcessError::SignatureNotFound(sign.to_string()))
    }

    fn read<T: TryInto<usize>>(
        &self,
        addr: T,
        len: usize,
        buff: &mut [u8],
    ) -> Result<(), ProcessError> {
        let addr: usize = addr
            .try_into()
            .map_err(|_| ProcessError::AddressConvertError)?;

        let remote = RemoteIoVec { base: addr, len };

        let slice = IoSliceMut::new(buff);

        let res =
            process_vm_readv(Pid::from_raw(self.pid), &mut [slice], &[remote]);

        match res {
            Ok(_) => (),
            Err(e) => match e {
                nix::errno::Errno::EFAULT => {
                    return Err(ProcessError::BadAddress(addr, len))
                }
                _ => return Err(e.into()),
            },
        }

        Ok(())
    }
}
