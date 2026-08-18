#!/usr/bin/env python3
"""监控界面 · server.py · v2 任务为中心

依据：融合蓝图-设计稿.md §13 任务为中心 · 会话为单元（2026-08-19 第三次推翻）
保留：§11 全部 7 端点 + 事件六字段契约
新增：/api/tasks, /api/sessions/{id}  + SSE 三种 payload

标准库零依赖。
"""

import http.server
import json
import os
import re
import socketserver
import sys
import threading
import time
import urllib.parse
from pathlib import Path

DEFAULT_PORT = 8080
SCAN_INTERVAL_SEC = 0.2
EVENT_BUS_MAX = 10000
TASK_ID_RE = re.compile(r"要求-(\d+)")
PROJECT_ROOT = Path(__file__).resolve().parents[3]
STATE_DIR = PROJECT_ROOT / ".上下文"
ROOMS_FILE = Path(__file__).parent / "monitor.rooms.json"

EVENT_BUS = []
EVENT_LOCK = threading.Lock()
SHARED = {
    "settings": {"port": DEFAULT_PORT, "interval_sec": 1.5, "theme": "dark"},
    "rooms": [],
    "start_ms": int(time.time() * 1000),
}

# ===== 4 源装配器 =====

def assemble_event_from_obs(line):
    try:
        d = json.loads(line)
        grid = d.get("格位名", "?")
        return {
            "ts": int(d.get("时间戳", int(time.time()*1000))),
            "源": f"识海/格位:{grid}",
            "动作": f"铭记·写入格位 «{grid}»",
            "影响": [{"类型": "格位", "名": grid, "字节": len(str(d.get("内容", "")))}],
            "token": {"提示词": 0, "输出": 0, "缓存": 0, "总计": 0},
            "耗时ms": 0,
            "证据": d.get("证据", ""),
        }
    except Exception:
        return None

def _tx_要求(line):
    try:
        d = json.loads(line)
        d_id = d.get("id", "?")
        return {
            "ts": int(time.time() * 1000),
            "源": "天庭/状态/要求.jsonl",
            "动作": f"要求 {d_id} → 状态{d.get("状态", "?")}",
            "影响": [{"类型": "要求", "名": d_id, "变化": f"阶段{d.get("阶段","?")}"}],
            "token": {"提示词": 0, "输出": 0, "缓存": 0, "总计": 0},
            "耗时ms": 0,
            "证据": d.get("方向", "")[:80],
            "_raw_id": d_id,
            "_task_id": d_id,
            "_raw": line,
        }
    except Exception:
        return None

def _tx_验收(line):
    try:
        d = json.loads(line)
        rid = d.get("要求id", "?")
        return {
            "ts": int(time.time() * 1000),
            "源": "天庭/状态/验收.jsonl",
            "动作": f"验收 {rid} → 结论{d.get("结论","?")}",
            "影响": [{"类型": "验收", "名": rid, "变化": f"结论={d.get("结论","?")} 产物={len(d.get("产物",[]) or [])}"}],
            "token": {"提示词": 0, "输出": 0, "缓存": 0, "总计": 0},
            "耗时ms": int((d.get("耗时秒", 0) or 0) * 1000),
            "证据": d.get("验收意见", "")[:80],
            "_task_id": rid,
            "_raw": line,
        }
    except Exception:
        return None

def _tx_世界状态(line):
    try:
        d = json.loads(line)
        if d.get("项目档案"):
            a = d["项目档案"]
            scale = a.get("规模", {})
            return {
                "ts": int(a.get("接手时间", int(time.time()*1000))),
                "源": "天庭/状态/世界状态.jsonl",
                "动作": f"档案 · crate {scale.get("crate数","?")} · rs {scale.get("rs文件数","?")} · 基线 {a.get("基线版本","?")}",
                "影响": [{"类型": "档案", "名": "世界", "变化": a.get("成熟度","?")}],
                "token": {"提示词": 0, "输出": 0, "缓存": 0, "总计": 0},
                "耗时ms": 0,
                "证据": a.get("风格约定","")[:80],
            }
        return None
    except Exception:
        return None

