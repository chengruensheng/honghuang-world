/* 洪荒 · 任务直播 · app.js v3
   依据：融合蓝图 §13 + §13.b
   UI 核心：每条动作是一个 <details>，点 summary 直接展开全量
   三层数据：
     /api/tasks                   → 任务列表
     /api/sessions/{id}          → 该任务 L2 高层动作 + L3/L4 子节点（如果存在）
     /api/sessions/{id}/thoughts → 仅 LLM 调用（可独立折叠层）
     /api/sessions/{id}/tools    → 仅工具调用 */

const $ = (s) => document.querySelector(s);

const state = {
    tasks: [],
    filterText: "",
    filterStatus: "",
    selectedTaskId: null,
    sessions: {},
    events: 0,
    rateEvents: [],
    es: null,
};

function fmtTs(ts) {
    if (!ts) return "--:--:--";
    const d = new Date(ts);
    const p = (n) => String(n).padStart(2, "0");
    return p(d.getMonth() + 1) + "-" + p(d.getDate()) + " " + p(d.getHours()) + ":" + p(d.getMinutes()) + ":" + p(d.getSeconds());
}

function escapeHtml(s) {
    return String(s == null ? "" : s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

function formatLLMContent(raw) {
    // 把 LLM 载荷里嵌套的 messages 数组转成可读
    // 多轮 + system prompt 展示
    try {
        const m = JSON.parse(raw);
        const msgs = m.messages || [];
        let out = "模型: " + escapeHtml(m.model || "?") + "\n";
        out += "max_tokens: " + escapeHtml(m.max_tokens || "?") + "\n";
        out += "消息数: " + msgs.length + "\n\n";
        msgs.forEach((mm, i) => {
            out += "── " + (i + 1) + ". [" + escapeHtml(mm.role) + "] ──\n";
            const c = mm.content || "";
            if (typeof c === "string") {
                out += escapeHtml(c.slice(0, 500));
                if (c.length > 500) out += "\n... (共 " + c.length + " 字)";
                out += "\n\n";
            } else {
                out += escapeHtml(JSON.stringify(c, null, 2));
                out += "\n\n";
            }
        });
        return out;
    } catch (e) {
        return escapeHtml(raw);
    }
}

function prettyJson(raw) {
    // 尝试把 raw 解析为 JSON 后漂亮输出，失败则原样输出（mask 过长）
    if (!raw) return "(空)";
    try {
        const o = JSON.parse(raw);
        return JSON.stringify(o, null, 2);
    } catch (e) {
        return raw.length > 2000 ? raw.slice(0, 2000) + "\n... (截断)" : raw;
    }
}

function pillClass(状态) {
    return "pill pill-" + (状态 || "?");
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
                '<span class="' + pillClass(t.状态) + '">' + escapeHtml(t.状态 || "?") + '</span>' +
                '<span class="pill">阶段:' + escapeHtml(t.阶段 || "?") + '</span>' +
                '<span class="pill">' + escapeHtml(t.类别 || "?") + '</span>' +
            '</div>' +
            '<div class="dir">' + escapeHtml(t.方向前 || "") + '</div>';
        row.addEventListener("click", () => selectTask(t.id));
        list.appendChild(row);
    }
}

async function loadTasks() {
    try {
        const r = await fetch("/api/tasks");
        const o = await r.json();
        state.tasks = o.tasks || [];
        $("#task-count").textContent = state.tasks.length + "/" + state.tasks.length;
        $("#footer-tasks").textContent = state.tasks.length;
        renderTasks();
        if (!state.selectedTaskId && state.tasks.length > 0) {
            const active = state.tasks.find(t => t.状态 === "实现中") || state.tasks.find(t => t.状态 === "待领") || state.tasks[0];
            if (active) selectTask(active.id);
        }
    } catch (e) { console.error("tasks", e); }
}

async function selectTask(id) {
    state.selectedTaskId = id;
    $("#cur-task").textContent = id;
    renderTasks();
    await loadSession(id);
}

async function loadSession(id) {
    try {
        const enc = encodeURIComponent(id);
        const r = await fetch("/api/sessions/" + enc);
        if (r.status === 404) {
            $("#session-tree").innerHTML = '<div class="task-empty">任务 ' + escapeHtml(id) + ' 未找到</div>';
            $("#session-title").textContent = id + "（无）";
            return;
        }
        const o = await r.json();
        state.sessions[id] = o.session;
        renderSession(o.session);
    } catch (e) { console.error("session", e); }
}

function buildActionNode(a, idx) {
    // 单个 L2/L3/L4 动作的 <details> 节点。点 summary 直接展开
    const detail = document.createElement("details");
    detail.className = "action action-" + (a.层 || "l2") + " type-" + (a.类型 || "其他");
    detail.dataset.idx = String(idx);

    const summary = document.createElement("summary");
    summary.innerHTML =
        '<span class="time">' + fmtTs(a.ts) + '</span>' +
        '<span class="type-pill type-' + (a.类型 || "其他") + '">' + escapeHtml(a.类型 || "其他") + '</span>' +
        '<span class="yuan">' + escapeHtml(a.庭 || "?") + '</span>' +
        '<span class="act">' + escapeHtml(a.动作 || "") + '</span>' +
        '<span class="tok">' + (a.token || 0) + ' tok</span>' +
        '<span class="dur">' + (a.耗时ms || 0) + 'ms</span>' +
        '<span class="hint">▾</span>';
    detail.appendChild(summary);

    const full = document.createElement("div");
    full.className = "full";
    let content = "";
    if (a.全量) {
        if (a.层 === "llm") {
            content = formatLLMContent(a.全量);
        } else {
            content = prettyJson(a.全量);
        }
    }
    const pre = document.createElement("pre");
    pre.className = "full-body";
    pre.textContent = content;
    full.appendChild(pre);

    if (a.影响 && a.影响.length) {
        const meta = document.createElement("div");
        meta.className = "full-meta";
        meta.textContent = "影响: " + JSON.stringify(a.影响);
        full.appendChild(meta);
    }
    if (a.证据) {
        const ev = document.createElement("div");
        ev.className = "full-evidence";
        ev.textContent = "证据: " + a.证据;
        full.appendChild(ev);
    }
    detail.appendChild(full);
    return detail;
}

function renderSession(sess) {
    const tree = $("#session-tree");
    tree.innerHTML = "";

    const title = document.createElement("div");
    title.className = "session-title";
    title.innerHTML =
        '<span class="id-tag">' + escapeHtml(sess.id) + '</span>' +
        '<span class="' + pillClass(sess.状态) + '">' + escapeHtml(sess.状态 || "?") + '</span>' +
        '<span class="pill">阶段:' + escapeHtml(sess.阶段 || "?") + '</span>' +
        '<span class="pill">' + escapeHtml(sess.类别 || "?") + '</span>';
    tree.appendChild(title);

    const direction = document.createElement("details");
    direction.className = "session-meta";
    direction.innerHTML =
        '<summary>方向 + 验收标准 + 约束（点击展开）</summary>' +
        '<div class="meta-body">' +
            '<div><b>方向:</b> ' + escapeHtml(sess.方向 || "(空)") + '</div>' +
            '<div><b>验收标准:</b> ' + escapeHtml(sess.验收标准 || "(空)") + '</div>' +
            '<div><b>约束:</b> ' + escapeHtml(JSON.stringify(sess.约束 || {}, null, 2)) + '</div>' +
        '</div>';
    tree.appendChild(direction);

    const l2 = (sess.动作们 || []).filter(a => !a.层 || a.层 === "l2");
    const l34 = (sess.动作们 || []).filter(a => a.层 && a.层 !== "l2");

    if (l2.length === 0 && l34.length === 0) {
        const empty = document.createElement("div");
        empty.className = "task-empty";
        empty.textContent = "暂无动作（等待世界执行）";
        tree.appendChild(empty);
        return;
    }

    const l2wrap = document.createElement("div");
    l2wrap.className = "actions-wrap";
    const l2head = document.createElement("div");
    l2head.className = "section-head";
    l2head.textContent = "动作列表 · 共 " + (l2.length + l34.length) + " 条（L2 = " + l2.length + ", L3/L4 = " + l34.length + "）";
    l2wrap.appendChild(l2head);
    for (let i = 0; i < l2.length; i++) {
        const node = buildActionNode(l2[i], i);
        l2wrap.appendChild(node);

        if (l2[i].类型 === "实现" || l2[i].类型 === "工具循环" || /实现|设计/.test(l2[i].动作 || "")) {
            const subs = l34.filter(c => c.父idx === i);
            if (subs.length > 0) {
                const sub = document.createElement("div");
                sub.className = "children";
                for (const c of subs) {
                    sub.appendChild(buildActionNode(c, -1));
                }
                l2wrap.appendChild(sub);
            }
        }
    }
    tree.appendChild(l2wrap);
}

function connectSSE() {
    if (state.es) try { state.es.close(); } catch (e) {}
    state.es = new EventSource("/api/stream");
    $("#live-indicator").textContent = "🟢 直播中";
    $("#live-state2").textContent = "直播";
    state.es.onmessage = (msg) => {
        try {
            const p = JSON.parse(msg.data);
            state.events++;
            $("#footer-events").textContent = state.events;
            state.rateEvents.push(Date.now());
            while (state.rateEvents.length > 0 && Date.now() - state.rateEvents[0] > 5000) state.rateEvents.shift();
            $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1);
            if (p.type === "event" && state.selectedTaskId) {
                if (p.任务id === state.selectedTaskId) loadSession(p.任务id);
            } else if (p.type === "task-new" || p.type === "task-status-change") {
                loadTasks();
                if (state.selectedTaskId === p.任务id) loadSession(p.任务id);
            }
        } catch (e) { console.error("sse", e); }
    };
    state.es.onerror = () => {
        $("#live-indicator").textContent = "🔴 断了 1s 重连";
        $("#live-state2").textContent = "重连";
    };
}

document.addEventListener("DOMContentLoaded", () => {
    loadTasks();
    connectSSE();
    setInterval(() => {
        $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1);
    }, 1000);
    $("#search").addEventListener("input", (e) => {
        state.filterText = e.target.value;
        renderTasks();
    });
    $("#status-filter").addEventListener("change", (e) => {
        state.filterStatus = e.target.value;
        renderTasks();
    });
});