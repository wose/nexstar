#![no_std]

use embedded_hal::blocking::serial::write::Default;
use embedded_hal::prelude::*;
use embedded_hal::serial;
use nb::block;

#[derive(Debug)]
pub enum Error<T, U> {
    UnexpectedResponse,
    Read(T),
    Write(U),
}

/// Sub Device Commands
#[derive(Copy, Clone)]
pub enum Command {
    GetDeviceVersion = 0xFE,
}

impl Command {
    fn bits(&self) -> u8 {
        *self as u8
    }
}

/// Date Time
#[derive(Copy, Clone)]
pub struct DateTime {
    /// Hour (24 hour clock)
    pub hour: u8,
    /// Minutes
    pub minutes: u8,
    /// Seconds
    pub seconds: u8,
    /// Offset from GMT.
    pub zone: i8,
    /// Daylight Savings or Standard Time
    pub daylight_saving: bool,
    /// Year with century assumed as 20.
    pub year: u8,
    /// Month
    pub month: u8,
    /// Day
    pub day: u8,
}

/// Sub Device
#[derive(Copy, Clone)]
pub enum Device {
    /// Main / Interconnection Board
    MainBoard = 0x01,
    /// Hand Controller (HC)
    HandController = 0x04,
    /// AZM/RA Motor
    AzmRaMotor = 0x10,
    /// ALT/DEC Motor
    AltDecMotor = 0x11,
    /// GPS Unit
    GPSUnit = 0xb0,
    /// RTC (CGE only)
    RTC = 0xb2,
}

impl Device {
    fn bits(&self) -> u8 {
        *self as u8
    }
}

/// Location of the mount
#[derive(Copy, Clone)]
pub struct Location {
    pub latitude: f32,
    pub longitude: f32,
}

impl Location {
    pub fn lat_dms(&self) -> [u8; 4] {
        dec_dms(self.latitude)
    }

    pub fn lon_dms(&self) -> [u8; 4] {
        dec_dms(self.longitude)
    }
}

fn dec_dms(dec: f32) -> [u8; 4] {
    let sign = if dec < 0.0 { 0x00 } else { 0x01 };
    let dec = if dec < 0.0 { -dec } else { dec };

    let deg = dec as u8;
    let min = (dec - deg as f32) * 60.0;
    let sec = ((min - min as u8 as f32) * 60.0 + 0.5) as u8;

    [deg, min as u8, sec, sign]
}

/// Telescope mount model
#[derive(Debug, Copy, Clone)]
pub enum Model {
    /// GPS Series
    GPSSeries,
    /// i-Series
    ISeries,
    /// i-Series SE
    ISeriesSE,
    /// CGE
    CGE,
    /// Advanced GT
    AdvancedGT,
    /// SLT
    SLT,
    /// CPC
    CPC,
    /// GT
    GT,
    /// 4/5 SE
    Se4_5,
    /// 6/8 SE
    Se6_8,
    /// Unknown Model
    Unknown(u8),
}

#[derive(Copy, Clone)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

#[derive(Clone)]
pub struct NexStar<T, U>
where
    T: serial::Read<u8>,
    U: serial::Write<u8>,
{
    rx: T,
    tx: U,
}

