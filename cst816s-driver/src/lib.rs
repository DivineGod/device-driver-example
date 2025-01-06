#![cfg_attr(not(test), no_std)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    i2c::{I2c, SevenBitAddress},
};

mod device;
use device::{Device, DeviceInterface};

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
    pub fn new(i2c: I2C, address: SevenBitAddress, interrupt_pin: TPINT, reset_pin: TPRST) -> Self {
        Self {
            device: Device::new(DeviceInterface::new(i2c, address)),
            interrupt_pin,
            reset_pin,
        }
    }

    pub fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), TPRST::Error> {
        self.reset_pin.set_low()?;
        delay.delay_ms(20);
        self.reset_pin.set_high()?;
        delay.delay_ms(50);
        Ok(())
    }

    pub fn read_chip_id(&mut self) -> Option<u8> {
        let int_pin_value = self.interrupt_pin.is_low().unwrap();
        if int_pin_value {
            let result = self.device.chip_id().read().unwrap().value();
            Some(result)
        } else {
            None
        }
    }

    pub fn event(&mut self) -> Option<TouchEvent> {
        let int_pin_value = self.interrupt_pin.is_low().unwrap();
        if int_pin_value {
            let xh = self.device.xpos_h().read().unwrap().value();
            let xl = self.device.xpos_l().read().unwrap().value();
            let yh = self.device.ypos_h().read().unwrap().value();
            let yl = self.device.ypos_l().read().unwrap().value();
            let x: u16 = ((xh as u16) << 2) | xl as u16;
            let y: u16 = ((yh as u16) << 2) | yl as u16;
            Some(TouchEvent {
                point: (x, y),
                gesture: device::Gesture::SingleClick,
            })
        } else {
            None
        }
    }
}

pub type Point = (u16, u16);

pub struct TouchEvent {
    pub point: Point,
    pub gesture: device::Gesture,
}
