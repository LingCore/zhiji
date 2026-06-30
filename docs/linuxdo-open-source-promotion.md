# Linux.do 开源推广发帖稿

## 推荐标题

〖开源推广〗知机：把 Windows 虚拟化、蓝屏和硬件状态一次看明白

## 分类与标签

- 分类：开发调优
- 标签：软件开发、开源推广

## 发帖前检查

- GitHub README 已加入 LINUX DO 友链/认可链接：

```markdown
本项目参与 [LINUX DO](https://linux.do) 社区开源推广，并已在项目主页链接认可 LINUX DO 社区。
- 友链：[LINUX DO](https://linux.do)
```

- 发帖时请附上本次 AI 生成/润色内容截图。
- 建议首图使用项目截图：`docs/images/overview.png`。

## 正文

#### 本帖使用社区开源推广，符合推广要求。我申明并遵循社区要求的以下内容：

* **我的帖子已经打上 #开源推广 标签：** 是
* **我的开源项目完整开源，无未开源部分：** 是
* **我的开源项目已链接认可 LINUX DO 社区：** 是
* **我帖子内的项目介绍，AI生成、润色内容部分已截图发出：** 是
* **以上选择我承诺是永久有效的，接受社区和佬友监督：** 是

*以下为项目介绍正文内容，AI生成、润色内容已使用截图方式发出*

---

做了一个 Windows 桌面小工具：**知机 Zhiji**。

它用来快速查看和调整虚拟化、Hyper-V、蓝屏转储、虚拟内存、硬件信息等常见配置，适合排查 WSL、虚拟机、安卓模拟器、游戏优化相关问题。

项目地址：<https://github.com/LingCore/zhiji>  
下载地址：<https://github.com/LingCore/zhiji/releases>  
技术栈：Tauri + Rust  
协议：MIT

主要功能：

- 查看 CPU 虚拟化、Hyper-V、安全启动、TPM、BCD 状态
- 启用/关闭 Hyper-V、虚拟机平台、Windows Hypervisor Platform
- 调整虚拟内存、小内存转储和 `hypervisorlaunchtype`
- 打开或导出蓝屏分析结果
- 提供可还原的低风险 FPS 相关优化
- 查看硬件信息和显示器身份

目前还是偏实用的小工具，欢迎佬友试用、提 issue、拍砖。
