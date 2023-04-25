#!/bin/bash
rustc --version > /dev/null #make sure rust is installed, install from https://www.rust-lang.org/tools/install
swipl --version > /dev/null #make sure swipl is installed, install from https://www.swi-prolog.org/build/unix.html

# install the extracting backends
cargo install rem-controller rem-borrower rem-repairer