def _tx_版本(line):
    try:
        d = json.loads(line)
        if not d:
            return None
        ver = d.get("版本号", "?")
        accept = d.get("验收结论", []) or []
        n_pass = sum(1 for a in accept if isinstance(a, dict) and a.get("结论") == "通过")
        return {
            "ts": int(d.get("时间", int(time.time()*1000))),
            "源": "天庭/状态/版本.jsonl",
            "动作": f"{ver} 入档 · 阶段{d.get("阶段","?")} · 通过{n_pass}/{len(accept)-n_pass}",
            "影响": [{"类型": "版本", "名": ver, "变化": f"阶段={d.get("阶段","?")}"}],
            "token": {"提示词": 0, "输出": 0, "缓存": 0, "总计": 0},
            "耗时ms": 0,
            "证据": d.get("改了什么", "")[:80],
        }
    except Exception:
        return None

def _tx_obs_dispatch(line):
    """观测/记录.jsonl 分发器：根据顶级 '接口' + '域' 字段分到 LLM/Tool"""
    try:
        d = json.loads(line)
        link = d.get("关联", {}) or {}
        tid = link.get("要求") or link.get("任务线")
        iface = d.get("接口", "") or ""
        域 = d.get("域", "") or ""
        ts = int(d.get("时间戳", int(time.time()*1000)))
        if 域 in {"提示词", "回复思考"} or "模型连接-府" in iface or "调用模型" in iface:
            return _llm_event(d, ts, tid, iface, 域, line)
        if 域 in {"工具调用", "工具返回"} or "道术施展-府" in iface or "工具循环" in iface:
            return _tool_event(d, ts, tid, iface, 域, line)
    except Exception:
        pass
    return None


def _llm_event(d, ts, tid, iface, 域, line):
    payload = d.get("载荷", {}) or {}
    cont = payload.get("内容", "")
    msg = json.loads(cont) if cont else {}
    msgs = msg.get("messages", []) or []
    last_user = ""
    for m in reversed(msgs):
        if m.get("role") == "user":
            last_user = (m.get("content") or "")
            break
    # 预解析嵌套：让前端不再是字符串化 JSON
    parsed = {
        "时间戳": d.get("时间戳"),
        "域": 域,
        "接口": iface,
        "角色": d.get("角色"),
        "载荷": {
            "模型": msg.get("model"),
            "max_tokens": msg.get("max_tokens"),
            "messages": msgs,
            "内容_raw": cont,
        },
        "关联": d.get("关联"),
    }
    return {
        "ts": ts,
        "源": "模型连接-府/LLM调用",
        "动作": f"{域} · {len(msgs)} msg · {msg.get('model','?')}",
        "影响": [{"类型":"llm调用","模型":msg.get("model","?"),"消息数":len(msgs),"角色":域,"尾部前60":last_user[:60]}],
        "token": {"提示词":0,"输出":0,"缓存":0,"总计":0},
        "耗时ms": 0,
        "证据": last_user[:80],
        "_task_id": tid,
        "_raw": line,
        "_parsed": parsed,
        "_role_kind": "llm",
        "层": "llm",
        "类型": "LLM",
    }


