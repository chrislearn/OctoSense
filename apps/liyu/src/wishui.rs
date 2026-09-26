//! 心愿单相关页面：商品详情、结算、我的心愿单、一张心愿单、发布 / 编辑、挑一件。
//!
//! 流程（好友那边）：挑礼页「熟人的心愿单」/ 熟人列表 → 一张心愿单 →
//! 具体的一件「送这件」→ 商品详情 → 送礼页（收礼人、日子都定好）→ 结算 → 成功；
//! 说了个大概的「帮 TA 挑」→ 只列符合的商品（附理由）→ 商品详情 → 同上。
//! 流程（自己这边）：我的心愿单 → 发布 / 编辑（挑具体的，或者说个大概）→ 心愿单详情看进度。

use crate::*;

const WEEKDAYS: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];

impl LiyuView {
    /// 心愿单上说「阿岚的生日」时用的名字（昵称里的第一个）。
    fn me_name(&self) -> String {
        split_aliases(&self.state.settings.nickname).into_iter().next().unwrap_or_else(|| "我".into())
    }

    /// 心愿单上的一件和它所在的单子（都是拷贝，方便边读边改界面）。
    fn wish_at(&self, at: Option<WishAt>) -> Option<(Wishlist, WishItem)> {
        let (id, k) = at?;
        let l = self.state.wishlist(id)?.clone();
        let wi = l.items.get(k)?.clone();
        Some((l, wi))
    }

    /// 这条心愿眼下还能不能认领（单子开着、这件没人送、单子不是我自己的）。
    fn wish_claimable(&self, at: Option<WishAt>) -> Option<(Wishlist, WishItem)> {
        let today = today_days();
        self.wish_at(at).filter(|(l, wi)| !l.is_mine() && l.is_open(today) && wi.is_open())
    }

    // ---- 挑礼页：熟人的心愿单 ----

    pub(crate) fn refresh_friend_wishes(&mut self, cx: &mut Cx) {
        let today = today_days();
        self.my_wish_row(cx, ids!(row_mywish));
        let lists: Vec<Wishlist> = self.state.friend_wishlists(today).into_iter().cloned().collect();
        self.friend_rows = lists.iter().map(|w| w.id).collect();
        for (j, card) in WISH_CARDS.iter().enumerate() {
            let hit = lists.get(j);
            self.show(cx, &[*card], hit.is_some());
            if let Some(w) = hit {
                self.fill_wish_card(cx, *card, w, today);
            }
        }
        self.show(cx, ids!(gw_head), !lists.is_empty());
        self.apply_list_state(cx, ids!(gw_empty), "熟人还没有发布心愿单", "", lists.len());
    }

    /// 「我的心愿单」那一行（挑礼页和「我」页共用）。
    pub(crate) fn my_wish_row(&mut self, cx: &mut Cx, row: &[LiveId]) {
        let today = today_days();
        let (count, sub) = {
            let mine = self.state.my_wishlists(today);
            let sub = match mine.iter().find(|w| w.is_open(today)) {
                Some(w) => format!("{} · {} · {}", w.title, w.countdown(today), w.progress_text()),
                None => "生日、结婚、乔迁…… 把想要的列出来".to_string(),
            };
            (mine.len(), sub)
        };
        let v = if count == 0 { String::new() } else { format!("{count} 张") };
        self.set_row(cx, row, "我的心愿单", &sub, &v);
    }

    /// 一张心愿单卡片：头像首字 / 标题 / 日子 / 场合，一排小图（有人送了的调暗），一行进度。
    fn fill_wish_card(&mut self, cx: &mut Cx, card: LiveId, w: &Wishlist, today: i64) {
        let open = w.is_open(today);
        let initial: String = if w.is_mine() {
            w.occasion_label().chars().next().map(String::from).unwrap_or_default()
        } else {
            w.owner.chars().next().map(String::from).unwrap_or_default()
        };
        self.set_text(cx, &[card, live_id!(wc_initial)], &initial);
        self.set_text(cx, &[card, live_id!(wc_title)], &w.title);
        let when = if w.is_mine() {
            format!("{} · {}", w.when_text(today), w.audience_text())
        } else {
            format!("{} · {}", w.owner, w.when_text(today))
        };
        self.set_text(cx, &[card, live_id!(wc_when)], &when);
        self.set_text(cx, &[card, live_id!(wc_badge)], if open { w.occasion_label() } else { "已结束" });
        for (k, t) in WISH_THUMBS.iter().enumerate() {
            let wi = w.items.get(k);
            self.show(cx, &[card, *t], wi.is_some());
            if let Some(wi) = wi {
                self.set_img(cx, &[card, *t], Some(wi.thumb()));
                if !wi.is_open() {
                    self.dim_img(cx, &[card, *t], 0.35);
                }
            }
        }
        let more = w.items.len().saturating_sub(WISH_THUMBS.len());
        let more = if more > 0 { format!("+{more}") } else { String::new() };
        self.set_text(cx, &[card, live_id!(wc_more)], &more);
        let prog = if !w.is_mine() && open {
            match w.open_count() {
                0 => format!("{} 件心愿 · 全都有人送啦", w.items.len()),
                n => format!("{} 件心愿 · 还有 {} 件没人送", w.items.len(), n),
            }
        } else {
            w.progress_text()
        };
        self.set_text(cx, &[card, live_id!(wc_prog)], &prog);
    }

