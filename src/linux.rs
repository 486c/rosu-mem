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

struct LinuxProcessInfo {
    pid: u32,
    cmd_buff: String,
    executable_dir: Option<PathBuf>,
}

impl Process {
    fn get_processes_infos() -> Result<Vec<LinuxProcessInfo>, ProcessError> {
        let paths = fs::read_dir("/proc")?;
        let mut infos: Vec<LinuxProcessInfo> = Vec::new();

        for path in paths {
            let p = path?.path();

            let info = match Self::get_proc_info(p) {
                Ok(info) => info,
                Err(_) => continue ,
            };

            infos.push(info);
        }

        Ok(infos)
    }

    fn get_proc_info(path: PathBuf) -> Result<LinuxProcessInfo, ProcessError> {
        if !path.is_dir() {
            return Err(ProcessError::ProcessNotFound);
        }

        let cmd_line = path.join("cmdline");

        if !cmd_line.exists() {
            return Err(ProcessError::ProcessNotFound);
        }

        let cmd_buff = fs::read_to_string(cmd_line)?;
        let mut cmd_buff_temp = cmd_buff.clone();

        let stat = path.join("stat");
        let buff = fs::read_to_string(stat)?;

        // Formatting path
        cmd_buff_temp.retain(|c| c != '\0');
        cmd_buff_temp = cmd_buff_temp.replace('\\', "/");

        cmd_buff_temp.remove(0);
        cmd_buff_temp.remove(0);

        let executable_path = PathBuf::from(cmd_buff_temp);
        let executable_dir =
            executable_path.parent().map(|v| v.to_path_buf());

        let pid_str = buff.split(' ').next().unwrap();

        let pid = pid_str.parse::<i32>()?;

        Ok(LinuxProcessInfo {
            pid: pid as i32,
            cmd_buff,
            executable_dir,
        })
    }
}

impl ProcessTraits for Process {
    fn initialize(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Process, ProcessError> {
        let process = Process::find_process(proc_name, exclude)?;

        process.read_regions()
    }

    fn initialize_manual(pid: u32) -> Result<Process, ProcessError> {
        let infos = match Self::get_processes_infos() {
            Ok(infos) => infos,
            Err(_) => return Err(ProcessError::ProcessNotFound),
        };

        let info = match infos.iter().find(|info| info.pid == pid) {
            Some(info) => info,
            None => return Err(ProcessError::ProcessNotFound),
        };

        let process = Process {
            pid: info.pid,
            maps: Vec::new(),
            executable_dir: info.executable_dir,
        };

        process.read_regions()
    }

    fn find_process(
        proc_name: &str,
        exclude: &[&str],
    ) -> Result<Process, ProcessError> {
        let infos = match Self::get_processes_infos() {
            Ok(infos) => infos,
            Err(_) => return Err(ProcessError::ProcessNotFound),
        };

        let info = infos.iter().find(|info| {
            let line = info.cmd_buff.split(' ').next().unwrap();

            if !line.contains(proc_name) {
                return false
            }

            for exclude_word in exclude {
                if line.contains(exclude_word) {
                    return false;
                }
            }

            true
        });

        let info = match info {
            Some(info) => info,
            None => return Err(ProcessError::ProcessNotFound),
        };

        return Ok(Process {
            pid: info.pid as i32,
            maps: Vec::new(),
            executable_dir: info.executable_dir,
        });
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
