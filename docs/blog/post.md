In this post, we take a look at developing a device driver in Rust using Dion Dokter’s [device driver crate][device-driver-crate]

Dion has very kindly put together a [book][device-driver-book] outlining the use of this crate.

The purpose of device-driver is to take the boring part out of writing a low-level interface to a device.o

```
  This post might not be directly aimed at beginners
  of Rust as some of the code incorporates concepts
  such as generics.
```

## Target device

The device we're going to implement a driver for is the Hynitron CST816S touch device. This is used on the
[Waveshare RP2040 Touch LCD 1.28 inch][ws-rp2040-t-lcd] and it will be the test device we're going to work with as well.

Digging deeper into this chip reveals that it's most likely going to be different from implementation to implementation
as it seems the supplier can load customer supplied configuration and maybe even modified firmwares to support a given
requirement. So any registers and functionality is not guaranteed to work even if the chip and overall use seem similar.

## Prior art

[Other][rust-driver-1] [libraries][rust-driver-2] in [rust][pinetime-rust-driver-blog] and
[other][adafruit-circuit-python-driver] [languages][cpp-driver-1] do exist for this device and I might reference their
implementations as I go through this implementation process to make sure that my implementation is at least as correct
if not better.

## The Journey

### Studying the device documentation

It's a bit light on information, but here's what we know.

[Waveshare Datasheet][ws-datasheet]

[Waveshare register information][ws-registers]

The device uses I²C for communications, as well as an interrupt pin for signalling that data is ready for collection.

#### Registers and Commands

Many registers

Sleep Command - how does it work? No one knows really.

### Outlining the public interface

Before I even consider implementing the low-level driver interface using the device-driver crate, we need to consider
how we want to use this from our own code.

In theory, we could take the output that gets generated and use it directly in our own projects, however, this can both
be tedious and error-prone. Especially if some operations need to happen repeatedly and in a specific manner. Maybe some
operations need specific delays or certain pins need to be read before a command can be sent.

So we will define a public interface for our crate, that will wrap the lower-level code generated
for us by `device-driver`.

```rust
struct CST816S {
  ...
}

impl CST816S {
  pub fn init() -> Self {
    todo!()
  }

  pub fn next_event(&self) -> Option<TouchEvent> {
    todo!()
  }
}

pub struct TouchEvent {
  position: Point,
  gesture: Gesture,
  finger_number: u32,
}

pub type Point = (u32, u32);

pub enum Gesture {
  ...
}
```

### Building the driver

Create a repository for the device driver

```bash
cargo new --lib cst816s-device-driver
```

Navigate to the newly created directory

```bash
cd cst816-device-driver
```

Install `device-driver` dependency

```bash
cargo add device-driver
```

Clean out the contents of `src/lib.rs` so we can start fresh.

To develop a driver the device-driver crate provides a macro that lets us specify everything using a DSL or a manifest
file in a variety of languages (JSON, Yaml, TOML, DSL)

I'm going to be using the macro for educational purposes as any errors encountered will surface through the use of
rust-analyzer before we even compile the project.

In `src/lib.rs`:
```rust
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
  }
);
```

### Filling out the DSL from the datasheet

A fairly tedious and manual task to copy the register definitions from the datasheet into the DSL for the macro.

We did run into some quirks of the device that are worth mentioning. For the `GestureId` we would like to convert the
returned unsigned integer value to a Rust Enum. This is possible with the field in the register defined in the
following way.

```rust
device_driver::create_device!(
  device_name: Cst816SDeviceDriver,
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
  }
}
```

Note that we are using `uint as try enum Gesture { ... } = 0..8` since each variant in the enum isn't covering the full
range of values a single byte could hold.

The first several registers are read-only registers and have their access permission specified with the
`type Access = RO;` line in their definition. The default behaviour for a register is read-write permissions so for the
ones that we can also write to, we don't need to specify this type.

### Custom Conversion Types

What if we could have types that we convert into that does validation for us.

/// TODO : Figure out this section with IrqPulseWidth register definition

### Teaching the driver to speech I²C

We've generated the code for our low-level driver, but it doesn't know how to speak with the outside world. We have to
help it along, by defining a struct that implements a few traits that we can give to it.

The way we do this is to implement the `RegisterInterface` trait supplied to us by `device-driver`.

```rust

```

### High-level Driver

But how do you use this new driver? You might very appropriately ask. I'm glad you did, or I assume you did, or whatever.

Let's create a binary project that we can use as an example for the use of the touch driver on actual hardware.

```bash
cargo new touch-example
```

```rust
/// I2C setup
        .into_function::<hal::gpio::FunctionI2c>() // Type return here is Pin<Gpio6, FunctionI2c, PullDown>
        .into_pull_type::<PullUp>(); // `hal::I2C::i2c1` expects `PullUp` instead of `PullDown`

/// OR
        .reconfigure() // simpler
```


## The Destination

We made a device driver! Huzzah!

Please navigate to the [driver and example source repository][driver-repo] to view the final implementation.

[device-driver-crate]: tab:https://crates.io/crates/device-driver
[device-driver-book]: tab:https://diondokter.github.io/device-driver/
[ws-rp2040-t-lcd]: tab:https://www.waveshare.com/wiki/RP2040-Touch-LCD-1.28#Application_Demo
[rust-driver-1]: tab:https://github.com/tstellanova/cst816s
[rust-driver-2]: tab:https://github.com/IniterWorker/cst816s
[cpp-driver-1]: tab:https://github.com/fbiego/CST816S
[ws-datasheet]: tab:https://files.waveshare.com/upload/5/51/CST816S_Datasheet_EN.pdf
[ws-registers]: tab:https://files.waveshare.com/upload/c/c2/CST816S_register_declaration.pdf
[pinetime-rust-driver-blog]: tab:https://www.pcbway.com/blog/Activities/Building_a_Rust_Driver_for_PineTime_s_Touch_Controller.html
[adafruit-circuit-python-driver]: tab:https://github.com/adafruit/Adafruit_CircuitPython_CST8XX/blob/main/adafruit_cst8xx.py
[driver-repo]: tab:https://github.com/DivineGod/device-driver-example
