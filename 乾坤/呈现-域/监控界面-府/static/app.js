// 洪荒 · 步骤直播 · v3
// 设计稿：监控界面-府/README.md §步骤直播视图(2026-08-19)
// 单源订阅 `.上下文/事件流.jsonl`；首屏拉 /api/events/recent，SSE 接 /api/events/stream。

const $ = (s) => document.querySelector(s);

const state = {
    tasks: [],
    filterText: "",
    filterStatus: "",
    selectedTaskId: null,
    activeTab: "timeline",   // timeline | tree
    recent: [],              // 首屏拉回的事件（已按 ts desc）
    live: [],                // SSE 接到的活事件
    tree: {},                // 任务id → 事件数组
    treeOrder: [],           // 任务id 顺序（最近活动优先）
    es: null,
    rateEvents: [],
    openTasks: new Set(),    // 任务树里展开的任务
};

// ===== 工具 =====
const TASK_ID_RE = /(要求-\d+)/;

function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function fmtRelTime(ts) {
    if (!ts) return "--:--";
    const diff = (Date.now() - ts) / 1000;
    if (diff < 60) return Math.floor(diff) + "s前";
    if (diff < 3600) return Math.floor(diff / 60) + "分前";
    return Math.floor(diff / 3600) + "小时前";
}

function inferTaskId(ev) {
    if (!ev || typeof ev !== "object") return null;
    const p = ev.载荷 || {};
    if (typeof p.要求id === "string") return p.要求id;
    if (typeof p.id === "string" && TASK_ID_RE.test(p.id)) return p.id;
    if (typeof p.想法id === "string" && /^要求-/.test(p.想法id)) return p.想法id;
    return null;
}

function inferDir(ev) {
    const p = ev.载荷 || {};
    return p.方向 || p.摘要 || p.内容 || "";
}

// 渲染契约：返回 { kind:'card'|'row', level:'重点'|'细行', prefix, body, detail? }
function renderShape(ev) {
    const 类型 = ev.类型 || "?";
    const 载荷 = ev.载荷 || {};
    const detailJson = JSON.stringify(载荷, null, 2);

    switch (类型) {
        case "验收结论": {
            const ok = 载荷.结论 === "通过";
            const tid = 载荷.要求id || "";
            return {
                kind: "card", level: "重点",
                prefix: ok ? "✓" : "✗",
                cls: ok ? "验收✓" : "验收✗",
                body: `${escapeHtml(tid)} 结论=${escapeHtml(载荷.结论 || "?")} · 尝试=${载荷.尝试 ?? 0}`,
                detail: detailJson,
            };
        }
        case "失败沉淀":
            return {
                kind: "card", level: "重点",
                prefix: "⚠", cls: "失败",
                body: `${escapeHtml(载荷.要求id || "")} 尝试=${载荷.尝试 ?? 0} · ${escapeHtml(载荷.终裁依据 || "")}`,
                detail: detailJson,
            };
        case "版本存档":
            return {
                kind: "card", level: "重点",
                prefix: "⚓", cls: "定档",
                body: `${escapeHtml(载荷.版本号 || "?")} · ${escapeHtml(载荷.说明 || "")}`,
                detail: detailJson,
            };
        case "设计上呈":
            return {
                kind: "card", level: "重点",
                prefix: "◈", cls: "设计",
                body: `${escapeHtml(载荷.要求id || "")} 拆解=${载荷.拆解数 ?? 0} · ${escapeHtml(载荷.摘要 || "")}`,
                detail: detailJson,
            };
        case "工具调用": {
            const ok = 载荷.失败 ? "失败" : "ok";
            return {
                kind: "row", level: "细行",
                prefix: "·", cls: "可展",
                body: `[${ok}] ${escapeHtml(载荷.工具 || "?")} · 轮 ${载荷.轮次 ?? 0} · ${escapeHtml(String(载荷.参数 || "").slice(0, 80))}`,
                detail: detailJson,
            };
        }
        case "想法投递":
            return {
                kind: "row", level: "细行",
                prefix: "·", cls: "可展",
                body: `[想法] ${escapeHtml(String(载荷.内容 || "").slice(0, 80))}`,
                detail: detailJson,
            };
        case "要求入池":
            return {
                kind: "row", level: "细行",
                prefix: "·", cls: "可展",
                body: `[入池] ${escapeHtml(载荷.id || "")} · ${escapeHtml(载荷.状态 || "")}`,
                detail: detailJson,
            };
        case "要求状态推进":
        case "想法状态推进":
            return {
                kind: "row", level: "细行",
                prefix: "·", cls: "可展",
                body: `[推进] ${escapeHtml(载荷.要求id || 载荷.想法id || "")} → ${escapeHtml(载荷.状态 || "")} · ${escapeHtml(载荷.说明 || "")}`,
                detail: detailJson,
            };
        default:
            return {
                kind: "row", level: "细行",
                prefix: "·", cls: "可展",
                body: `[${escapeHtml(类型)}] ${escapeHtml(detailJson.slice(0, 100))}`,
                detail: detailJson,
            };
    }
}

