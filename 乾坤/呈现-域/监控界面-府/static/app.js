/* 洪荒 · 任务直播 · app.js · 依据 融合蓝图 §13
   状态机以 selectedTaskId 为中心：
     tasks:        来自 /api/tasks
     sessions[id]: 当前已加载的会话全量
     expanded:     任务 id -> bool（是否展开全量）
   SSE 三种 payload：event / task-new / task-status-change */

const $ = (s) => document.querySelector(s);
const $$ = (s) => document.querySelectorAll(s);

const state = {
    tasks: [],
    filterText: "",
    filterStatus: "",
    selectedTaskId: null,
    sessions: {},         // id -> {id, 状态, 动作们: [...] }
    expanded: {},        // idx -> bool
    detailOpen: null,
    events: 0,
    rateEvents: [],
    es: null,
};

function fmtTs(ts) {
    if (!ts) return "--:--:--";
    const d = new Date(ts);
    const p = (n) => String(n).padStart(2, "0");
    return p(d.getMonth() + 1) + "-" + p(d.getDate()) + " " +
        p(d.getHours()) + ":" + p(d.getMinutes()) + ":" + p(d.getSeconds());
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
            '<div class="id">' + t.id + '</div>' +
            '<div class="meta">' +
                '<span class="' + pillClass(t.状态) + '">' + (t.状态 || "?") + '</span>' +
                '<span class="pill">阶段:' + (t.阶段 || "?") + '</span>' +
                '<span class="pill">' + (t.类别 || "?") + '</span>' +
            '</div>' +
            '<div class="dir">' + escapeHtml(t.方向前 || "") + '</div>';
        row.addEventListener("click", () => selectTask(t.id));
        list.appendChild(row);
    }
}

function escapeHtml(s) {
    return String(s || "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
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
    } catch (e) {
        console.error("tasks", e);
    }
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
            $("#session-tree").innerHTML = '<div class="node-empty">任务 ' + escapeHtml(id) + ' 未找到</div>';
            $("#session-title").textContent = id + "（无）";
            return;
        }
        const o = await r.json();
        state.sessions[id] = o.session;
        $("#session-title").textContent = id + " · " + escapeHtml(o.session.状态 || "?") + " · " + escapeHtml(o.session.阶段 || "?") + " · " + escapeHtml(o.session.类别 || "?");
        renderSession(o.session);
    } catch (e) {
        console.error("session", e);
    }
}

function renderSession(sess) {
    const tree = $("#session-tree");
    tree.innerHTML = "";
    const summary = document.createElement("div");
    summary.style.padding = "8px";
    summary.style.borderLeft = "3px solid var(--accent)";
    summary.style.background = "rgba(56,189,248,0.06)";
    summary.style.marginBottom = "12px";
    summary.innerHTML =
        '<div style="font-family:inherit">' +
            '<b>' + escapeHtml(sess.id) + '</b>' +
            ' · 状态 <b>' + escapeHtml(sess.状态 || "?") + '</b>' +
            ' · 阶段 <b>' + escapeHtml(sess.阶段 || "?") + '</b>' +
            ' · 类别 <b>' + escapeHtml(sess.类别 || "?") + '</b>' +
        '</div>' +
        '<div style="margin-top:6px;color:var(--fg-mute);font-size:11px">方向: ' + escapeHtml((sess.方向 || "").slice(0, 240)) + '</div>' +
        '<div style="margin-top:4px;color:var(--fg-mute);font-size:11px">验收标准: ' + escapeHtml((sess.验收标准 || "").slice(0, 240)) + '</div>';
    tree.appendChild(summary);

    const actions = sess.动作们 || [];
    if (actions.length === 0) {
        const empty = document.createElement("div");
        empty.className = "node-empty";
        empty.textContent = "暂无动作（等待世界执行）";
        tree.appendChild(empty);
        return;
    }
    for (let i = 0; i < actions.length; i++) {
        const a = actions[i];
        const node = document.createElement("div");
        node.className = "node";
        node.dataset.idx = String(i);
        const expanded = !!state.expanded[sess.id + ":" + i];
        let detail = "";
        if (expanded) {
            const raw = a.全量 || "";
            try {
                const parsed = JSON.parse(raw);
                detail = JSON.stringify(parsed, null, 2);
            } catch (e) {
                detail = raw;
            }
        }
        node.innerHTML =
            '<div class="node-info">' +
                '<span class="node-time">' + fmtTs(a.ts) + '</span>' +
                '<span class="node-type type-' + a.类型 + '">' + a.类型 + '</span>' +
                '<span class="node-act">' + escapeHtml(a.动作)
                    + '<small>' + escapeHtml(a.庭) + ' · token ' + (a.token || 0) + ' · ' + (a.耗时ms || 0) + 'ms</small>' +
                '</span>' +
                '<button class="btn-toggle">' + (expanded ? "收起" : "全量") + '</button>' +
            '</div>' +
            (expanded ? '<pre class="node-act" style="font-size:11px;color:var(--fg-mute);background:var(--bg);padding:10px;border-radius:6px;overflow-x:auto;white-space:pre-wrap;word-break:break-all;">' + escapeHtml(detail) + '</pre>' : '');
        node.querySelector(".btn-toggle").addEventListener("click", (e) => {
            e.stopPropagation();
            state.expanded[sess.id + ":" + i] = !expanded;
            renderSession(state.sessions[state.selectedTaskId]);
        });
        tree.appendChild(node);
    }
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
            if (p.type === "event") {
                const ev = p.ev || {};
                const tid = p.任务id || (ev._task_id || "");
                if (tid && tid === state.selectedTaskId && state.sessions[tid]) {
                    state.sessions[tid].动作们.push({
                        ts: ev.ts, 类型: (ev.动作 || "").startsWith("要求 ") ? "入队" : ((ev.源 || "").includes("验收") ? "验收" : "实现"),
                        庭: ev.源 || "?", 动作: ev.动作 || "?", token: (ev.token || {}).总计 || 0, 耗时ms: ev.耗时ms || 0,
                        影响: ev.影响 || [], 证据: ev.证据 || "", 全量: ev._raw || "",
                    });
                    renderSession(state.sessions[tid]);
                }
            } else if (p.type === "task-new") {
                loadTasks();
            } else if (p.type === "task-status-change") {
                loadTasks();
                if (state.selectedTaskId === p.任务id) loadSession(p.任务id);
            }
        } catch (e) {
            console.error("sse", e);
        }
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
    $("#detail-close").addEventListener("click", () => {
        $("#detail-drawer").classList.add("hidden");
    });
});