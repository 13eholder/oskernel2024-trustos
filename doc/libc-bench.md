# 测试代码运行到调用bench时遇到StorePageFault
问题：用户栈大小不够。
解决方法：将用户栈扩展到4096*8。
