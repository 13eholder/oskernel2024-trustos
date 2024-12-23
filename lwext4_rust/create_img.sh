# 制作一个全0的镜像文件
dd if=/dev/zero of=ext4.img bs=4M count=64

DIR=lwext4_rust

# 格式化为 ext4
sudo mkfs.ext4 ext4.img
sudo chmod 777 ext4.img
pwd
sudo mkdir ../${DIR}/fs 
sudo mount ../${DIR}/ext4.img ../${DIR}/fs 
# sudo cp ../user/target/riscv64gc-unknown-none-elf/release/user_shell ../${DIR}/fs/ 
# sudo mkdir ../${DIR}/fs/rCoretests
# sudo rm ../user/target/riscv64gc-unknown-none-elf/release/*.*
# sudo cp ../user/target/riscv64gc-unknown-none-elf/release/* ../${DIR}/fs/rCoretests/ 
# sudo rm ../${DIR}/fs/rCoretests/initproc 

# sudo cp ../user/target/riscv64gc-unknown-none-elf/release/usertests ../${DIR}/fs
# sudo cp -r ../pre_ctests/build/riscv64/* ../${DIR}/fs/ 


sudo cp -r ../final_tests/sdcard/* ../${DIR}/fs

sudo umount ../${DIR}/fs 
sudo rmdir ../${DIR}/fs
