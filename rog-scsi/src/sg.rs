use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::{self, IoSlice, IoSliceMut};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::time::Duration;

pub const SG_DXFER_NONE: i32 = -1;
pub const SG_DXFER_TO_DEV: i32 = -2;
pub const SG_DXFER_FROM_DEV: i32 = -3;
pub const SG_DXFER_TO_FROM_DEV: i32 = -4;

pub const SG_INFO_OK_MASK: u32 = 0x1;
pub const SG_INFO_OK: u32 = 0x0;
pub const SG_MAX_QUEUE: usize = 16;
pub const SG_IO: u64 = 0x2285;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SgIoHdr {
    pub interface_id: std::os::raw::c_int,
    pub dxfer_direction: std::os::raw::c_int,
    pub cmd_len: u8,
    pub mx_sb_len: u8,
    pub iovec_count: u16,
    pub dxfer_len: u32,
    pub dxferp: *mut c_void,
    pub cmdp: *mut u8,
    pub sbp: *mut u8,
    pub timeout: u32,
    pub flags: u32,
    pub pack_id: std::os::raw::c_int,
    pub usr_ptr: *mut c_void,
    pub status: u8,
    pub masked_status: u8,
    pub msg_status: u8,
    pub sb_len_wr: u8,
    pub host_status: u16,
    pub driver_status: u16,
    pub resid: i32,
    pub duration: u32,
    pub info: u32,
}

