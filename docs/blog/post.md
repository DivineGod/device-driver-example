In this post, we take a look at developing a device driver in Rust using Dion Dokter’s [device driver crate][device-driver-crate]

Dion has very kindly put together a [book][device-driver-book] outlining the use of this crate. We will also reference
the [documentation for the crate][device-driver-docs] when it come to understanding the data types that are exposed.

The promise of `device-driver` is to take the boring part out of writing a low-level interface to a device.

```
  This post might not be directly aimed at beginners
  of Rust as some of the code incorporates concepts
  such as generics.
```

# Target device

The device we're going to implement a driver for is the Hynitron CST816S touch device. This is used on the
[Waveshare RP2040 Touch LCD 1.28 inch][ws-rp2040-t-lcd] and it will be the test device we're going to work with as well.

Digging deeper into this chip reveals that it's most likely going to be different from implementation to implementation
as it seems the supplier can load customer supplied configuration and maybe even modified firmwares to support a given
requirement. So any registers and functionality is not guaranteed to work even if the chip and overall use seem similar.

# Prior art

[Other][rust-driver-1] [libraries][rust-driver-2] in [rust][pinetime-rust-driver-blog] and
[other][adafruit-circuit-python-driver] [languages][cpp-driver-1] do exist for this device and I might reference their
implementations as I go through this implementation process to make sure that my implementation is at least as correct
if not better.

# The Journey

## Studying the device documentation

It's a bit light on information, but here's what we know from the [Waveshare Datasheet][ws-datasheet].

The chip is described as "High performance self-capacitance touch chip".

The high performance part refers to the chip supporting ">100Hz" touch reporting frequency in "Dynamic Mode".
It also has low power consumption for each of it's three modes: <1.6mA, <6.0uA, <1.0uA in Dynamic, Standby, and Sleep
mode respectively.

