#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::convert::Infallible;

use crate::max7219_embassy::Error;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_rp::peripherals::{PIN_9, SPI1};
use embassy_rp::spi;
use embassy_rp::spi::Async;
use embassy_rp::spi::Spi;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::ascii::FONT_5X7;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Point, Primitive};
use embedded_graphics::primitives::Line;
use embedded_graphics::primitives::PrimitiveStyle;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_hal_async::spi::ExclusiveDevice;
use embedded_hal_async::spi::ExclusiveDeviceError;
use gpio::{Level, Output};
use max7219_embassy::{self, MAX7219LedMat};
use {defmt_rtt as _, panic_probe as _};

type Display<'d> = MAX7219LedMat<ExclusiveDevice<Spi<'d, SPI1, Async>, Output<'d, PIN_9>>, 256, 4>;
type DisplayError = Error<ExclusiveDeviceError<embassy_rp::spi::Error, Infallible>>;

async fn test<'a>(display: &mut Display<'a>) -> Result<(), DisplayError> {
    let txtstyle = MonoTextStyle::new(&FONT_5X7, BinaryColor::On);

    loop {
        Text::new("Test", Point::new(2, 6), txtstyle)
            .draw(display)
            .unwrap();
        display.flush().await?;

        Timer::after(Duration::from_secs(2)).await;
        display.clear();

        Line::new(Point::new(0, 0), Point::new(31, 7))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .unwrap();

        Line::new(Point::new(31, 0), Point::new(0, 7))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .unwrap();

        display.flush().await?;

        Timer::after(Duration::from_secs(2)).await;
        display.clear();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mosi = p.PIN_11;
    let clk = p.PIN_10;
    let cs = p.PIN_9;

    let mut config = spi::Config::default();
    config.frequency = 100_000;
    config.phase = spi::Phase::CaptureOnFirstTransition;
    config.polarity = spi::Polarity::IdleLow;

    let spi = Spi::new_txonly(p.SPI1, clk, mosi, p.DMA_CH0, config);
    let cs = Output::new(cs, Level::Low);

    let spi_dev = ExclusiveDevice::new(spi, cs);
    let mut display: MAX7219LedMat<_, 256, 4> = MAX7219LedMat::new(spi_dev);

    info!("MAX7219 - example 1");

    display.init_display().await.unwrap();
    test(&mut display).await.unwrap();
}
