'use strict';
// 纯本机交互草图。没有 fetch / GPS / 通讯录 / storage，刷新即重置。
const state = {
  page:'discover', published:false, sparse:false, area:'三里屯一带', time:'今天下午 · 14:00–18:00', intent:'随意走走', echo:'', sign:0,
  draft:null, contact:'lin', scenario:'normal', step:0, attempts:0, session:0, success:false, ordinary:false, personal:false, coupon:'none', memoryApplied:false, memoryChoice:'save', hiddenView:false,
  contacts:[{id:'lin',name:'林舟',label:'老同学'},{id:'chen',name:'陈晓',label:'跑步朋友'},{id:'xu',name:'许宁',label:'以前的同事'}],
  memories:[{id:1,contact:'lin',contactName:'林舟',hidden:false,date:'09 / 12',dateISO:'2026-09-12',note:'这次聊得很开心。'},{id:2,contact:'chen',contactName:'陈晓',hidden:false,date:'09 / 06',dateISO:'2026-09-06',note:'一起走了走。'}], nextMemory:3,
  trendWeeks:8,shareTheme:'warm',shareIncludeTrend:false
};
const content = document.querySelector('#content');
const policyNames = {save:'保存回忆',hidden:'隐藏回忆',none:'不保存'};
const policyNotes = {save:'本次只保存联系人与日期，不保存地点或精确时刻。',hidden:'只隐藏本次回忆，可在隐藏管理恢复；不是加密或删除。',none:'本次不保存、不累计次数；不影响旧回忆或下一次的选择。'};
const signs = ['绕一点路，去看看树。','找一间书店，给自己半小时。','今天的咖啡，可以慢慢喝。','在公共街区走走，让生活留白。'];
const contact = () => state.contacts.find(c => c.id === state.contact);
const importedContacts=state.contacts.map(c=>({...c}));
const memoryName=m=>state.contacts.find(c=>c.id===m.contact)?.name || m.contactName;
const memoryChoices=()=>`<div class="chips">${Object.entries(policyNames).map(([v,t])=>button(t,'memory-choice','chip '+(state.memoryChoice===v?'selected':''),`data-value="${v}" aria-pressed="${state.memoryChoice===v}" ${state.memoryApplied?'disabled':''}`)).join('')}</div><p class="policynote">${policyNotes[state.memoryChoice]}</p>`;
const button = (text,action,cls='secondary',extra='') => `<button class="${cls}" data-action="${action}" ${extra}>${text}</button>`;
const head = (tag,title,subtitle,back=false) => `<div class="pagehead"><div><span class="eyebrow">${tag}</span><h1>${title}</h1><p>${subtitle}</p></div>${back?button('← 返回','back','back'):''}</div>`;
const side = (title,body) => `<section class="card sidecard"><h3>${title}</h3>${body}</section>`;
function map(active) {return `<div class="map ${active?'':'sparse'}" role="img" aria-label="公共街区示意图，非实时地图，无个人位置"><div class="district b1"></div><div class="district b2"></div><div class="district b3"></div><div class="district b4"></div><div class="river"></div><div class="road"></div><div class="road vertical"></div><div class="park"></div><span class="maplabel m1">书店街区</span><span class="maplabel m2">公共绿地</span><span class="maplabel m3">滨水步道</span><div class="halo"><div class="star">${active?'✦':'☾'}</div><strong>${active?'给偶然留一点可能':'今天，随意走走'}</strong><span>${active?'区域机会 · 不知道是谁':'普通城市建议 · 与熟人无关'}</span></div><span class="mapfoot">街区插图 / 非实时地图 / 不显示个人位置</span></div>`;}
function discover() {
  const active = state.published && !state.sparse;
  return head('EVERYDAY, A LITTLE CHANCE','也许，刚好遇见。','照常过你的一天。给重逢留一点空间。') +
  `<div class="devbar"><span>草图场景</span>${button(state.sparse?'稀疏场景 · 切换正常':'正常场景 · 切换稀疏','sparse','chip')}<span>仅演示条件，不代表附近真实人数</span></div>
  <div class="columns"><div class="stack">
    <section class="card warm"><div class="cardtitle"><h3>${state.published?'你的可能去向':'今天，想去哪里？'}</h3><span class="pill warm">${state.published?'已发布 · 到期清除':'无需持续定位'}</span></div>
    <h2>${state.published?state.area:'一片街区，一段空闲。'}</h2><p>${state.published?`${state.time} / ${state.intent}`:'发布大致去向，发现顺路的可能。这里不会告诉别人你是谁。'}</p>
    <div class="actions">${button(state.published?'修改去向':'发布模糊去向 →','publish','primary warm')}${state.published?button('撤回','withdraw','linkbutton'):''}</div>
    </section>
    <section class="card"><div class="cardtitle"><h3>${active?'这一带，有相遇的可能':'先享受自己的今天'}</h3><span class="pill">${active?'匿名区域机会':'普通城市建议'}</span></div>
    <p class="subtle">${active?`${state.area} / ${state.time}。有机会，不代表一定会遇见。`:'不展示可被推断到某个人的提示，也不显示熟人数量。'}</p>${map(active)}
    <div class="opportunitybottom"><p>${active?'没有头像、名字或人数。先在线下认出彼此，再来确认。':'匿名条件未满足或未参与发布时，只给与他人行程无关的建议。'}</p>${button(active?'留一个轻轻的回声 →':'抽一张城市小签 →',active?'echo':'sign','secondary')}</div></section>
    <section class="card promptcard"><div class="bigicon">✦</div><div><h3>真的碰到朋友了？</h3><p>两人各自确认，让这次相遇有个小小的纪念。</p></div>${button('我们碰到了','verify','primary')}</section>
  </div><aside class="stack aside">
    ${side('偶遇不是找人雷达','<p>你看不到谁发布了行程，别人也看不到你的头像、姓名、电话和精确位置。</p><div class="notice">模糊去向 ≠ 实时定位<br>匿名机会 ≠ 见面保证</div>')}
    ${side('从可能，到真的相遇','<div class="numrow"><span class="number">1</span><span>发布粗区域与时段</span></div><div class="numrow"><span class="number">2</span><span>正常生活，线下认出彼此</span></div><div class="numrow"><span class="number">3</span><span>双方互认，可选相遇礼</span></div>')}
    ${side('你始终可以退出',`<p>发布可随时撤回，没有额外的参与开关。小圈场景宁可少提示，也不披露某个熟人。</p>${button('了解隐私边界','privacy','linkbutton')}`)}
  </aside></div>`;
}
function publish() {
  const d = state.draft;
  const field = (title,key,values) => `<h3 class="fieldtitle">${title}</h3><div class="chips">${values.map(v=>button(v,'choose','chip '+(d[key]===v?'selected':''),`data-key="${key}" data-value="${v}" aria-pressed="${d[key]===v}"`)).join('')}</div>`;
  return head('A LITTLE ROOM FOR CHANCE','说个大概，就够了。','不必报告行程，只分享一段可能。',true)+`<div class="columns"><section class="card">
  ${field('01 / 什么时候？','time',['今天下午 · 14:00–18:00','今天晚间 · 18:00–22:00','明天下午 · 14:00–18:00'])}
  ${field('02 / 哪一带？','area',['三里屯一带','朝阳公园周边','国贸公共街区'])}
  ${field('03 / 想做点什么？','intent',['随意走走','喝杯咖啡','吃点东西'])}<div class="rule"></div>
  <span class="eyebrow">别人能看到的样子 / 达到匿名保护条件才展示</span><div class="preview"><h2>${d.area}</h2><p>${d.time}<br>${d.intent}</p><small>没有你的身份、实时位置或发布时间</small></div>
  <div class="actions">${button('确认发布 →','commit-publish','primary warm')}${button('取消','cancel-publish')}</div>
  </section><aside class="stack aside">${side('你发布的不是实时坐标','<p>公共街区与至少 3 小时的粗时段。不会向熟人显示你在哪家店、几点到达。</p>')}${side('到期即退出匹配','<p>旧行程不变成历史档案。生产设计要求在线存储与缓存到期后 1 小时内完成清除。</p><p>草图中「到期」可在发布后模拟。</p>'+button('查看完整数据期限','privacy','linkbutton'))}</aside></div>`;
}
function echo() {
  const active = state.published && !state.sparse;
  return head('LET LIFE HAPPEN','不用约定，也有由头。','通用活动意愿，不是发给某一个人的邀请。',true)+`<div class="columns"><div class="stack"><section class="card warm"><span class="eyebrow">${active?'匿名机会 / '+state.area:'普通城市建议'}</span><h2>${active?'如果刚好碰上，就……':'给今天抽一个小签。'}</h2><p>${active?'不会显示是谁留的、谁响应了，也没有已读或响应人数。':'这张小签完全不依赖熟人行程，不代表有人在附近。'}</p>${active?`<div class="chips">${['喝杯咖啡','散散步','吃点东西'].map(v=>button(v,'send-echo','chip '+(state.echo===v?'selected':''),`data-value="${v}"`)).join('')}</div>${state.echo?`<div class="notice warm" style="margin-top:20px">已留下「${state.echo}」的通用意愿。照常活动即可，没有人需要应答。</div>`:''}`:''}<div class="sign">✧<br>${signs[state.sign]}<small>城市小签 / 不依赖他人位置 / 不必消费</small></div><div class="actions">${button('换一张','new-sign')}${button('照常过我的一天','discover','primary warm')}</div></section><section class="card promptcard"><div><h3>先线下认出人，再来确认。</h3><p>应用不会揭晓是谁，也不会为你制造约定。</p></div>${button('我们碰到了','verify','primary')}</section></div><aside class="stack aside">${side('轻一点，才自然','<p>没有自定义暗号、匿名聊天或锁定某个人。一个活动由头，已经足够。</p><p>如果没遇见，也不算失败。</p>')}</aside></div>`;
}
const scenarioLabels = {normal:'正常：独立互认与同地通过',mismatch:'双方信息不一致',denied:'定位未授权',waiting:'另一位尚未提交',expired:'现场会话已过期',stock:'互认通过，但奖励无库存'};
function verify() {
  if(state.success) return reward();
  const c = contact();
  if(!c)return head('IN PERSON','你的熟人列表暂时留白。','不需要好友申请，也不检查安装状态。')+`<section class="card"><p>可以从自己的通讯录重新导入联系人，再记下这次相遇。</p>${button('管理本机联系人','contacts','primary')}</section>`;
  let box = '', controls = '';
  if(state.step===0){box='<h3>先在线下认出彼此</h3><p>无需预先加好友或检查对方安装状态。只留自己的回忆可以独立完成；申请相遇礼时，再请另一位现场确认。</p>'; controls=button('模拟建立现场会话 →','session','primary')+button('只留回忆，不申请奖励','personal-memory');}
  if(state.step===1){box='<h3>现场会话已建立</h3><div class="notice">临时码 OU-2046 · 示例码 · 10 分钟有效<br>正式版由另一位现场扫码 / 输入，不展示账号身份。</div><p>提交后不会向对方回显你的联系人姓名或电话。</p>';controls=button('我确认，遇到的是这位朋友','self-confirm','primary');}
  if(state.step===2){
    box='<h3>你已确认，等待另一位现场确认</h3><p>双方独立指认彼此；A 认 B，B 认 A。对方填写的内容不会出现在这里。</p>';
    if(state.scenario==='mismatch' && state.attempts) box += `<div class="notice error">双方信息尚未一致。请各自检查自己的联系人。示例已尝试 ${state.attempts} / 3 次。</div>`;
    if(state.scenario==='waiting') box+='<div class="notice warm">等待另一位现场提交，没有已读或在线状态。会话有效期内可以继续，或现在取消。</div>';
    if(state.scenario==='denied') box+='<div class="notice warm">未提供同地定位凭证。双方仍可确认普通相遇，但不能领取需要同地验证的券。</div>';
    if(state.scenario==='expired') box+='<div class="notice error">本次会话已过期。不会自动保存回忆或发券。需要双方重新现场发起。</div>';
    if(state.scenario==='expired') controls=button('重新发起现场会话','restart','primary');
    else if(state.scenario==='denied') controls=button('模拟对方确认 · 只留普通回忆','ordinary','primary');
    else if(state.scenario==='waiting') controls=button('继续等待','wait','secondary');
    else controls=button('模拟另一位独立提交 →','other-confirm','primary',state.attempts>=3?'disabled':'');
  }
  return head('IN PERSON, TOGETHER','这次，真的遇见了。','线下已经认出彼此，才开始双人确认。')+`<div class="devbar"><label for="scenario">草图校验场景</label><select id="scenario" data-field="scenario">${Object.entries(scenarioLabels).map(([v,t])=>`<option value="${v}" ${state.scenario===v?'selected':''}>${t}</option>`).join('')}</select></div>
  <div class="columns"><div class="stack"><section class="card"><div class="steps"><div class="step ${state.step>0?'done':'current'}">01 现场会话</div><div class="step ${state.step>1?'done':state.step===1?'current':''}">02 本人确认</div><div class="step ${state.step===2?'current':''}">03 双方校验</div></div>
  <label for="known-contact">我在线下认出的朋友 / 本机联系人</label><select id="known-contact" data-field="contact" ${state.step===2?'disabled':''}>${state.contacts.map(c=>`<option value="${c.id}" ${state.contact===c.id?'selected':''}>${c.name} · ${c.label}</option>`).join('')}</select><p class="subtle">姓名只是本机称呼，不代表对方的账号或安装状态。领取双人奖励时才验证交叉身份；同名不能领券，草图不采集真实电话。</p><h3 class="fieldtitle">这次回忆，怎样留下？</h3>${memoryChoices()}<div class="rule"></div>${box}<div class="actions">${controls}${button('取消本次确认','cancel-verify')}</div></section>
  <div class="notice">现在选择的是你已经认出的朋友，不是从行程里揭晓身份。现场码、本人提交与另一位提交都仅为草图模拟。</div></div><aside class="stack aside">${side('定位只用在领奖验证','<p>不是持续定位，不向朋友或商户展示坐标。拒绝定位也能留下普通回忆。</p><p>真正同地协议与防作弊还需实现，草图不调用权限。</p>')}${side('每次相遇，重新选择','<p>保存、隐藏或不保存只影响这一次，不给联系人绑定永久策略。</p><p>只留回忆不需要对方操作；大额相遇礼保留双方现场确认。</p>')}</aside></div>`;
}
function recordMemory() {
  if(state.memoryApplied) return;
  state.memoryApplied=true;
  const c=contact();
  if(c && state.memoryChoice!=='none') state.memories.unshift({id:state.nextMemory++,contact:c.id,contactName:c.name,hidden:state.memoryChoice==='hidden',date:'今天',dateISO:'2026-09-17',note:'一次刚刚好的重逢。'});
}
function complete(ordinary=false,personal=false) {
  state.success=true;state.ordinary=ordinary;state.personal=personal;state.coupon='none';navigate('reward');
}
function reward() {
  if(!state.success) return `<div class="empty"><h2>先完成现场互认</h2><p>相遇礼出现在双方确认之后。</p>${button('我们碰到了','verify','primary')}</div>`;
  const c=contact(), available=!state.ordinary && state.scenario!=='stock';
  return head('A REAL MOMENT, A LITTLE GIFT','见到你，真好。','不消费，也是一场值得留下的相遇。')+`<div class="columns"><div class="stack"><section class="card green center"><div class="successmark">✓</div><h2>${state.personal?`与 ${c.name} 的相遇，留给自己`:`与 ${c.name} 的${state.ordinary?'普通相遇':'相遇'}已双方确认`}</h2><p>${state.personal?'这是本人的私人记录，不代表双人互认；不申请相遇礼。':state.ordinary?'双方互认成立；没有同地凭证，因此没有赞助券。':'双方独立互认与短时同地校验通过（模拟）。'}</p><span class="pill green">${state.personal?'私人回忆':state.ordinary?'普通相遇':'真实相遇流程模拟'} / 不披露对方个人资料</span></section>
  ${available?`<section class="card"><div class="cardtitle"><h3>这次相遇的小礼物</h3><span class="pill warm">商户赞助 · 虚构示例</span></div><div class="ticket"><small>禾间小馆 / 三里屯示例店</small><h2>¥60<span>双人满 ¥120 可用</span></h2><p>示例：消费 ¥120，券后 ¥60。7 天内有效，不兑现，不叠加；每对联系人 7 天限领一次。</p><div class="ticketbottom"><strong>${state.coupon==='used'?'✓ 已核销（模拟）':state.coupon==='claimed'?'✓ 我的券已领取':'双方各自领取自己的券'}</strong>${state.coupon==='none'?button('领取我的券','claim'):state.coupon==='claimed'?button('模拟到店核销','redeem'):''}</div></div><p class="subtle">示例店铺与金额不代表实际优惠承诺。商户只收到一次性核销令牌，不知道与你相遇的人。</p></section>`:`<section class="card"><h3>${state.ordinary?'普通相遇，同样值得':'相遇成功，本次没有可领的券'}</h3><p>${state.ordinary?'需要同地验证的奖励没有发放。你不需要为了回忆去开启定位。':'样例活动库存已用完。奖励不足不影响你们这次相遇，也不会伪装为验证失败。'}</p></section>`}
  <section class="card"><h3>这一次，怎样留下？</h3><p class="subtle">${c.name} / 只影响本次，下次重新选择</p>${memoryChoices()}<p class="policynote">${state.memoryApplied?'本次已处理，保存过的回忆可在回忆页隐藏、恢复或删除。':'结束或离开此页时按本次选择处理，尚未写入。'}</p><div class="actions">${button('结束这次相遇','finish','primary')}${button('查看我的回忆','memories')}</div></section></div><aside class="stack aside">${side('相遇是主角，优惠是配角','<p>没有购物任务，不消费也能结束。也不展示另一位领没领、花了多少。</p><p>附近推荐来自你主动选择的公共区域，不来自朋友的私人位置。</p>')}${side('每一次，自己决定','<p>默认保存；可只隐藏本次，或完全不保存。本次的选择不会自动延续到下一次。</p><p>熟人页仅管理联系人与已有回忆，没有保存策略下拉框。</p>'+button('管理已有回忆','contacts','linkbutton'))}</aside></div>`;
}
function contacts() {
  return head('PEOPLE YOU KNOW','熟人，来自你的生活。','不用好友申请，也不显示对方是否安装应用。')+`<div class="columns"><div class="stack"><section class="card"><div class="cardtitle"><h3>本机联系人 · ${state.contacts.length} 位</h3>${button('导入本机联系人','import-contacts','secondary')}</div><p class="subtle">相遇次数统计已保存回忆（含隐藏）；不保存不累计，删除后相应减少。</p>${state.contacts.length?state.contacts.map(c=>`<div class="contactrow"><div class="person"><span class="avatar ${c.id==='lin'?'a1':''}">${c.name[0]}</span><div><h3>${c.name}</h3><p>${c.label}</p></div></div><div class="encounter-count">相遇 <strong>${state.memories.filter(m=>m.contact===c.id).length}</strong> 次</div><div class="contacttools">${button('查看回忆','contact-memories','linkbutton',`data-contact="${c.id}"`)}${button('删除全部回忆','delete-contact','linkbutton',`data-contact="${c.id}"`)}${button('删除联系人','remove-contact','danger',`data-contact="${c.id}"`)}</div></div>`).join(''):'<div class="empty"><h3>熟人列表暂时留白。</h3><p>可以重新导入本机联系人，无需对方操作。</p></div>'}</section><div class="notice">这里的姓名只来自你的本机联系人。不会出现「已加入」「未安装」「在线」或附近状态。</div></div><aside class="stack aside">${side('记不记，留到每次相遇','<p>在本次确认中选保存、隐藏或不保存。熟人页不设永久回忆策略。</p>')}${side('删除联系人，不必删回忆','<p>删除时可选择是否同时删除此人的本机回忆。默认保留，回忆用你原来的本机称呼展示。</p><p>删除全部回忆会清除其计数，不保留一份隐藏的累计次数。</p>')}${side('不需要彼此授权','<p>添加或导入只是整理自己的熟人，不需要对方安装、申请或批准。</p><p>匿名机会仍需要真实发布数据；未参与发布的人不会被凭空定位。</p>')}</aside></div>`;
}
function memories() {
  const rows=state.memories.filter(m=>m.hidden===state.hiddenView && (!state.memoryFilter || m.contact===state.memoryFilter));
  return head('SMALL MOMENTS, PRIVATELY','留下一点，想记住的。','行程不会变成历史。这里只是你自己的相遇回忆。')+`<div class="columns"><div class="stack"><div class="chips">${button('我的回忆','normal-memories','chip '+(!state.hiddenView?'selected':''))}${button('隐藏管理','hidden-memories','chip '+(state.hiddenView?'selected':''))}${state.memoryFilter?button('显示所有联系人','clear-memory-filter','chip'):''}</div>${state.hiddenView?'<div class="notice warm">隐藏仍保存并计入本机相遇次数。可逐条恢复或删除，不影响同一联系人其他回忆。隐藏不是加密。</div>':''}<section class="card">${rows.length?rows.map(m=>`<article class="memoryrow"><div class="memorydate">${m.date}</div><div><h3>与 ${memoryName(m)} 重逢</h3><p>${m.note}${state.contacts.some(c=>c.id===m.contact)?'':' / 联系人已移除'}</p></div><div class="memory-actions">${button(m.hidden?'恢复':'隐藏','toggle-memory','secondary',`data-memory="${m.id}"`)}${button('删除','delete-memory','secondary',`data-memory="${m.id}"`)}</div></article>`).join(''):`<div class="empty"><div class="bigicon">✧</div><h3>${state.hiddenView?'没有隐藏的回忆':'这里暂时留白。'}</h3><p>不保存，也不影响下一次相遇。</p>${button('回到发现','discover')}</div>`}</section></div><aside class="stack aside">${side('默认不记地点','<p>每次选择是否保存，只记联系人与日期，不自动记录精确时刻、位置、券号、路线或照片。</p><p>不保存本次就不留记录，也不累计次数。</p>')}${side('删除只影响这一份','<p>你的选择不会删除对方的日记，也无法阻止对方自行记录。奖励防重数据另有有限期限。</p>'+button('隐私与数据边界','privacy','linkbutton'))}</aside></div>`;
}
function privacy() {
  const facts=[['无需好友授权或安装查询','熟人来自本机通讯录，不用互加好友，也不显示对方是否安装、注册或在线。匿名机会只使用实际发布数据，不会凭空定位未参与的人。'],['发现不指向某个人','无姓名、电话、头像、人数、实时坐标。小圈保护优先，匿名不足时仅给普通建议。外部背景与共谋仍可能造成推断，不能承诺绝对无法猜测。'],['行程与回忆分开','行程到期退出匹配，生产目标 1 小时内清除在线存储 / 缓存。每次独立选保存、隐藏或不保存。次数仅统计仍保存的记录；删除后相应减少。'],['仅申请奖励才验证双方','私人回忆可独立完成。大额相遇礼才要求双人互认与同地，生产目标支持免安装网页确认；草图只模拟，不调用权限。'],['输入与关系不给对方或商户','互认输入不回显，不能凭同名领券；商户只核销一次性令牌。奖励防重关系键最多 7 天 + 24 小时，匿名券令牌最多到期后 24 小时。'],['隐藏、删除与平台的限制','隐藏不是加密。删除联系人可保留独立回忆；删除本机回忆不删对方日记或未到期风控。认证平台可能掌握关系元数据，草图不代表生产保护已经实现。']];
  return head('PRIVACY, IN PLAIN WORDS','边界，说清楚才安心。','匿名不是一句口号，也不代表生活中不会被猜到。',true)+`<div class="columns"><section class="card">${facts.map(([t,p],i)=>`<div class="fact"><span class="number">${i+1}</span><div><h3>${t}</h3><p>${p}</p></div></div>`).join('')}<div class="actions">${button('回到发现','discover','primary')}${state.published?button('模拟当前行程到期','expire'):''}</div></section><aside class="stack aside">${side('这个草图实际做了什么','<p>全部数据虚构，仅当前页面内存；刷新重置。</p><p>不请求 GPS、相机、真实电话或通讯录，不使用远程字体、网络请求、分析 SDK 或浏览器存储。</p><p>最终 app 的协议、后端清除和反作弊仍需落地验证。</p>')}${side('随时停下','<p>发布即参与，撤回即退出，不需要额外开关。不领券、不留回忆也可以正常使用。</p>')}</aside></div>`;
}
function achievementStats() {
  // 隐藏回忆不进入成就或分享，删除后不保留另一个历史计数。
  const visible=state.memories.filter(m=>!m.hidden && m.dateISO);
  const weeks=Array.from({length:state.trendWeeks},(_,i)=>{
    const start=new Date(Date.UTC(2026,8,14)-(state.trendWeeks-1-i)*7*86400000);
    const iso=start.toISOString().slice(0,10),end=new Date(start.getTime()+7*86400000).toISOString().slice(0,10);
    return {label:iso.slice(5).replace('-','/'),value:visible.filter(m=>m.dateISO>=iso && m.dateISO<end && m.dateISO<='2026-09-17').length};
  });
  return {total:visible.length,days:new Set(visible.map(m=>m.dateISO)).size,weeks,periodTotal:weeks.reduce((n,w)=>n+w.value,0)};
}
function frequencyChart(stats) {
  const max=Math.max(1,...stats.weeks.map(w=>w.value)),height=160;
  const points=stats.weeks.map((w,i)=>[16+i*528/(stats.weeks.length-1),180-height*w.value/max]);
  const line=points.map((p,i)=>`${i?'L':'M'}${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join(' ');
  const description=stats.weeks.map(w=>`${w.label}起的一周 ${w.value} 次`).join('，');
  return `<div class="frequency-plot"><div class="ylabels"><span>${max} 次</span><span>0 次</span></div><div class="plot-main"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 205" fill="none" role="img" aria-labelledby="frequency-title frequency-desc"><title id="frequency-title">最近 ${state.trendWeeks} 周相遇次数曲线</title><desc id="frequency-desc">仅统计未隐藏的已保存回忆。${description}。最后一周尚未结束。</desc><defs><linearGradient id="frequency-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="#ffca91" stop-opacity=".22"/><stop offset="100%" stop-color="#ffca91" stop-opacity="0"/></linearGradient></defs><path d="M16 20H544 M16 100H544 M16 180H544" stroke="#2b3c52" stroke-dasharray="3 6"/><path d="${line} L544 180 L16 180Z" fill="url(#frequency-fill)"/><path d="${line}" stroke="#ffca91" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>${points.map(([x,y],i)=>`<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="4.5" fill="#ffca91" stroke="#172235" stroke-width="2"><title>${stats.weeks[i].label}起的一周：${stats.weeks[i].value}次</title></circle>`).join('')}</svg><div class="xlabels">${stats.weeks.map((w,i)=>`<span>${i%2===0 || i===stats.weeks.length-1?w.label:''}</span>`).join('')}</div></div></div><p class="subtle">每周汇总 · 最后一周尚未结束 · 0 表示没有保存的可见记录，并不代表没有见面</p><details class="chart-data"><summary>查看每周次数</summary><table><thead><tr><th>周起始</th><th>已保存相遇</th></tr></thead><tbody>${stats.weeks.map(w=>`<tr><td>${w.label}</td><td>${w.value} 次</td></tr>`).join('')}</tbody></table></details>`;
}
function achievements() {
  const s=achievementStats(),badges=[{icon:'✦',title:'第一次，刚刚好',text:'记住一次重逢',unlocked:s.total>=1},{icon:'✧',title:'生活有回响',text:'记住三次相遇',unlocked:s.total>=3},{icon:'☾',title:'把日常过成故事',text:'七个有相遇的日子',unlocked:s.days>=7}];
  return head('LITTLE MOMENTS, YOUR STORY','那些偶然，慢慢有了形状。','属于你的生活小成就，不是社交排名。')+`<div class="columns"><div class="stack"><section class="card warm"><span class="eyebrow">我的相遇手记 / 本机统计</span><h2>${s.total?'平常的日子，也有回响。':'下一次偶然，值得期待。'}</h2><p>统计来自未隐藏、仍保存的回忆。你可以随时删除，成就也会随之重新计算。</p><div class="achievement-metrics"><div><strong id="achievement-total">${s.total}</strong><span>记住的相遇</span></div><div><strong>${s.days}</strong><span>有相遇的日子</span></div><div><strong>${s.periodTotal}</strong><span>近 ${state.trendWeeks} 周相遇</span></div></div></section>
  <section class="card"><div class="cardtitle"><div><h3>相遇频率曲线</h3><p class="subtle" style="margin:5px 0 0">最近 ${state.trendWeeks} 周，记住 ${s.periodTotal} 次重逢</p></div><div class="chips">${button('4 周','trend-period','chip '+(state.trendWeeks===4?'selected':''),'data-value="4"')}${button('8 周','trend-period','chip '+(state.trendWeeks===8?'selected':''),'data-value="8"')}</div></div>${frequencyChart(s)}</section>
  <section class="card"><div class="cardtitle"><h3>我的小小里程碑</h3><span class="pill">不用凑次数</span></div><div class="badgegrid">${badges.map(b=>`<article class="badge ${b.unlocked?'unlocked':''}"><div class="badge-symbol">${b.icon}</div><h3>${b.title}</h3><p>${b.text}</p><span class="pill ${b.unlocked?'warm':''}">${b.unlocked?'已点亮':'等自然发生'}</span></article>`).join('')}</div><p class="subtle">隐藏或删除回忆后，里程碑随可见记录变化。不保存不会扣分，也不影响奖励资格。</p></section>
  <section class="card promptcard"><div class="bigicon">↗</div><div><h3>把生活里的偶然，分享给朋友。</h3><p>做一张只有你的故事、没有别人身份的分享卡。</p></div>${button('生成分享卡 →','share','primary warm')}</section></div><aside class="stack aside">${side('成就不是任务','<p>没有连续签到、排行榜或失去徽章的惩罚。一个真心的重逢，比凑够七次更值得。</p>')}${side('分享时留住边界','<p>默认只分享汇总次数与温和文案，不带联系人、地点、具体相遇日期或隐藏回忆。</p><p>曲线默认不放进分享卡，需要你主动选择。</p>')}${side('自己的记录，自己控制',`<p>隐藏和删除后，曲线及成就重新计算，不额外保存累计档案。</p>${button('管理我的回忆','memories','linkbutton')}`)}</aside></div>`;
}
function share() {
  const s=achievementStats();
  return head('A LITTLE STORY TO SHARE','让朋友看见，你生活里的光。','先看分享卡，再决定是否把它发出去。')+`<div class="columns"><div class="stack"><section class="card"><div class="cardtitle"><h3>分享卡预览</h3><span class="pill warm">仅用可见回忆</span></div><div class="share-preview"><canvas id="share-canvas" width="900" height="1200" role="img" aria-label="分享卡：我记住了 ${s.total} 次相遇，${state.shareIncludeTrend?'包含每周次数曲线':'没有相遇日期、地点或联系人'}"></canvas></div><div class="actions">${button('保存分享图片','save-share','primary warm')}${button('返回个人成就','achievements')}</div><p class="subtle">保存后，可在朋友圈、小红书或其他社交媒体选择这张图，自行发布。</p></section><div class="notice warm">汇总次数也可能透露你的社交活跃程度。请确认愿意公开后再分享。已经保存或发布的图片，不会随本机删除自动撤回。</div></div><aside class="stack aside">${side('选一个喜欢的样子',`<div class="chips">${button('暖杏','share-theme','chip '+(state.shareTheme==='warm'?'selected':''),'data-value="warm"')}${button('夜蓝','share-theme','chip '+(state.shareTheme==='night'?'selected':''),'data-value="night"')}</div>`)}${side('分享哪些内容？',`<p class="export-fields">汇总相遇次数、成就文案、偶遇署名。</p>${button(state.shareIncludeTrend?'✓ 包含每周曲线':'○ 不包含每周曲线','share-trend','full secondary',`aria-pressed="${state.shareIncludeTrend}"`)}<p>打开曲线会额外透露近 ${state.trendWeeks} 周的相遇频率。仍不包含具体联系人或地点。</p>`)}${side('朋友圈文案灵感','<p class="share-copy">不用专程约，刚好遇见。<br>给生活留一点偶然。 ✦</p><p>写自己的感受就好，提及或标记朋友之前，先问问对方。</p>')}</aside></div>`;
}
function drawShareCard() {
  const canvas=document.querySelector('#share-canvas');if(!canvas)return;
  const c=canvas.getContext('2d'),s=achievementStats(),warm=state.shareTheme==='warm';
  const bg=c.createLinearGradient(0,0,900,1200);bg.addColorStop(0,warm?'#f9dfb9':'#111e31');bg.addColorStop(1,warm?'#e8bd8f':'#243854');c.fillStyle=bg;c.fillRect(0,0,900,1200);
  const ink=warm?'#463324':'#f1e1ca',muted=warm?'#70533b':'#b9c7dc';
  c.strokeStyle=warm?'#9b775249':'#ffca9130';c.lineWidth=2;for(const [x,y,r] of [[730,270,200],[750,255,245],[140,1000,240]]){c.beginPath();c.arc(x,y,r,0,Math.PI*2);c.stroke();}
  c.textAlign='left';c.fillStyle=muted;c.font='24px "Microsoft YaHei", sans-serif';c.fillText('OUYU / 我的相遇手记',80,100);
  c.fillStyle=ink;c.font='64px "Segoe UI", sans-serif';c.fillText('✦',80,220);c.font='bold 62px "Microsoft YaHei", sans-serif';c.fillText('给生活',80,340);c.fillText('留一点偶然。',80,425);
  c.font='28px "Microsoft YaHei", sans-serif';c.fillStyle=muted;c.fillText('不用专程约，也许刚好遇见。',80,490);
  c.fillStyle=ink;c.font='bold 170px "Segoe UI", sans-serif';c.fillText(String(s.total),80,720);c.font='30px "Microsoft YaHei", sans-serif';c.fillText('次，我愿意记住的相遇',80,783);
  if(state.shareIncludeTrend){const max=Math.max(1,...s.weeks.map(w=>w.value));c.beginPath();s.weeks.forEach((w,i)=>{const x=80+i*740/(s.weeks.length-1),y=990-w.value/max*120;if(i)c.lineTo(x,y);else c.moveTo(x,y);});c.strokeStyle=ink;c.lineWidth=5;c.stroke();c.fillStyle=muted;c.font='22px "Microsoft YaHei", sans-serif';c.fillText(`近 ${state.trendWeeks} 周 / 每周相遇次数`,80,875);c.fillText('较早',80,1030);c.textAlign='right';c.fillText('最近',820,1030);c.textAlign='left';}
  else{c.font='32px "Microsoft YaHei", sans-serif';c.fillStyle=ink;c.fillText(s.total>=3?'生活有回响。':s.total?'第一次，刚刚好。':'下一次偶然，值得期待。',80,945);c.fillStyle=muted;c.font='25px "Microsoft YaHei", sans-serif';c.fillText('那些平常的日子，也在悄悄发光。',80,1000);}
  c.strokeStyle=warm?'#9b775249':'#ffca9130';c.lineWidth=2;c.beginPath();c.moveTo(80,1080);c.lineTo(820,1080);c.stroke();c.fillStyle=ink;c.font='32px "Microsoft YaHei", sans-serif';c.fillText('偶遇 OuYu',80,1140);c.textAlign='right';c.fillStyle=muted;c.font='21px "Microsoft YaHei", sans-serif';c.fillText('个人记录 · 非社交排名',820,1140);
}
function downloadShareCard() {
  const canvas=document.querySelector('#share-canvas');if(!canvas)return;
  canvas.toBlob(blob=>{if(!blob){toast('图片生成失败，请重试。');return;}const url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download='偶遇-我的相遇手记.png';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);toast('分享图片已生成。保存后可自行发布到朋友圈。');},'image/png');
}
function render(focus=false) {
  const previous=document.activeElement;
  const previousId=previous?.id;
  const previousAction=previous?.dataset.action;
  const previousValue=previous?.dataset.value;
  const views={discover,publish,echo,verify,reward,contacts,memories,privacy,achievements,share};
  if(state.page==='publish' && !state.draft) state.draft={area:state.area,time:state.time,intent:state.intent};
  content.innerHTML=views[state.page]();
  if(state.page==='achievements'){
    const weeks=achievementStats().weeks;
    content.querySelectorAll('.xlabels span').forEach((label,i)=>{
      label.style.left=((16+i*528/(weeks.length-1))/560*100)+'%';
      label.textContent=(i===0 || (i%2===0 && i<weeks.length-2) || i===weeks.length-1)?weeks[i].label:'';
    });
  }
  document.title=`偶遇 OuYu · ${content.querySelector('h1,h2')?.textContent || '草图'}`;
  document.querySelectorAll('[data-page]').forEach(b=>{const active=(['publish','echo'].includes(state.page)?'discover':state.page==='reward'?'verify':state.page==='share'?'achievements':state.page)===b.dataset.page;b.classList.toggle('active',active);b.setAttribute('aria-current',active?'page':'false');});
  if(state.page==='share')drawShareCard();
  if(focus){content.focus();window.scrollTo(0,0);}
  else if(previousId){document.getElementById(previousId)?.focus();}
  else if(previousAction){const replacement=[...content.querySelectorAll('button[data-action]')].find(b=>b.dataset.action===previousAction && b.dataset.value===previousValue);replacement?.focus();}
}
function navigate(page){
  if(state.success && ['reward','verify'].includes(state.page) && !['reward','verify'].includes(page)) recordMemory();
  if(state.page==='publish' && page!=='publish') state.draft=null;
  state.page=page; history.replaceState(null,'','#'+page);render(true);
}
let toastTimer;
function toast(text){const el=document.querySelector('#toast');el.textContent=text;el.classList.add('visible');clearTimeout(toastTimer);toastTimer=setTimeout(()=>el.classList.remove('visible'),4000);}
function resetSession(){state.step=0;state.attempts=0;state.success=false;state.ordinary=false;state.personal=false;state.coupon='none';state.memoryApplied=false;state.memoryChoice='save';}
let pendingDelete=null;
function askDelete(kind,id){pendingDelete={kind,id};document.querySelector('#delete-title').textContent=kind==='memory'?'删除这份回忆？':kind==='remove'?'删除此联系人？':'删除此人的全部回忆？';document.querySelector('#delete-description').textContent=kind==='memory'?'删除此条回忆与相关计数。不会改变下一次相遇的选择。':kind==='remove'?`从本机熟人列表移除 ${state.contacts.find(c=>c.id===id).name}。默认保留旧回忆，不影响手机系统通讯录。`:`删除本机与 ${state.contacts.find(c=>c.id===id).name} 的全部回忆（包含隐藏记录），相遇次数归零。不会移除联系人。`;document.querySelector('#delete-related-label').hidden=kind!=='remove';document.querySelector('#delete-related').checked=false;const dialog=document.querySelector('#delete-dialog');dialog.returnValue='cancel';dialog.showModal();}
document.querySelector('#delete-dialog').addEventListener('close',e=>{
  if(e.target.returnValue==='confirm' && pendingDelete){const {kind,id}=pendingDelete;if(kind==='memory')state.memories=state.memories.filter(m=>m.id!==id);else if(kind==='remove'){if(document.querySelector('#delete-related').checked)state.memories=state.memories.filter(m=>m.contact!==id);state.contacts=state.contacts.filter(c=>c.id!==id);if(id===state.contact){resetSession();state.contact=state.contacts[0]?.id;}}else state.memories=state.memories.filter(m=>m.contact!==id);render();toast(kind==='remove'?'已移除本机联系人；回忆按删除面板的选择处理。':'已删除本机回忆和相应计数；对方副本与奖励风控不受影响。');}
  pendingDelete=null;
});
document.addEventListener('click',e=>{
  const b=e.target.closest('button[data-page],button[data-action]');if(!b || b.disabled)return;
  if(b.dataset.page){navigate(b.dataset.page);return;}
  const a=b.dataset.action;
  if(['discover','publish','echo','verify','contacts','memories','privacy','achievements','share'].includes(a)){navigate(a);return;}
  if(a==='trend-period'){state.trendWeeks=Number(b.dataset.value);render();return;}
  if(a==='share-theme'){state.shareTheme=b.dataset.value;render();return;}
  if(a==='share-trend'){state.shareIncludeTrend=!state.shareIncludeTrend;render();toast(state.shareIncludeTrend?'分享卡已包含每周汇总，会透露相遇频率；请检查后再分享。':'分享卡不包含频率曲线。');return;}
  if(a==='save-share'){downloadShareCard();return;}
  if(a==='back'){navigate('discover');return;}
  if(a==='choose'){state.draft[b.dataset.key]=b.dataset.value;render();return;}
  if(a==='commit-publish'){Object.assign(state,state.draft);state.draft=null;state.published=true;state.echo='';navigate('discover');toast('模糊去向已发布，别人看不到是谁。');return;}
  if(a==='cancel-publish'){navigate('discover');return;}
  if(a==='withdraw' || a==='expire'){state.published=false;state.echo='';navigate('discover');toast(a==='expire'?'模拟行程到期，已退出匹配；不会生成行程历史。':'已撤回，停止参与机会。');return;}
  if(a==='sparse'){state.sparse=!state.sparse;state.echo='';render();return;}
  if(a==='sign'){navigate('echo');return;}
  if(a==='new-sign'){state.sign=(state.sign+1)%signs.length;render();return;}
  if(a==='send-echo'){if(!state.published || state.sparse)return;state.echo=b.dataset.value;render();toast('已留下通用活动意愿，没有个人响应或已读状态。');return;}
  if(a==='memory-choice'){if(!state.memoryApplied){state.memoryChoice=b.dataset.value;render();toast(policyNotes[state.memoryChoice]);}return;}
  if(a==='personal-memory'){if(state.step===0)complete(true,true);return;}
  if(a==='session'){state.session++;state.step=1;state.attempts=0;render();return;}
  if(a==='self-confirm'){state.step=2;render();toast('本人确认已提交，等待另一位独立确认。');return;}
  if(a==='other-confirm'){if(state.step!==2)return;if(state.scenario==='mismatch'){state.attempts++;render();toast(state.attempts>=3?'示例已达 3 次上限。结束后重新现场发起。':'双方信息尚未一致，请各自检查自己的联系人。');}else if(['normal','stock'].includes(state.scenario))complete();return;}
  if(a==='ordinary'){if(state.step===2 && state.scenario==='denied')complete(true);return;}
  if(a==='wait'){toast('继续等待，可随时取消；不会披露对方在线状态。');return;}
  if(a==='restart'){resetSession();render();toast('重新现场发起。可以用草图工具切换其他校验场景。');return;}
  if(a==='cancel-verify'){resetSession();navigate('discover');toast('本次确认已取消，没有自动生成回忆或券。');return;}
  if(a==='claim'){if(state.success && !state.ordinary && state.scenario!=='stock' && state.coupon==='none'){state.coupon='claimed';render();toast('我的示例券已领取；不展示另一位的领取状态。');}return;}
  if(a==='redeem'){if(state.coupon==='claimed'){state.coupon='used';render();toast('模拟核销成功，一次性券不会重复核销。');}return;}
  if(a==='finish'){recordMemory();resetSession();navigate('discover');toast('这次相遇已结束，回忆已按本次选择处理。');return;}
  if(a==='import-contacts'){for(const c of importedContacts)if(!state.contacts.some(x=>x.id===c.id))state.contacts.push({...c});if(!contact())state.contact=state.contacts[0]?.id;render();toast('模拟导入本机联系人，不查询安装状态或申请好友授权。');return;}
  if(a==='contact-memories'){state.memoryFilter=b.dataset.contact;state.hiddenView=false;navigate('memories');return;}
  if(a==='clear-memory-filter'){state.memoryFilter=null;render();return;}
  if(a==='toggle-memory'){const m=state.memories.find(m=>m.id===Number(b.dataset.memory));m.hidden=!m.hidden;render();toast(m.hidden?'仅隐藏这条回忆，计数仍包含它。':'已恢复这条回忆。');return;}
  if(a==='normal-memories' || a==='hidden-memories'){state.hiddenView=a==='hidden-memories';render();return;}
  if(a==='delete-memory'){askDelete('memory',Number(b.dataset.memory));return;}
  if(a==='delete-contact' || a==='remove-contact'){askDelete(a==='remove-contact'?'remove':'contact',b.dataset.contact);return;}
});
document.addEventListener('change',e=>{
  if(e.target.dataset.field==='contact'){state.contact=e.target.value;render();}
  if(e.target.dataset.field==='scenario'){state.scenario=e.target.value;resetSession();render();toast('已切换模拟场景，须重新完成现场会话与双方确认。');}
});
window.addEventListener('hashchange',()=>{const p=location.hash.slice(1);if(['discover','publish','echo','verify','reward','contacts','memories','privacy','achievements','share'].includes(p)){if(state.success && ['reward','verify'].includes(state.page) && !['reward','verify'].includes(p)) recordMemory();if(state.page==='publish' && p!=='publish')state.draft=null;state.page=p;render(true);}});
const initial=location.hash.slice(1);if(['discover','publish','echo','verify','reward','contacts','memories','privacy','achievements','share'].includes(initial))state.page=initial;
render();
