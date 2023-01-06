#![no_std]
#![feature(generic_const_exprs)]

use core::convert::Infallible;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
use heapless::Vec;

#[derive(Debug)]
pub enum Error<E = ()> {
    /// Communication error
    Comm(E),
    /// Pin setting error
    Pin(Infallible),
}

pub enum Command {
    NoOp = 0x00,
    DecodeMode = 0x09,
    Intensity = 0x0A,
    ScanLimit = 0x0B,
    Shutdown = 0x0C,
    DisplayTest = 0x0F,
}

#[repr(u8)]
pub enum DecodeMode {
    NoDecode = 0x00,
    CodeB0 = 0x01,
    CodeB30 = 0x0F,
    CodeB70 = 0xFF,
}

#[repr(u8)]
pub enum Intensity {
    Min = 0x00,
    Ratio3_32 = 0x01,
    Ratio5_32 = 0x02,
    Ratio7_32 = 0x03,
    Ratio9_32 = 0x04,
    Ratio11_32 = 0x05,
    Ratio13_32 = 0x06,
    Ratio15_32 = 0x07,
    Ratio17_32 = 0x08,
    Ratio19_32 = 0x09,
    Ratio21_32 = 0x0A,
    Ratio23_32 = 0x0B,
    Ratio25_32 = 0x0C,
    Ratio27_32 = 0x0D,
    Ratio29_32 = 0x0E,
    Max = 0x0F,
}

pub enum Shutdown {
    ShutDownMode,
    NormalOperation,
}

#[repr(u8)]
pub enum ScanLimit {
    Display0Only = 0x00,
    Display0And1 = 0x01,
    Display0To2 = 0x02,
    Display0To3 = 0x03,
    Display0To4 = 0x04,
    Display0To5 = 0x05,
    Display0To6 = 0x06,
    Display0To7 = 0x07,
}

pub struct MAX7219LedMat<SPI, const BUFLEN: usize, const COUNT: usize> {
    spi: SPI,
    framebuffer: [u8; BUFLEN],
}

impl<SPI, E, const BUFLEN: usize, const COUNT: usize> MAX7219LedMat<SPI, BUFLEN, COUNT>
where
    SPI: SpiDevice<Error = E>,
    SPI::Bus: SpiBus,
    [(); 2 * COUNT]: Sized,
{
    pub fn new(spi: SPI) -> Self {
        let max7219 = MAX7219LedMat::<SPI, BUFLEN, COUNT> {
            spi: spi,
            framebuffer: [0; BUFLEN],
        };
        max7219
    }

    pub async fn flush(&mut self) -> Result<(), Error<E>> {
        for addr in 0..8 {
            let data = (0..COUNT)
                .rev()
                .map(|disp| {
                    let base = (disp * 8) + (addr * (COUNT * 8));
                    let arr = &self.framebuffer[base..base + 8];
                    let mut b: u8 = 0;
                    for i in 0..arr.len() {
                        b |= arr[i] << (arr.len() - 1 - i);
                    }
                    b
                })
                .collect::<Vec<u8, COUNT>>();

            let mut buffer: [u8; 2 * COUNT] = [0; 2 * COUNT];

            for i in 0..data.len() {
                buffer[2 * i] = addr as u8 + 1;
                buffer[2 * i + 1] = data[data.len() - 1 - i];
            }
            self.transmit_raw_data(&buffer).await?
        }
        Ok(())
    }

    pub async fn transmit_raw_data(&mut self, arr: &[u8]) -> Result<(), Error<E>> {
        self.spi.write(&arr).await.map_err(Error::Comm)
    }

    pub async fn config_power_mode(&mut self, mode: Shutdown) -> Result<(), Error<E>> {
        let data: u8 = match mode {
            Shutdown::NormalOperation => 0x01,
            Shutdown::ShutDownMode => 0x00,
        };

        let send_array: [u8; 2] = [Command::Shutdown as u8, data];
        // Transmit Data
        self.transmit_raw_data(&send_array).await
    }

    pub async fn config_decode_mode(&mut self, mode: DecodeMode) -> Result<(), Error<E>> {
        // - Prepare Information to be Sent
        // 8-bit Data/Command Corresponding to No Decode Mode
        let data: u8 = mode as u8;
        // Package into array to pass to SPI write method
        // Write method will grab array and send all data in it
        let send_array: [u8; 2] = [Command::DecodeMode as u8, data];
        // Transmit Data
        self.transmit_raw_data(&send_array).await
    }

    pub async fn config_scan_limit(&mut self, mode: ScanLimit) -> Result<(), Error<E>> {
        // - Prepare Information to be Sent
        // 8-bit Data/Command Corresponding to No Decode Mode
        let data: u8 = mode as u8;
        // Package into array to pass to SPI write method
        // Write method will grab array and send all data in it
        let send_array: [u8; 2] = [Command::ScanLimit as u8, data];
        // Transmit Data
        self.transmit_raw_data(&send_array).await
    }

    pub async fn config_intensity(&mut self, mode: Intensity) -> Result<(), Error<E>> {
        let data: u8 = mode as u8;
        let send_array: [u8; 2] = [Command::Intensity as u8, data];
        // Transmit Data
        self.transmit_raw_data(&send_array).await
    }

    pub fn clear(&mut self) {
        self.framebuffer = [0; BUFLEN];
    }

    pub async fn init_display(&mut self) -> Result<(), Error<E>> {
        self.config_power_mode(Shutdown::NormalOperation).await?;
        self.config_decode_mode(DecodeMode::NoDecode).await?;
        self.config_scan_limit(ScanLimit::Display0To7).await?;
        self.config_intensity(Intensity::Ratio3_32).await
    }
}

extern crate embedded_graphics_core;
use self::embedded_graphics_core::{draw_target::DrawTarget, pixelcolor::BinaryColor, prelude::*};

impl<SPI, const BUFLEN: usize, const COUNT: usize> DrawTarget
    for MAX7219LedMat<SPI, BUFLEN, COUNT>
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bb = self.bounding_box();
        pixels
            .into_iter()
            .filter(|Pixel(pos, _color)| bb.contains(*pos))
            .for_each(|Pixel(pos, color)| {
                let index: u32 = pos.x as u32 + pos.y as u32 * 8 * (COUNT as u32);
                self.framebuffer[index as usize] = color.is_on() as u8;
            });
        Ok(())
    }
}

impl<SPI, const BUFLEN: usize, const COUNT: usize> OriginDimensions
    for MAX7219LedMat<SPI, BUFLEN, COUNT>
{
    fn size(&self) -> Size {
        Size::new(COUNT as u32 * 8, 8)
    }
}
