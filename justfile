_default:
    just --list --unsorted --list-submodules

setup:
    #!/usr/bin/env bash
    set -euxo pipefail

    if ! grep -q alias .git/config ; then
        echo '
    [alias]
    	l = "!f() { git log --oneline --graph main^.. "$@" || true ; }; f"
    ' >> .git/config
    fi

    packages=(
        bc bison flex openssl-devel openssl-devel-engine make ncurses-devel gcc-aarch64-linux-gnu
        clangd lld bindgen-cli llvm elfutils-libelf-devel
    )
    sudo dnf install -y "${packages[@]}"
    rustup component add rust-src

    # rust-analyzer support
    make LLVM=1 ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rust-analyzer

    # clangd support (well, at least make it less confused)
    ./scripts/clang-tools/gen_compile_commands.py
    sed -i 's/-fconserve-stack//g' compile_commands.json
    sed -i 's/-fmin-function-alignment=8//g' compile_commands.json
    sed -i 's/-fno-allow-store-data-races//g' compile_commands.json

build:
    # config
    make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- bcm2711_defconfig
    ./scripts/config --set-str "LOCALVERSION" "-senk-devel"
    ./scripts/config --disable "MODVERSIONS" # required for rust support
    make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rust.config
    ./scripts/config --module  "VIDEO_DS90UB954"

    # TODO find a better way to select REGMAP_I2C as builtin.
    # Probably, our driver itself has to become builtin?
    ./scripts/config --enable EEPROM_AT24

    # build
    make -j{{ num_cpus() }} ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- Image modules dtbs

gen-rust-kernel-docs:
    make LLVM=1 ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rustdoc
    xdg-open Documentation/output/rust/rustdoc/kernel/index.html

mount sdX:
    mkdir --parents mnt/{boot,root}
    sudo mount /dev/{{ sdX }}1 mnt/boot
    sudo mount /dev/{{ sdX }}2 mnt/root

unmount sdX:
    sudo umount /dev/{{ sdX }}1
    sudo umount /dev/{{ sdX }}2

export KERNEL := "kernel8"

copy sdX:
    #!/usr/bin/env bash
    set -euxo pipefail

    just mount {{ sdX }}

    sudo env PATH=$PATH make -j{{ num_cpus() }} ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- INSTALL_MOD_PATH=mnt/root modules_install
    sudo cp arch/arm64/boot/Image mnt/boot/$KERNEL.img
    sudo cp arch/arm64/boot/dts/broadcom/*.dtb mnt/boot/
    sudo cp arch/arm64/boot/dts/overlays/*.dtb* mnt/boot/overlays/
    sudo cp arch/arm64/boot/dts/overlays/README mnt/boot/overlays/

    # automate some steps in:
    # https://github.com/InES-HPMM/FPD-LinkIII_ds90ub95x?tab=readme-ov-file#add-driver-sources-to-raspberrypi
    if ! grep -q ds90ub954 mnt/root/etc/modules-load.d/modules.conf ; then
        echo ds90ub954 | sudo tee -a mnt/root/etc/modules-load.d/modules.conf > /dev/null
    fi
    if ! grep -q ds90ub954 mnt/boot/config.txt ; then
        echo dtoverlay=ds90ub954 | sudo tee -a mnt/boot/config.txt > /dev/null
        echo dtoverlay=imx219 | sudo tee -a mnt/boot/config.txt > /dev/null
        echo core_freq_min=250 | sudo tee -a mnt/boot/config.txt > /dev/null
    fi

    echo '#!/usr/bin/env bash
    sudo rmmod imx219
    sudo modprobe imx219
    libcamera-vid --width 1920 --height 1080 -t 10000' \
        | sudo install --mode +rwx /dev/stdin mnt/root/usr/local/bin/dothething > /dev/null

    just unmount {{ sdX }}
