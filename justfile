_default:
    just --list --unsorted --list-submodules

setup:
    #!/usr/bin/env bash
    set -euxo pipefail

    # sd 'refs/heads/\w*:refs/remotes/origin/\w*' \
    #     'refs/heads/*:refs/remotes/origin/*' \
    #     .git/config
    # if ! grep -q alias .git/config ; then
    #     echo '
    # [alias]
    #     l = "!f() { git log --oneline --graph rust-next^.. "$@" || true ; }; f"
    # ' >> .git/config
    # fi
    # git fetch --depth 24
    # if ! git branch | grep rust-next > /dev/null ; then
    #     git branch --track rust-next origin/rust-next
    # fi
    # if ! test -d .jj ; then
    #     jj git init --colocate
    # fi
    jj bookmark track 'glob:*@origin'

    packages=(
        bc bison flex openssl-devel openssl-devel-engine make ncurses-devel gcc-aarch64-linux-gnu
        clangd lld bindgen-cli llvm elfutils-libelf-devel llvm
        perl-open python3-GitPython # get_maintainers.pl
    )
    sudo dnf install -y "${packages[@]}"
    rustup override set 1.78
    rustup component add rust-src rust-analyzer

    just config
    just _gen-lsp-support
    just config

add-remotes:
    jj git remote add driver-core https://git.kernel.org/pub/scm/linux/kernel/git/driver-core/driver-core.git
    jj git remote add rpi         github:raspberrypi/linux

_gen-lsp-support:
    yes "" | make LLVM=1 ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rust-analyzer
    # clangd support (well, at least make it less confused)
    ./scripts/clang-tools/gen_compile_commands.py
    sed -i 's/-fconserve-stack//g' compile_commands.json
    sed -i 's/-fmin-function-alignment=8//g' compile_commands.json
    sed -i 's/-fno-allow-store-data-races//g' compile_commands.json

config:
    # config
    make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- bcm2711_defconfig
    ./scripts/config --set-str "LOCALVERSION" "-senk-devel"
    ./scripts/config --disable "MODVERSIONS" # required for rust support
    make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rust.config
    ./scripts/config --module  "VIDEO_DS90UB954"

    # TODO find a better way to select REGMAP_I2C as builtin.
    # Probably, our driver itself has to become builtin?
    ./scripts/config --enable EEPROM_AT24

    # ensure samples for Rust APIs compile
    ./scripts/config --enable "SAMPLES"
    ./scripts/config --enable "SAMPLES_RUST"
    ./scripts/config --enable "SAMPLE_RUST_DRIVER_PLATFORM"

    # ensure doc examples compile
    ./scripts/config --enable "KUNIT"
    ./scripts/config --enable "CONFIG_RUST_KERNEL_DOCTESTS"

    # test updated user of property_present
    ./scripts/config --enable CPUFREQ_DT_RUST

    yes "" | make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- oldconfig # pick defaults for rest

build:
    make -j{{ num_cpus() }} CLIPPY=1 ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- Image modules dtbs

build-ds90ub954:
    make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- drivers/media/i2c/ds90ub954.o

gen-rust-kernel-docs:
    yes "" | make LLVM=1 ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- rustdoc
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

format-patch:
    rm *.patch || true
    git format-patch -v{{v}} --cover-letter main..property-end
    sd --fixed-strings '*** SUBJECT HERE ***' "$(jj show -T description cover-letter-property | head -n1)" v{{v}}-0000-*.patch
    sd --fixed-strings '*** BLURB HERE ***' "$(jj show -T description cover-letter-property | tail -n +3)" v{{v}}-0000-*.patch

check-patch: format-patch
    ./scripts/checkpatch.pl *.patch

get-maintainers: format-patch
    rm v{{v}}-0000-*.patch
    ./scripts/get_maintainer.pl *.patch

to := """
Rob Herring <robh@kernel.org>
Saravana Kannan <saravanak@google.com>
Greg Kroah-Hartman <gregkh@linuxfoundation.org>
"Rafael J. Wysocki" <rafael@kernel.org>
Danilo Krummrich <dakr@kernel.org>
Miguel Ojeda <ojeda@kernel.org>
Alex Gaynor <alex.gaynor@gmail.com>
Boqun Feng <boqun.feng@gmail.com>
Gary Guo <gary@garyguo.net>
"Björn Roy Baron" <bjorn3_gh@protonmail.com>
Benno Lossin <lossin@kernel.org>
Andreas Hindborg <a.hindborg@kernel.org>
Alice Ryhl <aliceryhl@google.com>
Trevor Gross <tmgross@umich.edu>
Mark Brown <broonie@kernel.org>
Dirk Behme <dirk.behme@de.bosch.com>
Remo Senekowitsch <remo@buenzli.dev>
"""

cc := """
devicetree@vger.kernel.org
linux-kernel@vger.kernel.org
rust-for-linux@vger.kernel.org
"""

send-email: format-patch check-patch
    #!/usr/bin/env bash
    set -euxo pipefail

    git send-email \
        --thread \
        --no-chain-reply-to \
        --suppress-cc=all \
        --to="$(echo -n '{{ to }}' | sd '\n' ',' | sd ',$' '')" \
        --cc="$(echo -n '{{ cc }}' | sd '\n' ',' | sd ',$' '')" \
        *.patch \
        --dry-run

v := "2"
