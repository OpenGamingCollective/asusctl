use rog_anime::usb::{ANIME_HID_REPORT_DESCRIPTOR, PROD_ID, VENDOR_ID};
use rog_anime::AnimeType;
use uhid_virt::{Bus, CreateParams, OutputEvent, UHIDDevice};

/// Virtual AniMe Matrix USB HID device simulator wrapper
pub struct VirtualAniMeDevice {
    device: Option<UHIDDevice<std::fs::File>>,
    anime_type: AnimeType,
}

impl VirtualAniMeDevice {
    /// Attempt to create a virtual UHID device for the specified AniMe Matrix model
    pub fn try_create(anime_type: AnimeType) -> Result<Self, Box<dyn std::error::Error>> {
        let params = CreateParams {
            name: format!("ROG Virtual AniMe Matrix ({anime_type:?})"),
            phys: String::from(""),
            uniq: String::from(""),
            bus: Bus::USB,
            vendor: VENDOR_ID as u32,
            product: PROD_ID as u32,
            version: 0,
            country: 0,
            rd_data: ANIME_HID_REPORT_DESCRIPTOR.to_vec(),
        };

        match UHIDDevice::create(params) {
            Ok(device) => Ok(Self {
                device: Some(device),
                anime_type,
            }),
            Err(err) => {
                log::warn!("UHID device creation unpermitted or unsupported: {err}");
                Ok(Self {
                    device: None,
                    anime_type,
                })
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.device.is_some()
    }

    pub fn anime_type(&self) -> AnimeType {
        self.anime_type
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
