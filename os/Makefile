# make run 编译rust内核、rust测例
# make drun 直接运行,不编译
# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
FS := lwext4_rust
IMG := ext4.img
# 使用的时候把copy_to_fs的命令注释掉
# FS := final_tests
# IMG := sdcard.img
FS_IMG := ../$(FS)/$(IMG)
APPS := ../user/src/bin/*

# BOARD
BOARD := qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin
CPUS ?= 2 # default 2 cores for qemu

MODE_ARG := --release

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64
GDB_PATH := /usr/local/bin/riscv64-unknown-elf-gdb
gdb := $(GDB_PATH) rust-gdb

# Disassembly
DISASM ?= -x

# Run usertests or usershell
TEST ?=
# 定义目录变量
FINAL_DIR := $(shell cd ../final_tests && pwd)

# 定义需要跳转到final_tests执行的目标列表
FINAL_TARGETS := sdcard ltp build_all busybox lua lmbench libctest iozone libc-bench netperf iperf unix-bench cyclictest time-test test_all true copy-file-range-test interrupts-test

%:
	@if echo "$(FINAL_TARGETS)" | grep -wq $@; then \
		echo "Target $@ exists in $(FINAL_DIR), running there."&& \
		cd ../final_tests && docker run --rm -it -v $(FINAL_DIR):/code --privileged --entrypoint make alphamj/os-contest:v7.7 -C /code $@;\
	fi
# WORK_DIR := $(shell cd ../final_tests && pwd)
# build_all: build
# 	@cd ../final_tests&&docker run --rm -it -v $(WORK_DIR):/code --privileged --entrypoint make alphamj/os-contest:v7.7 -C /code sdcard

build: env initrust $(KERNEL_BIN)

initrust: 
	@cd ../user && make build

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

copy_to_fs: 
	@cd ../$(FS) && sh create_img.sh 

$(APPS):

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build $(MODE_ARG)
	@rm src/linker.ld

clean:
	@cargo clean
	@cd ../user && cargo clean

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: initrust run-inner

QEMU_ARGS := -machine virt \
			 -nographic \
			 -bios $(BOOTLOADER) \
			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
			 -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
			 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
			 -smp $(CPUS) \
			 -m 128M

run-inner: build copy_to_fs
	@qemu-system-riscv64 $(QEMU_ARGS)
drun: 
	@qemu-system-riscv64 $(QEMU_ARGS)
debug: initrust build copy_to_fs
	@tmux new-session -d \
		"qemu-system-riscv64 $(QEMU_ARGS) -s -S" && \
		tmux split-window -h "$(gdb) -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session 


gdbserver: build
	@qemu-system-riscv64 $(QEMU_ARGS) -s -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

.PHONY: build env kernel clean disasm disasm-vim run-inner copy-to-fs procinit gdbserver gdbclient run drun initrust 
