# 礼遇 LiYu · 设计文档

「礼遇」是 OctoSense 桌面里的演示社交应用：**匿名悬念送礼**。
送礼人挑一份轻礼物，给它加一道只有对方答得上的题（猜我是谁 / 私密问答 / 专属暗号），
可选再绑一句小契约（「收下就请我喝杯咖啡」），生成一张神秘礼卡发出去。
收礼人先解谜、再揭晓，最后决定：**开心收下**，或者**换购 / 折现成余额**，
折现后顺手给 TA 回一份「反击礼物」。

本目录是这次改造（偶遇 → 礼遇）的全部设计依据，代码实现以这里为准：

| 文档 | 内容 |
| --- | --- |
| [01-product.md](01-product.md) | 产品定位、角色、核心循环、商业化与传播 |
| [02-flows.md](02-flows.md) | 送礼 / 收礼 / 换购折现 / 回礼流程，礼物与契约状态机，资金流 |
| [03-pages.md](03-pages.md) | 信息架构、每一页的线框与交互细节、响应式规则 |
| [04-rules.md](04-rules.md) | 解谜规则、契约规则、费用计算、过期退回、隐私风控、AI 边界 |
| [05-data-model.md](05-data-model.md) | 数据模型、持久化边界、演示数据与模拟器 |
| [tasks.md](tasks.md) | 改造任务列表（按批次，带验收标准与完成状态） |

旧版「偶遇」的设计与复盘文档移到了 [`../archive/`](../archive/)，只作历史参考，
其中的「偶遇」字样保持原样（那是旧产品的名字）。

## 代码位置

- 应用：`apps/liyu`（crate `octosense-liyu`，模块 id `liyu`，宿主 feature `app-liyu`）
- 宿主图标：`resources/app-icons/liyu.svg`、`liyu-mono.svg`，壳内小图标 `resources/icons/gift.svg`
- 运行：`cargo run -p octosense-liyu`（独立窗口），或在 OctoSense 桌面里从启动器打开
- 测试：`cargo test -p octosense-liyu`

## 实现状态

[tasks.md](tasks.md) 里的批次 0–6 全部完成：`cargo test -p octosense-liyu` 65 个测试通过，
桌面宽度（1280×800）和手机宽度（400×820）都完整走过一遍。

实现和设计稿有出入、以代码为准的地方：

- 熟人只存一个称呼（`ContactLocal { id, label }`），另有一份通讯录名字列表，见 05；
- 送礼页的「4 · 付款」（余额抵扣开关）在表单末尾，底部固定栏只留校验提示和发送按钮，
  窄屏下不占太多高度；
- 通知条排在页面上方、占自己的高度，把页面往下推，不做浮层；
- 收下页的「我同意：契约」用开关行而不是复选框（契约长也能换行，开/关一眼看得出）。

## 实现备注（makepad）

- `Script` / `Widget` 派生宏不认带生命周期的字段类型，校验文案用别名 `type Msg = &'static str`；
- `TextInput` 没有 `visible` 属性，要显隐就外面包一层 `View`；
- 换主题走 ScriptReapply 时，Label 没写出来的 `flow` 会被重置成不换行的 `Right`，
  所以每个 Label 都显式写 `flow: Right{wrap: true}`；
- 中文标点避头尾（逗号、句号、右引号不出现在行首，左引号、左括号不留在行尾）和 ¥6 这种金额不拆行，是在引擎里修的：
  `../makepad` 分支 `fix/cjk-line-break-punct`（`draw/src/text/layouter.rs`），
  OctoSense 固定的 makepad 版本要等它合入后再升级。
