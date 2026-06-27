use std::error::Error;

use rog_platform::hid_raw::HidRaw;

pub fn main() -> Result<(), Box<dyn Error>> {
    let mut usb_node = None;
    match HidRaw::new("1abe") {
        Ok(node) => {
            let id = node.prod_id().to_owned();
            usb_node = Some(node);
            println!("Looked for keyboard controller 0x{id}: Found");
        }
        Err(err) => println!("Looked for keyboard controller: {err}"),
    }

    if usb_node.is_none() {
        return Err("RogError::NoAuraKeyboard".into());
    }

    let node = usb_node.unwrap();

    // node.write_bytes(&[0x5a, 0xd1, 0x0a, 0x01])?; // TODO: need to CHECK
    println!("Set mouse mode for 10 seconds");
    node.write_bytes(&[
        0x5a, 0xd1, 0x01, 0x01, 0x03,
    ])?;
    node.write_bytes(&[
        0x5a, 0xd1, 0x0f, 0x20,
    ])?;
    node.write_bytes(&[
        0x5a, 0xd1, 0x01, 0x01, 0x00,
    ])?;
    // sleep(Duration::from_secs(10));

    // println!("Set wasd mode for 10 seconds");
    // node.write_bytes(&[0x5a, 0xd1, 0x01, 0x01, 0x02])?;
    // node.write_bytes(&[0x5a, 0xd1, 0x0f, 0x20])?;
    // sleep(Duration::from_secs(10));

    // println!("Set back to gamepad mode");
    // node.write_bytes(&[0x5a, 0xd1, 0x01, 0x01, 0x01])?;
    // node.write_bytes(&[0x5a, 0xd1, 0x0f, 0x20])?;

    Ok(())
}