    // ---- 商品详情 ----

    /// 打开商品详情。`wish` 是从好友心愿单进来的那一条（决定按钮和「也符合」）。
    /// 在详情页里点「同类还有」是原地换一件，不压栈。
    pub(crate) fn open_product(&mut self, cx: &mut Cx, i: u16, wish: Option<WishAt>) {
        self.pd_item = i;
        self.pd_wish = wish;
        self.refresh_product(cx);
        self.nav_to(cx, Overlay::Product);
    }

    pub(crate) fn refresh_product(&mut self, cx: &mut Cx) {
        let i = self.pd_item;
        let it = item(i);
        self.set_img(cx, ids!(pd_img), Some(i));
        self.set_text(cx, ids!(pd_cat), it.cat.label());
        self.set_text(cx, ids!(pd_name), it.name);
        self.set_text(cx, ids!(pd_brand), it.brand);
        self.set_text(cx, ids!(pd_price), &yuan(it.price));
        self.set_text(cx, ids!(pd_tags), &it.tags.join(" · "));
        self.show(cx, ids!(pd_tags), !it.tags.is_empty());
        self.set_text(cx, ids!(pd_desc), it.desc);
        let form = if it.physical { "实物 · 包邮到家" } else { "电子券 · 券码直接进礼盒" };
        self.set_kv(cx, ids!(pd_kv_kind), "形态", form, "");
        self.set_kv(cx, ids!(pd_kv_spec), "规格", it.spec, "");
        let ship = if it.physical {
            "TA 收下时填地址，3 天内发出；不喜欢可以换一件或折成余额"
        } else {
            "TA 收下后券码马上可用；不喜欢可以换一件或折成余额"
        };
        self.set_kv(cx, ids!(pd_kv_ship), "送达", ship, "");

        // 心愿单上下文：只有这条心愿还能认领、而且这件对得上时才算数。
        let ctx = self.wish_claimable(self.pd_wish).filter(|(_, wi)| wi.fits(i));
        self.show(cx, ids!(pd_ctx), ctx.is_some());
        self.show(cx, ids!(pd_addwish), ctx.is_none());
        let mut notes: Vec<String> = Vec::new();
        match &ctx {
            Some((l, wi)) => {
                let me = self.me_name();
                self.set_text(cx, ids!(pd_ctx_t), &format!("{} · 想要{}", l.event_name(&me), wi.title()));
                let cands = wish_candidates(wi);
                let r = if wi.is_exact() {
                    "就是心愿单上的这一件".to_string()
                } else {
                    let why = cands.iter().find(|m| m.item == i).map(|m| m.reason()).unwrap_or_default();
                    let wants = if wi.wants.is_empty() { "没说具体要求".to_string() } else { wi.wants.clone() };
                    format!("TA 说：{} · {}。这件：{}", wants, budget_text(wi.max_price), why)
                };
                self.set_text(cx, ids!(pd_ctx_r), &r);
                self.set_text(cx, ids!(pd_send), "就送这件");
                self.set_text(cx, ids!(pd_rel_h), "也符合这条心愿");
                let others: Vec<&WishMatch> = if wi.is_exact() {
                    Vec::new()
                } else {
                    cands.iter().filter(|m| m.item != i).take(REL_CARDS.len()).collect()
                };
                self.rel_rows = others.iter().map(|m| m.item).collect();
                notes = others.iter().map(|m| m.reason()).collect();
            }
            None => {
                self.set_text(cx, ids!(pd_send), "送给 TA");
                self.set_text(cx, ids!(pd_rel_h), "同类还有");
                self.rel_rows = related_items(i, REL_CARDS.len());
            }
        }
        let rows = self.rel_rows.clone();
        self.show(cx, ids!(pd_rel_h), !rows.is_empty());
        self.show(cx, ids!(pd_rel), !rows.is_empty());
        self.fill_cards(cx, &REL_CARDS, &rows, &notes);
    }

    /// 「送给 TA / 就送这件」：进送礼页。从心愿单来的，收礼人和心愿都带上，
    /// 日子还没到就默认约在那天送到。
    fn send_from_product(&mut self, cx: &mut Cx) {
        let i = self.pd_item;
        match self.wish_claimable(self.pd_wish).filter(|(_, wi)| wi.fits(i)) {
            Some((l, _)) => {
                self.open_send(cx, i, l.owner.clone(), None);
                self.draft.wish = self.pd_wish;
                self.draft.deliver_on = Some(l.event_on).filter(|&e| e > today_days());
                self.refresh_send(cx);
                self.refresh_topbar(cx);
            }
            None => self.open_send(cx, i, String::new(), None),
        }
    }

    /// 「加到我的心愿单」：有进行中的就直接加上；没有就去发布一张，这件先放进去。
    fn add_product_to_wishlist(&mut self, cx: &mut Cx) {
        let today = today_days();
        let i = self.pd_item;
        match self.state.current_my_wishlist(today) {
            Some(id) => match self.state.add_to_wishlist(id, i, today) {
                Ok(()) => {
                    let title = self.state.wishlist(id).map(|w| w.title.clone()).unwrap_or_default();
                    self.after_data_change(cx);
                    self.toast(cx, &format!("已加到「{title}」"));
                }
                Err(e) => self.toast(cx, e),
            },
            None => self.open_wish_edit(cx, None, Some(i)),
        }
    }