def _tool_event(d, ts, tid, iface, 域, line):
    payload = d.get("载荷", {}) or {}
    cont = payload.get("内容", "") or ""
    tool_name = iface.split("::")[-1] if "::" in iface else iface
    # 预解析嵌套：载荷.内容 可能是 JSON 字符串
    inner = None
    if cont:
        try:
            inner = json.loads(cont)
        except Exception:
            inner = None
    parsed = {
        "时间戳": d.get("时间戳"),
        "域": 域,
        "接口": iface,
        "角色": d.get("角色"),
        "载荷": {
            "parsed": inner if inner is not None else cont,
            "内容_raw": cont,
            "附加": payload.get("附加"),
        },
        "关联": d.get("关联"),
    }
    return {
        "ts": ts,
        "源": "道术施展-府/工具调用",
        "动作": f"{域} · {tool_name}",
        "影响": [{"类型":"工具调用","接口":iface,"工具": tool_name, "parsed": inner is not None}],
        "token": {"提示词":0,"输出":0,"缓存":0,"总计":0},
        "耗时ms": 0,
        "证据": (cont or "")[:120],
        "_task_id": tid,
        "_raw": line,
        "_parsed": parsed,
        "_role_kind": "tool",
        "层": "tool",
        "类型": "工具",
    }


def _classify_str(msgs):
    sys_user = sum(1 for m in msgs if m.get("role") == "system")
    user = sum(1 for m in msgs if m.get("role") == "user")
    asst = sum(1 for m in msgs if m.get("role") == "assistant")
    tool = sum(1 for m in msgs if m.get("role") == "tool")
    return f"sys{sys_user}/user{user}/asst{asst}/tool{tool}"
# ===== 文件轨迹 =====
SOURCES = []
STATE_TX = {
    "要求.jsonl": _tx_要求,
    "验收.jsonl": _tx_验收,
    "世界状态.jsonl": _tx_世界状态,
    "版本.jsonl": _tx_版本,
}

def build_sources():
    SOURCES.clear()
    obs = STATE_DIR / "观测" / "记录.jsonl"
    if obs.exists():
        SOURCES.append({"path": obs, "transformer": assemble_event_from_obs, "last_size": 0, "name": "观测/记录.jsonl"})
        # 同一文件的 LLM/Tool 装配（只看 载荷.接口 以区别）
        SOURCES.append({"path": obs, "transformer": _tx_obs_dispatch, "last_size": 0, "name": "观测/记录.jsonl(LLM/Tool)"})
    state_dir = STATE_DIR / "状态"
    if state_dir.exists():
        for fname, tf in STATE_TX.items():
            p = state_dir / fname
            if p.exists():
                SOURCES.append({"path": p, "transformer": tf, "last_size": 0, "name": "状态/" + fname})

def read_incremental(src):
    try:
        cur = src["path"].stat().st_size
        if cur == src["last_size"]:
            return 0
        with open(src["path"], "rb") as f:
            f.seek(src["last_size"])
            raw = f.read().decode("utf-8", errors="replace")
        new = [ln for ln in raw.split("\n") if ln.strip()]
        added = 0
        for ln in new:
            ev = src["transformer"](ln)
            if ev:
                append_event(ev)
                added += 1
        src["last_size"] = cur
        return added
    except FileNotFoundError:
        return 0
    except Exception:
        return 0

def append_event(ev):
    with EVENT_LOCK:
        EVENT_BUS.append(ev)
        if len(EVENT_BUS) > EVENT_BUS_MAX:
            del EVENT_BUS[:len(EVENT_BUS) - EVENT_BUS_MAX]


def _read_jsonl_lines(raw_text):
    """逐行解析，损坏行跳过，不抛错。"""
    out = []
    for ln in raw_text.split("\n"):
        s = ln.strip()
        if not s:
            continue
        try:
            out.append(json.loads(s))
        except Exception:
            continue
    return out


def 读事件流增量(自字节):
    """从字节偏移起读新行；返回 (新行列表, 新字节偏移)。"""
    try:
        if not EVENT_FOLLOW_PATH.exists():
            return [], 0
        size = EVENT_FOLLOW_PATH.stat().st_size
        if size <= 自字节:
            return [], size
        with open(EVENT_FOLLOW_PATH, "rb") as f:
            f.seek(自字节)
            raw = f.read().decode("utf-8", errors="replace")
        rows = _read_jsonl_lines(raw)
        return rows, size
    except Exception:
        return [], 自字节


