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

- [x] 目录 `CATALOG`（12 件，批次 7 扩到 33 件）与 `Category`
- [x] `Gift` / `Pact` / `LedgerEntry` / `ContactLocal` / `Settings` / `LiyuState`，micro_serde 持久化，宽松读
- [x] 规则函数：`normalize_answer`、`answer_matches`、`fee`、`validate_pact`、`guess_candidates`
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
  - 序列化往返一致

## 批次 3 · 分享与 AI

- [x] `share.rs`：礼卡 SVG（900×1200）+ 写文件；测试：不含礼物名 / 价格 / 送礼人 / 答案
- [x] `ai.rs`：`list_gift_catalog`、`suggest_gift`、`get_gift_box_summary`（全部 Risk::Read）；
      测试：输出不含答案 / 暗号 / 未揭晓送礼人 / 地址；预算过滤正确
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
- [x] 礼盒：收到 / 送出分段、状态颜色
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

## 批次 7 · 心愿单与商品展示

- [x] 设计：01 心愿单玩法与规则、02 送礼流程（详情 → 送礼 → 结算 → 成功）与心愿单流程、
      03 宫格 / 商品详情 / 结算 / 心愿单四页、04 心愿单规则、05 目录与心愿单数据模型
- [x] 目录扩到 33 件：新增数码家电 / 家居生活 / 母婴亲子三类；`CatalogItem` 加 `brand`、`kind`、`tags`、`desc`
- [x] 商品示意图 33 张 + 问号礼盒图；挑礼、详情、送礼页、结算、礼盒、礼物卡、换购、送出详情、心愿单都带图
- [x] 挑礼改成商品宫格（2–5 列按宽度）；商品详情页（信息、介绍、须知、同类还有）
- [x] 结算页：订单卡、余额抵扣、付款方式（微信支付 / 支付宝 / 银行卡，模拟）、原地变成功页；返回栈
- [x] 心愿单数据：`WishItem`（具体的 / 说个大概）、`Wishlist`、`WishDraft`，`publish_wish` / `close_wish` /
      `delete_wish` / `add_to_wishlist`，认领 `check_claim`，放开 `release_wish`，`wish_candidates`
- [x] 约好日子送到：`Gift::booked_on`、待送达状态、收到列表隐藏、模拟器「送到」一步
- [x] 页面：熟人的心愿单分段、我的心愿单、心愿单详情（我的 / 好友的）、发布 / 编辑（说个大概带实时预览）、
      挑一件（给自己挑 / 帮 TA 挑）；熟人行「看心愿单」；「我」页入口；设置「熟人心愿单提醒」；心愿单通知
- [x] 演示：三张好友心愿单 + 一张我的；「模拟熟人来认领」（界面入口已撤下）
- 验收（单元测试）：
  - 说个大概「电视 · ¥3000 以内 · 55 寸以上」→ 候选依次是声屿 S6、澄光 Q5、澄光 Q6，43 寸和超预算的不出现；
    「120Hz」要求把高刷的排在前面；预算内没有时全部标「超出预算」
  - 发布校验：空、没选类、重复、日子太早 / 太远、标题太长各报对应的错；没写标题用默认标题；编辑时认领过的那件不能删
  - 认领：自己的单子 / 已结束 / 已被认领 / 对不上 / 日子太晚都被拒；认领后状态变「我送的」
  - 撤回、过期都放开认领；有人认领过的心愿单不能删、只能结束
  - 约好日子的礼物是「待送达」、只有「下单」一行时间线，模拟器第一步是送到；不是心愿单的礼物不能约日子
  - 付款流水写明付款方式；替熟人认领按「降噪」挑中降噪耳机，揭晓前主人只看到「已被认领」
  - 存档版本不对就不读（不迁移旧数据），用演示数据
- [x] 验证：桌面（1280×800）和手机宽度走一遍：宫格 → 详情 → 送礼 → 结算 → 成功 → 礼卡；
      好友心愿单 → 帮 TA 挑 → 详情 → 送礼（约好日子）→ 结算；发布（具体的 + 说个大概）→ 编辑 → 结束 / 删除
