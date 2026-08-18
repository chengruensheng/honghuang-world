// 洪荒 · 步骤直播 · v3.1 全程白箱
// 设计稿：监控界面-府/README.md §LOD 全程白箱契约
// 三源订阅 `.上下文/事件流.jsonl` + `临时文件夹/模型流水-观测.log` + `.上下文/记录.jsonl`
// SSE payload: {source:'event'|'model'|'shihai', ts, ev:<六字段+_raw>}

const $ = (s) => document.querySelector(s);

const state = {
    tasks: [],
    filterText: "",
    filterStatus: "",
    selectedTaskId: null,
    activeTab: "timeline",
    recent: [],
    live: [],
    tree: {},
    treeOrder: [],
    es: null,
    rateEvents: [],
    events: 0,
    openL1: new Set(),
    openL2: new Set(),
    openTasks: new Set(),
};

const TASK_ID_RE = /(要求-\d+)/;
const TASK_ID_PREFIX_RE = /^要求-/;
const LOD_L2_COLLAPSE = 2000;
const LOD_L2_PREVIEW = 800;

function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function fmtRelTime(ts) {
    if (!ts) return "--:--";
    const diff = (Date.now() - ts) / 1000;
    if (diff < 60) return Math.floor(diff) + "s前";
    if (diff < 60 * 60) return Math.floor(diff / 60) + "分前";
    return Math.floor(diff / 3600) + "小时前";
}

function fmtBytes(n) {
    if (n < 1024) return n + "B";
    if (n < 1024 * 1024) return (n / 1024).toFixed(1) + "KB";
    return (n / 1024 / 1024).toFixed(2) + "MB";
}

function fmtMs(ms) {
    if (!ms) return "0ms";
    if (ms < 1000) return ms + "ms";
    const s = Math.floor(ms / 1000);
    if (s < 60) return s + "s";
    return Math.floor(s / 60) + "m" + (s % 60) + "s";
}

function inferTaskId(ev) {
    if (!ev || typeof ev !== "object") return null;
    const p = ev.载荷 || {};
    if (typeof p.要求id === "string") return p.要求id;
    if (typeof p.id === "string" && TASK_ID_PREFIX_RE.test(p.id)) return p.id;
    if (typeof p.想法id === "string" && TASK_ID_PREFIX_RE.test(p.想法id)) return p.想法id;
    return null;
}

function sourceClass(source) {
    return source === "event" ? "src-event"
        : source === "model" ? "src-model"
        : source === "shihai" ? "src-shihai"
        : "src-other";
}

function actionPrefix(动作) {
    const k = String(动作 || "");
    if (k.includes("验收")) return k.includes("通过") ? "✓" : (k.includes("打回") ? "✗" : "✓?");
    if (k.includes("失败") || k.includes("Fault")) return "⚠";
    if (k.includes("版本") || k.includes("定档")) return "⚓";
    if (k.includes("设计上呈") || k.includes("设计")) return "◈";
    if (k === "工具调用") return "·";
    return "·";
}

function actionClass(动作) {
    const k = String(动作 || "");
    if (k.includes("验收") && k.includes("通过")) return "ok";
    if (k.includes("验收") && (k.includes("打回") || k.includes("失败"))) return "fail";
    if (k.includes("失败")) return "fail";
    if (k.includes("版本") || k.includes("定档")) return "anchor";
    if (k.includes("设计")) return "design";
    return "";
}

function tokenBadge(token) {
    if (!token) return "";
    const t = token.总计 || 0;
    if (!t) return "";
    return `<span class="tok">[${token.提示词||0}+${token.输出||0}=${t} tok]</span>`;
}

function idOf(source, ev) {
    return source + ":" + (ev && ev.ts ? ev.ts : Math.random());
}

