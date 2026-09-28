# 礼遇：按手机号 / 邮箱送礼的现状与改造方案

日期：2026-09-28；审查：外环(codex)。

审查基线：OctoSense `c084b400a6fd4bacd9a925afc5c135eeb0eb332b`（liyu）；liyu-server `d59f19f51c22476500e3cb63df2543c15a655bd0`（main）。两仓工作树审查开始时均干净，均无仓内 AGENTS.md，遵循父目录 AGENTS.md。本次交付为实现调查与设计方案，未实现下述新功能、未派内环、未向外部联系人发送消息。

## 1. 结论

目标流程可行：发送人输入收件人的联系方式即可付款送礼；后端匹配已验证的联系方式，已注册者收到 App 通知，未注册者收到邀请，验证并注册 / 登录后领取。双方各自有平台用户 ID；礼物保留发送人和收件人以支持权限与售后，不必因此创建双方的好友关系。

当前实现仍是“注册用户 + 双方确认好友”的模型，尚不能完成上述流程。只删除好友校验不足以实现：联系人没有联系方式，订单和礼物强制要求收件人用户 ID，认证没有真实联系方式验证，付款后没有自动通知 / 邀请。

## 2. 已有能力与证据

| 部分 | 实际实现 | 代码依据 |
| --- | --- | --- |
| 平台用户 | users.id 为用户 ID，identifier 是唯一测试登录标识，可为任意字符串 | liyu-server/src/main.rs authenticate；migrations/20260926000000_init/up.sql |
| 联系方式 | user_profiles.phone / email 可空且各自 UNIQUE；手机号仅支持 11 位数字 | liyu-server/src/profile.rs bind_contact；init SQL |
| 用户关系 | friendships 存两个规范排序的用户 ID、申请人、pending / accepted 状态 | init SQL friendships |
| 建立关系 REST | GET /api/v1/friends；POST /api/v1/friends/requests（user_id）；POST /api/v1/friends/requests/{id}/accept | liyu-server/src/wishlist.rs routes / request_friend / accept_friend |
| 送礼权限 | POST /api/v1/cart/items 要求 recipient_id 是现有用户，并查询 accepted friendship | liyu-server/src/commerce.rs add_item |
| 订单及礼物 | cart_items、order_items、gifts 的 recipient_id 均 NOT NULL，引用 users；测试付款创建 sealed 礼物 | init SQL；commerce.rs pay |
| 前端在线收件人 | commerce_client 加载 friends，add 再检查列表成员；购物车只让用户选已确认好友 | apps/liyu/src/commerce_client.rs；lib.rs refresh_cart / ca_add |
| 联系人导入 | ContactLocal 只有 id / label；本机导入读取演示 directory；vCard 只解析 FN / N | apps/liyu/src/data.rs ContactLocal / parse_vcard；lib.rs import_local / import_vcard |
| 文件选择 | 导入读固定 <MAKEPAD_HOME>/liyu/contacts.vcf，启动缺文件时写演示样例；没有该流程的文件选择器 | data.rs contacts_vcf / ensure_sample_vcard；lib.rs import_vcard |
| 注册 / 绑定 | 注册码、绑定验证码均固定 123456；注册仅写 users，不建立已验证的 profile 联系方式 | main.rs authenticate；profile.rs TEST_CODE / bind_contact |
| 站内通知 | notifications 表；GET /api/v1/notifications；POST /api/v1/notifications/{id}/read；后台可以人工写通知 | src/benefits.rs notifications / routes；management.rs notify |
| 自动送礼通知 | pay 创建礼物但没有写 notifications；App poll_notices 调用本机 state.due_notices，没有消费通知 API | commerce.rs pay；apps/liyu/src/lib.rs poll_notices |
| 邮件 / 短信 / 系统推送 | 无发送适配器、投递任务、邀请领取接口、设备 token 注册或推送链路 | 服务端源码、配置及依赖审查；README 明示无邮件短信集成 |

