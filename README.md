# Device Driver Example

This is the repository accompanying my [blog post](https://blog.mjolner.tech/device-driver-rust/) on implementing an embedded rust device driver using the [`device-driver` crate](https://crates.io/crates/device-driver).

## Driver Crate

In `driver/` we have the device driver we created in the post.

## Example

in `example/` we have a binary crate which uses the device driver and the waveshare lcd touch board with an rp2040.