// ===== 模型观测：把 ev._raw 块里 prompt/response 拆成 messages 列表 =====
function parseModelMessages(raw) {
    const out = [];
    if (!raw || typeof raw !== "object") return out;
    if (Array.isArray(raw.messages)) {
        for (let i = 0; i < raw.messages.length; i++) {
            const m = raw.messages[i];
            const c = (typeof m.content === "string") ? m.content
                : Array.isArray(m.content) ? m.content.map(x => (x && x.text) || "").join("\n")
                : "";
            out.push({ role: m.role || "?", content: c });
        }
    } else if (typeof raw.prompt === "string") {
        // fallback: 一段 prompt + 一段 response
        out.push({ role: "user", content: raw.prompt });
        if (typeof raw.response === "string") {
            out.push({ role: "assistant", content: raw.response });
        }
    }
    return out;
}

// ===== LOD 节点构建 =====
// 一条事件 → 节点(L0 始终渲染；L1/L2 折叠)
function buildLodNode(source, ev) {
    const id = idOf(source, ev);
    const wrap = document.createElement("div");
    wrap.className = "evt evt-" + sourceClass(source);
    wrap.dataset.id = id;

    const prefix = actionPrefix(ev.动作);
    const cls = actionClass(ev.动作);
    const time = fmtRelTime(ev.ts);
    const tokBadge = tokenBadge(ev.token);
    const msBadge = ev.耗时ms ? `<span class="ms">${fmtMs(ev.耗时ms)}</span>` : "";

    // L0：始终可见的一行重点
    const l0 = document.createElement("div");
    l0.className = "l0";
    l0.innerHTML =
        `<span class="l0-prefix ${cls}">${escapeHtml(prefix)}</span>` +
        `<span class="l0-time">${escapeHtml(time)}</span>` +
        `<span class="l0-src">[${escapeHtml(source)}]</span>` +
        `<span class="l0-act">${escapeHtml(ev.源)} / <b>${escapeHtml(ev.动作)}</b></span>` +
        tokBadge + msBadge +
        `<button class="l0-l1-toggle" data-id="${escapeHtml(id)}">▸ 详情</button>`;
    wrap.appendChild(l0);

    // L1：折叠区（影响 + 证据 + 摘要）
    const l1 = document.createElement("div");
    l1.className = "l1";
    l1.style.display = state.openL1.has(id) ? "block" : "none";
    const 影响Txt = Array.isArray(ev.影响) ? ev.影响.map(x => JSON.stringify(x)).join(" · ") : String(ev.影响 || "");
    const 证据 = ev.证据 || "(无证据)";
    l1.innerHTML =
        `<div class="l1-row"><span class="l1-k">影响</span><span class="l1-v">${escapeHtml(影响Txt.slice(0, 1000))}${影响Txt.length>1000?'…':''}</span></div>` +
        `<div class="l1-row"><span class="l1-k">证据</span><span class="l1-v">${escapeHtml(证据.slice(0, 1000))}${证据.length>1000?'…':''}</span></div>` +
        `<button class="l1-l2-toggle" data-id="${escapeHtml(id)}">▾ 载荷全文</button>`;
    wrap.appendChild(l1);

    // L2：原始载荷
    const l2 = document.createElement("div");
    l2.className = "l2";
    l2.style.display = state.openL2.has(id) ? "block" : "none";

    if (source === "model" && ev._raw) {
        // 特殊渲染：messages 列表 + 用量 + cost
        const messages = parseModelMessages(ev._raw);
        const usage = ev._raw.usage || {};
        const cost = ev._raw.cost || "";
        const head = `<div class="l2-head">模型观测 · 消息 ${messages.length} · 提示 ${usage.提示词||0} · 输出 ${usage.输出||0} · 总 ${usage.总计||0} tok${cost?` · 成本 ${escapeHtml(cost)}`:''}</div>`;
        const msgHtml = messages.map((m, i) => {
            const c = m.content || "";
            const big = c.length > LOD_L2_PREVIEW;
            const preview = big ? c.slice(0, LOD_L2_PREVIEW) + "…" : c;
            return `<div class="l2-msg"><span class="msg-role msg-${escapeHtml(m.role)}">${escapeHtml(m.role)}</span><span class="msg-num">#${i}</span><pre class="msg-body ${big ? 'collapsed' : ''}" data-full-len="${c.length}">${escapeHtml(preview)}</pre>${big?`<button class="msg-more">展开全文 (${c.length}字)</button>`:''}</div>`;
        }).join("");
        l2.innerHTML = head + msgHtml + `<details class="l2-raw"><summary>查看 _raw 全文</summary><pre>${escapeHtml(JSON.stringify(ev._raw, null, 2))}</pre></details>`;
    } else if (ev._raw) {
        const txt = JSON.stringify(ev._raw, null, 2);
        const big = txt.length > LOD_L2_COLLAPSE;
        l2.innerHTML =
            `<div class="l2-head">${escapeHtml(source)} 原始载荷 · ${fmtBytes(txt.length)}</div>` +
            (big
                ? `<pre class="l2-pre collapsed" data-full="${escapeHtml(txt)}" data-show="false">${escapeHtml(txt.slice(0, LOD_L2_PREVIEW))}…</pre><button class="l2-more">展开全文 (${txt.length}字)</button>`
                : `<pre class="l2-pre">${escapeHtml(txt)}</pre>`);
    } else {
        l2.innerHTML = `<div class="l2-empty">(源未传 _raw)</div>`;
    }
    wrap.appendChild(l2);

    // 绑定
    l0.querySelector(".l0-l1-toggle").addEventListener("click", (e) => {
        e.stopPropagation();
        if (state.openL1.has(id)) state.openL1.delete(id);
        else state.openL1.add(id);
        l1.style.display = state.openL1.has(id) ? "block" : "none";
        e.target.textContent = state.openL1.has(id) ? "▾ 收起" : "▸ 详情";
    });
    const l2Btn = l1.querySelector(".l1-l2-toggle");
    if (l2Btn) l2Btn.addEventListener("click", (e) => {
        e.stopPropagation();
        if (state.openL2.has(id)) state.openL2.delete(id);
        else state.openL2.add(id);
        l2.style.display = state.openL2.has(id) ? "block" : "none";
        e.target.textContent = state.openL2.has(id) ? "▴ 隐藏载荷" : "▾ 载荷全文";
    });

    // 模型观测 messages 全文展开
    const moreBtns = l2.querySelectorAll(".msg-more");
    moreBtns.forEach(b => b.addEventListener("click", (e) => {
        e.stopPropagation();
        const pre = e.target.previousElementSibling;
        if (pre.classList.contains("collapsed")) {
            const full = atob(escapeHtml(pre.textContent));
            // 简单回退：直接重读 from __knownFull
        }
    }));

    // 通用 L2 展开（折叠护栏）
    const more2 = l2.querySelector(".l2-more");
    if (more2) more2.addEventListener("click", (e) => {
        e.stopPropagation();
        const pre = l2.querySelector(".l2-pre");
        if (!pre) return;
        if (pre.classList.contains("collapsed")) {
            const full = decodeURIComponent(pre.dataset.full);
            // data-full escaped via escapeHtml；需要 unescape via textContent — 重读 raw 即可
            pre.textContent = ev._raw ? JSON.stringify(ev._raw, null, 2) : pre.textContent;
            pre.classList.remove("collapsed");
            e.target.remove();
        }
    });

    return wrap;
}