另外，wishlists 的可见范围和认领权限也依赖 accepted friendship。前端本机“熟人”与后端 friendship 是两套数据；导入名字不等于创建后端好友关系。客户端消费 friends 列表，但上述关系申请 / 接受 API 尚未接成完整前端流程。

本机配置指向的本地 PostgreSQL 已通过 information_schema 只读查询确认上述七张表的关键列，以及三处 recipient_id 的非空约束。没有读取用户联系方式或修改业务数据。未做发送或完整 UI 端到端测试。

## 3. 建议的领域模型

### 联系人

“我的联系人”是发送人自己的地址簿，包含称呼、多个手机号 / 邮箱、来源；导入不自动注册用户、不自动创建 friendship，不向发送人暴露对方是否注册。起步可继续本机保存；如需跨设备同步，再增加以 owner_user_id 隔离的 contacts 表和 CRUD API。

同名联系人不能按名字去重；按标准化联系方式识别重复，合并前让用户确认。旧仅有姓名的记录可保留，送礼时需补联系方式。通讯录权限由用户主动授予，只提交本次送礼选择的联系方式，不为匹配整本上传。

### 已验证身份

建议增加 user_contact_identities：user_id、kind、normalized_value、verified_at、解绑 / 失效状态；为有效已验证的 (kind, normalized_value) 建唯一约束。手机号按地区解析为 E.164，国内号码明确补 +86；邮箱采用统一规则（域名规范化，不擅自删除点号或 + 标签）。前后端规则一致，服务端做最终校验。

现有 users.identifier 与 profile.phone / email 的唯一性分别存在，缺乏统一身份归属；直接用三个字段 OR 匹配可能产生冲突。历史测试 identifier 和固定验证码绑定不得直接迁成已验证身份，需重新验证。注册与绑定均通过真正的短时验证码 challenge，包含用途、有效期、错误次数限制、一次性消费与发送频率限制。测试通道须显式开启，不能把固定码验证作为真实礼物归属依据。

### 收件目标与礼物

推荐增加 delivery_targets：id、sender_id、kind、normalized_value、发送时的联系人称呼快照、matched_user_id（可空）、claimed_at。订单项和礼物引用 target_id；recipient_id 可空，表示尚未归属。现有用户 ID 可保留为兼容输入，但必须走同一权限路径。

将“待领取 / 已归属 / 过期 / 撤回”作为投递状态，与现有 sealed / opened / handled 等拆礼状态分开。已注册和未注册都先有正式礼物记录，避免未注册者的资金、库存和退款成为另一套不一致业务。

所有送礼人列表查询须 LEFT JOIN 收件人并使用称呼快照，不能因 recipient_id 为空而漏掉已付款礼物。收礼、拆礼、钱包、回收、物流等 API 仍以已归属 recipient_id + 当前会话鉴权。付款先完成资金 / 库存 / 礼物 / 通知任务的原子事务；通知失败不重复扣款，不将付款误报失败。

有手机号和邮箱时，让发送人选一个明确投递渠道；不能猜测两个联系方式属于同一个人，不能向不同账号重复投递同一礼物。

## 4. 目标流程

1. 发送人手填或导入联系人，选联系方式、商品、寄语，确认并付款。不要求知道收件人是否注册。
2. 后端在付款事务中创建礼物与投递任务，匹配唯一有效已验证身份。
3. 匹配到有效账号：赋 recipient_id，创建含 gift_id 的站内通知；App 拉取未读消息。若要 App 关闭时也收到系统通知，另接 APNs / FCM 等设备推送与设备 token 生命周期。
4. 未匹配：保留待领取礼物，写邮件 / 短信 outbox，由独立 worker 调用运营方配置的发送服务。消息带 HTTPS 领取链接，遵循既有匿名拆礼规则，不泄露发送人、金额或答案等内容。
5. 收件人打开链接，可注册，也可登录已有账号并验证目标手机号 / 邮箱。领取链接提供定位信息；领取权限必须来自已验证联系方式与当前会话，不能仅凭链接归属礼物。
6. 验证后，事务锁住仍有效且未归属的礼物，设置 recipient_id，并幂等写入站内通知。匹配注册与发礼并发时都需要重查 / 补偿扫描，避免永远挂起的礼物。
7. 送礼方只看到“待领取 / 已领取 / 已退回”等必要状态；不返回注册匹配结果、对方资料或非必要用户 ID。双方礼物记录建立起来，但不插入 friendship。

