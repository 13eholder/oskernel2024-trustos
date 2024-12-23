export LD_LIBRARY_PATH=/lib:/lib/glibc
# cd /ltp && ./kirk -f ltp -r syscalls
busybox ls --color=never -1 ltp/testcases/bin/ | busybox xargs -n 1 -I {} ./test-ltp.sh ltp/testcases/bin/{}