    // ---- 结算 ----

    pub(crate) fn open_checkout(&mut self, cx: &mut Cx) {
        self.co_paid = None;
        self.co_err = None;
        if self.draft.pay.is_none() {
            self.draft.pay = Some(0);
        }
        self.refresh_checkout(cx);
        self.nav_to(cx, Overlay::Checkout);
    }

    pub(crate) fn refresh_checkout(&mut self, cx: &mut Cx) {
        let d = self.draft.clone();
        let it = item(d.item);
        let paid = self.co_paid.and_then(|id| self.state.gift(id)).cloned();
        // 订单：是什么、给谁、怎么玩、什么时候到
        self.set_img(cx, ids!(co_img), Some(d.item));
        self.set_text(cx, ids!(co_name), it.name);
        self.set_text(cx, ids!(co_sub), &format!("{} · {}", it.brand, item_sub(it)));
        self.set_text(cx, ids!(co_price), &yuan(it.price));
        let wish = self.wish_at(d.wish);
        let to = match &wish {
            Some((l, _)) => format!("{}（心愿单的主人）", l.owner),
            None if d.peer.trim().is_empty() => "先不指定 · 礼卡分享给谁，谁就能拆".to_string(),
            None => d.peer.trim().to_string(),
        };
        self.set_kv(cx, ids!(co_to), "送给", &to, "");
        let play = match d.unlock {
            Unlock::Free => d.unlock.label().to_string(),
            u if d.clue.trim().is_empty() => u.label().to_string(),
            u => format!("{} · 线索「{}」", u.label(), d.clue.trim()),
        };
        self.set_kv(cx, ids!(co_play), "玩法", &play, "");
        self.show(cx, ids!(co_pact), d.contract.is_some());
        if let Some(c) = &d.contract {
            self.set_kv(cx, ids!(co_pact), "契约", c, "");
        }
        let when = match d.deliver_on {
            Some(day) => format!("{} 当天出现在 TA 的礼盒里", md_cn(day)),
            None if paid.is_some() => "礼卡已经生成，现在就能拆".to_string(),
            None => "付完马上生成礼卡".to_string(),
        };
        self.set_kv(cx, ids!(co_when), "送达", &when, "");
        self.show(cx, ids!(co_wish), wish.is_some());
        if let Some((l, wi)) = &wish {
            self.set_kv(cx, ids!(co_wish), "心愿", &format!("{} · {}", l.title, wi.title()), "");
        }

        // 付完了：页面变成功页。
        self.show(cx, ids!(co_paid), paid.is_some());
        self.show(cx, ids!(co_pay), paid.is_none());
        self.show(cx, ids!(co_go), paid.is_none());
        self.show(cx, ids!(co_after), paid.is_some());
        if let Some(g) = &paid {
            let mut b = String::new();
            if g.is_scheduled(today_days()) {
                b.push_str(&format!("{} 那天礼卡才会出现在{}的礼盒里，在那之前 TA 的心愿单上只显示这件「已被认领」。", md_cn(g.sent_on), spaced(&g.shown_recipient())));
            } else if g.peer_name().is_empty() {
                b.push_str("还没指定送给谁：把礼卡链接分享给想送的人，谁点开谁来拆。");
            } else {
                b.push_str(&format!("礼卡已经发给{}，也可以把它分享出去。", spaced(&g.shown_recipient())));
            }
            if let Some((_, wi)) = &wish {
                b.push_str(&format!("其他熟人看到「{}」已有人送，不会撞礼。", wi.title()));
            }
            self.set_text(cx, ids!(co_ok_b), &b);
            self.show(cx, ids!(co_err), false);
            return;
        }

        // 付款：余额抵扣 + 还需支付 + 付款方式
        let bal = self.state.balance();
        self.show(cx, ids!(co_bal_row), bal > 0);
        self.set_switch(cx, ids!(co_bal_row), "用余额抵扣", &format!("礼遇余额 {}", yuan(bal)), d.use_balance && bal > 0);
        let (a, b) = self.state.pay_split(it.price, d.use_balance);
        self.set_kv(cx, ids!(co_l1), "商品金额", "", &yuan(it.price));
        self.show(cx, ids!(co_l2), a > 0);
        self.set_kv(cx, ids!(co_l2), "余额抵扣", "", &format!("-{}", yuan(a)));
        self.set_kv(cx, ids!(co_l3), "还需支付", "", &yuan(b));
        self.show(cx, ids!(co_pm), b > 0);
        let p = d.pay.unwrap_or(0) as usize;
        self.set_chip_group(cx, &PAY_CHIPS, p);
        let go = if b > 0 {
            format!("确认支付 {}（{}）", yuan(b), PAY_METHODS.get(p).copied().unwrap_or("模拟支付"))
        } else {
            format!("用余额支付 {}", yuan(a))
        };
        self.set_text(cx, ids!(co_go), &go);
        self.show(cx, ids!(co_err), self.co_err.is_some());
        if let Some(e) = self.co_err {
            self.set_text(cx, ids!(co_err), e);
        }
    }