// ===== 时间线渲染 =====
function renderTimeline() {
    const wrap = $("#timeline-view");
    if (state.recent.length === 0 && state.live.length === 0) {
        wrap.innerHTML = '<div class="evt-empty">暂无事件（等待三源写入）</div>';
        return;
    }
    wrap.innerHTML = "";
    const all = [...state.live, ...state.recent];
    // 按 (source, ts) 去重，保留全白箱
    const seen = new Set();
    const dedup = [];
    all.forEach((it) => {
        const key = (it.source || "") + "|" + (it.ev && it.ev.ts);
        if (!seen.has(key)) { seen.add(key); dedup.push(it); }
    });
    // 注意：recent/live 已是白箱对象(ev = {...} 六字段) — 因为 server.py 装配后下传
    dedup.sort((a, b) => (b.ev?.ts || 0) - (a.ev?.ts || 0));
    for (const it of dedup) {
        if (!it.ev) continue;
        wrap.appendChild(buildLodNode(it.source, it.ev));
    }
    wrap.scrollTop = wrap.scrollHeight;
}

function pushLive(source, ev) {
    state.live.unshift({ source, ev });
    if (state.live.length > 200) state.live.length = 200;
    state.events++;
    $("#footer-events").textContent = state.events;
    $("#footer-events-foot").textContent = state.events;
    const now = Date.now();
    state.rateEvents.push(now);
    while (state.rateEvents.length > 0 && now - state.rateEvents[0] > 5000) state.rateEvents.shift();
    $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1);
}