impl<T, U> NexStar<T, U>
where
    T: serial::Read<u8>,
    U: serial::Write<u8>,
{
    pub fn new(rx: T, tx: U) -> NexStar<T, U> {
        NexStar { rx, tx }
    }

    // Time/Location Commands (Hand Control)
    /// Gets the currently set location of the telescope.
    pub fn location(&mut self) -> Result<Location, Error<T::Error, U::Error>> {
        self.write_all(&[b'w'])?;

        let mut buffer = [0u8; 8];
        self.read_multiple(&mut buffer)?;
        self.check_ack()?;

        let latitude = buffer[0] as f32 + buffer[1] as f32 / 60.0 + buffer[2] as f32 / 3600.0;
        let latitude = match buffer[3] {
            0x00 => latitude,
            0x01 => -latitude,
            _ => return Err(Error::UnexpectedResponse),
        };

        let longitude = buffer[4] as f32 + buffer[5] as f32 / 60.0 + buffer[6] as f32 / 3600.0;
        let longitude = match buffer[7] {
            0x00 => longitude,
            0x01 => -longitude,
            _ => return Err(Error::UnexpectedResponse),
        };

        Ok(Location {
            latitude,
            longitude,
        })
    }

    /// Sets the location of the Hand Controller (HC).
    pub fn set_location(&mut self, location: Location) -> Result<(), Error<T::Error, U::Error>> {
        let mut buffer = [0u8; 9];
        buffer[0] = b'W';
        &buffer[1..5].copy_from_slice(&location.lat_dms());
        &buffer[5..].copy_from_slice(&location.lon_dms());

        self.write_all(&buffer)?;
        self.check_ack()?;

        Ok(())
    }

    /// Gets the currently set date and time of the Hand Controller (HC).
    pub fn datetime(&mut self) -> Result<DateTime, Error<T::Error, U::Error>> {
        self.write_all(&[b'h'])?;

        let mut buffer = [0u8; 8];
        self.read_multiple(&mut buffer)?;
        self.check_ack()?;

        Ok(DateTime {
            hour: buffer[0],
            minutes: buffer[1],
            seconds: buffer[2],
            zone: buffer[6] as i8,
            daylight_saving: buffer[7] == 1,
            year: buffer[5],
            month: buffer[3],
            day: buffer[4],
        })
    }

    /// Sets date and time of the Hand Controller (HC).
    pub fn set_datetime(&mut self, datetime: DateTime) -> Result<(), Error<T::Error, U::Error>> {
        let buffer = [
            b'H',
            datetime.hour,
            datetime.minutes,
            datetime.seconds,
            datetime.month,
            datetime.day,
            datetime.year,
            datetime.zone as u8,
            datetime.daylight_saving as u8,
        ];

        self.write_all(&buffer)?;
        self.check_ack()?;

        Ok(())
    }

    // Miscellaneous Commands
    /// Gets the version of the Hand Controller (HC) firmware.
    pub fn version(&mut self) -> Result<Version, Error<T::Error, U::Error>> {
        self.write_all(&[b'V'])?;
        self.read_version()
    }

    /// gets the version of the specified sub device.
    pub fn device_version(&mut self, device: Device) -> Result<Version, Error<T::Error, U::Error>> {
        let cmd = [
            0x50,
            0x01,
            device.bits(),
            Command::GetDeviceVersion.bits(),
            0x00,
            0x00,
            0x00,
            0x02,
        ];
        self.write_all(&cmd)?;
        self.read_version()
    }

    /// Gets the model of the telescope mount.
    pub fn model(&mut self) -> Result<Model, Error<T::Error, U::Error>> {
        self.write_all(&[b'm'])?;

        let model = match self.read()? {
            0x01 => Model::GPSSeries,
            0x03 => Model::ISeries,
            0x04 => Model::ISeriesSE,
            0x05 => Model::CGE,
            0x06 => Model::AdvancedGT,
            0x07 => Model::SLT,
            0x09 => Model::CPC,
            0x0A => Model::GT,
            0x0B => Model::Se4_5,
            0x0C => Model::Se6_8,
            id => Model::Unknown(id),
        };
        self.check_ack()?;

        Ok(model)
    }

    /// Gets the alignment state.
    pub fn is_alignment_complete(&mut self) -> Result<bool, Error<T::Error, U::Error>> {
        self.write_all(&[b'J'])?;
        let active = self.read()?;
        self.check_ack()?;
        Ok(active == 0x01)
    }

    /// Gets GOTO state.
    pub fn is_goto_in_progress(&mut self) -> Result<bool, Error<T::Error, U::Error>> {
        self.write_all(&[b'L'])?;
        let active = self.read()?;
        self.check_ack()?;
        Ok(active == b'1')
    }

    fn echo(&mut self) -> Result<(), Error<T::Error, U::Error>> {
        self.write_all(&[b'K', 0x42])?;
        let res = self.read()?;
        self.check_ack()?;

        match res {
            0x42 => Ok(()),
            _ => Err(Error::UnexpectedResponse),
        }
    }

    pub fn free(self) -> (T, U) {
        (self.rx, self.tx)
    }

    fn read_multiple(&mut self, buffer: &mut [u8]) -> Result<(), Error<T::Error, U::Error>> {
        for idx in 0..buffer.len() {
            buffer[idx] = self.read()?;
        }
        Ok(())
    }

    fn read(&mut self) -> Result<u8, Error<T::Error, U::Error>> {
        block!(self.rx.read()).map_err(|e| Error::Read(e))
    }

    fn write_all(&mut self, buffer: &[u8]) -> Result<(), Error<T::Error, U::Error>> {
        self.bwrite_all(buffer).map_err(|e| Error::Write(e))?;
        self.bflush().map_err(|e| Error::Write(e))
    }

    fn read_version(&mut self) -> Result<Version, Error<T::Error, U::Error>> {
        let major = self.read()?;
        let minor = self.read()?;

        self.check_ack()?;

        Ok(Version { major, minor })
    }

    fn check_ack(&mut self) -> Result<(), Error<T::Error, U::Error>> {
        let ack = self.read()?;

        match ack {
            b'#' => Ok(()),
            _ => {
                // consume the addidional byte sent when an error occurred
                self.read()?;
                Err(Error::UnexpectedResponse)
            }
        }
    }
}

impl<T, U> serial::Write<u8> for NexStar<T, U>
where
    T: serial::Read<u8>,
    U: serial::Write<u8>,
{
    type Error = U::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.tx.write(word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.tx.flush()
    }
}

impl<T, U> Default<u8> for NexStar<T, U>
where
    T: serial::Read<u8>,
    U: serial::Write<u8>,
{
}
