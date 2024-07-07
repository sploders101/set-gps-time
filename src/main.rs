use std::{io::{BufRead, BufReader}, process::Command, time::Instant};

use anyhow::Context;
use chrono::{DateTime, Utc};
use clap::Parser;
use serial2::{SerialPort, Settings};

#[derive(Parser)]
/// Sets the system time from a serial GPS device
struct Args {
    #[arg()]
    gps_device: String,

    #[arg(short = 'r', long)]
    baud_rate: Option<u32>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let port = SerialPort::open(args.gps_device, |mut settings: Settings| {
        if let Some(rate) = args.baud_rate {
            settings.set_baud_rate(rate)?;
        }
        return Ok(settings);
    })?;
    let mut port = BufReader::new(port);

    let mut line = String::new();
    let mut seen_gpgga = 0;
    while let Ok(_bytes_read) = port.read_line(&mut line) {
        let received_at = Instant::now();
        let mut splitline = line.split(',');
        let header = splitline.next().context("Missing GPS header")?;
        match header {
            "$GPZDA" | "$GNZDA" =>  {
                let time_status: &str = splitline.next().context("Missing UTC time status")?;
                let day = splitline.next().context("Missing day")?.parse()?;
                let month = splitline.next().context("Missing month")?.parse()?;
                let year = splitline.next().context("Missing year")?.parse()?;

                if time_status.len() != 9 {
                    anyhow::bail!("Invalid time status length");
                }

                let hour = time_status[0..2].parse()?;
                let minute = time_status[2..4].parse()?;
                let second = time_status[4..6].parse()?;
                let millis = time_status[7..9].parse::<u32>()? * 10u32;

                let date = chrono::NaiveDate::from_ymd_opt(year, month, day).context("Invalid date received")?;
                let time = chrono::NaiveTime::from_hms_milli_opt(hour, minute, second, millis).context("Invalid time received")?;
                let datetime = chrono::NaiveDateTime::new(date, time).and_utc();

                #[cfg(target_os = "macos")]
                set_datetime_macos(datetime, received_at)?;
                #[cfg(target_os = "linux")]
                set_datetime_linux(datetime, received_at)?;

                println!("Successfully set time!");

                break;
            }
            "$GPGGA" => {
                seen_gpgga += 1;
                if seen_gpgga < 5 {
                    println!("Skipping $GPGGA. Fallback in {} reports.", 5 - seen_gpgga);
                    line.truncate(0);
                    continue;
                }
                println!("$GPZDA/$GNZDA not seen in 5 reports. Using UTC data from $GPGGA.");
                let time_status: &str = splitline.next().context("Missing UTC time status")?;
                let hour = time_status[0..2].parse()?;
                let minute = time_status[2..4].parse()?;
                let second = time_status[4..6].parse()?;
                let millis = time_status[7..9].parse::<u32>()? * 10u32;
                let date = chrono::offset::Utc::now();
                let time = chrono::NaiveTime::from_hms_milli_opt(hour, minute, second, millis).context("Invalid time received")?;
                let datetime = date.with_time(time).unwrap();

                #[cfg(target_os = "macos")]
                set_datetime_macos(datetime, received_at)?;
                #[cfg(target_os = "linux")]
                set_datetime_linux(datetime, received_at)?;

                break;
            }
            _ => {}
        }

        // Reset for the next line, but keep the space allocated
        line.truncate(0);
    }

    return Ok(());
}

#[cfg(target_os = "macos")]
/// Makes sure NTP is off while running `callback`
fn ntp_off_macos() -> anyhow::Result<()> {
    use std::process::Stdio;


    let command = Command::new("systemsetup")
        .arg("-getusingnetworktime")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("Couldn't set Mac OS time")?;
    if !command.status.success() {
        println!("Failed to get network time setting. See console output for errors");
    } else if command.stdout == b"Network Time: On\n" {
        println!("Disabling network time. You can re-enable it in System Settings -> General -> Date and Time.");

        let command = Command::new("systemsetup")
            .arg("-setusingnetworktime")
            .arg("off")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .context("Couldn't set Mac OS time")?;
        if !command.status.success() {
            anyhow::bail!("Failed to disable ntp. See console output for errors");
        }
    }

    return Ok(());
}

#[cfg(target_os = "macos")]
/// Sets the time on a mac os system. Uses the `systemsetup` command since Mac OS doesn't
/// support the `settimeofday` function.
pub fn set_datetime_macos(datetime: DateTime<Utc>, received_at: Instant) -> anyhow::Result<()> {
    use std::process::Stdio;

    let newtime = datetime.with_timezone(&chrono::offset::Local);

    ntp_off_macos()?;

    let new_datetime = newtime + received_at.elapsed();
    let date_str = new_datetime.format("%m/%d/%Y").to_string();
    let command = Command::new("systemsetup")
        .arg("-setdate")
        .arg(date_str)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context("Couldn't set Mac OS time")?;
    if !command.status.success() {
        anyhow::bail!("Failed to set Mac OS time. See console output for errors");
    }

    let new_datetime = newtime + received_at.elapsed();
    let time_str = new_datetime.format("%T%.3f").to_string();
    let command = Command::new("systemsetup")
        .arg("-settime")
        .arg(time_str)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context("Couldn't set Mac OS time")?;
    if !command.status.success() {
        anyhow::bail!("Failed to set Mac OS time. See console output for errors");
    }

    return Ok(());
}

#[cfg(target_os = "linux")]
/// Sets the time on a linux machine. This function should theoretically work,
/// but has not been tested.
pub fn set_datetime_linux(datetime: DateTime<Utc>, received_at: Instant) -> anyhow::Result<()> {
    let datetime = datetime + received_at.elapsed();
    let timestamp = datetime.timestamp();
    let millis = datetime.timestamp_millis();
    unsafe {
        let timeval = libc::timeval {
            tv_sec: timestamp,
            tv_usec: (millis * 1000) as _,
        };
        let result = libc::settimeofday(&timeval, std::ptr::null());
        if result == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        return Ok(());
    }
}
