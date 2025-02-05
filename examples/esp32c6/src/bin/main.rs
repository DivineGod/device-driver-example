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
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};
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
use gc9a01a_driver::{FrameBuffer, Orientation, Region, GC9A01A};
use heapless::String;

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

    let driver = spi::master::Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default()
            .with_frequency(40.MHz())
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso);

    // Initialize the display using SPI
    let mut display = GC9A01A::new(
        driver, lcd_dc, lcd_cs, lcd_rst, false, LCD_WIDTH, LCD_HEIGHT,
    );

    let mut delay = Delay::new();
    // We need to wrap the delay in a newtype to be able to implement embedded_hal::delay::DelayNs as this is required
    // for the display and the touch driver

    // Use the delay wrapper when initializing the display
    display.init(&mut delay).unwrap();

    // Set the orientation so the USB port is down and text is left-to-right
    display.set_orientation(&Orientation::Portrait).unwrap();

    // Using a frame buffer for managing drawing to the display will make the updates look a lot smoother
    // Allocate the buffer in main and pass it to the FrameBuffer
    let mut background_buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut background_framebuffer =
        FrameBuffer::new(&mut background_buffer, LCD_WIDTH, LCD_HEIGHT);
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut framebuffer = FrameBuffer::new(&mut buffer, LCD_WIDTH, LCD_HEIGHT);
    background_framebuffer.clear(Rgb565::BLACK);

    // Clear the screen before turning on the backlight
    display.clear_screen(Rgb565::BLACK.into_storage()).unwrap();
    delay.delay_millis(1); // Delay a little bit to avoid a screen flash
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

    /* End Touch Driver Setup */

    let mut character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CYAN);
    character_style.background_color = Some(Rgb565::BLACK);
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
                device::Gesture::NoGesture => Rgb565::WHITE,
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
        let text_bounding_region = draw_text_with_background(
            &mut framebuffer,
            data.as_str(),
            display.bounding_box().center(),
            text_style,
            color,
            Rgb565::BLACK,
        );
        display.store_region(text_bounding_region).unwrap();

        //Display the next set of regions.
        display.show_regions(framebuffer.get_buffer()).unwrap();
        //reset the display frame buffer from the background for the regions just displayed.
        framebuffer.copy_regions(background_framebuffer.get_buffer(), display.get_regions());
        //clear out the regions from the display so its ready to start again.
        display.clear_regions();
    }

    info!("Driver configured!");

    loop {
        info!("Hello world!");
        delay.delay_millis(500);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}

fn draw_text_with_background(
    framebuffer: &mut FrameBuffer,
    text: &str,
    position: Point,
    text_style: TextStyle,
    text_color: Rgb565,
    background_color: Rgb565,
) -> Region {
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
        .draw(framebuffer)
        .unwrap();

    // Draw the text
    text.draw(framebuffer).unwrap();

    // Return the bounding box
    // Added 22 width on the Region to accom0date larger numbers
    Region {
        x: text_area.top_left.x as u16,
        y: text_area.top_left.y as u16,
        width: text_area.size.width + 22,
        height: text_area.size.height * 2,
    }
}
