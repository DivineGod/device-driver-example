#![cfg_attr(not(test), no_std)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    i2c::{I2c, SevenBitAddress},
};

device_driver::create_device!(
  device_name: Device,
  dsl: {
    config {
      type RegisterAddressType = u8;
      type BufferAddressType = u8;
    }
    register GestureId {
      type Access = RO;
      const ADDRESS = 0x01;
      const SIZE_BITS = 8;
      value: uint as try enum Gesture {
        NoGesture = 0x00,
        SlideUp = 0x01,
        SlideDown = 0x02,
        SlideLeft = 0x03,
        SlideRight = 0x04,
        SingleClick = 0x05,
        DoubleClick = 0x0B,
        LongPress = 0x0C,
      } = 0..8,
    },
    /// Number of fingers
    /// Zero or One
    register FingerNum {
      type Access = RO;
      const ADDRESS = 0x02;
      const SIZE_BITS = 8;
      value: uint = 0..1
    },
    /// 4 High bits of the 12bit x-position
    register XposH {
      type Access = RO;
      const ADDRESS = 0x03;
      const SIZE_BITS = 8;
      value: uint = 0..4,
    },
    /// 8 low bits of the 12bit x-position
    register XposL {
      type Access = RO;
      const ADDRESS = 0x04;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// 4 High bits of the 12bit y-position
    register YposH {
      type Access = RO;
      const ADDRESS = 0x05;
      const SIZE_BITS = 8;
      value: uint = 0..4,
    },
    /// 8 low bits of the 12bit y-position
    register YposL {
      type Access = RO;
      const ADDRESS = 0x06;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// 8 high bits of the 16bit BPC0 value
    register BPC0H {
      type Access = RO;
      const ADDRESS = 0xB0;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// 8 low bits of the 16bit BPC0 value
    register BPC0L {
      type Access = RO;
      const ADDRESS = 0xB1;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// 8 high bits of the 16bit BPC1 value
    register BPC1H {
      type Access = RO;
      const ADDRESS = 0xB2;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// 8 low bits of the 16bit BPC1 value
    register BPC1L {
      type Access = RO;
      const ADDRESS = 0xB3;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    register ChipId {
      type Access = RO;
      const ADDRESS = 0xA7;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    register ProjId {
      type Access = RO;
      const ADDRESS = 0xA8;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    register FwVersion {
      type Access = RO;
      const ADDRESS = 0xA9;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// Deep sleep command register (not entirely sure how someone got this one)
    /// Found referenced here: https://github.com/IniterWorker/cst816s/blob/master/src/command.rs#L87
    /// Send `0x03` to this register to enter deep sleep mode maybe?
    register DeepSleep {
      // type Access = WO;
      const ADDRESS = 0xE5;
      const SIZE_BITS = 8;
      const RESET_VALUE = 0x03;

      value: uint = 0..8,
    },
    /// Control which motion actions are enabled
    register MotionMask {
      const ADDRESS = 0xEC;
      const SIZE_BITS = 3;

      /// Enable Double Click Action
      EnDClick: bool = 0,
      /// Enable Continuous Up-Down Scrolling Action
      EnConUD: bool = 1,
      /// Enable Continuous Left-Right Scrolling Action
      EnConLR: bool = 2,
    },
    /// Interrupt low-pulse output width.
    /// Unit: 0.1ms
    /// Range: 1-200
    /// Default: 10
    register IrqPulseWidth {
      const ADDRESS = 0xED;
      const SIZE_BITS = 8;
      const RESET_VALUE = 10;

      value: uint = 0..8,
    },
    /// Normal quick-scanning period
    /// This value affects [`LpAutoWakeTime`] and [`AutoSleepTime`]
    /// Unit: 10ms
    /// Range: 1-30
    /// Default: 1
    register NorScanPer {
      const ADDRESS = 0xEE;
      const SIZE_BITS = 8;
      const RESET_VALUE = 1;

      value: uint = 0..8,
    },
    /// Gesture Detection sliding area angle control.
    /// Angle = tan(c) * 10 where c is the angle with respect to
    /// the position x-axis.
    register MotionSlAngle {
      const ADDRESS = 0xEF;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// High 8 bits of the reference value for low-power scanning channel 1
    register LpScanRaw1H {
      const ADDRESS = 0xF0;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// Low 8 bits of the reference value for low-power scanning channel 1
    register LpScanRaw1L {
      const ADDRESS = 0xF1;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// High 8 bits of the reference value for low-power scanning channel 2
    register LpScanRaw2H {
      const ADDRESS = 0xF2;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// Low 8 bits of the reference value for low-power scanning channel 2
    register LpScanRaw2L {
      const ADDRESS = 0xF3;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// Automatic recalibration period during low power mode.
    /// Unit: 1 minute
    /// Range: 1～5,
    /// Default: 5
    register LpAutoWakeTime {
      const ADDRESS = 0xF4;
      const SIZE_BITS = 3;
      const RESET_VALUE = 5;

      value: uint = 0..3,
    },
    /// Low power scanning wake-up threshold.
    /// The smaller it is, the more sensitive it is.
    /// Range: 1～255
    /// Default: 48
    register LpScanTH {
      const ADDRESS = 0xF5;
      const SIZE_BITS = 8;
      const RESET_VALUE = 48;

      value: uint = 0..8,
    },
    /// Low-power scanning range. The greater it is, the more sensitive
    /// and the more power consumption it is.
    /// Range: 0-3
    /// Default: 3
    register LpScanWin {
      const ADDRESS = 0xF6;
      const SIZE_BITS = 2;
      const RESET_VALUE = 3;

      value: uint = 0..2,
    },
    /// Low-power scanning frequency, the smaller it is, the more sensitive it is.
    /// Range: 1-255
    /// Default: 7
    register LpScanFreq {
      const ADDRESS = 0xF7;
      const SIZE_BITS = 8;
      const RESET_VALUE = 7;

      value: uint = 0..8,
    },
    /// Low-power scanning current. The smaller it is the more sensitive it is.
    /// Range: 1-255
    register LpScanIdac {
      const ADDRESS = 0xF8;
      const SIZE_BITS = 8;

      value: uint = 0..8,
    },
    /// Automatically enter low-power mode if there is no touch in x seconds
    /// Unit: 1 second
    /// Default: 2
    register AutoSleepTime {
      const ADDRESS = 0xF9;
      const SIZE_BITS = 8;
      const RESET_VALUE = 2;

      value: uint = 0..8,
    },
    /// Control when to pulse the interrupt pin low.
    /// [`EnTest`]: Interrupt pin test, automatically generates low pulses periodically after being enabled
    /// [`EnTouch`]: Generates low pulses when the touch is detected
    /// [`EnChange`]: Generates low pulses when the touch is changed
    /// [`EnMotion`]: Generates low pulses when gesture is detected
    /// [`OnceWLP`]: Only generates one low pulse when long press is detected
    register IrqCtl {
      const ADDRESS = 0xFA;
      const SIZE_BITS = 8;

      OnceWLP: bool = 0,
      EnMotion: bool = 4,
      EnChange: bool = 5,
      EnTouch: bool = 6,
      EnTest: bool = 7,
    },
    /// Automatically reset if there is touch but no valid gesture within x seconds
    /// Unit: 1s
    /// Disable: 0
    /// Range: 0-255
    register AutoReset {
      const ADDRESS = 0xFB;
      const SIZE_BITS = 8;
      const RESET_VALUE = 0;

      value: uint = 0..8,
    },
    /// Auto reset after long press x seconds
    /// Unit: 1s
    /// Disable: 0
    /// Default: 10
    register LongPressTime {
      const ADDRESS = 0xFC;
      const SIZE_BITS = 8;
      const RESET_VALUE = 10;

      value: uint = 0..8,
    },
    /// IO Control.
    /// [`SOFT_RST`]: The main controller achieves touch soft reset functionality by pulling down the IRQ pin
    ///   0: Disable soft reset
    ///   1: Enable soft reset
    /// [`IIC_OD`]: IIC pin driver mode, pull resistor by default.
    ///   0: pull up resistor
    ///   1: OD
    /// [`En1v8`]: IIC and IRQ pin level selection, VDD level by default.
    ///   0: VDD
    ///   1: 1.8V
    register IOCtl {
      const ADDRESS = 0xFD;
      const SIZE_BITS = 3;

      En1v8: bool = 0,
      IIC_OD: bool = 1,
      SOFT_RST: bool = 2,
    },
    /// Control automatic entry into low-power mode.
    /// 0: Default. Automatic low-power entry enabled
    /// non-0: Automatic low-power entry disabled
    register DisAutoSleep {
      const ADDRESS = 0xFE;
      const SIZE_BITS = 8;
      const RESET_VALUE = 0;

      value: uint = 0..8,
    },
  }
);

// const BLOB_BUF_LEN: usize = (10 * 6) + 3; // (MAX_TOUCH_CHANNELS * RAW_TOUCH_EVENT_LEN) + GESTURE_HEADER_LEN;
// const ONE_EVENT_LEN: usize = 6 + 3; // RAW_TOUCH_EVENT_LEN + GESTURE_HEADER_LEN

struct DeviceInterface<I2C> {
    device_address: SevenBitAddress,
    i2c: I2C,
}

impl<I2C> DeviceInterface<I2C> {
    pub(crate) const fn new(i2c: I2C, device_address: SevenBitAddress) -> Self {
        Self {
            i2c,
            device_address,
        }
    }
}

impl<I2C: embedded_hal::i2c::I2c> device_driver::RegisterInterface for DeviceInterface<I2C> {
    type Error = DeviceError<I2C::Error>;

    type AddressType = u8;

    fn write_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.i2c.write(self.device_address, &[address])?;
        self.i2c.write(self.device_address, data)?;
        Ok(())
    }

    fn read_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.i2c.write_read(self.device_address, &[address], data)?;
        Ok(())
    }
}

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
                gesture: Gesture::SingleClick,
            })
        } else {
            None
        }
    }
}

pub type Point = (u16, u16);

pub struct TouchEvent {
    pub point: Point,
    pub gesture: Gesture,
}

/// Low level interface error that wraps the I2C error
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct DeviceError<I2c>(pub I2c);

impl<I2c> From<I2c> for DeviceError<I2c> {
    fn from(value: I2c) -> Self {
        Self(value)
    }
}

impl<I2c> core::ops::Deref for DeviceError<I2c> {
    type Target = I2c;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I2c> core::ops::DerefMut for DeviceError<I2c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::eh1::i2c;
    use futures_test::test;

    #[test]
    async fn read_chip_id() {
        let mut i2c_device =
            i2c::Mock::new(&[i2c::Transaction::write_read(0x15, vec![0xA7], vec![0x23])]);
        let mut s2 = Device::new(DeviceInterface::new(&mut i2c_device, 0x15));

        let version = s2.chip_id().read().unwrap().value();

        println!("Version: {version:X}");
        assert_eq!(version, 0x23);

        i2c_device.done();
    }
}
