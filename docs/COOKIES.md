# Cookies 导出教程

Video Toolbox **不读你浏览器的 cookies**，也不写浏览器扩展。
要下需要登录的视频（YouTube 私密、B 站大会员、抖音登录态），
你需要手动从浏览器导出 cookies。

## 准备工作

安装浏览器扩展：**Get cookies.txt LOCALLY**
- Chrome / Edge: [Chrome Web Store](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbocc)
- Firefox: [Firefox Add-ons](https://addons.mozilla.org/en-US/firefox/addon/get-cookies-txt-locally/)

> ⚠️ **不要用名字类似的"分享 cookies"扩展**——那种会上传你的 cookies 到第三方服务器。
> **Get cookies.txt LOCALLY** 顾名思义只在本地生成文件。

## 导出步骤

1. 在浏览器登录目标平台（YouTube / B 站 / 抖音等）
2. 打开任意一个目标平台的页面（让 cookies 注入）
3. 点扩展图标 → 选 "Export" 或 "Download as TXT"
4. 浏览器会下载一个 `youtube.com.txt` 之类的文件

## 导入到 Video Toolbox

1. 打开 Video Toolbox
2. 左侧菜单 → Cookies
3. 点 "导入 cookies 文件"
4. 选你刚才下载的 .txt 文件
5. Video Toolbox 会自动按域名识别平台并保存

## 文件存放位置

导入后，cookies 文件会复制到：
```
%APPDATA%\video-toolbox\cookies\
├── youtube.com.txt
├── bilibili.com.txt
└── ...
```

下次启动自动加载。

## 注意事项

- **cookies 有效期**：通常几周到几个月，到期需要重新导出
- **不要分享你的 cookies**：那是你的登录身份，泄露 = 别人能用你账号
- **导出时确保扩展版本最新**：旧版 Netscape 格式可能不被 yt-dlp 接受
- **只导出当前平台的 cookies**：扩展默认会导出所有 cookies 域，导的时候建议先关闭其他网站

## 验证

导入后，试着下个登录态才能看的视频。能下 = 配置成功。
不能下 = cookies 过期或格式不对，重新导出。

## 故障排查

| 现象 | 原因 | 解决 |
|---|---|---|
| 导入后报"格式错误" | 用了别的扩展，输出不是 Netscape 格式 | 换 "Get cookies.txt LOCALLY" |
| 导入后还是"需要登录" | cookies 过期 | 重新登录后导出 |
| 导入后下载 403 | 你的 IP 被平台风控 | 换 IP / 等待 / 联系平台 |
