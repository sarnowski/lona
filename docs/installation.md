# Installing Lona OS

This guide explains how to install Lona OS on physical hardware. Lona currently supports two platforms out-of-the-box:

- **x86_64**: Standard PCs and servers with UEFI firmware
- **Raspberry Pi 4B**: 4GB and 8GB RAM variants

## Prerequisites

Before building, ensure you have:

- Docker installed and running
- GNU Make 4.0+ (on macOS: `brew install make`, then use `gmake`)
- A USB flash drive (for x86_64) or microSD card (for Raspberry Pi 4B)

## Building Release Images

### For x86_64 (PCs and Servers)

```bash
make release-x86_64
```

This creates a UEFI-bootable bundle in `build/release-x86_64/` containing:

```
build/release-x86_64/
├── EFI/
│   └── BOOT/
│       └── BOOTX64.EFI      # GRUB bootloader
└── lona/
    ├── grub.cfg             # Boot configuration
    ├── kernel-x86_64.elf    # seL4 kernel
    └── lona-x86_64.elf      # Lona application
```

### For Raspberry Pi 4B

```bash
# For 8GB model (default)
make release-rpi4b

# For specific RAM variants
make release-rpi4b-8gb
make release-rpi4b-4gb
```

This creates an SD card image in `build/release-rpi4b/` containing:

```
build/release-rpi4b/
├── start4.elf               # GPU firmware
├── fixup4.dat               # GPU memory config
├── bcm2711-rpi-4-b.dtb      # Device tree
├── overlays/                # Device tree overlays
├── config.txt               # Pi boot configuration
├── u-boot.bin               # U-Boot bootloader
├── boot.scr                 # Auto-boot script
└── lona/
    └── lona-rpi4b.elf       # Lona image
```

## Installing on x86_64 (USB Drive)

The USB drive must be formatted as **FAT32** with a **GPT** partition scheme.

### Option A: Command Line (Linux)

```bash
# Identify your USB drive (BE CAREFUL - wrong device = data loss!)
lsblk

# Assuming your USB drive is /dev/sdX (replace X with actual letter)
# Create a GPT partition table with an EFI System Partition
sudo parted /dev/sdX mklabel gpt
sudo parted /dev/sdX mkpart primary fat32 1MiB 100%
sudo parted /dev/sdX set 1 esp on

# Format as FAT32
sudo mkfs.fat -F32 /dev/sdX1

# Mount and copy files
sudo mount /dev/sdX1 /mnt
sudo cp -r build/release-x86_64/* /mnt/
sudo umount /mnt
```

### Option B: Command Line (macOS)

```bash
# List disks to find your USB drive
diskutil list

# Assuming your USB drive is /dev/diskN (replace N with actual number)
# Format as FAT32 with GPT
diskutil eraseDisk FAT32 LONA GPT /dev/diskN

# Copy files
cp -r build/release-x86_64/* /Volumes/LONA/
```

### Option C: GUI (macOS)

1. Insert your USB drive
2. Open **Disk Utility** (Applications > Utilities > Disk Utility)
3. Select your USB drive in the sidebar (select the drive, not the partition)
4. Click **Erase**
5. Configure:
   - Name: `LONA`
   - Format: `MS-DOS (FAT)`
   - Scheme: `GUID Partition Map`
6. Click **Erase**
7. Open the `build/release-x86_64/` folder in Finder
8. Select all files and folders (`EFI` and `lona`)
9. Drag and drop them to the `LONA` drive in Finder

### Option D: GUI (Windows)

1. Insert your USB drive
2. Open **Disk Management** (right-click Start > Disk Management)
3. Delete all partitions on the USB drive
4. Create a new simple volume:
   - Use all available space
   - Assign a drive letter
   - Format as **FAT32** (if over 32GB, use a third-party tool like Rufus)
5. Open `build\release-x86_64\` in File Explorer
6. Select all files and folders
7. Copy (Ctrl+C) and paste (Ctrl+V) to the USB drive

### Booting x86_64

1. Insert the USB drive into the target machine
2. Enter the UEFI boot menu (usually F12, F2, or Del during POST)
3. Select the USB drive from the boot menu
4. Lona should boot automatically after a 3-second timeout

## Installing on Raspberry Pi 4B (SD Card)

The SD card must be formatted as **FAT32**.

### Option A: Command Line (Linux)

```bash
# Identify your SD card
lsblk

