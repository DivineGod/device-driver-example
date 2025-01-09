//! Example of graphics on the LCD of the Waveshare RP2040-LCD-1.28
//!
//! Draws a red and green line with a blue rectangle.
//! After that it fills the screen line for line, at the end it starts over with
//! another colour, RED, GREEN and BLUE.
#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cst816s_device_driver::{device, CST816S};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder};
use embedded_hal::delay::DelayNs;
use fugit::RateExtU32;
use gc9a01a_driver::{FrameBuffer, Orientation, Region, GC9A01A};
use panic_halt as _;
use rp2040_hal::Timer;

use core::fmt::Write;
use heapless::String;

use waveshare_rp2040_touch_lcd_1_28::entry;
use waveshare_rp2040_touch_lcd_1_28::{
    hal::{
        self,
        clocks::{init_clocks_and_plls, Clock},
        pac,
        pio::PIOExt,
        watchdog::Watchdog,
        Sio,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

const LCD_WIDTH: u32 = 240;
const LCD_HEIGHT: u32 = 240;
// Define static buffers
const BUFFER_SIZE: usize = (LCD_WIDTH * LCD_HEIGHT * 2) as usize;
// 16 FPS  Is as fast as I can update the arrow smoothly so all frames are as fast as the slowest.
const DESIRED_FRAME_DURATION_US: u32 = 1_000_000 / 16;

pub struct DelayWrapper<'a> {
    delay: &'a mut Delay,
}

impl<'a> DelayWrapper<'a> {
    pub fn new(delay: &'a mut Delay) -> Self {
        DelayWrapper { delay }
    }
}

impl<'a> DelayNs for DelayWrapper<'a> {
    fn delay_ns(&mut self, ns: u32) {
        let us = (ns + 999) / 1000; // Convert nanoseconds to microseconds
        self.delay.delay_us(us); // Use microsecond delay
    }
}

/// Main entry point for the application
#[entry]
fn main() -> ! {
    // Take ownership of peripheral instances
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Initialize watchdog
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Initialize clocks and PLLs (Phase-locked loop)
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // Initialize SIO (Single Cycle I/O)
    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set up the delay for the first core
    let sys_freq = clocks.system_clock.freq().to_Hz();
    let mut delay = Delay::new(core.SYST, sys_freq);

    let (mut _pio, _sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // Initialize LCD pins
    let lcd_dc = pins.lcd_dc.into_push_pull_output();
    let lcd_cs = pins.lcd_cs.into_push_pull_output();
    let lcd_clk = pins.lcd_clk.into_function::<hal::gpio::FunctionSpi>();
    let lcd_mosi = pins.lcd_mosi.into_function::<hal::gpio::FunctionSpi>();
    let lcd_rst = pins
        .lcd_rst
        .into_push_pull_output_in_state(hal::gpio::PinState::High);
    let lcd_bl = pins
        .lcd_bl
        .into_push_pull_output_in_state(hal::gpio::PinState::Low);

    // Initialize SPI from the LCD pins
    let spi = hal::Spi::<_, _, _, 8>::new(pac.SPI1, (lcd_mosi, lcd_clk));
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        40.MHz(),
        embedded_hal::spi::MODE_0,
    );

    // Initialize the display using SPI
    let mut display = GC9A01A::new(spi, lcd_dc, lcd_cs, lcd_rst, false, LCD_WIDTH, LCD_HEIGHT);

    // We need to wrap the delay in a newtype to be able to implement embedded_hal::delay::DelayNs as this is required
    // for the display and the touch driver
    let mut delay_wrapper = DelayWrapper::new(&mut delay);

    // Use the delay wrapper when initializing the display
    display.init(&mut delay_wrapper).unwrap();

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
    delay_wrapper.delay_ms(1); // Delay a little bit to avoid a screen flash
    lcd_bl.into_push_pull_output_in_state(hal::gpio::PinState::High);

    // Setup Touch Driver
    //
    // Set up the pins needed for the driver
    let sda_pin = pins.i2c1_sda.reconfigure();
    let scl_pin = pins.i2c1_scl.reconfigure();
    let touch_interrupt_pin = pins.tp_int.into_pull_up_input();
    // Setup reset pin for touch driver
    let touch_reset_pin = pins
        .tp_rst
        .into_push_pull_output_in_state(hal::gpio::PinState::High);

    // Create the I²C driver, using the two pre-configured pins.
    // This will fail  at compile time if the pins are in the wrong mode, or if
    // this I²C peripheral isn't available on these pins!
    let i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut touchpad = CST816S::new(i2c, 0x15, touch_interrupt_pin, touch_reset_pin);

    // Setup Touch Driver
    touchpad.reset(&mut delay_wrapper).unwrap();

    /* End Touch Driver Setup */

    let mut character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CYAN);
    character_style.background_color = Some(Rgb565::BLACK);
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    // Initialize the timer for frame time counting
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let mut last_touch = (0, 0);
    let mut color = Rgb565::CSS_NAVAJO_WHITE;

    loop {
        let start_ticks = timer.get_counter_low();

        // Read a touch event from the touch driver and update last_touch if there is a valid event
        if let Some(touch_event) = touchpad.event() {
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

        // Ensure each frame takes the exact same amount of time
        let end_ticks = timer.get_counter_low();
        let frame_ticks = end_ticks - start_ticks;
        if frame_ticks < DESIRED_FRAME_DURATION_US {
            delay_wrapper.delay_us(DESIRED_FRAME_DURATION_US - frame_ticks);
        }
    }
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
