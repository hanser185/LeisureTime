# 休息提醒助手（Rest Reminder）

后台守护你的工作节奏的 Windows 桌面应用。仅监听键盘/鼠标**活动事件**（不记录内容、不上传），按节奏提醒休息与喝水。

- 技术栈：**Tauri 2 + Vue 3 + TypeScript + Rust**
- 原生 Windows 桌面程序（非 Web 应用），体积小、内存低
- 数据全部存本地 `%LOCALAPPDATA%/rest-reminder/`

## 功能

- 工作状态检测：首次活动起算；连续工作达阈值（默认 60 分）提醒；连续无活动达休息阈值（默认 10 分）判为休息并开新片段
- 休息提醒：Toast / 弹窗（首版全屏回退弹窗）；「稍后 5/10 分钟」重置计时、「跳过本次」本片段不再提醒；弹窗 20 秒自动关
- 喝水提醒：默认开，间隔 60 分，仅活跃时触发；与休息提醒冲突时错开排队
- 系统托盘常驻：暂停/恢复、打开设置、退出；关闭主窗口仅最小化到托盘
- 设置：阈值/休息判定/提醒方式/喝水/工作时段/自启/主题 + 清空今日数据 + 查看数据位置 + 恢复默认
- Dashboard：当前状态、累计工作/休息、休息次数、喝水次数、最长连续工作、下次喝水倒计时、今日时间轴、本周趋势、今日活动明细
- 隐私：不记内容、不上传；首次启动隐私引导 + 设置常驻说明

## 开发环境（Windows）

1. 安装 [Rust](https://rustup.rs/)（stable）
2. 安装 [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) 运行时（Win10/11 已预装）
3. 安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（含「使用 C++ 的桌面开发」）
4. 安装 Node.js 18+

## 运行与构建

```bash
# 安装前端依赖并启动开发（前端热更新 + Rust 重新编译）
npm install
npm run tauri dev

# 生产构建：生成安装包与便携 EXE（位于 src-tauri/target/release/bundle/）
npm run tauri build
```

> v1 暂未做代码签名：首次运行可能看到 Windows SmartScreen 警告，点「仍要运行」即可。
> 后续如需消除警告，可购买 OV/EV 代码签名证书并用 `signtool` 对 `rest-reminder.exe` 签名。

## 目录结构

```
rest-reminder/
├── src/                     # Vue 3 前端
│   ├── components/          # Dashboard / SettingsPanel / RestPopup / WaterCard / PrivacyNotice / WeeklyChart
│   ├── stores/appStore.ts   # Pinia 状态管理与 Tauri 命令桥接
│   ├── views/MainWindow.vue  # 主窗口（今日/设置 Tab）
│   ├── App.vue              # 按 URL hash 区分 主窗/休息窗/喝水窗
│   └── main.ts
├── src-tauri/               # Rust 核心
│   ├── src/
│   │   ├── main.rs          # 入口：装配 tray/监听/调度/命令/窗口事件
│   │   ├── state.rs         # 状态机与数据模型
│   │   ├── activity.rs      # 全局键鼠监听（rdev，不记内容）
│   │   ├── scheduler.rs     # 每秒调度：触发休息/喝水提醒
│   │   ├── storage.rs       # 本地 JSON 存储
│   │   ├── tray.rs          # 系统托盘与菜单
│   │   └── commands.rs      # Tauri 命令（前端调用）
│   ├── icons/               # 应用图标（脚本生成）
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
├── scripts/make_icon.py     # 纯 Python 生成图标（无需额外工具）
├── package.json
└── vite.config.ts
```

## 验证状态

- ✅ 前端 `npm run build` 通过（Node 环境可验证）
- ⏳ Rust / Tauri 打包需在 Windows + Rust 工具链下执行 `npm run tauri build`（本仓库已提供完整源码）
