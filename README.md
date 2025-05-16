# panometer

# Status

This is an extremely alpha project that I am developing for use in my own car projects and hope to generalize for use by other hotrodders someday soon!

## What is panometer?

This is an embedded platform for retrofitting automotive gauge clusters with stepper motors. It allows replacing the old mechanical gauge mechanisms with modern stepper motors, drivers, and a microcontroller. The gauges can be driven from CAN signals or IO pins.

## Why the name?

The Greek prefix 'pan-' meaning everything and the suffix '-meter' meaning to measure. Basically, this can drive your speedometer, tachometer, ammeter, etc.

## Architecture

### Hardware

The favored stepper motors are x27s from GM products. They're lightweight, fast, affordable, and have a lot of support.
https://www.adafruit.com/product/2424

The microcontroller architecture is the ESP32-S2, a WiFi enabled chip that's very common in the hobbyist electronics and IoT space. The built-in WiFi is useful for configuration (see Firmware) and the ESP32-Sx series chips have a great set of IO for the application.

### Firmware

The firmware for panometer is written in Rust. It consists of two parts: a hardware driver and a configuration server. The hardware driver that runs all the time and translates the CAN or discrete signals into stepper motor position commands. The configuration server is a WiFi network and webpage that is only active when modifications to the gauge setup need to be made.

# Implementing panometer

The intention is to provide three levels of abstraction:

1. The panometer project will develop a flexible firmware framework, suggested part numbers for stepper motors and chips, and reference PCB designs. The firmware framework will be composable to drive a semi-arbitrary number of stepper motors, indicator lights, backlighting, and inputs.
2. A vehicle model specific application will take the generics offered by panometer and specify it to the stock gauge cluster. This application pattern would include the hardware design required to modify the gauge cluster, the electrical architecture to suit the set of gauges, lights, and buttons in that cluster, and a firmware program matched to the hardware.
3. Lastly, the firmware will need to be configured for the modifications done to your vehicle. For example, if you've done an EFI upgrade you may need to connect the gauges to your CAN bus signals. Or wire the discrete inputs to new sensors installed on the vehicle and set up the transfer functions. This would be done without having to recompile the firmware but only configure it through the web interface.
