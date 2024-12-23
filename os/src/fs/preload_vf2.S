# 将initproc提前放到内存里
    .section .data
    .global initproc_start
    .global initproc_end
    .align 12
initproc_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/initproc"
initproc_end:
    .align 12

# 将test提前放到内存里
    .section .data
    .global test_start
    .global test_end
    .align 12
test_start:
    .incbin "../final_tests/test_all_1stage.sh"
test_end:
    .align 12


# 将磁盘块提前放到内存里
    .section .data
    .global sd_start
    .global sd_end
    .align 12
sd_start:
    .incbin "../lwext4_rust/ext4.img"
sd_end:
    .align 12