默认建议未领取到期自动退回，退款 / 库存归还仅执行一次，沿用项目既有期限规则；过期、撤回与领取竞争时必须同事务锁定。联系方式后续解绑不重新分配已领取礼物；手机号回收后的历史未领礼物不得凭新号码持有人身份无限期领取，需限制有效期并制定号码变更规则。

## 5. REST API 调整建议（以下尚未实现）

| API | 变化 / 用途 |
| --- | --- |
| POST /api/v1/cart/items | 接收 recipient: {kind: phone或email, value, label}；普通送礼移除 friendship 前置条件；不暴露匹配结果 |
| 现有 quote / orders / pay | 贯通 target_id；请求幂等；付费成功后才创建实际邀请任务 |
| POST /api/v1/auth/challenges | 为注册、登录 / 绑定发验证码；真实通道或明确测试通道 |
| POST /api/v1/auth/challenges/{id}/verify | 一次性验证，返回短时用途受限的凭据；接入注册 / 绑定 |
| GET /api/v1/gift-invitations/{token} | 只返回邀请有效性和脱敏提示，避免令牌泄漏礼物隐私；原 token 随机、库中存 hash |
| POST /api/v1/gift-invitations/{token}/claim | 登录后按已验证目标领取；幂等、过期 / 已撤回拒绝、错账号拒绝 |
| GET /api/v1/notifications | 复用现有 API，增加 type、gift_id、稳定事件键、分页 / 游标 |
| POST /api/v1/notifications/{id}/read | 复用现有 API，App 点击消息并标记已读 |
| POST /api/v1/devices（可选阶段） | 需要系统推送时注册当前账号的设备 token；登出解除关联 |

无需增加公开“手机号 / 邮箱查用户”的 API。匹配是服务器内部步骤。普通送礼取消 friendship 条件时，心愿单的可见性仍单独授权；不能让知道手机号的人自动看到对方心愿或认领明细。

notifications 增加事件唯一键防重；增加 notification_outbox，保存渠道、礼物 / 目标 ID、事件键、pending / sending / sent / failed、尝试次数、下次重试、供应商回执。worker 用锁 / 租约领任务，退避重试；供应商支持时传幂等键。外部发送与数据库提交无法原子完成，发送超时的歧义须记录，不能承诺绝对只投递一次。投递日志脱敏，失败在后台可查，发送人不应看到“已经通知”直到有相应证据。

## 6. 前端必须补齐

- ContactLocal 扩为多联系方式，兼容读取旧本机数据；姓名与地址簿身份分开。联系人编辑、详情和发送入口真实使用联系方式。
- vCard 解析 TEL / EMAIL，包括多个值和折叠行；增加 CSV（姓名、手机号、邮箱）的文件导入与列映射、预览、无效行反馈、重复确认。文件入口使用宿主选择器，不再依赖固定文件或自动写入样例。
- 本机通讯录通过各平台桥接及权限处理；不支持的平台明确引导文件导入。当前演示 directory 不应显示为真实系统通讯录。
- 购物车收件人改为联系人选择或手动手机号 / 邮箱，不再只列 commerce.friends；缺少有效联系方式时明确要求补充。
- 在线通知客户端读取 notifications，点击进入真实礼物并已读；切号隔离缓存，登出清空。离线演示消息与在线消息来源明确分开。
- 完成领取落地页、App 深链与注册 / 登录后返回领取流程；仅导向注册页并不能完成领取。
- AI 草稿增加收件目标，仍要求本人确认；联系人手机号 / 邮箱不进入公开模型摘要，在线 API 未提交成功不能称礼物已发送。

