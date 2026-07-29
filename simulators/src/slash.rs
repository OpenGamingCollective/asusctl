use rog_slash::usb::{PROD_ID1, SLASH_HID_REPORT_DESCRIPTOR, VENDOR_ID};
use uhid_virt::{Bus, CreateParams, OutputEvent, UHIDDevice};

/// Virtual Slash Lightbar USB HID device simulator wrapper
pub struct VirtualSlashDevice {
    device: Option<UHIDDevice<std::fs::File>>,
}

impl VirtualSlashDevice {
    pub fn try_create() -> Result<Self, Box<dyn std::error::Error>> {
        let params = CreateParams {
            name: String::from("ROG Virtual Slash Lighting"),
            phys: String::from(""),
            uniq: String::from(""),
            bus: Bus::USB,
            vendor: VENDOR_ID as u32,
            product: PROD_ID1 as u32,
            version: 0,
            country: 0,
            rd_data: SLASH_HID_REPORT_DESCRIPTOR.to_vec(),
        };

        match UHIDDevice::create(params) {
            Ok(device) => Ok(Self {
                device: Some(device),
            }),
            Err(err) => {
                log::warn!("UHID Slash device creation unpermitted or unsupported: {err}");
                Ok(Self { device: None })
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.device.is_some()
    }

    /// Read and return available HID output events sent to this virtual device
    pub fn poll_events(&mut self) -> Vec<OutputEvent> {
        let mut events = Vec::new();
        if let Some(dev) = &mut self.device {
            while let Ok(event) = dev.read() {
                events.push(event);
            }
        }
        events
    }
}