    fn pay_checkout(&mut self, cx: &mut Cx) {
        if self.co_paid.is_some() {
            return;
        }
        let today = today_days();
        let mut d = self.draft.clone();
        let (_, b) = self.state.pay_split(item(d.item).price, d.use_balance);
        d.pay = if b > 0 { d.pay.or(Some(0)) } else { None };
        match self.state.send_gift(&d, today) {
            Ok(id) => {
                self.co_paid = Some(id);
                self.co_err = None;
                self.card_gift = Some(id);
                self.card_style = ShareStyle::Warm;
                self.return_banner = None;
                // 付完之后「返回」不该再回到送礼页重新付一次。
                self.back_stack.clear();
                self.after_data_change(cx);
                self.refresh_checkout(cx);
                self.refresh_topbar(cx);
                self.scroll_top(cx, Overlay::Checkout);
                self.start_fade(cx);
            }
            Err(e) => {
                self.co_err = Some(e);
                self.refresh_checkout(cx);
            }
        }
    }

    // ---- 我的心愿单 ----

    pub(crate) fn open_wishes(&mut self, cx: &mut Cx) {
        self.refresh_wishes(cx);
        self.nav_to(cx, Overlay::Wishes);
    }

    pub(crate) fn refresh_wishes(&mut self, cx: &mut Cx) {
        let today = today_days();
        let lists: Vec<Wishlist> = self.state.my_wishlists(today).into_iter().cloned().collect();
        self.my_rows = lists.iter().map(|w| w.id).collect();
        for (j, card) in MY_WISH_CARDS.iter().enumerate() {
            let hit = lists.get(j);
            self.show(cx, &[*card], hit.is_some());
            if let Some(w) = hit {
                self.fill_wish_card(cx, *card, w, today);
            }
        }
        self.apply_list_state(cx, ids!(mw_empty), "还没有发布过心愿单", "发布一张", lists.len());
    }

    // ---- 一张心愿单 ----

    pub(crate) fn open_wish(&mut self, cx: &mut Cx, id: u64) {
        if self.state.wishlist(id).is_none() {
            self.toast(cx, "这张心愿单已经不在了");
            return;
        }
        self.wish_id = Some(id);
        self.wish_armed = 0;
        self.refresh_wish(cx);
        self.nav_to(cx, Overlay::Wish);
    }

    pub(crate) fn refresh_wish(&mut self, cx: &mut Cx) {
        let today = today_days();
        let Some(w) = self.wish_id.and_then(|id| self.state.wishlist(id)).cloned() else {
            return;
        };
        let open = w.is_open(today);
        let mine = w.is_mine();
        let initial: String = if mine {
            w.occasion_label().chars().next().map(String::from).unwrap_or_default()
        } else {
            w.owner.chars().next().map(String::from).unwrap_or_default()
        };
        self.set_text(cx, ids!(wd_initial), &initial);
        self.set_text(cx, ids!(wd_title), &w.title);
        let when = if mine { w.when_text(today) } else { format!("{} · {}", w.owner, w.when_text(today)) };
        self.set_text(cx, ids!(wd_when), &when);
        self.set_text(cx, ids!(wd_badge), if open { w.occasion_label() } else { "已结束" });
        self.show(cx, ids!(wd_note), !w.note.is_empty());
        self.set_text(cx, ids!(wd_note), &w.note);
        self.set_text(cx, ids!(wd_prog), &w.progress_text());
        self.wish_frac = if w.items.is_empty() { 0.0 } else { w.claimed_count() as f64 / w.items.len() as f64 };
        self.layout_wish_fill(cx);
        let meta = if mine {
            w.audience_text()
        } else {
            format!("{} 发布于 {}", w.owner, md_cn(w.created_on))
        };
        self.set_text(cx, ids!(wd_meta), &meta);
        let tip = match (mine, open) {
            (false, true) => format!(
                "挑一件还没人送的。送出之后别人会看到「已有人送」，不会撞礼；{}在拆开礼物之前也不知道是你。",
                spaced(&w.owner)
            ),
            (false, false) => "这张心愿单已经结束，不能再认领了。".to_string(),
            (true, true) => "谁认领了哪一件，礼物揭晓之前你只看得到「已被认领」。想要的变了可以编辑，已经有人认领的那几件不能删。".to_string(),
            (true, false) => "这张心愿单已经结束。收到的礼物都在礼盒里。".to_string(),
        };
        self.set_text(cx, ids!(wd_tip_t), &tip);

        for (k, row) in WISH_ROWS.iter().enumerate() {
            let Some(wi) = w.items.get(k).cloned() else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            self.set_img(cx, &[*row, live_id!(wr_img)], Some(wi.thumb()));
            if !mine && wi.state == WISH_BY_OTHERS {
                self.dim_img(cx, &[*row, live_id!(wr_img)], 0.35);
            }
            self.set_text(cx, &[*row, live_id!(wr_name)], &wish_item_name(&wi));
            self.set_text(cx, &[*row, live_id!(wr_sub)], &wi.sub());
            let status = self.state.wish_item_status(&w, k, today);
            self.set_text(cx, &[*row, live_id!(wr_state)], &status);
            let c = match wi.state {
                WISH_OPEN => self.pal.blue,
                WISH_BY_ME => self.pal.good,
                _ if mine => self.pal.good,
                _ => self.pal.ink_3,
            };
            self.tint_text(cx, &[*row, live_id!(wr_state)], c);
            let can_give = !mine && open && wi.is_open();
            self.show(cx, &[*row, live_id!(wr_go)], can_give);
            self.set_text(cx, &[*row, live_id!(wr_go)], if wi.is_exact() { "送这件" } else { "帮 TA 挑" });
            self.show(cx, &[*row, live_id!(wr_alt)], false);
        }

        // 自己的：编辑 / 分享 / 结束 / 删除。
        self.show(cx, ids!(wd_owner), mine);
        if mine {
            self.show(cx, ids!(wd_edit), open);
            self.show(cx, ids!(wd_close), open);
            self.show(cx, ids!(wd_del), w.claimed_count() == 0);
            self.show(cx, ids!(wd_confirm), self.wish_armed > 0);
            let (t, yes) = match self.wish_armed {
                1 => ("提前结束后熟人还看得到这张单子，但不能再认领。", "确认结束"),
                _ => ("删除后这张心愿单就没了，熟人那边也看不到。", "确认删除"),
            };
            self.set_text(cx, ids!(wd_ctext), t);
            self.set_text(cx, ids!(wd_cyes), yes);
        }
    }

