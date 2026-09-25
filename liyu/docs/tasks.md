# 改造任务列表（偶遇 → 礼遇）

按批次推进，每批结束都要 `cargo build -p octosense-liyu` 通过。勾选 = 已完成并验证。

## 批次 0 · 改名与归档

- [x] 新建分支 `liyu`（基于 `a1dadfb`）
- [x] `apps/ouyu` → `apps/liyu`，crate `octosense-liyu`，类型 / 常量 / DSL 模块名 Ouyu → Liyu
- [x] 宿主接线：`Cargo.toml` feature `app-liyu`、`config/apps.json`、`src/apps.rs`、启动器 / 桌面图标映射
- [x] 宿主图标：`liyu.svg` / `liyu-mono.svg`（礼盒），壳内小图标 `gift.svg` 取代 `encounter.svg`
- [x] 旧设计文档归档到 `liyu/archive/`
- 验收：全仓 grep 不再有 `ouyu` / `偶遇`（归档目录除外）；`cargo build -p octosense-liyu` 通过

## 批次 1 · 设计文档

- [x] `01-product.md` 产品、角色、循环、商业化、传播、非目标
- [x] `02-flows.md` 流程、状态机、资金流、通知
- [x] `03-pages.md` 信息架构与每页线框
- [x] `04-rules.md` 解谜 / 契约 / 费用 / 过期 / 隐私 / AI 边界
- [x] `05-data-model.md` 实体、持久化、演示数据、模拟器
- [x] 本任务列表

## 批次 2 · 数据层 `data.rs`

- [x] 目录 `CATALOG`（12 件）与 `Category`
- [x] `Gift` / `Pact` / `LedgerEntry` / `ContactLocal` / `Settings` / `LiyuState`，micro_serde 持久化，宽松读
- [x] 规则函数：`normalize_answer`、`answer_matches`、`fee`、`validate_pact`、`gen_code`、`guess_candidates`
- [x] 业务操作：`send_gift`、`open`、`submit_answer`、`accept`、`cash_out`、`exchange`、`withdraw`、
      `sweep`（过期）、`fulfil_pact` / `waive_pact` / `nudge_pact`、`top_up`、`simulate_step`
- [x] 演示数据 `LiyuState::demo(today)`；通知计算 `due_notices`
- 验收（单元测试）：
  - ¥109 折现手续费 ¥9、退 ¥100；换购抵扣 ¥103；¥10 礼物手续费最低 ¥1
  - 答案规范化：「 星际 穿越！」== 「星际穿越」；别名「林舟 / 舟舟」都算对
  - 3 次答错后揭晓且 `identity_known = false`；空答案不扣机会
  - 契约 25 字 / 含「红包」被拒
  - 余额永不为负；送礼余额不足时模拟支付补齐
  - 过期退回幂等（sweep 两次只退一次）；撤回只在待拆可用
  - 模拟器从待拆推进到终态最多 3 步；终态后不动
  - 口令不含 0 / O / 1 / I 且唯一；序列化往返一致

## 批次 3 · 分享与 AI

- [x] `share.rs`：礼卡 SVG（900×1200）+ 写文件；测试：不含礼物名 / 价格 / 送礼人 / 答案
- [x] `ai.rs`：`list_gift_catalog`、`suggest_gift`、`get_gift_box_summary`（全部 Risk::Read）；
      测试：输出不含答案 / 暗号 / 未揭晓送礼人 / 地址 / 口令；预算过滤正确
- [x] 删除 `areas.rs`

## 批次 4 · 界面骨架

- [x] 导航图标：gift、pact、box、wallet（`apps/liyu/resources/icons`）
- [x] 5 个 Tab（挑礼 / 礼盒 / 契约 / 熟人 / 我）侧栏 + 底栏，顶栏动作按页切换
- [x] 新预设：`LiyuGiftRow`、`LiyuBoxRow`、`LiyuPactRow`、`LiyuLedgerRow`、`LiyuStep`、`LiyuChoice`
- [x] `canvas.rs`：删地图 / 图表，`LiyuShareCard` 改画礼卡预览
- 验收：主题测试（深浅两套 DSL 求值）通过；布局测试通过

## 批次 5 · 页面实现

- [x] 挑礼：品类筛选 + 12 行 + aside
- [x] 送礼页：礼物卡、送给谁、四种解锁输入、契约开关 / 芯片 / 自定义、寄语、余额抵扣、校验
- [x] 礼卡页：预览、复制链接、保存图片、以 TA 视角预览
- [x] 礼盒：收到 / 送出分段、口令打开、状态颜色
- [x] 拆礼页四阶段：解密（候选芯片 / 机会 / 红字）、揭晓、收下（契约勾选 / 地址 / 券码）、
      换购折现（算式 / 列表 / 多退少补）、完成卡 + 回礼
- [x] 送出详情：时间线、撤回、模拟下一步
- [x] 契约：两个分段、兑现 / 提醒 / 免了吧
- [x] 熟人：送礼按钮、送收统计，去掉合并与回忆
- [x] 我 / 钱包 / 设置 / 引导 3 屏
- [x] 通知：礼物将过期、契约将到期

## 批次 6 · 验证

- [x] `cargo test -p octosense-liyu` 全绿
- [x] 宿主 `cargo build`、`cargo test`（style 图标测试）通过
- [x] 运行应用走一遍：送礼 → 礼卡 → 模拟对方到终态；拆三份演示礼物（答对 / 答错 3 次 / 暗号）；
      收下（实物填地址 / 电子券）、折现 → 回礼、换购补差；契约兑现；过期通知
- [x] 手机宽度（< 720）与桌面宽度各看一遍