def 读事件流最近(limit):
    """读最近 limit 条事件（按时间正序；调用方负责倒序）。"""
    try:
        if not EVENT_FOLLOW_PATH.exists():
            return [], 0
        size = EVENT_FOLLOW_PATH.stat().st_size
        with open(EVENT_FOLLOW_PATH, "r", encoding="utf-8", errors="replace") as f:
            text = f.read()
        rows = _read_jsonl_lines(text)
        if limit and len(rows) > limit:
            rows = rows[-limit:]
        return rows, size
    except Exception:
        return [], 0

# ===== 事件流 jsonl 单源 工具 =====
EVENT_FOLLOW_PATH = STATE_DIR / "事件流.jsonl"
EVENT_FOLLOW_LOCK = threading.Lock()
EVENT_FOLLOW_LAST_SIZE = 0
EVENT_FOLLOW_RECENT_LIMIT = 200

# ===== 会话索引（§13.5）=====
TASKS_RAW = []      # 要求.jsonl 全部行（按 id 倒序）
TASKS_LOCK = threading.Lock()

def load_tasks_raw():
    """一次性加载全部要求.jsonl 进内存（TASKS_RAW）。"""
    p = STATE_DIR / "状态" / "要求.jsonl"
    out = []
    if p.exists():
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                if line.strip():
                    try:
                        d = json.loads(line)
                        if d.get("id"):
                            out.append(d)
                    except Exception:
                        pass
    return out

def reload_tasks_raw():
    global TASKS_RAW
    with TASKS_LOCK:
        TASKS_RAW = load_tasks_raw()

def get_tasks_list():
    """返回任务列表摘要（按 id 倒序，最多 50）。"""
    with TASKS_LOCK:
        tasks = list(TASKS_RAW)
    # 按 id 数字倒序
    def num(d):
        m = TASK_ID_RE.search(d.get("id",""))
        return int(m.group(1)) if m else 0
    tasks.sort(key=num, reverse=True)
    out = []
    for d in tasks[:50]:
        sid = d.get("id", "?")
        out.append({
            "id": sid,
            "状态": d.get("状态", "?"),
            "阶段": d.get("阶段", "?"),
            "类别": d.get("类别", "?"),
            "方向前": (d.get("方向", "") or "")[:60],
            "ts": d.get("时间戳", int(time.time()*1000)),
        })
    return out

def _get_step_title(a):
    """从 LLM 事件提步骤标题。跳过首个 user（通常是 system prompt 模板），找任务关键词；找不到取末尾 80 字。"""
    parsed = a.get('全量')
    if isinstance(parsed, dict):
        msgs = parsed.get('载荷', {}).get('messages', [])
        if not msgs:
            msgs = parsed.get('载荷', {}).get('parsed', {}).get('messages', [])
        # 跳过首个 user（鸿钧主政注入的 system prompt + 项目记忆背景）
        start_idx = 1 if len(msgs) > 1 else 0
        # 优先找任务关键词
        for m in msgs[start_idx:]:
            if m.get('role') != 'user':
                continue
            content = m.get('content', '') or ''
            if isinstance(content, list):
                content = ' '.join(str(x.get('text', '')) for x in content if isinstance(x, dict))
            content = content.replace('\n', ' ').replace('\r', ' ').strip()
            for kw in ['【要求方向】', '【重点】', '重点：', '【任务】', '【指令】', '【问题】', '【职司】']:
                if kw in content:
                    idx = content.find(kw)
                    sub = content[idx:idx+80]
                    return sub.replace('\n', ' ').strip()
        # 找不到关键词: 末尾 80 字
        for m in reversed(msgs[start_idx:]):
            if m.get('role') == 'user':
                content = m.get('content', '') or ''
                if isinstance(content, list):
                    content = ' '.join(str(x.get('text', '')) for x in content if isinstance(x, dict))
                content = content.replace('\n', ' ').replace('\r', ' ').strip()
                if content:
                    return content[-80:] if len(content) > 80 else content
    return (a.get('动作') or '思考')[:60]