## 7. 实施顺序与验收

建议分别在两仓记录任务，不合成跨仓 commit；后端先明确契约，前端随后接入。当前仅有设计文档，不声称下列工作已完成。

1. **身份与验证**：实现标准化、唯一归属、真实验证码适配器和测试通道。验收冲突身份、错误码、重放、过期、错误用途、解绑与旧测试资料重新验证。
2. **普通送礼按联系方式**：贯通购物车 / 订单 / 礼物目标，取消普通送礼好友条件；兼容旧数据、修所有收件人 JOIN 和管理员视图。验收已注册非好友可以收礼、自送拒绝、未知用户可付款且在送出列表可见；心愿权限不扩大。
3. **邀请、领取与可靠通知**：付费事务写 outbox / 站内事件，增加领取及到期处理。验收短信 / 邮件失败重试不重复礼物和扣款；注册与付款并发；两会话竞争领取；领取与到期 / 撤回竞争；错误账号与泄漏链接不能领取。
4. **前端联系人 / 发礼 / 通知闭环**：真正导入并保留联系方式、手填与选择、通知消息点击、注册后回到礼物。验收同名不同人、多号码、重复导入、旧数据迁移、取消权限、实际四尺寸界面、断网失败保留输入、切号隔离。
5. **真实通道验证**：运营方配置短信 / 邮件供应商和域名 / HTTPS 落地地址；用受控测试收件人完成邮件、短信与站内通知三条端到端验证。系统推送若纳入需求，再验前后台与登出设备隔离。

数据库升级必须保留既有业务数据并可重复执行；先补齐新表 / 列与旧数据回填，再切换业务路径。遵循 liyu-server 当前初始化与升级机制，不能只改 init SQL 后宣称旧数据库已升级。每个切片只提交自身改动、不 push；内环交付必须由外环在独立工作树复验。

friendships 暂保留服务旧心愿授权。若产品最终取消全局好友概念，再设计明确的心愿分享链接 / 受众授权并迁移旧数据；送礼记录本身不能成为自动公开心愿的凭据。

## 2026-09-28 改造落地

上述审计中的缺口已落实到 `apps/liyu` 与 `liyu-server`：普通送礼可使用一个手机号／邮箱，不再依赖好友关系；未注册收件人通过邀请和联系方式验证自动领取，注册／绑定自动关联礼物，已有已验证账号收到站内通知。服务端增加身份、验证码、持久化投递队列和管理重试；旧的未验证资料不会被用作送礼身份。

客户端新增联系人手机号／邮箱编辑、真实 CSV／VCF 文件选择和导入预览、macOS Contacts 按需导入、多地址选择、验证码获取，以及待领取／投递状态展示。通知从后端读取，可标记已读并打开礼物。旧的仅姓名联系人保留，但需补充地址。

供应商使用可配置 HTTPS 适配接口，配置和契约见 `liyu-server/docs/contact-delivery.md`。没有配置时不假装发送成功。当前仍使用测试支付；站内通知是 App 轮询，未实现 APNs／FCM；非 macOS 系统通讯录使用文件导入，CSV 不支持任意列映射。

验证涵盖真实隔离数据库的 HTTP 流程、随机验证码的 HTTPS 通道、前端 412/820/1000/1280 宽度控件动作和邀请页浏览器检查。未访问用户真实通讯录，未发送真实邮件／短信；原生通讯录授权交互与供应商真实收件箱需配置后验证。

## 2026-09-28 直接送礼订单更新

购物车入口、加购、列表和购物车 REST API 已移除。以上历史审计和初版建议保留为过程记录；当前合同以 `liyu-server/docs/contact-delivery.md` 为准。商品详情可选“按联系方式送礼”，在单件礼物页面选联系人或手填地址，直接生成服务器订单并测试支付。既有订单、礼物和通知照常保留；旧购物车数据表只为升级兼容保留，应用不再读写。