impl Default for SgIoHdr {
    fn default() -> Self {
        Self {
            interface_id: b'S' as std::os::raw::c_int,
            dxfer_direction: SG_DXFER_NONE,
            cmd_len: 0,
            mx_sb_len: 0,
            iovec_count: 0,
            dxfer_len: 0,
            dxferp: std::ptr::null_mut(),
            cmdp: std::ptr::null_mut(),
            sbp: std::ptr::null_mut(),
            timeout: 0,
            flags: 0,
            pack_id: 0,
            usr_ptr: std::ptr::null_mut(),
            status: 0,
            masked_status: 0,
            msg_status: 0,
            sb_len_wr: 0,
            host_status: 0,
            driver_status: 0,
            resid: 0,
            duration: 0,
            info: 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    None,
    ToDevice,
    FromDevice,
    ToFromDevice,
}

impl Direction {
    fn to_underlying(self) -> std::os::raw::c_int {
        match self {
            Direction::None => SG_DXFER_NONE,
            Direction::ToDevice => SG_DXFER_TO_DEV,
            Direction::FromDevice => SG_DXFER_FROM_DEV,
            Direction::ToFromDevice => SG_DXFER_TO_FROM_DEV,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    inner: SgIoHdr,
    cmd: Vec<u8>,
    data: Vec<u8>,
    sense: Vec<u8>,
}

impl Default for Task {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: Task manages its internal buffers and SgIoHdr raw pointers. The raw pointers
// are updated prior to any ioctl execution to point directly to owned heap vectors, making
// Send and Sync safe across thread boundaries.
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    pub fn new() -> Self {
        Task {
            inner: SgIoHdr::default(),
            cmd: Vec::new(),
            data: Vec::new(),
            sense: Vec::new(),
        }
    }

    /// Prepares raw internal pointers in SgIoHdr to match current buffer memory addresses.
    pub fn sync_pointers(&mut self) {
        if !self.cmd.is_empty() {
            self.inner.cmdp = self.cmd.as_mut_ptr();
            self.inner.cmd_len = self.cmd.len() as u8;
        } else {
            self.inner.cmdp = std::ptr::null_mut();
            self.inner.cmd_len = 0;
        }

        if !self.data.is_empty() {
            self.inner.dxferp = self.data.as_mut_ptr() as *mut c_void;
            self.inner.dxfer_len = self.data.len() as u32;
        } else {
            self.inner.dxferp = std::ptr::null_mut();
            self.inner.dxfer_len = 0;
        }

        if !self.sense.is_empty() {
            self.inner.sbp = self.sense.as_mut_ptr();
            self.inner.mx_sb_len = self.sense.len() as u8;
        } else {
            self.inner.sbp = std::ptr::null_mut();
            self.inner.mx_sb_len = 0;
        }
    }

    pub fn set_cdb(&mut self, buf: &[u8]) -> &mut Self {
        self.cmd = buf.to_vec();
        self.sync_pointers();
        self
    }

    pub fn cdb(&self) -> &[u8] {
        &self.cmd
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.inner.timeout = timeout.as_millis() as u32;
        self
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(u64::from(self.inner.timeout))
    }

    pub fn set_data(&mut self, buf: &[u8], direction: Direction) -> &mut Self {
        self.data = buf.to_vec();
        self.inner.dxfer_direction = direction.to_underlying();
        self.sync_pointers();
        self
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn set_sense_buffer(&mut self, buf: &[u8]) -> &mut Self {
        self.sense = buf.to_vec();
        self.sync_pointers();
        self
    }

    pub fn sense_buffer(&self) -> &[u8] {
        &self.sense
    }

    pub fn set_flags(&mut self, flags: u32) -> &mut Self {
        self.inner.flags = flags;
        self
    }

    pub fn flags(&self) -> u32 {
        self.inner.flags
    }

    pub fn duration(&self) -> u32 {
        self.inner.duration
    }

    pub fn residual_data(&self) -> i32 {
        self.inner.resid
    }

    pub fn status(&self) -> u8 {
        self.inner.status
    }

    pub fn host_status(&self) -> u16 {
        self.inner.host_status
    }

    pub fn driver_status(&self) -> u16 {
        self.inner.driver_status
    }

    pub fn ok(&self) -> bool {
        (self.inner.info & SG_INFO_OK_MASK) == SG_INFO_OK
    }
}

pub struct Device(File);

impl Device {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Device> {
        Ok(Device(
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)?,
        ))
    }

    /// Sends multiple tasks to the SCSI device queue.
    pub fn send(&self, tasks: &mut [Task]) -> io::Result<usize> {
        if tasks.is_empty() {
            return Ok(0);
        }

        for task in tasks.iter_mut() {
            task.sync_pointers();
        }

        // SAFETY: SgIoHdr is a repr(C) struct with standard byte layout. We create a read-only
        // slice over the byte buffer of each SgIoHdr for writev.
        let iovecs: Vec<IoSlice> = tasks
            .iter()
            .map(|task| {
                IoSlice::new(unsafe {
                    std::slice::from_raw_parts(
                        &task.inner as *const SgIoHdr as *const u8,
                        std::mem::size_of::<SgIoHdr>(),
                    )
                })
            })
            .collect();

        loop {
            match nix::sys::uio::writev(&self.0, &iovecs[..tasks.len()]) {
                Ok(n) => break Ok(n / std::mem::size_of::<SgIoHdr>()),
                Err(nix::errno::Errno::EINTR) => {}
                Err(e) => break Err(e.into()),
            }
        }
    }

    /// Receives completed tasks from the SCSI device queue.
    pub fn receive(&self, tasks: &mut Vec<Task>) -> io::Result<usize> {
        let mut hdrs = [SgIoHdr::default(); SG_MAX_QUEUE];

        // SAFETY: SgIoHdr is a repr(C) struct. We create a mutable byte slice over each header
        // buffer to allow readv to populate header data.
        let mut iovecs: Vec<IoSliceMut> = hdrs
            .iter_mut()
            .map(|hdr| {
                IoSliceMut::new(unsafe {
                    std::slice::from_raw_parts_mut(
                        hdr as *mut SgIoHdr as *mut u8,
                        std::mem::size_of::<SgIoHdr>(),
                    )
                })
            })
            .collect();

        let bytes_read = loop {
            match nix::sys::uio::readv(&self.0, iovecs.as_mut_slice()) {
                Ok(n) => break n,
                Err(e) if e == nix::errno::Errno::EINTR || e == nix::errno::Errno::EAGAIN => {}
                Err(e) => return Err(e.into()),
            }
        };

        if bytes_read == 0 {
            return Ok(0);
        }

        let tasks_read = bytes_read / std::mem::size_of::<SgIoHdr>();
        for hdr in hdrs.iter().take(tasks_read) {
            let mut task = Task::new();
            task.inner = *hdr;
            tasks.push(task);
        }
        Ok(tasks_read)
    }

    /// Performs a synchronous SCSI IO operation via ioctl.
    pub fn perform(&self, task: &Task) -> io::Result<()> {
        let mut task_copy = task.clone();
        task_copy.sync_pointers();

        #[cfg(target_env = "musl")]
        let request = SG_IO as i32;
        #[cfg(not(target_env = "musl"))]
        let request: u64 = SG_IO;

        // SAFETY: The raw file descriptor is open and valid, and task_copy has valid synced
        // buffer pointers matching the initialized memory of task_copy.inner.
        let ret = unsafe { libc::ioctl(self.0.as_raw_fd(), request, &task_copy.inner) };
        if ret == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