    fn wish_confirm(&mut self, cx: &mut Cx) {
        let Some(id) = self.wish_id else { return };
        let today = today_days();
        let armed = std::mem::take(&mut self.wish_armed);
        let r = if armed == 1 { self.state.close_wish(id, today) } else { self.state.delete_wish(id) };
        match r {
            Ok(()) => {
                self.after_data_change(cx);
                if armed == 1 {
                    self.toast(cx, "心愿单已结束");
                    self.refresh_wish(cx);
                    self.refresh_topbar(cx);
                } else {
                    self.wish_id = None;
                    self.toast(cx, "心愿单已删除");
                    self.pop_back(cx);
                }
            }
            Err(e) => {
                self.toast(cx, e);
                self.refresh_wish(cx);
            }
        }
    }

    // ---- 发布 / 编辑 ----

    /// `id` = 编辑哪一张（`None` = 新发布）；`preset` = 从商品详情带过来的那一件。
    pub(crate) fn open_wish_edit(&mut self, cx: &mut Cx, id: Option<u64>, preset: Option<u16>) {
        let today = today_days();
        let mut d = match id.and_then(|id| self.state.wishlist(id)) {
            Some(w) => WishDraft::from_list(w),
            None => WishDraft::new(today),
        };
        if let Some(i) = preset {
            if !d.has_item(i) && d.items.len() < WISH_MAX_ITEMS {
                d.items.push(WishItem::exact(i));
            }
        }
        self.set_text(cx, ids!(we_title), &d.title);
        self.set_text(cx, ids!(we_note), &d.note);
        self.set_text(cx, ids!(we_wants), "");
        self.wish_draft = d;
        self.wish_err = None;
        self.vague_open = false;
        self.vague_kind = 0;
        self.vague_budget = 0;
        self.refresh_wish_edit(cx);
        self.nav_to(cx, Overlay::WishEdit);
    }

