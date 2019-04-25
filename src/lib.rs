use embedded_hal::blocking::serial::write::Default;
use embedded_hal::prelude::*;
use embedded_hal::serial;
use nb::block;

#[derive(Debug)]
pub enum Error<T, U>
where
    T: serial::Read<u8>,
    U: serial::Write<u8>,
{
    UnexpectedResponse,
    Read(T::Error),
    Write(U::Error),
}

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

    pub fn version(&mut self) -> Result<Version, Error<T, U>> {
        block!(self.write_all(&['V' as u8])).map_err(|e| Error::Write(e))?;

        let major = self.read().map_err(|e| Error::Read(e))?;
        let minor = self.read().map_err(|e| Error::Read(e))?;
        let terminator = self.read().map_err(|e| Error::Read(e))?;
        if terminator != b'#' {
            return Err(Error::UnexpectedResponse);
        }

        Ok(Version { major, minor })
    }

    pub fn free(self) -> (T, U) {
        (self.rx, self.tx)
    }

    fn read_multiple(&mut self, buffer: &mut [u8]) -> Result<(), T::Error> {
        for idx in 0..buffer.len() {
            buffer[idx] = self.read()?
        }
        Ok(())
    }

    fn read(&mut self) -> Result<u8, T::Error> {
        block!(self.rx.read())
    }

    fn write_all(&mut self, buffer: &[u8]) -> nb::Result<(), U::Error> {
        self.bwrite_all(buffer).map_err(|e| nb::Error::Other(e))
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

