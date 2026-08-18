# -*- coding: utf-8 -*-
"""洪荒世界 · 乾坤独立监控域 —— 纯只读观测服务
定位: 乾坤/呈现-域/监控-域 独立工具, 不进 workspace, 不依赖任何 crate,
只读分析 `.上下文` 既有数据(事件流/白箱观测记录/spill/状态文件), 不写任何生产。
约束: 零依赖 · 可整体删除(删本目录即干净) · 不影响其他域。
白箱观测: 主数据源为 `观测探针-府` 落的 `.上下文/观测/记录.jsonl`(统一积和类型),
本页按 域/角色/接口/关联 分层呈现, 支持按 要求id 还原执行链。
监听: http://127.0.0.1:3082
"""
import http.server, json, os, re, time

ROOT = r"D:\洪荒 - 世界"          # 世界根
CTX = os.path.join(ROOT, ".上下文")
EVENTS = os.path.join(CTX, "事件流.jsonl")
SPILL = os.path.join(CTX, "spill")
STATUS = os.path.join(CTX, "状态")
OBSDIR = os.path.join(CTX, "观测")
OBSREC = os.path.join(OBSDIR, "记录.jsonl")
HTML_F = os.path.join(ROOT, "乾坤", "呈现-域", "监控-域", "监控.html")
PORT = 3082

# 噪音工具: 聚合时间线时折叠
噪音工具 = {"读文件", "列举目录", "寻找文件", "搜索内容"}

# 概要模式: 观测列表/还原链默认只返回每条内容前 概要前缀字符 + 元数据,
# 避免一次性拉 5MB 全文卡死页面; 点开某条时前端请求 /api/record?id=<时间戳> 取完整正文。
概要前缀字符 = 400

def 概要化(r):
    """把一条观测记录压成概要(元数据 + 内容前 概要前缀字符), 不含完整正文。"""
    载荷 = r.get("载荷") or {}
    内容 = 载荷.get("内容") or ""
    截断 = 内容[:概要前缀字符]
    截断 = 截断 + ("…" if len(内容) > 概要前缀字符 else "")
    return {
        "时间戳": r.get("时间戳"),
        "域": r.get("域"),
        "接口": r.get("接口"),
        "角色": r.get("角色"),
        "关联": r.get("关联"),
        "载荷": {"内容": 截断, "附加": 载荷.get("附加")},
        "_有全文": bool(内容) and len(内容) > 概要前缀字符,
    }

def exists(p):
    return os.path.isfile(p)

def load_jsonl(path):
    out = []
    if not exists(path): return out
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line: continue
                try: out.append(json.loads(line))
                except Exception: pass
    except OSError: pass
    return out

def load_events():
    return load_jsonl(EVENTS)

