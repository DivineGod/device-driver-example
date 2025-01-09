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
use device::{Device, DeviceInterface, PulseWidth};

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
        self.reset_pin.set_low()?;
        delay.delay_ms(20);
        self.reset_pin.set_high()?;
        delay.delay_ms(50);
        Ok(())
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
        let int_pin_value = self.interrupt_pin.is_low();
        match int_pin_value {
            Ok(true) => {
                let x = self.device.xpos().read();
                let y = self.device.ypos().read();
                let gesture = self.device.gesture_id().read();
                if x.is_err() || y.is_err() || gesture.is_err() {
                    return None;
                }
                let x = x.unwrap().value();
                let y = y.unwrap().value();
                let gesture = gesture.unwrap().value().unwrap();
                let point: Point = (x, y);

                Some(TouchEvent { point, gesture })
            }
            _ => None,
        }
    }
}

/// Named type `Point`. represent the point a touch was registered at.
pub type Point = (u16, u16);

/// `TouchEvent` struct contains the point and gesture of a received touch event.
pub struct TouchEvent {
    /// Where on the screen was the touch registered.
    pub point: Point,
    /// What type of gesture was registered,
    pub gesture: device::Gesture,
}
