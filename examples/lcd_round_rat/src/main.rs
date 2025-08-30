//! Uses `Ratatui` to display a TUI on the rp2040 LCD 1.28 inch Touch
#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cst816s_device_driver::CST816S;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use gc9a01a_driver::{Orientation, GC9A01A};

use rp2040_panic_usb_boot as _;

use ratatui::Terminal;

use rp2040_hal::{
    self as hal,
    clocks::{init_clocks_and_plls, Clock},
    gpio::{
        bank0::{Gpio10, Gpio11, Gpio13, Gpio8, Gpio9},
        FunctionSio, FunctionSpi, Pin, PullDown, SioOutput,
    },
    pac::{self, SPI1},
    sio::Sio,
    spi::Enabled,
    watchdog::Watchdog,
    Spi,
};

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal::delay::DelayNs;

const LCD_WIDTH: u32 = 240;
const LCD_HEIGHT: u32 = 240;

extern crate alloc;
// Linked-List First Fit Heap allocator (feature = "llff")
use embedded_alloc::LlffHeap as Heap;
use mousefood::prelude::*;

mod app;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

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

#[rp2040_hal::entry]
fn main() -> ! {
    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        // We need a pretty big heap for ratatui. if the device reconnects as UF2, you probably hit this limit
        const HEAP_SIZE: usize = 100000;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    // info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Initialize LCD pins
    let lcd_dc = pins.gpio8.into_push_pull_output();
    let lcd_cs = pins.gpio9.into_push_pull_output();
    let lcd_clk = pins.gpio10.into_function::<hal::gpio::FunctionSpi>();
    let lcd_mosi = pins.gpio11.into_function::<hal::gpio::FunctionSpi>();
    let lcd_rst = pins
        .gpio13
        .into_push_pull_output_in_state(hal::gpio::PinState::High);
    let mut _lcd_bl = pins
        .gpio25
        .into_push_pull_output_in_state(hal::gpio::PinState::Low);

    // Set up the delay for the first core
    let sys_freq = clocks.system_clock.freq().to_Hz();
    let mut delay = Delay::new(core.SYST, sys_freq);
    let mut delay_wrapper = DelayWrapper::new(&mut delay);

    // Setup Touch Driver
    //
    // Set up the pins needed for the driver
    let sda_pin = pins.gpio6.reconfigure();
    let scl_pin = pins.gpio7.reconfigure();
    let touch_interrupt_pin = pins.gpio17.into_pull_up_input();
    // Setup reset pin for touch driver
    let touch_reset_pin = pins
        .gpio22
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
    touchpad.init_config().unwrap();

    // Initialize SPI
    let spi = hal::Spi::<_, _, _, 8>::new(pac.SPI1, (lcd_mosi, lcd_clk));
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        40.MHz(),
        embedded_hal::spi::MODE_0,
    );

    // Initialize the display
    let mut display = GC9A01A::new(spi, lcd_dc, lcd_cs, lcd_rst, false, LCD_WIDTH, LCD_HEIGHT);
    display.init(&mut timer).unwrap();
    display.set_orientation(&Orientation::Landscape).unwrap();

    // Clear the screen before turning on the backlight
    display.clear(Rgb565::BLACK).unwrap();
    timer.delay_ms(1000);

    // Turn the backlight on
    _lcd_bl.set_high().unwrap();

    let backend = EmbeddedBackend::new(&mut display, EmbeddedBackendConfig::default());
    let terminal = Terminal::new(backend);
    if let Ok(mut terminal) = terminal {
        let mut app = app::App::new(touchpad);
        loop {
            if let Err(_) = app.run(&mut terminal) {
                error_blink(&mut _lcd_bl, &mut timer.clone(), 500);
            }
        }
    } else {
        loop {
            error_blink(&mut _lcd_bl, &mut timer.clone(), 500);
        }
    }
}

pub type GC9A01ABackend<'a> = EmbeddedBackend<
    'a,
    GC9A01A<
        Spi<
            Enabled,
            SPI1,
            (
                Pin<Gpio11, FunctionSpi, PullDown>,
                Pin<Gpio10, FunctionSpi, PullDown>,
            ),
        >,
        Pin<Gpio8, FunctionSio<SioOutput>, PullDown>,
        Pin<Gpio9, FunctionSio<SioOutput>, PullDown>,
        Pin<Gpio13, FunctionSio<SioOutput>, PullDown>,
    >,
    Rgb565,
>;
pub type EmbeddedTerminal<'a> = Terminal<GC9A01ABackend<'a>>;

fn error_blink(
    led: &mut impl embedded_hal::digital::OutputPin,
    timer: &mut impl embedded_hal::delay::DelayNs,
    delay: u32,
) {
    led.set_high().unwrap();
    timer.delay_ms(delay);
    led.set_low().unwrap();
    timer.delay_ms(delay);
}
