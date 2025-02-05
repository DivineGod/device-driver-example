#![no_std]
#![no_main]

use core::fmt::Write;
use cst816s_device_driver::{device, CST816S};
use defmt::info;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, Triangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, Level, Output, Pull},
    i2c::{self},
    main,
    spi::{self, Mode},
};
use fugit::RateExtU32;
use heapless::String;
use mipidsi::{interface::SpiInterface, Builder};

const LCD_WIDTH: u32 = 240;
const LCD_HEIGHT: u32 = 240;
// Define static buffers
const BUFFER_SIZE: usize = (LCD_WIDTH * LCD_HEIGHT * 2) as usize;
// 16 FPS  Is as fast as I can update the arrow smoothly so all frames are as fast as the slowest.
// const DESIRED_FRAME_DURATION_US: u32 = 1_000_000 / 16;

#[main]
fn main() -> ! {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let miso = peripherals.GPIO3;
    let mosi = peripherals.GPIO2;
    let sclk = peripherals.GPIO11;
    let lcd_cs = Output::new(peripherals.GPIO10, Level::Low);
    let lcd_dc = Output::new(peripherals.GPIO8, Level::Low);
    let lcd_rst = Output::new(peripherals.GPIO1, Level::Low);
    let mut lcd_bl = Output::new(peripherals.GPIO0, Level::Low);

    let i2c_sda = peripherals.GPIO7;
    let i2c_scl = peripherals.GPIO6;
    let touch_int = peripherals.GPIO5;
    let touch_reset = peripherals.GPIO4;

    info!("Driver configured!");

    let driver = spi::master::Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default()
            .with_frequency(40.MHz())
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)
    .with_mosi(mosi);

    let spi_device = ExclusiveDevice::new_no_delay(driver, lcd_cs).unwrap();

    info!("Spi Driver configured!");
    let mut delay = Delay::new();

    // Initialize the display using SPI
    let mut buffer = [0_u8; 512];
    let di = SpiInterface::new(spi_device, lcd_dc, &mut buffer);
    // Use the delay wrapper when initializing the display
    let mut display = Builder::new(mipidsi::models::GC9A01, di)
        .reset_pin(lcd_rst)
        .init(&mut delay)
        .unwrap();

    info!("Display Driver configured!");

    // Make the display all black
    display.clear(Rgb565::BLACK).unwrap();

    delay.delay_millis(1);
    lcd_bl.set_high();

    // Setup Touch Driver
    //
    // Set up the pins needed for the driver
    let touch_interrupt_pin = Input::new(touch_int, Pull::Up);
    // Setup reset pin for touch driver
    let touch_reset_pin = Output::new(touch_reset, Level::High);

    // Create the I²C driver, using the two pre-configured pins.
    // This will fail  at compile time if the pins are in the wrong mode, or if
    // this I²C peripheral isn't available on these pins!
    let i2c = i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(400.kHz()),
    )
    .unwrap()
    .with_sda(i2c_sda)
    .with_scl(i2c_scl);

    let mut touchpad = CST816S::new(i2c, 0x15, touch_interrupt_pin, touch_reset_pin);

    // Setup Touch Driver
    touchpad.reset(&mut delay).unwrap();
    touchpad.init_config().unwrap();
    info!("Driver configured!");

    /* End Touch Driver Setup */

    let mut character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CYAN);
    character_style.background_color = Some(Rgb565::WHITE);
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    let mut last_touch = (0, 0);
    let mut color = Rgb565::CSS_NAVAJO_WHITE;

    loop {
        // Read a touch event from the touch driver and update last_touch if there is a valid event
        if let Some(touch_event) = touchpad.event() {
            info!("touch Event {}", touch_event.point.0);
            color = match touch_event.gesture {
                device::Gesture::NoGesture => {
                    info!("no gesture");
                    Rgb565::WHITE
                }
                device::Gesture::SlideUp => Rgb565::RED,
                device::Gesture::SlideDown => Rgb565::BLUE,
                device::Gesture::SlideLeft => Rgb565::YELLOW,
                device::Gesture::SlideRight => Rgb565::GREEN,
                device::Gesture::SingleClick => Rgb565::MAGENTA,
                device::Gesture::DoubleClick => Rgb565::CSS_TAN,
                device::Gesture::LongPress => Rgb565::CSS_PINK,
            };
            last_touch = touch_event.point;
        }

        // `write` for `heapless::String` returns an error if the buffer is full,
        // but because the buffer here is 9 bytes large, the `(xxx:yyy)` will fit.
        let mut data = String::<9>::new(); // 9 byte string buffer
        let (x, y) = last_touch;
        let _ = write!(data, "({x:03},{y:03})").unwrap();

        // Draw centered text
        let center = display.bounding_box().center();
        draw_text_with_background(
            &mut display,
            data.as_str(),
            center,
            text_style,
            color,
            Rgb565::BLACK,
        )
        .unwrap();

        delay.delay_millis(10);
    }
}

fn draw_smiley<T: DrawTarget<Color = Rgb565>>(display: &mut T) -> Result<(), T::Error> {
    // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 100), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 200), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw an upside down red triangle to represent a smiling mouth
    Triangle::new(
        Point::new(130, 140),
        Point::new(130, 200),
        Point::new(160, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(display)?;

    // Cover the top part of the mouth with a black triangle so it looks closed instead of open
    Triangle::new(
        Point::new(130, 150),
        Point::new(130, 190),
        Point::new(150, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(display)?;

    Ok(())
}

fn draw_text_with_background<T: DrawTarget<Color = Rgb565>>(
    framebuffer: &mut T,
    text: &str,
    position: Point,
    text_style: TextStyle,
    text_color: Rgb565,
    background_color: Rgb565,
) -> Result<(), T::Error> {
    let character_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(text_color)
        .background_color(background_color)
        .build();

    // Calculate the size of the text
    let text = Text::with_text_style(text, position, character_style, text_style);
    let text_area = text.bounding_box();

    // Draw the background
    Rectangle::new(position, text_area.size)
        .into_styled(PrimitiveStyle::with_fill(background_color))
        .draw(framebuffer)?;

    // Draw the text
    text.draw(framebuffer)?;

    // Return the bounding box
    // Added 22 width on the Region to accom0date larger numbers
    Ok(())
}
