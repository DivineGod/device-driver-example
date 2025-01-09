#![cfg_attr(not(test), no_std)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    i2c::{I2c, SevenBitAddress},
};

pub mod device;
use device::{Device, DeviceInterface, PulseWidth};

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

    pub fn set_irq_pulse_width(&mut self, pulse_width: PulseWidth) {
        self.device
            .irq_pulse_width()
            .write(|write_object| write_object.set_value(pulse_width))
            .unwrap();
    }

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

pub type Point = (u16, u16);

pub struct TouchEvent {
    pub point: Point,
    pub gesture: device::Gesture,
}
