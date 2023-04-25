#!/bin/bash
rustc --version > /dev/null #make sure rust is installed, install from https://www.rust-lang.org/tools/install
swipl --version > /dev/null #make sure swipl is installed, install from https://www.swi-prolog.org/build/unix.html

# components needed for rustup to install the backend
rustup component add rust-src rustc-dev llvm-tools-preview

# add the rust nightly library to path
export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH

# install the extracting backends
cargo install rem-controller rem-borrower rem-repairer rustfmt