Note that in sleep-mode, the chip is effectively turned off and no touch events will be reported. In Standby mode, the
chip will scan for inputs much less frequently than in Dynamic mode. The chip can be "woken up", that is go from Standby
to Dynamic mode by specifying a touch gesture wake command.
Sleep mode can either be entered by the chip automatically by configuring the auto-sleep register values or by sending
an undocumented sleep-command (I'm still not sure if this command is real.) Exiting Sleep mode requires that the reset-
procedure be followed.

Self-capacitance refers to the way the chip implements the touch sensing part by have wires with current passing over
each-other, a finger near those wires affect the capacitance between the wires, which can be read by the chip. CST816S
supports up to 14 sensing channels (or 13 according to section 4.2).

For communication with the main processor, the chip implements I²C at rates from 10KHz-400KHz. Do note that the chip
will only respond to read and write requests on the bus just after a reset and then subsequently only after it has
received touch inputs. To help with this minor inconvenience the chip has an extra pin for communication:
The Interrupt Request (IRQ) pin. The chip will pull this pin low for an amount of time as configured by the
`IrqPulseWidth` register.

Resetting the chip to wake it from sleep mode or in general to put it into dynamic mode requires pull the reset-pin low
for a little bit, then setting it high again. The reset-circuit inside the chip has a pull-up resistor and filters to
make sure there aren't any spurious resets due to issues with floating voltage on the wire.


## Low-level driver

Now on to implementing the low-level driver. Here we mostly manually convert the register information from the
[Waveshare register documentation][ws-registers] into the DSL the `device-driver` crate needs to generate code.

### Driver DSL implementation

In this section we will go through a pretty thorough setup of the repository for our driver as well as going through
the DSL code we will write to convert the registers.

Firstly we will host the driver and example code in a cargo workspace to we create a new folder
with the following structure:

```bash
mkdir driver-workspace
cd driver-workspace
```

For `cargo` to understand that we're working in a repository with a workspace the top-level `Cargo.toml` will have to
contain a specific `workspace` section. We can populate it with only `resolver = "2"` since we're using 2021 edition but
workspace resolution defaults to version `1`.

```toml
[workspace]
resolver = "2"
```

Then we create a library crate for the device driver and a binary create for the example.

```bash
cargo new --lib driver
cargo new --bin example
```

This also automatically adds a new value to the top-level `Cargo.toml` for each of the members in the workspace

```toml
members = ["driver", "example"]
```

Navigate to the newly created library crate directory

```bash
cd driver
```

Install `device-driver` dependency as well as some other dependencies that are needed

```bash
cargo add device-driver
cargo add embedded-hal
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
    // Global config
    // Registers
    // Commands
    // Buffers
  }
);
```

The global config will have to contain the address types for register, buffer, or command access.

```rust
dsl: {
  config {
    type RegisterAddressType = u8;
  }
}
```

If we need to use buffers or commands we have to include the `BufferAddressType` and `CommandAddressType` respectively.

### Filling out the DSL from the datasheet

A fairly tedious and very manual task to copy the register definitions from the datasheet into the DSL for the macro.

For most of the registers they convert straight into integer values so they can be transcribed simply.
E.g. like this for the `ChipId` register.

 - Access Type is `RO` meaning Read Only.
 - Address `0xA7`
 - Value size in bits is `8`
 - The field set is a single entry with the name `value`, type uint, and the bits selected from the read is index 0 to 8

```rust
    register ChipId {
      type Access = RO;
      const ADDRESS = 0xA7;
      const SIZE_BITS = 8;
      value: uint = 0..8,
    },
```

For fields that we can write values to, we can leave out the `type Access` to opt-in to the default value or we could
specify `WO` for write-only access.

Some of the registers are a little more complex. The first register overall is actually best represented as an `enum`.
We see this because it will return an integer value that can be interpreted based off the table from the
register definition.

| Variant     | Value       |
| ----------- | ----------- |
| NoGesture   | 0x00        |
| SlideUp     | 0x01        |
| SlideDown   | 0x02        |
| SlideLeft   | 0x03        |
| SlideRight  | 0x04        |
| SingleClick | 0x05        |
| DoubleClick | 0x0B        |
| LongPress   | 0x0C        |

As we can see we aren't covering every single option in the range of values that could be given for an integer to be
converted to this enum, so we need to use a special invocation of the conversion.

`value: uing as try enum Gesture { ... } = 0..8` With this we tell `device-driver` that the 8 bits of the register value
should be used to reconstruct the enum variant needed. However, since we don't cover all options fully, the conversion
might fail so we use `as try enum` as opposed to `as enum`. `Gesture` is our chosen name for the enum type that gets
generated. The resulting register definition then looks like this:

```rust
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
```

Some fields are bit-fields, meaning the value needs to be translated into more than one flag type value. We do this
by defining more than one field in the field set list in the register. For instance, the `MotionMask` register will need
such a setup.

```rust
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
```


### Custom Conversion Types

`device-driver` supports converting the data in a field set to a custom type. We implement this for fields
that are `uint` by implementing either `From<u8>` or `TryFrom<u8>` for infallible or infallible conversions
respectively.

To explore this topic I've decided to implement conversion for the `IrqPulseWidth` register.

The DSL for this looks like the following

```rust
  register IrqPulseWidth {
    ...

    value: uint as PulseWidth = 0..8
  }
```

In this case we can safely implement the conversion as we know the value coming from the device should be from 1-200.

```rust
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
        dbg_assert!(value > 0);
        dbg_assert!(value <= 200);
        Self { value }
    }
}

impl From<PulseWidth> for u8 {
    fn from(value: PulseWidth) -> Self {
        *value
    }
}
```

So while we're developing out our final application and running debug builds. Rust will help us out upholding the
invariants of this particular register.


### Teaching the driver to speak I²C

We've generated the code for our low-level driver, but it doesn't know how to speak with the outside world. We have to
help it along, by defining a struct that implements a few traits that we can give to it.

The way we do this is to implement the `{Buffer,Command,Register}Interface` and/or `Async{Buffer,Command,Register}Interface` traits supplied to us by `device-driver`.

```rust
impl<BUS: embedded_hal::i2c::I2c> RegisterInterface for DeviceInterface<BUS> {
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
```

Here we have implemented the two provided methods for this trait.

For writing to a register we have to first write
the register address to the device, then we write out the data buffer and return `Ok(())`. We do this in a
transaction so the i2c physical protocol is upheld and the device can respond correctly.

For reading from a register we can use the `embedded_hal` provided `write_read` method. It'll deal with sending
the right data to the right address.


## High-level Driver

### Outlining the public interface

In theory, we could take the output that gets generated and use it directly in our own projects, however, this can both
be tedious and error-prone. Especially if some operations need to happen repeatedly and in a specific manner. Maybe some
operations need specific delays or certain pins need to be read before a command can be sent.

So we will define a public interface for our crate, that will wrap the lower-level code generated
for us by `device-driver`.

in `src/lib.rs`:
```rust
pub struct CST816S<I2C, TPINT, TPRST> {
    device: Device<DeviceInterface<I2C>>,
    interrupt_pin: TPINT,
    reset_pin: TPRST,
}
```

We have here a struct that has three generic parameters. As we will need to support a variety of different embedded
targets we need to be generic over the types that implement functionality. `I2C` for the communication protocol, `TPINT`
for the pin for touch interrupt, `TPRST` for the pin for resetting the chip.

We then implement methods for the struct

```rust
impl<I2C, TPINT, TPRST> CST816S<I2C, TPINT, TPRST>
where
    I2C: embedded_hal::i2c::I2c,
    TPINT: embedded_hal::digital::InputPin,
    TPRST: embedded_hal::digital::OutputPin,
{
    pub fn new(i2c: I2C, address: SevenBitAddress, interrupt_pin: TPINT, reset_pin: TPRST) -> Self { ... }

    pub fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), TPRST::Error> { ... }

    pub fn event(&mut self) -> Option<TouchEvent> { ... }
}
```

In the beginning of the `impl` block we say that the generic types `I2C`, `TPINT`, and `TPRST` must be types that
implement their respective `embedded_hal` traits.

The `new` associated function, invoked to setup the device interface initially, is fairly straightforward.

```rust
pub fn new(i2c: I2C, address: embedded_hal::i2c::SevenBitAddress, interrupt_pin: TPINT, reset_pin: TPRST) -> Self {
    Self {
        device: Device::new(DeviceInterface::new(i2c, address)),
        interrupt_pin,
        reset_pin,
    }
}
```

We need to take ownership of an instance of the communication interface. We also need to store the address for the
device, and the two pins. Then return an instance of the struct with a device created from the low-level driver
instantiated with the `DeviceInterface` we created to speak the communications protocol.

For the `reset` method, I referenced other implementation for the sequence of pin states and delays as this isn't
actually documented anywhere. It does seem to work here, so we're keeping it. It might be possible to tweak the delays
to waste less time in starting up the device.

```rust
pub fn reset(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) -> Result<(), TPRST::Error> {
    self.reset_pin.set_low()?;
    delay.delay_ms(20);
    self.reset_pin.set_high()?;
    delay.delay_ms(50);
    Ok(())
}
```

Note that we need to take a mutable reference to a type that implements the embedded hal trait `DelayNs` to be able to
do the delay. This function is blocking so nothing else will be running.

Reading an `event` from the device requires us to make sure that the interrupt pin is low. This is because it's the best
indicator that we're able to get a response on the I²C interface from the touch chip. We could also let the user setup
an interrupt handler for the falling edge of the interrupt pin.

```rust
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
```

We do several reads here to put together the data needed for a proper touch event to reported back to the user code.
Some slightly advanced error handling is also going on, I've tried to keep it fairly simple. The first line in the
method, reading the interrupt pin `is_low()` returns a `Result<bool, _>` style value. So we will match on this value
to ensure we only proceed if it is `Ok(true)`. Then we read the x-position, y-position, and gesture id. All three of
these reads also return Results, that can error if the protocol encounters a problem. We don't really care for the error
in this example, so we just do an early return if any of them returns true for `is_err()`. We can then unwrap each of
the returned results safely and extract the `value` field from the register field set.

Note that for `gesture` we need to unwrap the result of the `value()` call as we defined the conversion with
`as try enum`, which means it could fail if the value read from the chip isn't within the values required in the enum.
If we wanted to be extra safe, we could also handle this error case but in the interest of not getting to bogged down,
I'll leave this as an exercise for later.

# The Destination

We made a device driver! Huzzah! Now, there are several improvements that could be made. For instance, it's possible to
initialise the and then try to use it without knowing it's in the correct running state. With the
[typestate pattern][typestate] the state of the device can be encoded ensuring we use the device as intended.

Please navigate to the [driver and example source repository][driver-repo] to view the final implementation. I have also
included an example repository that utilises the driver on the target hardware to report the touch events on the display.

[device-driver-crate]: tab:https://crates.io/crates/device-driver
[device-driver-docs]: tab:https://docs.rs/device-driver/latest/device_driver/
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
[typestate]: tab:https://cliffle.com/blog/rust-typestate/
