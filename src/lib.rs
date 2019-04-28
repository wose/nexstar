#![no_std]

use embedded_hal::blocking::serial::write::Default;
use embedded_hal::prelude::*;
use embedded_hal::serial;
use nb::block;

#[derive(Debug)]
pub enum Error<T, U>
{
    UnexpectedResponse,
    Read(T),
    Write(U)
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

    /// Gets the version of the Hand Controller (HC) firmware.
    pub fn version(&mut self) -> Result<Version, Error<T::Error, U::Error>> {
        self.write_all(&[b'V' as u8])?;
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
        self.write_all(&[b'm' as u8])?;

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

        Ok(model)
    }

    pub fn free(self) -> (T, U) {
        (self.rx, self.tx)
    }

    fn read_multiple(&mut self, buffer: &mut [u8]) -> Result<(), Error<T::Error, U::Error>> {
        for idx in 0..buffer.len() {
            buffer[idx] = self.read()?
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
        let ack = self.read()?;

        if ack != b'#' {
            let _ = self.read()?;
            return Err(Error::UnexpectedResponse);
        }

        Ok(Version { major, minor })
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
