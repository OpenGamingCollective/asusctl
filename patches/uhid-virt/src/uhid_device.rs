use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{self, prelude::*};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::codec::*;

pub struct UHIDDevice<T: Read + Write> {
    handle: T,
}

/// Contains information about your HID device, sent when UHIDDevice is created
#[derive(Debug, Clone, PartialEq)]
pub struct CreateParams {
    pub name: String,
    pub phys: String,
    pub uniq: String,
    pub bus: Bus,
    pub vendor: u32,
    pub product: u32,
    pub version: u32,
    pub country: u32,
    pub rd_data: Vec<u8>,
}

/// Character misc-device handle for a specific HID device
impl<T: Read + Write> UHIDDevice<T> {
    /// The data parameter should contain a data-payload. This is the raw data that you read from your device. The kernel will parse the HID reports.
    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let event: [u8; UHID_EVENT_SIZE] = InputEvent::Input { data }.into();
        self.handle.write(&event)
    }

    /// Write a SetReportReply, only use in reponse to a read SetReport event
    pub fn write_set_report_reply(&mut self, id: u32, err: u16) -> io::Result<usize> {
        let event: [u8; UHID_EVENT_SIZE] = InputEvent::SetReportReply { id, err }.into();
        self.handle.write(&event)
    }

    /// Write a GetReportReply, only use in reponse to a read GetReport event
    pub fn write_get_report_reply(
        &mut self,
        id: u32,
        err: u16,
        data: Vec<u8>,
    ) -> io::Result<usize> {
        let event: [u8; UHID_EVENT_SIZE] = InputEvent::GetReportReply { id, err, data }.into();
        self.handle.write(&event)
    }

    /// Reads a queued output event. No reaction is required to an output event, but you should handle them according to your needs.
    pub fn read(&mut self) -> Result<OutputEvent, StreamError> {
        let mut event = [0u8; UHID_EVENT_SIZE];
        self.handle
            .read_exact(&mut event)
            .map_err(StreamError::Io)?;
        OutputEvent::try_from(event)
    }

    /// This destroys the internal HID device. No further I/O will be accepted. There may still be pending output events that you can receive but no further input events can be sent to the kernel.
    pub fn destroy(&mut self) -> io::Result<usize> {
        let event: [u8; UHID_EVENT_SIZE] = InputEvent::Destroy.into();
        self.handle.write(&event)
    }
}

impl UHIDDevice<File> {
    /// Opens the character misc-device at /dev/uhid
    pub fn create(params: CreateParams) -> io::Result<UHIDDevice<File>> {
        UHIDDevice::create_with_path(params, Path::new("/dev/uhid"))
    }
    pub fn create_with_path(params: CreateParams, path: &Path) -> io::Result<UHIDDevice<File>> {
        let mut options = OpenOptions::new();
        options.read(true);
        options.write(true);
        if cfg!(unix) {
            options.custom_flags(libc::O_RDWR | libc::O_CLOEXEC | libc::O_NONBLOCK);
        }
        let mut handle = options.open(path)?;
        let event: [u8; UHID_EVENT_SIZE] = InputEvent::Create(params).into();
        handle.write_all(&event)?;
        Ok(UHIDDevice { handle })
    }
}