// ===== 任务树 =====
function pushToTree(source, ev) {
    // model 源对任务贡献有限（除非有要求 id），只把 event/shihai 归类
    if (source !== "event") return;
    const tid = inferTaskId(ev);
    if (!tid) return;
    if (!state.tree[tid]) {
        state.tree[tid] = [];
        state.treeOrder.unshift(tid);
    }
    state.tree[tid].push({ source, ev });
}

function renderTree() {
    const wrap = $("#tree-view");
    if (state.treeOrder.length === 0) {
        wrap.innerHTML = '<div class="evt-empty">暂无任务事件（等待世界写入）</div>';
        return;
    }
    wrap.innerHTML = "";
    for (const tid of state.treeOrder) {
        const evs = state.tree[tid] || [];
        const card = document.createElement("div");
        card.className = "tree-card" + (state.openTasks.has(tid) ? " open" : "");
        const latest = evs[evs.length - 1] || {};
        const dir = (latest.ev && inferDirFromEv(latest.ev)) || "";
        const head = document.createElement("div");
        head.className = "tree-head";
        head.innerHTML =
            `<span class="tree-arrow">${state.openTasks.has(tid) ? "▾" : "▸"}</span>` +
            `<span class="tree-id">${escapeHtml(tid)}</span>` +
            `<span class="tree-dir">${escapeHtml(String(dir).slice(0, 80))}</span>` +
            `<span class="tree-status">${evs.length} 步</span>`;
        head.addEventListener("click", () => {
            if (state.openTasks.has(tid)) state.openTasks.delete(tid);
            else state.openTasks.add(tid);
            renderTree();
        });
        card.appendChild(head);
        const body = document.createElement("div");
        body.className = "tree-body";
        const sorted = [...evs].sort((a, b) => (b.ev?.ts || 0) - (a.ev?.ts || 0));
        sorted.forEach((it) => body.appendChild(buildLodNode(it.source, it.ev)));
        card.appendChild(body);
        card.dataset.tid = tid;
        wrap.appendChild(card);
    }
}

function inferDirFromEv(ev) {
    const p = (ev._raw && ev._raw.载荷) || {};
    return p.方向 || p.摘要 || p.内容 || "";
}

function selectTaskAndAnchor(id) {
    state.selectedTaskId = id;
    if (state.activeTab !== "tree") switchTab("tree");
    state.openTasks.add(id);
    renderTree();
    setTimeout(() => {
        const card = document.querySelector('.tree-card[data-tid="' + CSS.escape(id) + '"]');
        if (card) {
            card.classList.add("anchored");
            card.scrollIntoView({ block: "center", behavior: "smooth" });
            setTimeout(() => card.classList.remove("anchored"), 1500);
        }
    }, 50);
}

