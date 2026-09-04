# LDCodex 品牌化与收尾 - 会话交接 (2026-09-04)

## 本会话完成的工作

### 1. 图标(真正替换 .ico)
- manager 图标 = LDcodex.png -> src-tauri/icons/icon.ico + icon.png + assets/images/codex-plus-plus.png(内容已换,文件名保留)
- 启动 Codex 图标 = LDcodex-2.png -> src-tauri/icons/launcher.ico + launcher.png
- 用 System.Drawing 生成 16/24/32/48/64/128/256 多尺寸 32bpp ICO(经 Icon 加载验证)
- launcher/manager exe 均重新 cargo build,ExtractAssociatedIcon 验证图标为深蓝新图

### 2. 密钥眼睛图标(补齐 2 处)
- Stepwise "API Key"(App.tsx SettingsScreen):加 showStepwiseApiKey state + secret-input-wrap 包裹 + 眼睛按钮
- 微信连接 "登录凭据"(WeixinConnectScreen):加 showWeixinToken state + 眼睛按钮
- 此前已有:Relay 供应商 "Key"(showApiKey)、VLM API Key(showVlmApiKey)
- CSS:合并了 styles.css 中重复的两份 .secret-input-wrap/.secret-toggle 样式(8064/8099),保留带垂直居中的版本

### 3. docs/ 官网品牌化(Codex++ -> LDCodex)
- docs/index.html、changelog.html、site.js、changelog.js:品牌、GitHub 链接(luoda2023/LDCodex)、域名 codexpp.cc->dicad.cn、下载文件名 CodexPlusPlus->LDCodex、作者 Codex++ Contributors->LUODA
- docs/CNAME: codexpp.cc -> dicad.cn
- docs/images/codex-plus-plus.png/.ico 换成 LDCodex 图标

### 4. 元数据
- 根 Cargo.toml workspace repository -> https://github.com/luoda2023/LDCodex

## 验证状态
- manager 前端: tsc --noEmit 通过; vite build 通过(dist 4 个 secret-toggle 按钮)
- core lib: cargo check 通过; cargo test --lib 343 过 1 环境相关失败(vision.rs connection refused,非回归)
- launcher: cargo build 通过; exe 图标验证
- manager tauri: cargo build 通过

## 已确认此前完成(勿重复)
- 品牌显示名: tauri.conf productName=LDCodex、title=LDCodex 管理工具、identifier=cn.dicad.ldcodex.manager
- install/mod.rs: SILENT_NAME=LDCodex、MANAGER_NAME=LDCodex 管理工具(二进制名 codex-plus-plus 有意保留兼容)
- 导航菜单已删"推荐内容"(早期截图确认),概览页仅健康检查+最近启动,无广告
- 后端 ads fetch_ad_list 短路返回空;update check/perform_update 命令已短路
- 标题栏自绘+routeAccentHue 随页面变色
- 深灰主题 CSS(:root/.dark 240 6% 蓝黑基底)
- About 页作者 LUODA/官网 dicad.cn
- NSIS 安装:独立目录/注册表,可卸载,可与原版共存(进程同名 codex-plus-plus 是有意兼容设计,安装时 taskkill 同名进程)

## 遗留/注意
- 二进制名仍为 codex-plus-plus(.exe)/codex-plus-plus-manager(.exe)(Cargo bin name + NSIS)。若需彻底改名共存需大范围改动(未做,风险高)
- core 内部远端源常量(script_market/dream_skin_market/update/ads)仍含 BigPizzaV3 上游仓库名,不显示给用户;皮肤/脚本市场依赖上游源,若要独立需自建仓库
- freebuff 集成(freebuff 配置页 3 面板)已完成于此前会话,本次未动
- 备份:_ldcodex_backup/phase2-0904-022446、core-rs-0904-024430、docs-0904-023642
