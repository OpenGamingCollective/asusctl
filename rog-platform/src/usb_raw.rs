use std::time::Duration;

use rusb::{Device, DeviceHandle};

use crate::error::{PlatformError, Result};

#[derive(Debug, PartialEq, Eq)]
pub struct USBRaw(DeviceHandle<rusb::GlobalContext>);

impl USBRaw {
    pub fn new(id_product: u16) -> Result<Self> {
        for device in rusb::devices()?.iter() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == 0x0b05 && device_desc.product_id() == id_product {
                let handle = Self::get_dev_handle(&device)?;
                return Ok(Self(handle));
            }
        }

        Err(PlatformError::MissingFunction(format!(
            "USBRaw dev {id_product} not found"
        )))
    }

    fn get_dev_handle(
        device: &Device<rusb::GlobalContext>,
    ) -> Result<DeviceHandle<rusb::GlobalContext>> {
        // We don't expect this ID to ever change
        let device = device.open()?;
        device.reset()?;
        device.set_auto_detach_kernel_driver(true)?;
        device.claim_interface(0)?;
        Ok(device)
    }

    pub fn write_bytes(&self, message: &[u8]) -> Result<usize> {
        self.0
            .write_control(
                0x21,  // request_type
                0x09,  // request
                0x35e, // value
                0x00,  // index
                message,
                Duration::from_millis(200),
            )
            .map_err(PlatformError::USB)
    }
}
