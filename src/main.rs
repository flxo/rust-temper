use anyhow::{anyhow, Context, Result};
use hidapi::HidApi;
use structopt::StructOpt;

static FIRMWARE_1F: &str = "TEMPer1F";

#[derive(StructOpt)]
struct Opt {
    /// Vendor id
    #[structopt(short, long, default_value = "0c45")]
    vid: String,

    /// Product id
    #[structopt(short, long, default_value = "7401")]
    pid: String,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let vid = u16::from_str_radix(&opt.vid, 16).context("Failed to parse vendor id")?;
    let pid = u16::from_str_radix(&opt.pid, 16).context("Failed to parse product id")?;

    let api = HidApi::new().context("Failed to open api")?;
    'outer: for device in api.device_list() {
        if device.vendor_id() == vid && device.product_id() == pid {
            let device = api
                .open_path(device.path())
                .with_context(|| format!("Failed to open device {:?}", device.path()))?;
            let mut buffer = [0u8; 8];

            // get firmware
            device.write(&[0x01, 0x86, 0xff, 0x01, 0, 0, 0, 0])?;
            device.read(&mut buffer)?;
            let firmware =
                String::from_utf8(buffer.to_vec()).context("Failed to read firmware version")?;
            if firmware != FIRMWARE_1F {
                return Err(anyhow!(format!("unsupported firmware: {}", firmware)));
            }

            // First read always delivers garbadge
            device.write(&[0x01, 0x80, 0x33, 0x01, 0, 0, 0, 0])?;
            device.read(&mut buffer)?;

            device.write(&[0x01, 0x80, 0x33, 0x01, 0, 0, 0, 0])?;
            device.read(&mut buffer)?;

            let b2 = buffer[2] as u16;
            let b3 = buffer[3] as u16;
            if b2 == 0x4e && b3 == 0x20 {
                return Err(anyhow!("Failed to read"));
            }

            let t: i16 = (b2 as i16) << 8u32 | b3 as i16;
            let temperature = (t as f32) / 256.0;

            println!("{}", temperature);
            break 'outer;
        }
    }
    Ok(())
}
