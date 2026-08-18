/* 洪荒 · 步骤直播 · app.js v3.5
   依据：融合蓝图 §13.c 步骤流（2026-08-19 第四次推翻）
   端点：/api/tasks / /api/sessions/{id}/steps / /api/stream (SSE) */

const $ = (s) => document.querySelector(s);

const state = { tasks: [], filterText: "", filterStatus: "", selectedTaskId: null, steps: [], expandedSteps: {}, events: 0, rateEvents: [], es: null };

function fmtTs(ts) {
    if (!ts) return "--:--:--";
    const d = new Date(ts);
    const p = (n) => String(n).padStart(2, "0");
    return p(d.getMonth() + 1) + "-" + p(d.getDate()) + " " + p(d.getHours()) + ":" + p(d.getMinutes()) + ":" + p(d.getSeconds());
}

function fmtDuration(ms) {
    if (ms < 1000) return ms + "ms";
    const s = Math.floor(ms / 1000);
    if (s < 60) return s + "s";
    const m = Math.floor(s / 60);
    return m + "m" + (s % 60) + "s";
}

function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function summarize_comp(a) {
    // 返回 { kind, lines: [htmlString] } —— 不再截短, 全量展示
    const t = a["类型"];
    const 动作 = a["动作"] || "?";
    const full = a["全量"];
    if (t === "LLM") {
        const msgs = (full && full.载荷 && full.载荷.messages) || [];
        const lines = [];
        for (let i = 0; i < msgs.length; i++) {
            const m = msgs[i];
            const role = m.role || "?";
            let c = m.content || "";
            if (Array.isArray(c)) c = c.map(x => x.text || "").join(" ");
            lines.push('<div class="msg">' +
                '<span class="msg-role msg-role-' + role + '">' + escapeHtml(role) + '</span>' +
                '<span class="msg-body">' + escapeHtml(String(c)) + '</span>' +
                '</div>');
        }
        return { kind: 'llm', lines: lines, label: msgs.length + ' 消息' };
    }
    if (t === "工具") {
        const inner = (full && full.载荷 && full.载荷.parsed) || null;
        const 附加 = (full && full.载荷 && full.载荷.附加) || null;
        const isCall = (a["动作"] || "").includes("调用");
        const lines = [];
        if (isCall) {
            lines.push('<div class="msg">调用参数</div>');
            const params = (附加 && typeof 附加 === "object") ? 附加 : (inner && typeof inner === "object" ? inner : null);
            if (params) {
                lines.push('<pre class="msg-body-pre">' + escapeHtml(JSON.stringify(params, null, 2)) + '</pre>');
            } else {
                lines.push('<pre class="msg-body-pre">' + escapeHtml(动作) + '</pre>');
            }
        } else {
            lines.push('<div class="msg">返回内容</div>');
            const result = (inner && typeof inner === "object") ? inner : (附加 && typeof 附加 === "object" ? 附加 : null);
            if (result) {
                lines.push('<pre class="msg-body-pre">' + escapeHtml(JSON.stringify(result, null, 2)) + '</pre>');
            } else {
                // 实在没东西就给原始
                lines.push('<pre class="msg-body-pre">' + escapeHtml(动作) + '</pre>');
            }
        }
        return { kind: 'tool', lines: lines, label: isCall ? "调用" : "返回" };
    }
    return { kind: 'other', lines: ['<pre class="msg-body-pre">' + escapeHtml(动作) + '</pre>'], label: t };
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
        row.addEventListener("click", () => selectTask(t.id));
        list.appendChild(row);
    }
}

async function selectTask(id) {
    state.selectedTaskId = id;
    $("#cur-task").textContent = id;
    renderTasks();
    await loadSteps(id);
}

async function loadSteps(id) {
    try {
        const r = await fetch("/api/sessions/" + encodeURIComponent(id) + "/steps");
        if (r.status === 404) {
            $("#steps-stream").innerHTML = '<div class="step-empty">任务 ' + escapeHtml(id) + ' 未找到</div>';
            return;
        }
        const o = await r.json();
        state.steps = o.steps || [];
        $("#task-meta").textContent = id + " · " + state.steps.length + " 步";
        $("#progress").textContent = state.steps.length + " 步";
        renderSteps();
    } catch (e) { console.error("steps", e); }
}

