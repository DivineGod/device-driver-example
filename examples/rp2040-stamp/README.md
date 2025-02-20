# CST816S Device Driver Example for rp2040

This example runs on the [rp2040 based waveshare board which includes a round 1.28inch LCD Display and Touch](https://www.waveshare.com/wiki/RP2040-Touch-LCD-1.28)

The firmware showcases the use of the driver we created as an example of using the
[`device-driver` crate](https://crates.io/crates/device-driver)

It's also using a custom Board Support Crate (also known as BSP/board support package) for the waveshare rp2040
touch lcd 1.28inch board.

# Pre-requisites

This example uses stable rust, [`probe-rs`](https://probe.rs/) and the `thumbv6m-none-eabi` target.

## Rust
Make sure you have rust installed via [rustup](https://rustup.rs/) (or other distribution method as required by your OS of choice)

## probe-rs
Install `probe-rs` using the method outline on the [probe-rs website](https://probe.rs/)

## Cross compilations target

Finally make sure to have the correct cross-compilation target installed

```sh
rustup target add thumbv6m-none-eabi`
```

# Compilation and firmware upload.

To build and upload the firmware to the board press and hold the `BOOT` button on the back of the device while either plugging in the USB-C cable or pressing the `RESET` button while the board is plugged in to the USB cable.

Then you can run `cargo run --release` and probe-rs will automatically upload the compiled binary after a successful build.