def build_steps(actions):
    """按 ts 升序遍历 actions，每条 LLM 触发新步骤；同 LLM 之后的 Tool/L2 归入当前步骤。"""
    steps = []
    cur = None
    for a in actions:
        t = a.get('层', 'l2')
        ts = a.get('ts', 0)
        if t == 'llm':
            if cur:
                steps.append(cur)
            cur = {'步骤号': len(steps) + 1, '标题': _get_step_title(a), '开始 ts': ts, '结束 ts': ts, '组件': [a], 'token sum': 0, 'LLM 数': 0, 'Tool 数': 0}
            tok = (a.get('token') or {}).get('总计', 0) if isinstance(a.get('token'), dict) else 0
            cur['token sum'] += tok
            cur['LLM 数'] += 1
        else:
            if cur is None:
                cur = {'步骤号': 1, '标题': '前序', '开始 ts': ts, '结束 ts': ts, '组件': [], 'token sum': 0, 'LLM 数': 0, 'Tool 数': 0}
            cur['组件'].append(a)
            cur['结束 ts'] = ts
            cur['Tool 数'] += 1
    if cur:
        steps.append(cur)
    return steps

def classify_action(ev):
    """把 ev 分类为 入队/设计/实现/验收/结果/定档/其他。
    优先用 ev 自带的 类型 字段（_llm_event/_tool_event 已设）；fallback 按源匹配。"""
    t = ev.get("类型")
    if t and t not in ("其他", ""):
        return t
    src = ev.get("源", "")
    action = ev.get("动作", "")
    if "要求.jsonl" in src and "→ 状态" in action:
        return "入队"
    if "验收.jsonl" in src and "结论打回" in action:
        return "结果"
    if "验收.jsonl" in src and "结论通过" in action:
        return "结果"
    if "验收.jsonl" in src:
        return "验收"
    if "版本.jsonl" in src:
        return "定档"
    if "观测" in src or "识海" in src:
        return "实现"
    return "其他"

def get_session(task_id):
    """返回单任务会话全量。"""
    with TASKS_LOCK:
        t_raw = next((d for d in TASKS_RAW if d.get("id") == task_id), None)
    if not t_raw:
        return None
    with EVENT_LOCK:
        evs = [e for e in EVENT_BUS if e.get("_task_id") == task_id or e.get("_raw_id") == task_id]
    # 也从原始要求.jsonl 行扫匹配（即使 _task_id 没提取到）
    if not evs:
        for d in TASKS_RAW:
            if d.get("id") == task_id:
                evs.append({
                    "ts": int(d.get("时间戳", int(time.time()*1000))),
                    "源": "天庭/状态/要求.jsonl",
                    "动作": f"要求 {task_id} → 状态{d.get("状态","?")}",
                    "影响": [{"类型":"要求","名":task_id,"变化":f"阶段{d.get("阶段","?")}"}],
                    "token":{"提示词":0,"输出":0,"缓存":0,"总计":0},
                    "耗时ms":0,
                    "证据":d.get("方向","")[:80],
                    "_task_id":task_id,
                    "_raw":json.dumps(d, ensure_ascii=False),
                })
                break
    # 按时间正序
    evs.sort(key=lambda e: e.get("ts", 0))
    actions = []
    for e in evs:
        actions.append({
            "ts": e.get("ts", 0),
            "类型": classify_action(e),
            "庭": e.get("源", "?"),
            "动作": e.get("动作", "?"),
            "token": (e.get("token") or {}).get("总计", 0),
            "耗时ms": e.get("耗时ms", 0),
            "影响": e.get("影响", []),
            "证据": e.get("证据", ""),
            "全量": e.get("_parsed", e.get("_raw", "")),
            "层": e.get("层", "l2"),
        })
    summary = {
        "id": task_id,
        "方向": t_raw.get("方向", ""),
        "验收标准": t_raw.get("验收标准", ""),
        "约束": t_raw.get("约束", {}),
        "状态": t_raw.get("状态", "?"),
        "阶段": t_raw.get("阶段", "?"),
        "类别": t_raw.get("类别", "?"),
        "来源": t_raw.get("来源", "?"),
        "想法id": t_raw.get("想法id", "?"),
        "确认意见": t_raw.get("确认意见"),
        "验收时间": t_raw.get("验收"),
        "版本": t_raw.get("版本"),
        "动作们": actions,
    }
    return summary