function buildEventNode(ev, idx) {
    const sh = renderShape(ev);
    const node = document.createElement("div");
    if (sh.kind === "card") {
        node.className = "evt-card " + (sh.cls || "");
        if (sh.level === "重点") node.classList.add("重点");
        node.innerHTML =
            '<span class="evt-prefix">' + escapeHtml(sh.prefix) + '</span>' +
            '<span class="evt-time">' + escapeHtml(fmtRelTime(ev.时间戳)) + '</span>' +
            '<span class="evt-kind">' + escapeHtml(ev.类型 || "?") + '</span>' +
            '<span class="evt-body">' + sh.body + '</span>';
    } else {
        node.className = "evt-row " + (sh.cls || "");
        node.innerHTML =
            '<span class="evt-prefix">' + escapeHtml(sh.prefix) + '</span>' +
            '<span class="evt-time">' + escapeHtml(fmtRelTime(ev.时间戳)) + '</span>' +
            '<span class="evt-kind">' + escapeHtml(ev.类型 || "?") + '</span>' +
            '<span class="evt-body">' + sh.body + '</span>' +
            '<div class="evt-detail">' + escapeHtml(sh.detail || "") + '</div>';
        node.addEventListener("click", () => node.classList.toggle("open"));
    }
    node.dataset.idx = String(idx);
    return node;
}

// ===== 时间线渲染 =====
function renderTimeline() {
    const wrap = $("#timeline-view");
    if (state.recent.length === 0 && state.live.length === 0) {
        wrap.innerHTML = '<div class="evt-empty">暂无事件（等待世界执行）</div>';
        return;
    }
    // 全部事件按时间倒序，最新在前 → DOM 顺序中最新 = firstChild
    // 但你要"新内容在下、默认显示最新" → 把最新 append 到 wrap 末尾，wrap.scrollTop 锁到 max
    // 先清空，统一倒序重渲（first=distant, last=latest）
    wrap.innerHTML = "";
    const all = state.live.concat(state.recent); // live 在前，recent 在后，但二者都需要按 ts desc 排
    all.sort((a, b) => (b.时间戳 || 0) - (a.时间戳 || 0));
    all.forEach((ev, idx) => wrap.appendChild(buildEventNode(ev, idx)));
    // 视口贴底
    wrap.scrollTop = wrap.scrollHeight;
}

function renderTimelineAppend(newEv) {
    // 新事件应该贴底（latest）。在 normal DOM 顺序下就是 appendChild。
    const wrap = $("#timeline-view");
    const empty = wrap.querySelector(".evt-empty");
    if (empty) empty.remove();
    // 找正确位置：按 ts desc 已有尾部，要插到"比自己旧的"之前
    const node = buildEventNode(newEv, 0);
    let inserted = false;
    for (const child of Array.from(wrap.children)) {
        const t = Number(child.dataset.ts || 0);
        if (t < newEv.时间戳) {
            wrap.insertBefore(node, child);
            inserted = true;
            break;
        }
    }
    if (!inserted) wrap.appendChild(node);
    // 视口贴底
    wrap.scrollTop = wrap.scrollHeight;
}

// ===== 任务树渲染 =====
function pushToTree(ev) {
    const tid = inferTaskId(ev);
    if (!tid) return;
    if (!state.tree[tid]) {
        state.tree[tid] = [];
        state.treeOrder.unshift(tid);
    }
    state.tree[tid].push(ev);
    if (!state.openTasks.has(tid)) {
        // 默认折叠。仅在 aside 点击某条时才展开。
    }
}