def fmt_hm(ms):
    s = ms/1000
    if s < 60: return f"{s:.0f}秒"
    m = int(s//60); ss = int(s%60)
    return f"{m}分{ss}秒"

def task_summary():
    任务线们 = load_jsonl(os.path.join(STATUS, "任务线.jsonl"))
    要求们 = load_jsonl(os.path.join(STATUS, "要求.jsonl"))
    验收们 = load_jsonl(os.path.join(STATUS, "验收.jsonl"))
    要求map = {r.get("id"): r for r in 要求们}
    验收map = {}
    for a in 验收们:
        rid = a.get("验收", {}).get("要求id") or a.get("要求id")
        if rid: 验收map[rid] = a
    汇总 = []
    for t in 任务线们:
        rid = t.get("要求id") or ""
        验收 = 验收map.get(rid) or {}
        汇总.append({
            "任务线": t.get("id",""), "要求": rid,
            "想法": (t.get("想法内容") or "")[:80],
            "状态": t.get("状态",""), "结论": t.get("结论",""),
            "时间": t.get("时间",0),
            "主意": (验收.get("终裁依据") or (验收.get("验收") or {}).get("验收意见") or "")[:120],
        })
    汇总.sort(key=lambda x: x.get("时间",0))
    return 汇总

def assess():
    任务线们 = load_jsonl(os.path.join(STATUS, "任务线.jsonl"))
    要求们 = load_jsonl(os.path.join(STATUS, "要求.jsonl"))
    验收们 = load_jsonl(os.path.join(STATUS, "验收.jsonl"))
    指标们 = load_jsonl(os.path.join(STATUS, "指标.jsonl"))
    要求map = {r.get("id"): r for r in 要求们}
    验收map = {}
    for a in 验收们:
        rid = a.get("验收", {}).get("要求id") or a.get("要求id")
        if rid: 验收map.setdefault(rid, []).append(a)
    指标map = {m.get("要求id"): m for m in 指标们}
    评估 = []
    for t in 任务线们:
        rid = t.get("要求id")
        if not rid: continue
        要求 = 要求map.get(rid, {})
        验 = 验收map.get(rid, [])
        指标 = 指标map.get(rid, {})
        评估.append({
            "要求": rid, "目标": (t.get("想法内容") or "")[:60],
            "结论": t.get("结论",""),
            "耗时": fmt_hm(指标.get("耗时毫秒") or 0),
            "验收次数": len(验),
            "打回原因": (验[-1].get("终裁依据") if 验 else ""),
            "方向": 要求.get("方向",""),
        })
    return 评估

def tasks():
    """任务级聚合（监控页主视图）：一行一个任务线。
    列：要求id / 方向 / 状态/结论 / 耗时 / token / 产物数 / 验收意见摘要。
    附每任务的完整执行链(白箱记录按 阶段 分组)，供前端点行展开。
    """
    任务线们 = load_jsonl(os.path.join(STATUS, "任务线.jsonl"))
    要求们 = load_jsonl(os.path.join(STATUS, "要求.jsonl"))
    验收们 = load_jsonl(os.path.join(STATUS, "验收.jsonl"))
    指标们 = load_jsonl(os.path.join(STATUS, "指标.jsonl"))
    要求map = {r.get("id"): r for r in 要求们}
    验收map = {}
    for a in 验收们:
        rid = a.get("验收", {}).get("要求id") or a.get("要求id")
        if rid: 验收map.setdefault(rid, a)
    指标map = {}
    for m in 指标们:
        rid = m.get("要求id")
        if rid: 指标map.setdefault(rid, m)
    # 观测记录按 要求id 归类（白箱执行链）：tasks 不内嵌全文链（会撑爆 payload），
    # 只统计条数供前端提示；点行时前端再调 /api/chain?要求=xx 拉取(概要/全文)。
    链map = {}
    for r in load_jsonl(OBSREC):
        关联 = r.get("关联") or {}
        rid = 关联.get("要求")
        if rid:
            链map.setdefault(rid, []).append(r)
    行们 = []
    for t in 任务线们:
        rid = t.get("要求id") or ""
        验收 = 验收map.get(rid) or {}
        指标 = 指标map.get(rid) or {}
        # 产物/token 从最新验收回执取
        用量 = (验收.get("用量") or {})
        产物 = 验收.get("产物") or []
        链 = 链map.get(rid) or []
        行们.append({
            "要求": rid,
            "任务线": t.get("id", ""),
            "方向": (要求map.get(rid, {}).get("方向", "") or t.get("想法内容") or "")[:90],
            "状态": t.get("状态", ""),
            "结论": t.get("结论", ""),
            "耗时": fmt_hm(指标.get("耗时毫秒") or 0),
            "token": (用量.get("总计") or 0),
            "产物数": len(产物) if isinstance(产物, list) else 0,
            "打回原因": (验收.get("终裁依据") or "")[:120],
            "链数": len(链),
            "时间": t.get("时间", 0),
        })
    行们.sort(key=lambda x: x.get("时间", 0))
    return 行们


def timeline(require_filter=True):
    events = load_events()
    行s = []
    for e in events:
        p = e.get("载荷", {}) or {}
        t = e.get("类型","")
        if t == "工具调用":
            tool = p.get("工具","")
            if require_filter and tool in 噪音工具: continue
            概 = f"【{tool}】轮{p.get('轮次','')} {'✗' if p.get('失败') else '✓'} {(p.get('参数') or '')[:50]}"
        elif t == "要求状态推进":
            概 = f"{p.get('要求id','')}→{p.get('状态','')}"
        elif t in ("验收结论","失败沉淀","版本存档","想法投递","要求入池","设计上呈"):
            概 = str(p)[:80]
        else:
            概 = str(p)[:60]
        行s.append({"ts": e.get("时间戳",0), "type": t, "概": 概})
    # 限最近 600 条
    return 行s[-600:]

def obs_blocks(maxn=40):
    """白箱观测: 读 记录.jsonl 尾部 maxn 条, 概要模式(轻量)。"""
    recs = load_jsonl(OBSREC)
    return [概要化(r) for r in recs[-maxn:]]

def records(q=None):
    """白箱观测: 读 记录.jsonl, 支持查询参数过滤。
    q: dict, 可含 域/角色/要求/任务线/接口, 及 n(条数), full=1(返回完整正文)。
    默认概要模式, 避免一次性拉 5MB 全文卡死页面。
    """
    recs = load_jsonl(OBSREC)
    if any(q and q.get(k) for k in ("域", "角色", "要求", "任务线", "接口")):
        out = []
        for r in recs:
            if q.get("域") and r.get("域") != q["域"]: continue
            if q.get("角色") and r.get("角色") != q["角色"]: continue
            if q.get("要求"):
                关联 = r.get("关联") or {}
                if 关联.get("要求") != q["要求"]: continue
            if q.get("任务线"):
                关联 = r.get("关联") or {}
                if 关联.get("任务线") != q["任务线"]: continue
            if q.get("接口") and q["接口"] not in (r.get("接口") or ""): continue
            out.append(r)
    else:
        out = recs
    limit = int(q.get("n")) if q and q.get("n") else 100
    out = out[-limit:]
    if not (q and q.get("full") == "1"):
        out = [概要化(r) for r in out]
    return out

def chain(requirement_id, full=False):
    """白箱还原: 按 要求id 抽出该要求关联的全部观测记录, 按时间戳重排。
    默认概要; full=True 返回完整正文。
    """
    recs = [r for r in load_jsonl(OBSREC)
            if (r.get("关联") or {}).get("要求") == requirement_id]
    recs.sort(key=lambda r: r.get("时间戳", 0))
    if not full:
        recs = [概要化(r) for r in recs]
    return recs

def record_by_id(ts):
    """按 时间戳 取单条完整观测记录(点开概要时调用)。"""
    if ts is None:
        return None
    try:
        ts = int(ts)
    except (TypeError, ValueError):
        return None
    for r in load_jsonl(OBSREC):
        if r.get("时间戳") == ts:
            return r
    return None

def spill_index():
    """spill 目录超大结果索引(下探文件正文)。"""
    if not os.path.isdir(SPILL): return []
    out = []
    for name in os.listdir(SPILL):
        if name.endswith(".txt"):
            out.append({"file": name, "path": os.path.join("spill", name)})
    return sorted(out)[-200:]

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def _json(self, obj):
        b = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)
    def do_GET(self):
        path, _, query = self.path.partition("?")
        try:
            import urllib.parse as up
            q = {}
            if query:
                for kv in query.split("&"):
                    if "=" in kv:
                        k, v = kv.split("=", 1)
                        q[up.unquote(k)] = up.unquote(v)
            if path == "/api/summary": self._json(task_summary())
            elif path == "/api/assess": self._json(assess())
            elif path == "/api/tasks": self._json(tasks())
            elif path == "/api/timeline": self._json(timeline())
            elif path == "/api/obs": self._json(obs_blocks())
            elif path == "/api/records": self._json(records(q))
            elif path == "/api/chain":
                self._json(chain(q.get("要求", ""), full=(q.get("full") == "1")))
            elif path == "/api/record":
                self._json(record_by_id(q.get("id")))
            elif path == "/api/spill": self._json(spill_index())
            else:
                html = b""
                if os.path.isfile(HTML_F):
                    with open(HTML_F, "rb") as f: html = f.read()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(html)))
                self.end_headers()
                self.wfile.write(html)
        except Exception:
            try:
                self.send_response(500); self.send_header("Content-Length","0"); self.end_headers()
            except Exception: pass

if __name__ == "__main__":
    srv = http.server.HTTPServer(("127.0.0.1", PORT), Handler)
    print(f"乾坤监控域已启动 http://127.0.0.1:{PORT}")
    srv.serve_forever()
