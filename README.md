# mb2-lsm-gpio: demonstrate MB2 LSM303AGR interrupts
Bart Massey 2024

This code shows how to take a LSM303AGR IMU interrupt on the
MicroBit v2.

Part of it is pretty straightforward. The LSM303AGR IMU has an
interrupt output that is connected to a GPIO pin on the
MB2. Enabling that interrupt allows the IMU to notify the
MB2 when it should pay attention.

However, this interrupt pin is shared by the Interface MCU (IMCU)
on the MB2. At program startup

## Build and Run

## Acknowledgements



Thanks to the `microbit-v2` and `lsm303agr` crate authors
for fantastic work.
