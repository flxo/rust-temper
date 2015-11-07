[![Build Status](https://travis-ci.org/flxo/rust_temper.svg?branch=master)](http://travis-ci.org/flxo/rust_temper)

Rust Temper
===========

Simple tool to access the [RDing Tech](http://pcsensor.com/) temper USB temperature sensors written in [Rust](https://www.rust-lang.org/).
Currently only the [0c45::7401](http://web.archive.org/web/20090413071502/http://www.pcsensor.com/index.php?_a=viewProd&productId=15)
devices are supported, but adding support for 0130::0x660c should just be a simple copy and paste task.
The USB sequence is shamelessly borrowed from [rbtemper](https://github.com/elpeo/rbtemper) and implemented
using [libusb](https://crates.io/crates/libusblib).

This mini project is just a hack, done in order to learn Rust (and to monitor the temperature of the authors pepper growhouse).

# Build and Run

```sh
$ cargo build
[...]
$ sudo cargo run
     Running `target/debug/temper`
21.4375
```

# License 

MIT License (MIT). Copyright (c) 2015 Felix Obenhuber
