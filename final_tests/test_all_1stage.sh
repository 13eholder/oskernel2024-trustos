busybox echo "run time-test"
./time-test
busybox echo "run busybox_testcode.sh"
./busybox_testcode.sh
busybox echo "run libctest_testcode.sh"
./libctest_testcode.sh
busybox echo "run libc-bench"
./libc-bench
busybox echo "run splice-test"
./splice-test-1
./splice-test-2
./splice-test-3
./splice-test-4
./splice-test-5
busybox echo "run lua_testcode.sh"
./lua_testcode.sh
busybox echo "run iozone_testcode.sh"
./iozone_testcode.sh
busybox echo "run lmbench_testcode.sh"
./lmbench_testcode.sh
# busybox echo "run netperf_testcode.sh"
# ./netperf_testcode.sh
# busybox echo "run iperf_testcode.sh"
# ./iperf_testcode.sh
busybox echo "run unixbench_testcode.sh"
./unixbench_testcode.sh

# busybox time test-ltp.sh ltp/testcases/bin/abort01 | busybox tee ltp-result.txt


# busybox echo "run ltp-test"
# busybox touch ./ltp-result.txt
# busybox time busybox sh -c "busybox ls --color=never -1 ltp/testcases/bin/ | busybox xargs -n 1 -I {} ./test-ltp.sh ltp/testcases/bin/{}" 
# ./ltp_count.sh

# #busybox echo "run cyclic_testcode.sh"
# #./cyclictest_testcode.sh