    pub(crate) fn refresh_wish_edit(&mut self, cx: &mut Cx) {
        let today = today_days();
        let d = self.wish_draft.clone();
        self.set_chip_group(cx, &OCC_CHIPS, d.occasion as usize);
        let e = d.event_on;
        let rel = match e - today {
            0 => "就是今天".to_string(),
            1 => "明天".to_string(),
            n if n > 1 => format!("还有 {n} 天"),
            n => format!("已过 {} 天", -n),
        };
        self.set_text(cx, ids!(we_date_l), &format!("{} · 周{} · {}", md_cn(e), WEEKDAYS[weekday(e)], rel));
        let ph = format!("标题（不写就叫「{}」）", d.default_title(&self.state.settings.nickname));
        self.view.text_input(cx, ids!(we_title)).set_empty_text(cx, ph);

        // 已经放进去的
        for (k, row) in EDIT_ROWS.iter().enumerate() {
            let Some(wi) = d.items.get(k) else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            self.set_img(cx, &[*row, live_id!(wr_img)], Some(wi.thumb()));
            self.set_text(cx, &[*row, live_id!(wr_name)], &wish_item_name(wi));
            self.set_text(cx, &[*row, live_id!(wr_sub)], &wi.sub());
            let state = if wi.is_open() { "" } else { "已有人认领，不能移除" };
            self.show(cx, &[*row, live_id!(wr_state)], !state.is_empty());
            self.set_text(cx, &[*row, live_id!(wr_state)], state);
            self.tint_text(cx, &[*row, live_id!(wr_state)], self.pal.good);
            self.show(cx, &[*row, live_id!(wr_go)], false);
            self.show(cx, &[*row, live_id!(wr_alt)], wi.is_open());
        }
        self.show(cx, ids!(we_items_empty), d.items.is_empty());
        let room = d.items.len() < WISH_MAX_ITEMS;
        self.show(cx, ids!(we_add), room && !self.vague_open);

        // 说个大概：品类 + 预算 + 要求，下面实时预览目录里有几件符合。
        self.show(cx, ids!(we_vague), self.vague_open);
        if self.vague_open {
            self.set_chip_group(cx, &KIND_CHIPS, self.vague_kind);
            self.set_chip_group(cx, &BUDGET_CHIPS, self.vague_budget);
            let wants = self.input_text(cx, ids!(we_wants));
            let probe = WishItem::vague(WISH_KINDS[self.vague_kind], WISH_BUDGETS[self.vague_budget] * 100, &wants);
            let cands = wish_candidates(&probe);
            let within = cands.iter().filter(|m| m.within).count();
            let prev = if within == 1 {
                "目录里有 1 件符合，送礼的人会看到它".to_string()
            } else if within > 1 {
                format!("目录里有 {within} 件符合，送礼的人会从这些里挑")
            } else if !cands.is_empty() {
                format!("预算内暂时没有，送礼的人会看到最接近的 {} 件（超出预算）", cands.len())
            } else {
                "目录里暂时没有这一类".to_string()
            };
            self.set_text(cx, ids!(we_v_prev), &prev);
            for (j, t) in PREVIEW_THUMBS.iter().enumerate() {
                let hit = cands.get(j).map(|m| m.item);
                self.show(cx, &[*t], hit.is_some());
                if hit.is_some() {
                    self.set_img(cx, &[*t], hit);
                }
            }
        }

        // 给谁看
        self.aud_names = self.state.contacts.iter().take(AUD_CHIPS.len()).map(|c| c.label.clone()).collect();
        self.view.check_box(cx, ids!(aa0)).set_active(cx, d.audience.is_empty(), Animate::Yes);
        let names = self.aud_names.clone();
        for (j, id) in AUD_CHIPS.iter().enumerate() {
            let name = names.get(j);
            self.show(cx, &[*id], name.is_some());
            if let Some(n) = name {
                self.set_text(cx, &[*id], n);
                let on = d.audience.contains(n);
                self.view.check_box(cx, &[*id]).set_active(cx, on, Animate::Yes);
            }
        }
        let aud = if d.audience.is_empty() {
            "所有熟人都能看到".to_string()
        } else {
            format!("只有选中的 {} 位能看到", d.audience.len())
        };
        self.set_text(cx, ids!(we_aud_n), &aud);

        self.show(cx, ids!(we_err), self.wish_err.is_some());
        if let Some(e) = self.wish_err {
            self.set_text(cx, ids!(we_err), e);
        }
        self.set_text(cx, ids!(we_go), if d.id.is_some() { "保存修改" } else { "发布心愿单" });
    }

    fn submit_wish(&mut self, cx: &mut Cx) {
        let today = today_days();
        self.wish_draft.title = self.input_text(cx, ids!(we_title));
        self.wish_draft.note = self.input_text(cx, ids!(we_note));
        let editing = self.wish_draft.id.is_some();
        match self.state.publish_wish(&self.wish_draft, today) {
            Ok(id) => {
                self.wish_err = None;
                self.after_data_change(cx);
                let msg = match (editing, self.wish_draft.audience.len()) {
                    (true, _) => "已保存".to_string(),
                    (false, 0) => "心愿单发布了，熟人现在就能看到".to_string(),
                    (false, n) => format!("心愿单发布了，选中的 {n} 位熟人现在就能看到"),
                };
                self.toast(cx, &msg);
                // 发布 / 保存完落到这张心愿单上；从心愿单点「编辑」进来的，把原来那层换掉。
                if self.back_stack.last() == Some(&Overlay::Wish) {
                    self.back_stack.pop();
                }
                self.wish_id = Some(id);
                self.wish_armed = 0;
                self.refresh_wish(cx);
                self.open_overlay(cx, Overlay::Wish);
            }
            Err(e) => {
                self.wish_err = Some(e);
                self.refresh_wish_edit(cx);
            }
        }
    }

    // ---- 挑一件 ----

    /// `Some` = 替好友说了个大概的那条心愿挑；`None` = 给正在编辑的心愿单挑一件具体的。
    pub(crate) fn open_wish_pick(&mut self, cx: &mut Cx, wish: Option<WishAt>) {
        self.wp_wish = wish;
        self.wp_cat = 0;
        self.refresh_wish_pick(cx);
        self.nav_to(cx, Overlay::WishPick);
    }

