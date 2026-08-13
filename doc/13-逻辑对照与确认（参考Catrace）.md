# 13 · 逻辑对照与确认（参考项目 Catrace）

> 关联：[`doc/README.md`](./README.md)（对标项目：Catrace）。
> 目的：按用户要求，认真阅读文档指定的参考项目 **Catrace**
>（`github.com/lanxiuyun/Catrace`，Tauri 2 + Rust + Vue，与本项目同源），
> 将其与本项目的「状态检测 / 休息提醒 / 喝水提醒 / 暂停 / 弹窗」逻辑逐块对照，
> 确认本项目逻辑无误。

## 0. 参考项目读了什么
克隆 `Catrace` 后重点阅读：
- `src-tauri/src/eye.rs` —— 护眼（休息）提醒状态机与 `check_and_notify`
- `src-tauri/src/water.rs` —— 喝水提醒状态机与 `check_and_notify`
- `src-tauri/src/reminder.rs` —— 通用提醒状态机（snooze / skip / break_timer）
- `src-tauri/src/lib.rs` —— **每分钟结算主循环**（活动判定、提醒触发、托盘）
- `src-tauri/src/db.rs` —— 活跃/休息落库、`get_last_real_rest_ts`（真实休息判定）

Catrace 的模型：键鼠采样线程按分钟计 `count`；每分钟结算 `active = count>=3 || media_active`；
休息/喝水提醒基于**持久化时间戳 + 每分钟检查 + `snooze_until` + 1 秒防重**；
**真实休息**（连续不活跃 ≥ break_minutes）会把提醒计时起点推到休息结束。

## 1. 逐功能对照表

| 功能点 | Catrace（参考） | 本项目 LeisureTime | 结论 |
|---|---|---|---|
| 活动检测 | 键鼠采样计数，`count>=3`/分钟 或 媒体活跃 判为活跃 | rdev 全局监听，任意输入更新 `last_activity_ms` | 一致；本项目更灵敏，无碍 |
| 连续工作计时 | `eye_last_reminder_ts` / DB 记录，按时间戳算"连续活跃分钟" | `segment_start_ms` + `current_segment_ms(now)` | 一致（内存 vs 持久化，目标相同） |
| **真实休息重置** | 不活跃分钟 → 清 snooze；`base_ts = max(上次提醒, 上次真实休息结束)`，休息后重新计时 | `tick`：无活动 ≥ rest_threshold → `Idle/Resting`；恢复活动 `segment_start_ms=now` 新片段 | **一致**：真实休息都会把计时起点推到休息后 |
| 休息提醒触发 | `check_should_notify`：连续活跃满 window 分钟且未 snooze/skip | `in_work && reached && !rest_fired_for_segment && not_snoozed && in_wh` | 一致 |
| 多次提醒（长时工作） | 时间戳天然每 interval 重算 → 可多次弹 | `rearm_rest_if_due`：距上次提醒满一个工作阈值解除 `rest_fired_for_segment` | 一致（修复"只弹一次"正确） |
| 稍后 / 跳过 | `snooze_until = now + minutes`；`skip = now + interval` | `snooze_rest`：重置片段 + snooze + 解除抑制；`skip_rest`：置位 + 刷新 `last_rest_prompt_ms` | 一致 |
| 防重复弹窗 | `can_send_reminder`（1 秒守卫）+ `last_reminder_sent` | `rest_fired_for_segment` 标志 + rearm 阈值 | 一致（等效守卫） |
| 喝水提醒重算 | 触发后 `snooze_until = now + interval`；`record_drink` 清 snooze | 触发后 `last_water_prompt_ms = now`；`record_water` 刷新该基准 | 一致 |
| 托盘 / 单例 | `TrayIconBuilder` + `single_instance` 插件 | `tray.rs` + 主窗口关闭隐藏到托盘 | 一致（本项目已实现） |
| 弹窗关闭 | 复用单一窗口 `hide/show` + `window.eval` 切 hash | Rust 直接 `open_rest_window/open_water_window`（已存在则 show），前端 `getCurrentWindow().close()` | 一致（已通过 capability 修复关闭权限） |

## 2. 已修复的 6 个问题 · 对照确认仍正确
1. **休息只弹一次** → `rearm_rest_if_due` 按工作阈值循环解除抑制；Catrace 也靠时间戳每 interval 重算。✅ 正确。
2. **暂停误判休息** → `set_paused` 恢复时前移所有时间锚点；调度循环 `if paused { continue }` 双保险（tick 与 scheduler 都拦）。Catrace 无暂停功能，不冲突，本设计自洽。✅ 正确。
3. **清空今日复活旧片段** → `clear_today` 走 `reset_daily_and_tracking` 复位在途状态。Catrace 用 DB 重建，等价。✅ 正确。
4. **休息次数恒 0** → 提醒触发时 `rest_count += 1`，`tick` 被动转入不再计数（测试 `tick_passive_rest_does_not_increment_rest_count` 守护）。✅ 正确。
5. **喝水下溢 + 死 emit** → `saturating_sub` 防时钟回拨；移除前端无人监听的 `rest_reminder/water_reminder`（Catrace 也只用窗口+`emit_to` 自有事件，从不依赖未监听事件）。✅ 正确。
6. **喝水弹窗不自关** → `WaterCard` 加 20s 倒计时；并与 `RestPopup` 一致的 `close/destroy` 兜底。✅ 正确。

## 3. 设计差异（非 bug，均合理）
- **暂停**：Catrace 无此功能，本项目自有 `paused` + 锚点前移，逻辑自洽、有单测守护。
- **窗口模型**：Catrace 复用单一 popup 窗口（hide/show）；本项目每类提醒独立 WebviewWindow（已存在则 `show`），Rust 直接按 hash 路由，前端只 `listen('navigate')`。两种皆可。
- **活动灵敏度**：Catrace 需 ≥3 次/分钟才算活跃；本项目任意输入即算。本项目更灵敏，不会漏提醒，可接受。
- **喝水让位**：本项目 `water_deferred` 在休息提醒在场时抑制喝水，待休息弹窗关闭（`reset_water_defer`）或 skip/snooze 后恢复；Catrace 用时间 snooze。本项目逻辑可行，**但喝水能否再弹与"休息弹窗是否真正关闭"弱耦合**——该耦合已通过修复窗口关闭权限（capability）消解；若想彻底解耦，可改为 Catrace 式"时间 snooze"（可选加固，非必须）。

## 4. 结论
本项目（休息提醒助手）的核心逻辑——状态机计账、连续工作提醒、真实休息重置、稍后/跳过、防重复、喝水间隔——**与成熟参考项目 Catrace 的设计一致，未发现功能性 bug**。
此前修复的 6 个低级错误经对照确认仍正确、且方向符合参考实现。
唯一可选加固：将喝水 `water_deferred` 由"依赖休息弹窗关闭"改为"时间 snooze"，彻底解耦（YAGNI 角度当前可不做）。