# ===== 后台扫描 =====
def background_scan():
    build_sources()
    while True:
        try:
            for src in SOURCES:
                read_incremental(src)
            reload_tasks_raw()  # 任务列表要实时新
        except Exception:
            pass
        time.sleep(SCAN_INTERVAL_SEC)

# ===== HTTP 路由 =====

class MonHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        return

    def _send_json(self, data, status=200):
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        try:
            self.wfile.write(body)
        except Exception:
            pass

    def _send_bytes(self, body, ct, status=200):
        self.send_response(status)
        self.send_header("Content-Type", ct)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except Exception:
            pass

    def _send_static(self, name):
        if ".." in name or "/" in name or "\\" in name:
            self._send_json({"error":"bad path"}, 400)
            return
        path = Path(__file__).parent / "static" / name
        if not path.exists():
            self._send_json({"error":"not found"}, 404)
            return
        try:
            data = path.read_bytes()
            if name.endswith(".html"): ct = "text/html; charset=utf-8"
            elif name.endswith(".css"): ct = "text/css; charset=utf-8"
            elif name.endswith(".js"): ct = "application/javascript; charset=utf-8"
            else: ct = "application/octet-stream"
            self._send_bytes(data, ct)
        except Exception as e:
            self._send_json({"error": str(e)}, 500)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path, qs = parsed.path, urllib.parse.parse_qs(parsed.query)
        if path == "/" or path == "/index.html":
            self._send_static("index.html")
        elif path.startswith("/static/"):
            self._send_static(path[len("/static/"):])
        elif path == "/api/snapshot":
            src_filter = qs.get("源", [None])[0]
            limit = int(qs.get("limit", ["500"])[0])
            with EVENT_LOCK:
                if src_filter:
                    evs = [e for e in EVENT_BUS if src_filter in str(e.get("源", ""))]
                else:
                    evs = list(EVENT_BUS)
                if len(evs) > limit:
                    evs = evs[-limit:]
            self._send_json({"events": evs, "state": SHARED, "ts": int(time.time()*1000)})
        elif path == "/api/tasks":
            self._send_json({"tasks": get_tasks_list(), "ts": int(time.time()*1000)})
        elif path == "/api/events/recent":
            q = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
            try:
                limit = int(q.get("limit", [str(EVENT_FOLLOW_RECENT_LIMIT)])[0])
            except Exception:
                limit = EVENT_FOLLOW_RECENT_LIMIT
            if limit < 1:
                limit = 1
            if limit > 1000:
                limit = 1000
            rows, size = 读事件流最近(limit)
            rows = list(reversed(rows))
            self._send_json({"events": rows, "size": size, "limit": limit, "ts": int(time.time()*1000)})
        elif path == "/api/events/stream":
            self._send_events_sse()
        elif path.startswith("/api/sessions/") and path.endswith("/steps"):
            print(f"[steps trace] path={path}", flush=True)
            sid = urllib.parse.unquote(path[len("/api/sessions/"):-len("/steps")], encoding='utf-8', errors='replace')
            sess = get_session(sid)
            if sess is None:
                self._send_json({"error":"task not found", "id": sid}, 404)
            else:
                all_steps = build_steps(sess.get('动作们', []))
                q = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
                limit = int(q.get('limit', [str(len(all_steps))])[0])
                steps = all_steps[-limit:] if all_steps else []
                self._send_json({"session_id": sid, "steps": steps, "total": len(all_steps), "shown": len(steps)})
        elif path.startswith("/api/sessions/"):
            sid = path[len("/api/sessions/"):]
            sid = urllib.parse.unquote(sid, encoding='utf-8', errors='replace')
            sess = get_session(sid)
            sess = get_session(sid)
            if sess is None:
                self._send_json({"error":"task not found", "id": sid}, 404)
            else:
                self._send_json({"session": sess})
        elif path == "/api/stream":
            self._send_sse()
        elif path == "/api/replay":
            self._send_replay(qs)
        elif path == "/api/rooms":
            self._send_json({"rooms": SHARED["rooms"]})
        elif path == "/api/settings":
            self._send_json(SHARED["settings"])
        elif path == "/api/health":
            self._send_json({"ok": True, "events": len(EVENT_BUS), "sources": len(SOURCES), "tasks": len(TASKS_RAW)})
        else:
            self._send_json({"error":"not found"}, 404)

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path != "/api/settings":
            self._send_json({"error":"method not allowed"}, 405)
            return
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode("utf-8") if length else "{}"
        try:
            data = json.loads(body)
            SHARED["settings"].update({k: v for k, v in data.items() if k in {"interval_sec", "theme"}})
            self._send_json({"ok": True, "settings": SHARED["settings"]})
        except Exception as e:
            self._send_json({"error": str(e)}, 400)

    def _send_sse(self):
        """SSE 长连接：§11.9.2 + §13.6 三种 payload (event / task-new / task-status-change)."""
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream; charset=utf-8")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.send_header("X-Accel-Buffering", "no")
        self.end_headers()
        last_idx = len(EVENT_BUS)
        try:
            self.wfile.write(b": stream-open\n\n")
            self.wfile.flush()
            beat = 0
            while True:
                cur_total = len(EVENT_BUS)
                with EVENT_LOCK:
                    cur = EVENT_BUS
                while last_idx < cur_total:
                    ev = cur[last_idx]
                    tid = ev.get("_task_id") or ""
                    payload = {"type": "event", "任务id": tid, "ev": ev}
                    line = f"data: {json.dumps(payload, ensure_ascii=False)}\n\n".encode("utf-8")
                    try:
                        self.wfile.write(line)
                        self.wfile.flush()
                    except (BrokenPipeError, ConnectionResetError):
                        return
                    last_idx += 1
                beat += 1
                if beat % 5 == 0:
                    try:
                        self.wfile.write(b": ping\n\n")
                        self.wfile.flush()
                    except Exception:
                        return
                time.sleep(0.3)
        except (BrokenPipeError, ConnectionResetError):
            return
        except Exception:
            return

    def _send_events_sse(self):
        """步骤直播 v3：单源订阅 .上下文/事件流.jsonl 字节增量；payload 固定 {type:'event', ev:<row>}。"""
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream; charset=utf-8")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.send_header("X-Accel-Buffering", "no")
        self.end_headers()
        with EVENT_FOLLOW_LOCK:
            last = EVENT_FOLLOW_LAST_SIZE
        try:
            self.wfile.write(b": stream-open\n\n")
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            return
        beat = 0
        try:
            while True:
                new_rows, new_last = 读事件流增量(last)
                if new_rows:
                    for ev in new_rows:
                        payload = {"type": "event", "ev": ev}
                        line = f"data: {json.dumps(payload, ensure_ascii=False)}\n\n".encode("utf-8")
                        try:
                            self.wfile.write(line)
                            self.wfile.flush()
                        except (BrokenPipeError, ConnectionResetError):
                            return
                    with EVENT_FOLLOW_LOCK:
                        EVENT_FOLLOW_LAST_SIZE = new_last
                    last = new_last
                beat += 1
                if beat % 5 == 0:
                    try:
                        self.wfile.write(b": ping\n\n")
                        self.wfile.flush()
                    except (BrokenPipeError, ConnectionResetError):
                        return
                time.sleep(0.3)
        except (BrokenPipeError, ConnectionResetError):
            return
        except Exception:
            return

    def _send_replay(self, qs):
        since = int(qs.get("since", ["0"])[0])
        until = int(qs.get("until", [str(int(time.time()*1000))])[0])
        room = qs.get("庭", [None])[0]
        with EVENT_LOCK:
            evs = [e for e in EVENT_BUS if since <= e.get("ts", 0) <= until and (room is None or room in str(e.get("源", "")))]
        body = "\n".join(json.dumps(e, ensure_ascii=False) for e in evs).encode("utf-8")
        self._send_bytes(body, "application/x-ndjson; charset=utf-8")

class ThreadingHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True

def load_default_rooms():
    return [
        {"id":"shihai_fu","name":"识海承载-府","源":"鸿蒙/基础设施 - 域/识海承载-府","关切字段":["格位","编码","归档","三档命中率"]},
        {"id":"tianting_fu","name":"天庭治理-府","源":"鸿蒙/基础设施 - 域/天庭治理-府","关切字段":["八态队列","进行中要求","等待设计","终裁待审","鸿钧轮数"]},
        {"id":"daoshu_fu","name":"道术施展-府","源":"鸿蒙/基础设施 - 域/道术施展-府","关切字段":["工具循环","token预算","最近失败","回滚垫"]},
        {"id":"moxing_fu","name":"模型连接-府","源":"鸿蒙/基础设施 - 域/模型连接-府","关切字段":["最近5次token","缓存命中率","平均耗时","5xx重试"]},
        {"id":"rizhi_fu","name":"日志记录-府","源":"鸿蒙/基础设施 - 域/日志记录-府","关切字段":["订阅构建","兜底文件","并行落地","流式渲染"]},
        {"id":"peizhi_fu","name":"配置管理-府","源":"鸿蒙/世界配置 - 域/配置管理-府","关切字段":[".env项数","缺失告警","占位密钥"]},
        {"id":"guance_fu","name":"观测探针-府","源":"鸿蒙/观测探针-府","关切字段":["探针条数","正在写盘span","跨界异常"]},
        {"id":"mingling_fu","name":"命令操作-府","源":"乾坤/呈现-域/命令操作-府","关切字段":["鉴权令牌","最近10条号令","解析失败率"]},
        {"id":"zhengdao_fu","name":"单元测试-府","源":"证道/鸿蒙-域/单元测试-府","关切字段":["最近 cargo test","总用例数","耗时"]},
    ]