# Assuming your SD card is /dev/mmcblkX or /dev/sdX
# Create a single FAT32 partition
sudo parted /dev/mmcblk0 mklabel msdos
sudo parted /dev/mmcblk0 mkpart primary fat32 1MiB 100%

# Format as FAT32
sudo mkfs.fat -F32 /dev/mmcblk0p1

# Mount and copy files
sudo mount /dev/mmcblk0p1 /mnt
sudo cp -r build/release-rpi4b/* /mnt/
sudo umount /mnt
```

### Option B: Command Line (macOS)

```bash
# List disks to find your SD card
diskutil list

# Format as FAT32
diskutil eraseDisk FAT32 LONA MBR /dev/diskN

# Copy files
cp -r build/release-rpi4b/* /Volumes/LONA/
```

### Option C: GUI (macOS)

1. Insert your SD card
2. Open **Disk Utility**
3. Select the SD card
4. Click **Erase**
5. Configure:
   - Name: `LONA`
   - Format: `MS-DOS (FAT)`
   - Scheme: `Master Boot Record`
6. Click **Erase**
7. Open `build/release-rpi4b/` in Finder
8. Select **all files and folders**
9. Drag and drop to the `LONA` SD card

### Option D: GUI (Windows)

1. Insert your SD card
2. Open File Explorer
3. Right-click the SD card > **Format**
4. Configure:
   - File system: `FAT32`
   - Allocation unit size: `Default`
5. Click **Start**
6. Copy all contents of `build\release-rpi4b\` to the SD card

### Booting Raspberry Pi 4B

1. Insert the SD card into the Raspberry Pi 4B
2. Connect a serial console (optional but recommended):
   - Use a USB-to-serial adapter (3.3V logic level)
   - Connect TX to GPIO 15 (pin 10)
   - Connect RX to GPIO 14 (pin 8)
   - Connect GND to any ground pin (e.g., pin 6)
   - Serial settings: 115200 baud, 8N1
3. Power on the Raspberry Pi
4. Lona will boot automatically

## Troubleshooting

### x86_64: Machine doesn't see the USB drive

- Ensure the USB drive is formatted as FAT32 (not exFAT or NTFS)
- Ensure the partition scheme is GPT (not MBR) for UEFI
- Try a different USB port (USB 2.0 ports are more compatible)
- Check that Secure Boot is disabled in UEFI settings

### x86_64: GRUB appears but Lona doesn't boot

- Check that all files are in the correct locations
- Verify the `lona/grub.cfg` file exists and is readable
- Try the serial console menu option to see boot messages

### Raspberry Pi 4B: No output on serial console

- Verify serial cable connections (TX/RX may need swapping)
- Ensure `enable_uart=1` is in `config.txt`
- Check that `dtoverlay=disable-bt` is present
- Try a different serial adapter (some have inverted logic)

### Raspberry Pi 4B: Boots to U-Boot but stops

- Press any key to interrupt U-Boot auto-boot
- Manually load and boot:
  ```
  fatload mmc 0 0x10000000 lona/lona-rpi4b.elf
  bootelf 0x10000000
  ```
- If this works, regenerate `boot.scr`

### Raspberry Pi 4B: Wrong RAM variant

If you built for a different RAM size than your Pi has:
- Rebuild with the correct variant: `make release-rpi4b-4gb`
- Building for a smaller RAM size than available is safe (just wastes RAM)
- Building for a larger RAM size than available will fail to boot

## Serial Console Access

Both platforms output to a serial console, which is useful for debugging.

### x86_64

Most servers have serial ports. Connect at 115200 baud, 8N1.
Alternatively, some servers support Serial-over-LAN (SOL) via IPMI/BMC.

### Raspberry Pi 4B

Use GPIO pins 14 (TX) and 15 (RX) at 3.3V logic levels:

```
Pi GPIO Header:
  Pin 6  - GND (black wire)
  Pin 8  - GPIO 14 / UART TX (green wire - connect to adapter RX)
  Pin 10 - GPIO 15 / UART RX (white wire - connect to adapter TX)
```

Serial settings: 115200 baud, 8 data bits, no parity, 1 stop bit (8N1).

Common serial terminal programs:
- Linux: `screen /dev/ttyUSB0 115200` or `minicom -D /dev/ttyUSB0 -b 115200`
- macOS: `screen /dev/tty.usbserial-* 115200`
- Windows: PuTTY or TeraTerm with the appropriate COM port

---

## Appendix: Adding Support for Other ARM Boards

Lona can be ported to other ARM boards that seL4 supports. This appendix provides a high-level overview of what's involved.

### Prerequisites

Before starting, verify that:

1. **seL4 supports your board** - Check the [seL4 supported platforms](https://docs.sel4.systems/Hardware/) list
2. **You have documentation** - Board schematic, memory map, and peripheral addresses
3. **You have a serial console** - Essential for debugging early boot issues

### Step 1: Gather Board Information

Collect the following details about your target board:

| Information | Example (RPi4B) |
|-------------|-----------------|
| seL4 platform name | `rpi4` |
| Architecture | `aarch64` |
| CPU cores | 4 (Cortex-A72) |
| RAM size(s) | 4GB, 8GB |
| Boot method | GPU firmware → U-Boot → seL4 |
| Serial UART | PL011 on GPIO 14/15 |
| Device tree | `bcm2711-rpi-4-b.dtb` |

### Step 2: Build seL4 for Your Board

Add the seL4 build to the Docker image. In `docker/Dockerfile.aarch64`:

```dockerfile
# Build seL4 for your-board
RUN mkdir -p build-your-board && cd build-your-board && \
    cmake -G Ninja \
        -DCMAKE_INSTALL_PREFIX=/opt/seL4/your-board \
        -DCMAKE_TOOLCHAIN_FILE=../gcc.cmake \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=your-platform-name \
        -DKernelSel4Arch=aarch64 \
        -DKernelMaxNumNodes=<core-count> \
        -DKernelVerificationBuild=OFF \
        <additional-platform-flags> \
        .. && \
    ninja all && ninja install
```

### Step 3: Build the Kernel Loader

Build `sel4-kernel-loader` against your new seL4 installation:

```dockerfile
RUN CC=aarch64-linux-gnu-gcc AS=aarch64-linux-gnu-as AR=aarch64-linux-gnu-ar \
    SEL4_PREFIX=/opt/seL4/your-board cargo build \
    --release \
    -Z build-std=core,alloc,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --target support/targets/aarch64-sel4.json \
    --package sel4-kernel-loader && \
    mkdir -p /opt/seL4/your-board/bin && \
    cp target/aarch64-sel4/release/sel4-kernel-loader.elf \
       /opt/seL4/your-board/bin/sel4-kernel-loader
```

### Step 4: Create Boot Configuration

Most ARM boards use U-Boot. You'll need:

1. **U-Boot binary** - Either pre-built for your board or compiled from source
2. **Board-specific config** - Equivalent to RPi's `config.txt` (varies by board)
3. **Boot script** - `boot.scr` that loads and boots the Lona ELF
4. **Device tree** - Usually provided by the board vendor or Linux kernel

Create `support/boot/your-board-boot.txt`:

```
# Boot script for your-board
echo "Booting Lona OS..."
fatload mmc 0 <load-address> lona/lona-your-board.elf
bootelf <load-address>
```

The load address depends on your board's memory map. Common values are `0x10000000` or `0x40000000`.

### Step 5: Add Makefile Target

Add to the Makefile:

```makefile
.PHONY: release-your-board
release-your-board: ## Build release for your-board
    $(COMPOSE) run --rm builder make _release-your-board

.PHONY: _release-your-board
_release-your-board:
    @echo "==> Building Lona for your-board..."
    # ... (follow the pattern from _release-rpi4b)
```

### Step 6: Platform-Specific Runtime Code

If your board has different UART hardware or memory layout, you may need to add platform-specific code to `lona-runtime`:

```rust
// crates/lona-runtime/src/platform/your_board.rs

pub fn uart_init() {
    // Initialize your board's UART
}

pub fn uart_putc(c: u8) {
    // Write character to UART
}
```

### Common Challenges

| Challenge | Solution |
|-----------|----------|
| No serial output | Check UART base address, baud rate divisor, pin mux |
| U-Boot hangs | Verify U-Boot config, check for required firmware blobs |
| seL4 doesn't start | Check device tree memory node matches actual RAM |
| Wrong memory size | seL4 needs exact memory at build time for ARM |
| Cache issues | Some U-Boot versions need patches to disable caches before bootelf |

### Resources

- [seL4 Porting Guide](https://docs.sel4.systems/projects/sel4/porting.html)
- [seL4 Hardware Support](https://docs.sel4.systems/Hardware/)
- [rust-sel4 Repository](https://github.com/seL4/rust-sel4)
- [U-Boot Documentation](https://docs.u-boot.org/)