    pub(crate) fn refresh_wish_pick(&mut self, cx: &mut Cx) {
        let notes: Vec<String>;
        match self.wish_at(self.wp_wish) {
            Some((l, wi)) => {
                self.set_text(cx, ids!(wp_t), &format!("{}想要{}", l.owner, wi.title()));
                let mut s = budget_text(wi.max_price);
                if !wi.wants.is_empty() {
                    s.push_str(&format!(" · 要求：{}", wi.wants));
                }
                s.push_str(" · 已按符合程度排好");
                self.set_text(cx, ids!(wp_s), &s);
                self.show(cx, ids!(wp_chips), false);
                let cands = wish_candidates(&wi);
                let over = cands.iter().any(|m| !m.within);
                self.wp_rows = cands.iter().map(|m| m.item).collect();
                notes = cands.iter().map(|m| m.reason()).collect();
                let n = if over {
                    "预算内没有完全符合的，上面是最接近的几件（超出 TA 的预算）。"
                } else {
                    "只列出符合这条心愿的商品。点开看详情，再决定送哪件。"
                };
                self.set_text(cx, ids!(wp_note), n);
            }
            None => {
                self.set_text(cx, ids!(wp_t), "点一件，就放进心愿单");
                let s = format!("已经放了 {} 件，最多 {} 件", self.wish_draft.items.len(), WISH_MAX_ITEMS);
                self.set_text(cx, ids!(wp_s), &s);
                self.show(cx, ids!(wp_chips), true);
                self.set_chip_group(cx, &PICK_CHIPS, self.wp_cat);
                let cat = self.wp_cat.checked_sub(1).map(|i| Category::ALL[i]);
                self.wp_rows = catalog_in(cat);
                notes = self
                    .wp_rows
                    .iter()
                    .map(|&i| if self.wish_draft.has_item(i) { "已在心愿单上".to_string() } else { String::new() })
                    .collect();
                self.set_text(cx, ids!(wp_note), "没有想要的那一款？回上一页点「说个大概」，只写品类、预算和要求。");
            }
        }
        let rows = self.wp_rows.clone();
        self.fill_cards(cx, &PICK_CARDS, &rows, &notes);
    }

    fn pick_for_draft(&mut self, cx: &mut Cx, i: u16) {
        if self.wish_draft.has_item(i) {
            self.toast(cx, "已经在心愿单上了");
            return;
        }
        if self.wish_draft.items.len() >= WISH_MAX_ITEMS {
            self.toast(cx, "心愿单满了，最多 8 件");
            return;
        }
        self.wish_draft.items.push(WishItem::exact(i));
        self.wish_err = None;
        self.pop_back(cx);
    }

    // ---- 事件 ----

    pub(crate) fn handle_wish_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let today = today_days();

        // 挑礼页 / 我的心愿单：卡片进详情
        if self.overlay.is_none() && self.tab == 0 {
            for (j, card) in WISH_CARDS.iter().enumerate() {
                if self.clicked(cx, &[*card, live_id!(wc_hit)], actions) {
                    if let Some(id) = self.friend_rows.get(j).copied() {
                        self.open_wish(cx, id);
                    }
                }
            }
        }
        if self.overlay == Some(Overlay::Wishes) {
            for (j, card) in MY_WISH_CARDS.iter().enumerate() {
                if self.clicked(cx, &[*card, live_id!(wc_hit)], actions) {
                    if let Some(id) = self.my_rows.get(j).copied() {
                        self.open_wish(cx, id);
                    }
                }
            }
            if self.clicked(cx, ids!(mw_new), actions)
                || self.clicked(cx, &[live_id!(mw_empty), live_id!(em_action)], actions)
            {
                self.open_wish_edit(cx, None, None);
            }
        }