function renderSteps() {
    const wrap = $("#steps-stream");
    wrap.innerHTML = "";
    if (state.steps.length === 0) {
        wrap.innerHTML = '<div class="step-empty">暂无步骤 (等待世界执行)</div>';
        return;
    }
    const lastIdx = state.steps.length - 1;
    for (let i = 0; i < state.steps.length; i++) {
        const s = state.steps[i];
        const isLast = (i === lastIdx);
        const isCurrent = isLast;
        const isDone = !isLast;
        const card = document.createElement("div");
        card.className = "step-card" + (isCurrent ? " current" : "") + (isDone ? " done" : "");
        const expanded = !!state.expandedSteps[i];
        const elapsed = (s['结束 ts'] || s['开始 ts']) - s['开始 ts'];
        const head = document.createElement("div");
        head.className = "step-head";
        head.innerHTML =
            '<span class="step-num">#' + s['步骤号'] + '</span>' +
            '<span class="step-title">' + escapeHtml(s['标题'] || '(无标题)') + '</span>' +
            '<span class="step-meta">' +
                '<span>⏱ <b>' + fmtDuration(elapsed) + '</b></span>' +
                '<span>🎯 <b>' + (s['token sum'] || 0) + '</b> tok</span>' +
                '<span>LLM <b>' + (s['LLM 数'] || 0) + '</b></span>' +
                '<span>Tool <b>' + (s['Tool 数'] || 0) + '</b></span>' +
            '</span>' +
            '<span class="step-status">' + (isCurrent ? '▸' : (isDone ? '✓' : '○')) + '</span>';
        head.addEventListener("click", () => {
            state.expandedSteps[i] = !expanded;
            renderSteps();
        });
        card.appendChild(head);
        if (expanded) {
            const body = document.createElement("div");
            body.className = "step-body";
            for (const a of (s['组件'] || [])) {
                const comp = summarize_comp(a);
                const ts = a['ts'] || 0;
                const head = document.createElement('div');
                head.className = 'comp-head';
                head.innerHTML =
                    '<span class="comp-icon comp-icon-' + comp.kind + '">' + escapeHtml(comp.label) + '</span>' +
                    '<span class="comp-time">' + fmtTs(ts) + '</span>';
                body.appendChild(head);
                for (const line of (comp.lines || [])) {
                    const div = document.createElement('div');
                    div.className = 'comp-line';
                    div.innerHTML = line;
                    body.appendChild(div);
                }
            }
            card.appendChild(body);
        }
        wrap.appendChild(card);
    }
}

function connectSSE() {
    if (state.es) try { state.es.close(); } catch (e) {}
    state.es = new EventSource("/api/stream");
    $("#live-indicator").textContent = "🟢 直播中";
    state.es.onmessage = (msg) => {
        try {
            const p = JSON.parse(msg.data);
            state.events++;
            $("#footer-events").textContent = state.events;
            state.rateEvents.push(Date.now());
            while (state.rateEvents.length > 0 && Date.now() - state.rateEvents[0] > 5000) state.rateEvents.shift();
            $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1);
            if (p.type === "event" || p.type === "task-status-change" || p.type === "task-new") {
                if (p.type === "task-new" || p.type === "task-status-change") loadTasks();
                if (state.selectedTaskId === p.任务id) loadSteps(state.selectedTaskId);
            }
        } catch (e) { console.error("sse", e); }
    };
    state.es.onerror = () => { $("#live-indicator").textContent = "🔴 1s 重连"; };
}

document.addEventListener("DOMContentLoaded", () => {
    loadTasks();
    connectSSE();
    setInterval(() => { $("#footer-rate").textContent = (state.rateEvents.length / 5).toFixed(1); }, 1000);
    $("#search").addEventListener("input", (e) => { state.filterText = e.target.value; renderTasks(); });
    $("#status-filter").addEventListener("change", (e) => { state.filterStatus = e.target.value; renderTasks(); });
});