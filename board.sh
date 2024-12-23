sudo dd bs=4M if=./sdcard.img of=/dev/sdb conv=fsync # 将磁盘镜像写入sdcard
mw 0x80200000 0x0 0x200000 # 清空内存区域
md 0x80200000 16 # 查看指定地址内存
sudo hdparm -I /dev/sdb # 查看sdcard信息
udisksctl power-off -b /dev/sdb # 弹出设备