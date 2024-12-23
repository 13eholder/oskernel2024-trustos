# 警告Running on unknown kernel version
原因：测试代码对os的版本信息进行了读取处理，详情见测试代码。
解决方法：修改uname获取的os的版本信息。
# 执行mprotect之后卡住，没有进程继续执行
原因：剩下的操作页为不可读类型，如果要继续执行代码需要mprotect正确执行
解决方法：实现mprotect的正确完成，需要重点注意划分边界和对懒分配的frame的特殊处理！
