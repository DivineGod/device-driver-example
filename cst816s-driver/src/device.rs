use core::ops::{Deref, DerefMut};

use embedded_hal::i2c::{self, Operation, SevenBitAddress};

device_driver::create_device! {
  device_name: Device,
  dsl: {
    config {
      type RegisterAddressType = u8;
    }
    /// GestureID stores the type of gesture registered by the touch device
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
        /// Double Click registered. Registration can be controlled using the [`field_sets::MotionMask`] register.
        DoubleClick = 0x0B,
        /// Long Press detected. The time to register a long press is controlled by setting
        /// the [`field_sets::LongPressTime`] register.
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
      const ALLOW_ADDRESS_OVERLAP = true;
      value: uint = 0..4,
    },
    /// 8 low bits of the 12bit x-position
    register XposL {
      type Access = RO;
      const ADDRESS = 0x04;
      const SIZE_BITS = 8;
      const ALLOW_ADDRESS_OVERLAP = true;
      value: uint = 0..8,
    },
    /// X-coordinate for the touch event position.
    /// This is a "virtual" register in the sense that the documentation does
    /// specify it, but we combine the XposH and XposL registers automatically
    /// by reading 16 bits starting from the address of `XposH` then mapping
    /// the field into `value` by taking bit 0 to 12.
    register Xpos {
      type Access = RO;
      type ByteOrder = BE;
      const ADDRESS = 0x03;
      const ALLOW_ADDRESS_OVERLAP = true;
      const SIZE_BITS = 16;

      value: uint = 0..12,
    },
    /// 4 High bits of the 12bit y-position
    register YposH {
      type Access = RO;
      const ADDRESS = 0x05;
      const SIZE_BITS = 8;
      const ALLOW_ADDRESS_OVERLAP = true;
      value: uint = 0..4,
    },
    /// 8 low bits of the 12bit y-position
    register YposL {
      type Access = RO;
      const ADDRESS = 0x06;
      const SIZE_BITS = 8;
      const ALLOW_ADDRESS_OVERLAP = true;
      value: uint = 0..8,
    },
    /// Y-coordinate for the touch event position.
    /// This is a "virtual" register in the sense that the documentation does
    /// specify it, but we combine the YposH and YposL registers automatically
    /// by reading 16 bits starting from the address of `YposH` then mapping
    /// the field into `value` by taking bit 0 to 12.
    register Ypos {
      type Access = RO;
      type ByteOrder = BE;
      const ADDRESS = 0x05;
      const ALLOW_ADDRESS_OVERLAP = true;
      const SIZE_BITS = 16;

      value: uint = 0..12,
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
    /// ProjectId Register
    register ChipId {
      type Access = RO;
      const ADDRESS = 0xA7;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// ProjectId Register
    register ProjId {
      type Access = RO;
      const ADDRESS = 0xA8;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
    /// Firmware Version Register
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

      value: uint as crate::PulseWidth = 0..8,
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
}

pub(crate) struct DeviceInterface<I2C> {
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

impl<BUS: i2c::I2c> device_driver::RegisterInterface for DeviceInterface<BUS> {
    type Error = DeviceError<BUS::Error>;

    type AddressType = u8;

    fn write_register(
        &mut self,
        address: Self::AddressType,
        _size_bits: u32,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.i2c.transaction(
            self.device_address,
            &mut [Operation::Write(&[address]), Operation::Write(data)],
        )?;
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

#[derive(Debug)]
pub struct PulseWidth {
    value: u8,
}

impl PulseWidth {
    pub fn new(value: u8) -> Self {
        debug_assert!(value > 0);
        debug_assert!(value <= 200);
        Self { value }
    }
}

impl From<u8> for PulseWidth {
    fn from(value: u8) -> Self {
        assert!(value > 0);
        assert!(value <= 200);
        Self { value }
    }
}

impl From<PulseWidth> for u8 {
    fn from(value: PulseWidth) -> Self {
        *value
    }
}

impl Deref for PulseWidth {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for PulseWidth {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
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

    #[test]
    async fn read_xpos() {
        let mut i2c_device = i2c::Mock::new(&[
            i2c::Transaction::write_read(0x15, vec![0x03], vec![0x01]),
            i2c::Transaction::write_read(0x15, vec![0x04], vec![0x02]),
            i2c::Transaction::write_read(0x15, vec![0x03], vec![0x01, 0x02]),
        ]);
        let mut s2 = Device::new(DeviceInterface::new(&mut i2c_device, 0x15));

        let xh = s2.xpos_h().read().unwrap().value();
        let xl = s2.xpos_l().read().unwrap().value();
        let x = s2.xpos().read().unwrap().value();

        println!("xh: {xh:X}");
        println!("xl: {xl:X}");
        println!("x: {x:X}");
        assert_eq!(xh, 0x01);
        assert_eq!(xl, 0x02);
        assert_eq!(x, 0x0102);

        i2c_device.done();
    }
}