        match self.overlay {
            Some(Overlay::Product) => {
                if self.clicked(cx, ids!(pd_send), actions) {
                    self.send_from_product(cx);
                } else if self.clicked(cx, ids!(pd_addwish), actions) {
                    self.add_product_to_wishlist(cx);
                }
                for (j, card) in REL_CARDS.iter().enumerate() {
                    if self.clicked(cx, &[*card, live_id!(pk_hit)], actions) {
                        if let Some(i) = self.rel_rows.get(j).copied() {
                            self.open_product(cx, i, self.pd_wish);
                        }
                    }
                }
            }
            Some(Overlay::Checkout) => {
                if self.co_paid.is_none() {
                    if self.clicked(cx, &[live_id!(co_bal_row), live_id!(sw_hit)], actions) {
                        self.draft.use_balance = !self.draft.use_balance;
                        self.co_err = None;
                        self.refresh_checkout(cx);
                    }
                    for (j, id) in PAY_CHIPS.iter().enumerate() {
                        if self.toggled(cx, &[*id], actions) {
                            self.draft.pay = Some(j as u8);
                            self.refresh_checkout(cx);
                        }
                    }
                    if self.clicked(cx, ids!(co_go), actions) {
                        self.pay_checkout(cx);
                    }
                } else {
                    if self.clicked(cx, ids!(co_card), actions) {
                        self.refresh_card(cx);
                        self.open_overlay(cx, Overlay::Card);
                    }
                    if self.clicked(cx, ids!(co_home), actions) {
                        let wish = self.co_paid.and_then(|id| self.state.gift(id)).and_then(|g| g.wish_id);
                        match wish {
                            Some(w) => {
                                self.gift_wish_seg = true;
                                self.set_tab(cx, 0);
                                self.open_wish(cx, w);
                            }
                            None => self.set_tab(cx, 0),
                        }
                    }
                }
            }
            Some(Overlay::Wish) => {
                let id = self.wish_id.unwrap_or(0);
                for (k, row) in WISH_ROWS.iter().enumerate() {
                    if self.clicked(cx, &[*row, live_id!(wr_go)], actions) {
                        let wi = self.state.wishlist(id).and_then(|w| w.items.get(k)).cloned();
                        match wi {
                            Some(wi) if wi.is_exact() => self.open_product(cx, wi.thumb(), Some((id, k))),
                            Some(_) => self.open_wish_pick(cx, Some((id, k))),
                            None => {}
                        }
                        return;
                    }
                }
                if self.clicked(cx, ids!(wd_edit), actions) {
                    self.open_wish_edit(cx, Some(id), None);
                    return;
                }
                if self.clicked(cx, ids!(wd_copy), actions) {
                    cx.copy_to_clipboard(&wish_link(id));
                    self.toast(cx, "心愿单链接已复制，发给熟人就能看");
                }
                if self.clicked(cx, ids!(wd_close), actions) {
                    self.wish_armed = 1;
                    self.refresh_wish(cx);
                }
                if self.clicked(cx, ids!(wd_del), actions) {
                    self.wish_armed = 2;
                    self.refresh_wish(cx);
                }
                if self.clicked(cx, ids!(wd_cno), actions) {
                    self.wish_armed = 0;
                    self.refresh_wish(cx);
                }
                if self.clicked(cx, ids!(wd_cyes), actions) {
                    self.wish_confirm(cx);
                    return;
                }
            }
            Some(Overlay::WishEdit) => self.handle_wish_edit(cx, actions, today),
            Some(Overlay::WishPick) => {
                if self.wp_wish.is_none() {
                    for (j, id) in PICK_CHIPS.iter().enumerate() {
                        if self.toggled(cx, &[*id], actions) {
                            self.wp_cat = j;
                            self.refresh_wish_pick(cx);
                        }
                    }
                }
                for (j, card) in PICK_CARDS.iter().enumerate() {
                    if self.clicked(cx, &[*card, live_id!(pk_hit)], actions) {
                        let Some(i) = self.wp_rows.get(j).copied() else { continue };
                        match self.wp_wish {
                            Some(at) => self.open_product(cx, i, Some(at)),
                            None => self.pick_for_draft(cx, i),
                        }
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_wish_edit(&mut self, cx: &mut Cx, actions: &Actions, today: i64) {
        let mut dirty = false;
        for (j, id) in OCC_CHIPS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.wish_draft.occasion = j as u8;
                dirty = true;
            }
        }
        let shift = if self.clicked(cx, ids!(we_dm), actions) {
            -1
        } else if self.clicked(cx, ids!(we_dp), actions) {
            1
        } else if self.clicked(cx, ids!(we_dw), actions) {
            7
        } else {
            0
        };
        if shift != 0 {
            let e = self.wish_draft.event_on + shift;
            self.wish_draft.event_on = e.clamp(today, today + WISH_MAX_AHEAD_DAYS);
            self.wish_err = None;
            dirty = true;
        }
        if self.clicked(cx, ids!(we_add_exact), actions) {
            self.open_wish_pick(cx, None);
            return;
        }
        if self.clicked(cx, ids!(we_add_vague), actions) {
            self.vague_open = true;
            dirty = true;
        }
        for (j, id) in KIND_CHIPS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.vague_kind = j;
                dirty = true;
            }
        }
        for (j, id) in BUDGET_CHIPS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.vague_budget = j;
                dirty = true;
            }
        }
        if self.view.text_input(cx, ids!(we_wants)).changed(actions).is_some() {
            dirty = true;
        }
        if self.clicked(cx, ids!(we_v_ok), actions) {
            if self.wish_draft.items.len() >= WISH_MAX_ITEMS {
                self.wish_err = Some("一张心愿单最多 8 件");
            } else {
                let wants: String = self.input_text(cx, ids!(we_wants)).chars().take(WISH_WANTS_MAX_CHARS).collect();
                let max = WISH_BUDGETS[self.vague_budget] * 100;
                self.wish_draft.items.push(WishItem::vague(WISH_KINDS[self.vague_kind], max, &wants));
                self.wish_err = None;
                self.vague_open = false;
                self.set_text(cx, ids!(we_wants), "");
            }
            dirty = true;
        }
        if self.clicked(cx, ids!(we_v_no), actions) {
            self.vague_open = false;
            dirty = true;
        }
        for (k, row) in EDIT_ROWS.iter().enumerate() {
            if self.clicked(cx, &[*row, live_id!(wr_alt)], actions) {
                if self.wish_draft.items.get(k).is_some_and(|w| w.is_open()) {
                    self.wish_draft.items.remove(k);
                    self.wish_err = None;
                }
                dirty = true;
                break;
            }
        }
        if self.toggled(cx, ids!(aa0), actions) {
            self.wish_draft.audience.clear();
            dirty = true;
        }
        for (j, id) in AUD_CHIPS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                if let Some(n) = self.aud_names.get(j).cloned() {
                    let aud = &mut self.wish_draft.audience;
                    match aud.iter().position(|a| *a == n) {
                        Some(p) => {
                            aud.remove(p);
                        }
                        None => aud.push(n),
                    }
                }
                dirty = true;
            }
        }
        if self.clicked(cx, ids!(we_go), actions) {
            self.submit_wish(cx);
            return;
        }
        if dirty {
            self.refresh_wish_edit(cx);
        }
    }
}

/// 列表上的名字：说了个大概的后面标一个「大概」，和具体的一件分开。
fn wish_item_name(wi: &WishItem) -> String {
    if wi.is_exact() {
        wi.title()
    } else {
        format!("{} · 大概", wi.title())
    }
}
