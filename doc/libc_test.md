# Lictest
## run_static.sh
### pipe error
现象: fscanf卡死在pipe.read,单线程读写，只写入13字节，但是需要读取1024字节,Pipe一直让出cpu，无法结束读取
出错代码:
``` c
TEST(i, write(p[1], "hello, world\n", 13), 13, "write error %d!=%d (%s)");
TEST(i, fscanf(f, "%s %[own]", a, b), 2, "got %d fields, expected %d");
TEST_S(a, "hello,", "wrong result for %s");
```
错误原因: fscanf默认传入buffer大小为1024字节,pipe实现采用阻塞实现，若未读满1024字节会一直等待写入  
解决方案: 将pipe实现改为非阻塞式

### sys_unlinkat error
现象: tmp_file调用未成功执行,open后调用unlinkat随后fdopen,fdopen操作失败
出错代码:
``` c
FILE *tmpfile(void)
{
    // ......
	for (try=0; try<MAXTRIES; try++) {
		__randname(s+13);
		fd = sys_open(s, O_RDWR|O_CREAT|O_EXCL, 0600);
		if (fd >= 0) {
			__syscall(SYS_unlink, s);
			f = __fdopen(fd, "w+");
			if (!f) __syscall(SYS_close, fd);
			return f;
		}
	}
}

```
错误原因: 对于ext4文件,若是只有一个链接且有活跃的fd未关闭,调用unlink时应该将该操作延迟到所有对应的fd全部关闭为止。  
解决方案: 为Ext4Inode添加delay标识变量并实现RAII自动管理

### sys_munmap error
错误原因: munmmap时只回收了mmap的一个区间的前一部分,mprotect会拆分mmap的区间;   
解决方案: 支持拆分区间的回收

### sys_mmap 重复映射后卡死
错误原因: mmap使用MAP_FIXED标志位映射一段已被映射的区间时,不应该再次映射,应该使用新的标志位设置已映射的区间
解决方法: 使用mprotect拆分区间,修改访问权限

### 信号处理LoadPageFault
现象: handle signal后报错LoadPageFault,反汇编run-static.exe,使用addr2line将出错地址转换成出错文件行数,发现出错代码位置如下
```asm
00000000000414c4 <cancel_handler>:
   414c4:	00020793          	mv	a5,tp
   414c8:	0b063583          	ld	a1,176(a2) # ffffffff800000b0 <__BSS_END__+0xffffffff7ff521f8>
```
查看musl源码,sigaction设置了标志位SA_SIGINFO,内核没有对此进行过处理;
```c
static void init_cancellation()
{
	struct sigaction sa = {
		.sa_flags = SA_SIGINFO | SA_RESTART | SA_ONSTACK,
		.sa_sigaction = cancel_handler
	};
	memset(&sa.sa_mask, -1, _NSIG/8);
	__libc_sigaction(SIGCANCEL, &sa, 0);
}
```
查看cancel_handler源码,含有siginfo标志位的信号处理函数,要将第三个参数设置为ucontext;参考musl/riscv64目录下的C代码,实现SignalStack,UserContex等结构并对齐到指定长度;处理SIGINFO标志位;  

mcontext不含sepc寄存器,mcontext在riscv64架构下对应32个通用寄存器,那如何解决对pc的赋值呢?查看MC_PC源码,发现借用了x0寄存器(该寄存器始终为0),至此信号处理功能完整。
```c
static void cancel_handler(int sig, siginfo_t *si, void *ctx)
{
	pthread_t self = __pthread_self();
	ucontext_t *uc = ctx;
	uintptr_t pc = uc->uc_mcontext.MC_PC;
}
```

### pthread测试中某些进程会提前退出
错误原因: 根据debug信息定位,发现提前退出的进程都是为了响应信号,而且都发生在execve之后  
解决方法: 根据Linux文档, 创建线程时应该保留sigmask,清空sigpending;但是execve此前clone继承的sigpending如何处理是未定义的,尝试在execve后清空sigpending,问题解决。  

## run-dynamic
### 动态链接
1. 动态链接的理论很复杂,但是内核的实现非常简单,因为绝大部分的动态链接工作都在用户态完成
2. 动态链接的可执行文件中,需要使用的依赖库代码不在可执行文件中,因此内核需要映射依赖库
3. 如果Execve的文件需要动态链接,则内核将 AuxType::BASE 这一辅助向量的地址设置为自定义的动态链接起始地址,随后映射依赖库,最后在映射该文件时将地址加上偏移量即可