def main():
    global EVENT_FOLLOW_LAST_SIZE
    SHARED["rooms"] = load_default_rooms()
    if ROOMS_FILE.exists():
        try:
            data = json.loads(ROOMS_FILE.read_text(encoding="utf-8"))
            SHARED["rooms"] = data.get("rooms", SHARED["rooms"])
        except Exception:
            pass
    reload_tasks_raw()
    # 步骤直播 v3：把 last_size 初始化为当前文件大小，避免新连接回放整段历史（首屏走 /api/events/recent 拉最近 200）。
    try:
        if EVENT_FOLLOW_PATH.exists():
            EVENT_FOLLOW_LAST_SIZE = EVENT_FOLLOW_PATH.stat().st_size
    except Exception:
        pass
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    SHARED["settings"]["port"] = port
    t = threading.Thread(target=background_scan, daemon=True)
    t.start()
    server = ThreadingHTTPServer(("0.0.0.0", port), MonHandler)
    print(f"[监控界面 v2 任务为中心] 启动 http://127.0.0.1:{port}")
    print(f"[监控界面 v2] 任务数={len(TASKS_RAW)} 事件总线={len(EVENT_BUS)}")
    print(f"[监控界面 v2] 项目根={PROJECT_ROOT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()

if __name__ == "__main__":
    main()