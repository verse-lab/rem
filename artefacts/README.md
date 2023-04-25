# Adventure of a lifetime artefacts

## Install
To run our plugin, you would need to:  
1. install rust from https://www.rust-lang.org/tools/install.  
2. install swipl from https://www.swi-prolog.org/build/unix.html and please make sure it's available in `$PATH`.  
3. install the rust backends: `rem-controller, rem-borrower, rem-repairer` that are published on crates.io.
4. install either IntelliJ IDEA or Clion (must be version 2022.3.*).
5. install the IntelliJ Rust plugin distribution we wrote.

there are two objects here:  
1. [setup.sh](./setup.sh) which checks whether rust and swipl are installed, and then install the rust backend using cargo.
2. [intellij-rust-0.4.188.SNAPSHOT-223-dev.zip](./intellij-rust-0.4.188.SNAPSHOT-223-dev.zip) is our IntelliJ Rust plugin.

Simply run the setup with:
```bash
./setup.sh
```

Then install the plugin by opening IntelliJ IDEA/Clion and then search for `Install Plugin from Disk...`.  Navigate to this folder and select the distribution zip: [intellij-rust-0.4.188.SNAPSHOT-223-dev.zip](./intellij-rust-0.4.188.SNAPSHOT-223-dev.zip).  You can also follow a guide [here](https://www.jetbrains.com/help/idea/managing-plugins.html#install_plugin_from_disk).

In the [setup.sh](./setup.sh) script, the linker path is updated with:
```
export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH
```

Please add that to your `.bashrc` or other shell initializer.

## Extractions
