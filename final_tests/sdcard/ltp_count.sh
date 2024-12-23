# 统计文件的行数
line_count=$(busybox wc -l < ltp-result.txt)

# 输出格式化的字符串
echo "ltp pass ${line_count} tests"