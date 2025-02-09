//! # Device Driver
//!
//! Device Driver Crate for CST816S
//!
//! # Examples
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    i2c::{I2c, SevenBitAddress},
};

pub mod device;
use device::{Device, DeviceError, DeviceInterface, PulseWidth};

/// Public interface struct for our High-level driver
pub struct CST816S<I2C, TPINT, TPRST> {
    device: Device<DeviceInterface<I2C>>,
    interrupt_pin: TPINT,
    reset_pin: TPRST,
}

impl<I2C, TPINT, TPRST> CST816S<I2C, TPINT, TPRST>
where
    I2C: I2c,
    TPINT: InputPin,
    TPRST: OutputPin,
{
    /// make a new instance, yeah!
    ///
    /// ```rust
    ///     let driver = CST816S::new(...);
    /// ```
    pub fn new(i2c: I2C, address: SevenBitAddress, interrupt_pin: TPINT, reset_pin: TPRST) -> Self {
        Self {
            device: Device::new(DeviceInterface::new(i2c, address)),
            interrupt_pin,
            reset_pin,
        }
    }

    /// Reset the device
    ///
    /// Make sure the device is in "dynamic mode" by pulling the reset pin low for 20ms, then setting it high again.
    pub fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), TPRST::Error> {
        self.reset_pin.set_high()?;
        delay.delay_ms(50);
        self.reset_pin.set_low()?;
        delay.delay_ms(5);
        self.reset_pin.set_high()?;
        delay.delay_ms(50);
        Ok(())
    }

    /// Set initial default config
    pub fn init_config(&mut self) -> Result<(), DeviceError<I2C::Error>> {
        self.device.irq_ctl().write(|irq_ctl| {
            irq_ctl.set_en_test(false);
            irq_ctl.set_en_touch(true);
            irq_ctl.set_once_wlp(true);
            irq_ctl.set_en_change(true);
            irq_ctl.set_en_motion(true);
        })?;
        self.device.motion_mask().write(|mask| {
            mask.set_en_d_click(true);
            mask.set_en_con_lr(true);
            mask.set_en_con_ud(true);
        })?;
        // self.device.motion_sl_angle().write(|m| m.set_value(0))?;
        // self.device.lp_scan_th().write(|m| m.set_value(48))?;
        // self.device.lp_scan_win().write(|m| m.set_value(3))?;
        // self.device.lp_scan_freq().write(|m| m.set_value(7))?;
        // self.device.lp_scan_idac().write(|m| m.set_value(1))?;
        // self.device.auto_reset().write(|m| m.set_value(5))?;
        self.device.dis_auto_sleep().write(|m| m.set_value(0xfe))?;
        self.device
            .irq_pulse_width()
            .write(|m| m.set_value(PulseWidth::new(1)))?;
        self.device.nor_scan_per().write(|m| m.set_value(1))?;
        return Ok(());
    }

    /// Read the ChipId register if the device is available for reads
    pub fn read_chip_id(&mut self) -> Option<u8> {
        let int_pin_value = self.interrupt_pin.is_low().unwrap();
        if int_pin_value {
            let result = self.device.chip_id().read().unwrap().value();
            Some(result)
        } else {
            None
        }
    }

    /// Set the IrqPulseWidth register.
    ///
    /// Allows you to set the time the interrupt pin is low.
    /// unit is 0.1ms and the range is 1-200. Default is 10
    pub fn set_irq_pulse_width(&mut self, pulse_width: PulseWidth) {
        self.device
            .irq_pulse_width()
            .write(|write_object| write_object.set_value(pulse_width))
            .unwrap();
    }

    /// Read a single event.
    ///
    /// Will return a [`TouchEvent`] struct if the device has a valid touch ready.
    pub fn event(&mut self) -> Option<TouchEvent> {
        if self.interrupt_pin.is_high().unwrap() {
            return None;
        }
        let x = self.device.xpos().read();
        let y = self.device.ypos().read();
        let b0 = self.device.bpc_0().read();
        let b1 = self.device.bpc_1().read();
        let gesture = self.device.gesture_id().read();
        if x.is_err() || y.is_err() || gesture.is_err() || b0.is_err() || b1.is_err() {
            return None;
        }
        let x = x.unwrap().value();
        let y = y.unwrap().value();
        let bpc0 = b0.unwrap().value();
        let bpc1 = b1.unwrap().value();
        let gesture = gesture.unwrap().value().unwrap();
        let point: Point = (x, y);

        Some(TouchEvent {
            point,
            bpc0,
            bpc1,
            gesture,
        })
    }
}

/// Named type `Point`. represent the point a touch was registered at.
pub type Point = (u16, u16);

/// `TouchEvent` struct contains the point and gesture of a received touch event.
pub struct TouchEvent {
    /// Where on the screen was the touch registered.
    pub point: Point,
    pub bpc0: u16,
    pub bpc1: u16,
    /// What type of gesture was registered,
    pub gesture: device::Gesture,
}