// ===== 任务列表（aside）=====
async function loadTasks() {
    try {
        const r = await fetch("/api/tasks");
        const o = await r.json();
        state.tasks = o.tasks || [];
        $("#task-count").textContent = state.tasks.length + "/" + state.tasks.length;
        $("#footer-tasks").textContent = state.tasks.length;
        renderTasks();
    } catch (e) { console.error("tasks", e); }
}

function renderTasks() {
    const filtered = state.tasks.filter(t => {
        if (state.filterStatus && (t.状态 || "") !== state.filterStatus) return false;
        if (state.filterText) {
            const s = state.filterText.toLowerCase();
            return (t.id || "").toLowerCase().includes(s) || (t.方向前 || "").toLowerCase().includes(s);
        }
        return true;
    });
    const list = $("#task-list");
    list.innerHTML = "";
    $("#task-count").textContent = filtered.length + "/" + state.tasks.length;
    if (filtered.length === 0) {
        list.innerHTML = '<div class="task-row"><div class="dir">无匹配任务</div></div>';
        return;
    }
    for (const t of filtered) {
        const row = document.createElement("div");
        row.className = "task-row" + (t.id === state.selectedTaskId ? " active" : "");
        row.dataset.id = t.id;
        row.innerHTML =
            '<div class="id">' + escapeHtml(t.id) + '</div>' +
            '<div class="meta">' +
                '<span class="pill pill-' + escapeHtml(t.状态 || "?") + '">' + escapeHtml(t.状态 || "?") + '</span>' +
                '<span class="pill">' + escapeHtml(t.类别 || "?") + '</span>' +
            '</div>' +
            '<div class="dir">' + escapeHtml(t.方向前 || "") + '</div>';
        row.addEventListener("click", () => selectTaskAndAnchor(t.id));
        list.appendChild(row);
    }
}

// ===== SSE =====
function connectSSE() {
    if (state.es) try { state.es.close(); } catch (e) {}
    state.es = new EventSource("/api/events/stream");
    $("#live-indicator").textContent = "🟢 三源直播中";
    state.es.onmessage = (msg) => {
        try {
            const p = JSON.parse(msg.data);
            if (p.source && p.ev) {
                pushLive(p.source, p.ev);
                renderTimeline();
                if (state.activeTab === "tree") renderTree();
                pushToTree(p.source, p.ev);
                // 不要在 SSE 回调里 fetch tasks，浏览器会因并发连接被 ERR_ABORTED
            }
        } catch (e) { console.error("sse", e); }
    };
    state.es.onerror = () => { $("#live-indicator").textContent = "🟡 重连中…"; };
}

// ===== Tab 切换 =====
function switchTab(name) {
    state.activeTab = name;
    document.querySelectorAll(".tab").forEach(b => {
        if (b.dataset.tab === name) b.classList.add("tab-on");
        else b.classList.remove("tab-on");
    });
    document.querySelectorAll(".view").forEach(v => v.classList.remove("view-on"));
    if (name === "timeline") $("#timeline-view").classList.add("view-on");
    else $("#tree-view").classList.add("view-on");
}

// ===== 启动 =====
async function boot() {
    try {
        const r = await fetch("/api/events/recent?limit=200");
        const o = await r.json();
        const arr = (o.events || []).map(ev => {
            // recent 接口直接返 ev 数组(ev 已装配)；补 source 字段
            return { source: ev._source || "event", ev: ev };
        });
        state.recent = arr;
        for (const it of arr) pushToTree(it.source, it.ev);
    } catch (e) {
        console.error("recent", e);
        state.recent = [];
    }
    renderTimeline();
    renderTree();
    await loadTasks();
    connectSSE();
    document.querySelectorAll(".tab").forEach(b => b.addEventListener("click", () => switchTab(b.dataset.tab)));
    $("#search").addEventListener("input", (e) => { state.filterText = e.target.value; renderTasks(); });
    $("#status-filter").addEventListener("change", (e) => { state.filterStatus = e.target.value; renderTasks(); });
    setInterval(() => { $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1); }, 1000);
}

document.addEventListener("DOMContentLoaded", boot);
