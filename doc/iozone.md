# lseek返回INVALID ARGUMENT
原因：lseek未正确处理从末尾往回计算offset的情况。
解决方法：去掉对offset小于0的判断，对SEEK_CUR和SEEK_END两种情况添加小于0的offset计算方法。