function renderTree() {
    const wrap = $("#tree-view");
    if (state.treeOrder.length === 0) {
        wrap.innerHTML = '<div class="evt-empty">暂无任务事件（等待世界写入事件流）</div>';
        return;
    }
    wrap.innerHTML = "";
    for (const tid of state.treeOrder) {
        const evs = state.tree[tid] || [];
        const card = document.createElement("div");
        card.className = "tree-card" + (state.openTasks.has(tid) ? " open" : "");
        const latest = evs[evs.length - 1] || {};
        const head = document.createElement("div");
        head.className = "tree-head";
        const dir = inferDir(latest) || (latest.载荷 && latest.载荷.方向) || "";
        head.innerHTML =
            '<span class="tree-arrow">' + (state.openTasks.has(tid) ? "▾" : "▸") + '</span>' +
            '<span class="tree-id">' + escapeHtml(tid) + '</span>' +
            '<span class="tree-dir">' + escapeHtml(String(dir).slice(0, 60)) + '</span>' +
            '<span class="tree-status">' + escapeHtml(evs.length) + ' 步</span>';
        head.addEventListener("click", () => {
            if (state.openTasks.has(tid)) state.openTasks.delete(tid);
            else state.openTasks.add(tid);
            renderTree();
        });
        card.appendChild(head);
        const body = document.createElement("div");
        body.className = "tree-body";
        // 步骤倒序：最新在上（呼应"最新浮上来"）
        const sorted = [...evs].sort((a, b) => (b.时间戳 || 0) - (a.时间戳 || 0));
        sorted.forEach(ev => body.appendChild(buildEventNode(ev, 0)));
        card.appendChild(body);
        card.dataset.tid = tid;
        wrap.appendChild(card);
    }
}

function selectTaskAndAnchor(id) {
    state.selectedTaskId = id;
    // 切到任务树 tab
    if (state.activeTab !== "tree") switchTab("tree");
    // 展开该任务
    state.openTasks.add(id);
    renderTree();
    // 锚定
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
function pushEvent(ev) {
    state.live.unshift(ev);  // 最新在 head，但时间线渲染按 ts desc 排序
    state.live = state.live.slice(0, 200);
    state.events = (state.events || 0) + 1;
    $("#footer-events").textContent = state.events;
    $("#footer-events-foot").textContent = state.events;
    state.rateEvents.push(Date.now());
    while (state.rateEvents.length > 0 && Date.now() - state.rateEvents[0] > 5000) state.rateEvents.shift();
    $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1);
    pushToTree(ev);
}

function connectSSE() {
    if (state.es) try { state.es.close(); } catch (e) {}
    state.es = new EventSource("/api/events/stream");
    $("#live-indicator").textContent = "🟢 直播中";
    state.es.onmessage = (msg) => {
        try {
            const p = JSON.parse(msg.data);
            if (p.type === "event" && p.ev) {
                pushEvent(p.ev);
                // 时间线只渲染最近的 + 当前视图
                renderTimeline();
                // 如果当前是任务树 tab，重渲一次
                if (state.activeTab === "tree") renderTree();
                // 任务列表也可能要重渲
                loadTasks();
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
    // 1) 拉最近 N 条
    try {
        const r = await fetch("/api/events/recent?limit=200");
        const o = await r.json();
        state.recent = o.events || [];
        // recent 是 ts desc，逐条塞进树
        for (const ev of state.recent) pushToTree(ev);
    } catch (e) {
        console.error("recent", e);
        state.recent = [];
    }
    renderTimeline();
    renderTree();
    await loadTasks();
    connectSSE();

    // tab 绑定
    document.querySelectorAll(".tab").forEach(b => b.addEventListener("click", () => switchTab(b.dataset.tab)));
    // 搜索/筛选
    $("#search").addEventListener("input", (e) => { state.filterText = e.target.value; renderTasks(); });
    $("#status-filter").addEventListener("change", (e) => { state.filterStatus = e.target.value; renderTasks(); });
    // 流速显示
    setInterval(() => { $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1); }, 1000);
}

document.addEventListener("DOMContentLoaded", boot);
