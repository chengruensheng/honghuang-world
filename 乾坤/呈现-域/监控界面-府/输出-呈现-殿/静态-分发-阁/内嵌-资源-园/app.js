/* 洪荒 · 轨迹账本 —— 账本视角监控界面逻辑
 * 设计：融合蓝图 §13.f · 对齐 dsh 轨迹表格白箱
 *   - 三栏可拖拽布局：pointer capture + rAF 节流
 *   - 7 种事件类型派生（§13.f.8）：system/user/context/compacted/message/tool/subtool
 *   - 按轮次 Turn 分组，sticky 组头
 *   - 虚拟滚动：行数 > 100 时仅渲染可视区 ± 缓冲行（方案 B，零依赖）
 *   - 信源 流式追加：/api/trajectory/stream，新行底部淡入 + 短暂高亮
 *   - 历史加载：向上滚到顶拉 /api/trajectory?before=...，顶部插入 + 序号重编
 *   - 折叠：按轮次 / 按助手消息 / 全部
 *   - 搜索：节流 3s，命中高亮，非命中行淡化
 *   - 时间线：4 模式（sequence/duration/time/actual），拖拽选范围筛选
 *   - token 累计：底部 sticky 汇总条，5 分量 + 总计
 */
(function () {
  "use strict";

  // ===== 常量 =====
  var 虚拟化阈值 = 100;          // 行数 > 100 启用虚拟滚动
  var 缓冲行数 = 5;              // 上下各缓冲 5 行
  var 历史加载阈值px = 48;       // 距顶 48px 触发历史加载
  var 贴底阈值px = 2;            // 距底 2px 视为贴底跟随
  var 搜索节流ms = 3000;         // 搜索节流 3s
  var 历史页大小 = 200;          // 历史加载每页 200 条
  var 行高 = 32;                 // 事件行高（与 CSS --行高 同步）
  var 组头高 = 36;               // 轮次组头高（与 CSS --组头高 同步）
  var 流式高亮ms = 300;          // 新行高亮时长

  // 7 种事件类型（§13.f.2）
  var 类型标签 = {
    system: "SYSTEM",
    user: "USER",
    context: "CONTEXT",
    compacted: "COMPACTED",
    message: "ASSISTANT",
    tool: "TOOL",
    subtool: "SUBTOOL"
  };

  // ===== 状态（§13.f.12） =====
  var 状态 = {
    事件们: [],                  // 事件行列表（升序 ts）
    轮次们: new Map(),           // 轮次 → { 起始ts, 角色, 事件序号区间, 累计token, 累计耗时 }
    展开行: null,                // 当前展开详情面板的事件 id
    折叠集: { 轮次: new Set(), 消息: new Set() },  // 折叠状态
    折叠模式: "无",              // 无 / 按轮次 / 按消息 / 全部
    时间线模式: "sequence",
    搜索词: "",
    搜索命中: new Set(),
    时间范围: null,             // {since, until}
    token汇总: { 输入: 0, 输出: 0, 缓存读: 0, 缓存写: 0, 推理: 0, 总计: 0 },
    最早ts: null,                // 已加载最早事件 ts（用于历史加载 before 参数）
    已到最早: false,
    历史加载中: false,
    跟随: true,                  // 贴底跟随
    群聊跟随: true,              // 群聊流贴底跟随
    信源暂停: false,
    选中id: null,
    选中轮次: null,
    流式行id: null               // 当前正在流式 partial 的行 id
  };

  var 信源 = null;
  var 启动时刻ms = Date.now();
  var 服务启动时刻ms = 0;
  var 房间池 = new Map();        // 房间 id → { id, 名, 事件数 }
  var 房间序 = [];
  var 当前房间id = null;

  // 三栏布局状态
  var 左栏宽 = 240;
  var 右栏宽 = 380;
  var 左栏最小 = 200;
  var 左栏最大 = 480;
  var 右栏最小 = 300;
  var 右栏最大 = 760;
  var 中栏最小 = 400;
  var 左栏收起 = false;
  var 右栏收起 = false;

  // 虚拟滚动
  var 虚拟化启用 = false;
  var 可见起 = 0;
  var 可见止 = 0;
  var 滚动rAF = null;

  // ===== DOM 引用 =====
  var $ = function (id) { return document.getElementById(id); };
  var 元素 = {};

  function 缓存元素() {
    元素.三栏 = $("三栏");
    元素.左栏 = $("左栏");
    元素.左分隔 = $("左分隔");
    元素.中栏 = $("中栏");
    元素.右分隔 = $("右分隔");
    元素.右栏 = $("右栏");
    元素.表格容器 = $("表格容器");
    元素.行区 = $("行区");
    元素.虚拟顶 = $("虚拟顶");
    元素.虚拟底 = $("虚拟底");
    元素.历史加载 = $("历史加载");
    元素.已到最早 = $("已到最早");
    元素.脱离最新 = $("脱离最新");
    元素.汇总条 = $("汇总条");
    元素.房间列表 = $("房间列表");
    元素.详情面板 = $("详情面板");
    元素.状态点 = $("状态点");
    元素.状态文 = $("状态文");
    元素.运行时长 = $("运行时长");
    元素.时刻 = $("时刻");
    元素.搜索条 = $("搜索条");
    元素.搜索输入 = $("搜索输入");
    元素.搜索计数 = $("搜索计数");
    元素.时间线色块 = $("时间线色块");
    元素.时间线模式 = $("时间线模式");
    元素.清除范围 = $("清除范围");
    元素.诸圣列表 = $("诸圣列表");
  }

  // ===== 工具函数 =====

  // 7 种事件类型派生（§13.f.8）
  function 派生事件类型(事件) {
    var 源 = 事件.源 || "";
    var 动作 = 事件.动作 || "";
    var 载荷 = {};
    try {
      if (typeof 事件.证据 === "string") {
        载荷 = JSON.parse(事件.证据);
      } else if (事件.证据 && typeof 事件.证据 === "object") {
        载荷 = 事件.证据;
      }
    } catch (e) {
      载荷 = {};
    }

    if (源.indexOf("提示词") >= 0 || 动作.indexOf("提示词") >= 0) {
      var 角色 = 载荷.角色;
      if (角色 === "系统" || 角色 === "system") return "system";
      if (角色 === "界主" || 角色 === "user") return "user";
      if (载荷.注入类) return "context";
      return "user";
    }
    if (动作.indexOf("压缩") >= 0 || 载荷.压缩标记) return "compacted";
    if (动作.indexOf("回复") >= 0 || 动作.indexOf("思考") >= 0 || 源.indexOf("模型连接") >= 0) {
      return "message";
    }
    if (动作.indexOf("工具调用") >= 0 || 源.indexOf("道术施展") >= 0) {
      if (载荷.子工具标记) return "subtool";
      return "tool";
    }
    return "message";  // 兜底，避免漏派生
  }

  // 摘要提炼（§13.e.2）：去 Shields/<标签>/JSON 片段，留可读首句，≤80 字
  function 提炼摘要(原文) {
    if (!原文) return "";
    var s = String(原文);
    // 去 <Shields> 标签
    s = s.replace(/<Shields[^>]*>[\s\S]*?<\/Shields>/g, "");
    // 去 <标签>
    s = s.replace(/<[^>]+>/g, "");
    // 去 JSON 片段（{...} / [...]）
    s = s.replace(/\{[\s\S]*?\}/g, " ").replace(/\[[\s\S]*?\]/g, " ");
    // 折叠空白
    s = s.replace(/\s+/g, " ").trim();
    // 取首句
    var 句点 = s.search(/[。.!？?\n]/);
    if (句点 > 0) s = s.slice(0, 句点 + 1);
    // ≤80 字
    if (s.length > 80) s = s.slice(0, 79) + "…";
    return s;
  }

  // 耗时格式化（参考 trajectory-record.ts formatElapsedSeconds）
  function 格式化耗时(ms) {
    if (ms === null || ms === undefined || !isFinite(ms)) return "—";
    if (ms < 1000) return Math.round(ms) + "ms";
    var s = ms / 1000;
    if (s < 60) return s.toFixed(1) + "s";
    var m = Math.floor(s / 60);
    return m + "m" + Math.round(s - m * 60) + "s";
  }

  // token 格式化（千分位）
  function 格式化token(n) {
    if (!n || n <= 0) return "";
    return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  }

  // 时间戳格式化（HH:MM:SS）
  function 格式化时刻(ts) {
    if (!ts) return "";
    var d = new Date(ts);
    var h = String(d.getHours()).padStart(2, "0");
    var m = String(d.getMinutes()).padStart(2, "0");
    var s = String(d.getSeconds()).padStart(2, "0");
    return h + ":" + m + ":" + s;
  }

  // 生成事件 id
  function 事件id(事件) {
    if (事件.id) return 事件.id;
    return 事件.类型 + "\u0000" + 事件.序号 + "\u0000" + 事件.时间戳;
  }

  // ===== 白箱六字段 → 轨迹行（§13.f.8 派生） =====
  // 详情字段集对齐 dsh TrajectoryCellProps（§13.f.3 字段全集）
  function 装配轨迹行(白箱事件, 序号) {
    var 类型 = 派生事件类型(白箱事件);
    var token = 白箱事件.token || {};
    var 载荷 = {};
    try {
      if (typeof 白箱事件.证据 === "string") 载荷 = JSON.parse(白箱事件.证据);
      else if (白箱事件.证据 && typeof 白箱事件.证据 === "object") 载荷 = 白箱事件.证据;
    } catch (e) { 载荷 = {}; }
    var 行 = {
      id: 事件id({ 类型: 类型, 序号: 序号, 时间戳: 白箱事件.ts }),
      序号: 序号,
      类型: 类型,
      摘要: 提炼摘要(白箱事件.证据 || 白箱事件.动作 || ""),
      token: {
        输入: token.提示词 || token.输入 || 0,
        输出: token.输出 || 0,
        缓存读: token.缓存读 || token.cacheRead || 0,
        缓存写: token.缓存写 || token.cacheWrite || 0,
        推理: token.推理 || token.reasoning || 0
      },
      耗时ms: 白箱事件.耗时ms || (白箱事件.耗时 != null ? 白箱事件.耗时 * 1000 : null),
      轮次: 白箱事件.轮次 || 白箱事件.turn || null,
      时间戳: 白箱事件.ts || Date.now(),
      角色: 白箱事件.角色 || "",
      是否错误: (白箱事件.动作 && 白箱事件.动作.indexOf("失败") >= 0) ||
                (白箱事件.影响 && JSON.stringify(白箱事件.影响).indexOf("错误") >= 0) || false,
      // §13.f.3 详情字段全集（按类型选择性展示）
      完整原文: 白箱事件.证据 || "",
      源: 白箱事件.源 || "",
      动作: 白箱事件.动作 || "",
      影响: 白箱事件.影响 || null,
      供应者: 白箱事件.供应者 || 载荷.供应者 || "",
      模型: 白箱事件.模型 || 载荷.模型 || "",
      // inputDetail: 完整请求/消息原文
      inputDetail: 白箱事件.inputDetail || 载荷.inputDetail || "",
      // promptDetail: 完整系统提示词+工具目录
      promptDetail: 白箱事件.promptDetail || 载荷.promptDetail || "",
      // previousPromptDetail: 旧提示状态（diff 对比用）
      previousPromptDetail: 白箱事件.previousPromptDetail || 载荷.previousPromptDetail || "",
      // outputDetail: 完整助手/工具结果原文
      outputDetail: 白箱事件.outputDetail || 载荷.outputDetail || "",
      // thinkingDetail: 完整推理过程（§13.f.7a 兼容 思考链 字段）
      thinkingDetail: 白箱事件.thinkingDetail || 载荷.thinkingDetail || 白箱事件.思考链 || 载荷.思考链 || "",
      // sourceBlocks: 原始消息块（按模型顺序）
      sourceBlocks: 白箱事件.sourceBlocks || 载荷.sourceBlocks || null,
      // outputBlocks: 工具结果块
      outputBlocks: 白箱事件.outputBlocks || 载荷.outputBlocks || null,
      // schemaDetail: 工具 schema 定义
      schemaDetail: 白箱事件.schemaDetail || 载荷.schemaDetail || "",
      // assistantMetrics: TTFT/解码吞吐/时间指标
      助手指标: 白箱事件.assistantMetrics || 载荷.assistantMetrics || null,
      // result: 工具结果摘要
      result: 白箱事件.result || 载荷.result || "",
      // 重试信息
      retry: 白箱事件.retry != null ? 白箱事件.retry : 载荷.retry,
      maxRetries: 白箱事件.maxRetries != null ? 白箱事件.maxRetries : 载荷.maxRetries,
      retryDelayMs: 白箱事件.retryDelayMs != null ? 白箱事件.retryDelayMs : 载荷.retryDelayMs,
      // 起止时刻（时间线 actual 模式用）
      startedAt: 白箱事件.startedAt || 载荷.startedAt || 白箱事件.ts || null,
      partial: 白箱事件.partial || false,
      // 详情面板 LOD 层级（§13.f.9）：0=未展开, 1=重点字段, 2=全量载荷
      lod: 0,
      // 详情是否已拉全量
      已拉全量: false
    };
    return 行;
  }

  // ===== 三栏可拖拽布局（pointer capture + rAF 节流） =====
  function 应用三栏宽() {
    if (左栏收起 && 右栏收起) {
      元素.三栏.style.gridTemplateColumns = "minmax(0,1fr)";
    } else if (左栏收起) {
      元素.三栏.style.gridTemplateColumns = "minmax(0,1fr) 6px " + 右栏宽 + "px";
    } else if (右栏收起) {
      元素.三栏.style.gridTemplateColumns = 左栏宽 + "px 6px minmax(0,1fr)";
    } else {
      元素.三栏.style.gridTemplateColumns = 左栏宽 + "px 6px minmax(0,1fr) 6px " + 右栏宽 + "px";
    }
    元素.左栏.dataset.收起 = 左栏收起 ? "true" : "false";
    元素.右栏.dataset.收起 = 右栏收起 ? "true" : "false";
    元素.左分隔.style.display = 左栏收起 ? "none" : "block";
    元素.右分隔.style.display = 右栏收起 ? "none" : "block";
  }

  function 装分隔条(分隔元素, 侧) {
    var 拖中 = false;
    var 起点x = 0;
    var 起始宽 = 0;
    var 最新x = 0;
    var 帧id = null;

    function onDown(e) {
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      拖中 = true;
      起点x = e.clientX;
      最新x = e.clientX;
      起始宽 = 侧 === "左" ? 左栏宽 : 右栏宽;
      分隔元素.dataset.拖 = "true";
      元素.三栏.dataset.dragging = "true";
    }
    function onMove(e) {
      if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
      最新x = e.clientX;
      if (帧id === null) {
        帧id = requestAnimationFrame(function () {
          帧id = null;
          var 横移 = 最新x - 起点x;
          if (侧 === "左") {
            var 新宽 = 起始宽 + 横移;
            新宽 = Math.max(左栏最小, Math.min(左栏最大, 新宽));
            左栏宽 = 新宽;
          } else {
            var 新宽2 = 起始宽 - 横移;
            新宽2 = Math.max(右栏最小, Math.min(右栏最大, 新宽2));
            右栏宽 = 新宽2;
          }
          应用三栏宽();
        });
      }
    }
    function onUp(e) {
      if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
      e.currentTarget.releasePointerCapture(e.pointerId);
      if (帧id !== null) { cancelAnimationFrame(帧id); 帧id = null; }
      var 横移 = 最新x - 起点x;
      if (侧 === "左") {
        左栏宽 = Math.max(左栏最小, Math.min(左栏最大, 起始宽 + 横移));
      } else {
        右栏宽 = Math.max(右栏最小, Math.min(右栏最大, 起始宽 - 横移));
      }
      应用三栏宽();
      拖中 = false;
      分隔元素.dataset.拖 = "false";
      元素.三栏.dataset.dragging = "false";
    }
    分隔元素.addEventListener("pointerdown", onDown);
    分隔元素.addEventListener("pointermove", onMove);
    分隔元素.addEventListener("pointerup", onUp);
    分隔元素.addEventListener("pointercancel", onUp);
  }

  // ===== 渲染：轮次分组结构 =====
  // 把 状态.事件们 按轮次分组，返回 [{ 轮次, 角色, 起始ts, 行们: [行] }]
  function 计算轮次分组() {
    var 分组 = [];
    var 当前 = null;
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      var 轮次 = 行.轮次;
      if (!当前 || 当前.轮次 !== 轮次) {
        当前 = { 轮次: 轮次, 角色: 行.角色 || "", 起始ts: 行.时间戳, 行们: [] };
        分组.push(当前);
      }
      当前.行们.push(行);
    }
    return 分组;
  }

  // 折叠判定
  function 轮次是否折叠(轮次) {
    if (状态.折叠模式 === "全部") return true;
    if (状态.折叠模式 === "按轮次") return 状态.折叠集.轮次.has(轮次);
    return false;
  }
  function 消息后是否折叠(行) {
    if (状态.折叠模式 === "按消息") return 状态.折叠集.消息.has(行.id);
    return false;
  }

  // ===== 渲染：单行 DOM =====
  function 创建行元素(行) {
    var div = document.createElement("div");
    div.className = "事件行";
    div.dataset.类型 = 行.类型;
    div.dataset.id = 行.id;
    div.dataset.序号 = 行.序号;
    if (行.是否错误) div.classList.add("错误");
    if (状态.选中id === 行.id) div.classList.add("选中");
    if (行.流式中) div.classList.add("流式中");
    if (状态.搜索词 && !状态.搜索命中.has(行.id)) div.classList.add("淡化");

    // 序号
    var 序号span = document.createElement("span");
    序号span.className = "行序号";
    序号span.textContent = "#" + 行.序号;
    div.appendChild(序号span);

    // 类型标签
    var 类型span = document.createElement("span");
    类型span.className = "行类型";
    类型span.textContent = 类型标签[行.类型] || 行.类型;
    div.appendChild(类型span);

    // 摘要（含搜索高亮）
    var 摘要span = document.createElement("span");
    摘要span.className = "行摘要";
    摘要span.appendChild(渲染摘要(行.摘要));
    div.appendChild(摘要span);

    // token：[输入][输出][推理]
    var tokenDiv = document.createElement("span");
    tokenDiv.className = "行token";
    var 输 = document.createElement("span"); 输.className = "tok tok-输"; 输.textContent = 格式化token(行.token.输入);
    var 出 = document.createElement("span"); 出.className = "tok tok-出"; 出.textContent = 格式化token(行.token.输出);
    var 思 = document.createElement("span"); 思.className = "tok tok-思"; 思.textContent = 格式化token(行.token.推理);
    tokenDiv.appendChild(输); tokenDiv.appendChild(出); tokenDiv.appendChild(思);
    div.appendChild(tokenDiv);

    // 耗时
    var 耗时span = document.createElement("span");
    耗时span.className = "行耗时";
    耗时span.textContent = 格式化耗时(行.耗时ms);
    div.appendChild(耗时span);

    // 点击：选中 + 发详情事件
    div.addEventListener("click", function (e) {
      e.stopPropagation();
      选中行(行);
    });

    return div;
  }

  // 摘要渲染（含搜索命中高亮）
  function 渲染摘要(摘要) {
    var 片段 = document.createDocumentFragment();
    if (!状态.搜索词 || !摘要) {
      片段.appendChild(document.createTextNode(摘要));
      return 片段;
    }
    var q = 状态.搜索词;
    var 位置 = 摘要.indexOf(q);
    if (位置 < 0) {
      片段.appendChild(document.createTextNode(摘要));
      return 片段;
    }
    片段.appendChild(document.createTextNode(摘要.slice(0, 位置)));
    var 命中span = document.createElement("span");
    命中span.className = "命中";
    命中span.textContent = q;
    片段.appendChild(命中span);
    片段.appendChild(document.createTextNode(摘要.slice(位置 + q.length)));
    return 片段;
  }

  // 创建轮次组 DOM
  function 创建轮次组元素(组) {
    var 组div = document.createElement("div");
    组div.className = "轮次组";
    组div.dataset.轮次 = 组.轮次;

    // sticky 组头
    var 头div = document.createElement("div");
    头div.className = "轮次组头";
    var 折叠 = 轮次是否折叠(组.轮次);
    if (折叠) 头div.classList.add("折叠");

    var 箭头 = document.createElement("span");
    箭头.className = "轮次折叠箭头";
    箭头.textContent = "▼";
    头div.appendChild(箭头);

    var 标 = document.createElement("span");
    标.className = "轮次标";
    标.textContent = "Turn " + 组.轮次;
    头div.appendChild(标);

    if (组.角色) {
      var 角 = document.createElement("span");
      角.className = "轮次角色";
      角.textContent = "· " + 组.角色;
      头div.appendChild(角);
    }

    var 刻 = document.createElement("span");
    刻.className = "轮次时刻";
    刻.textContent = "· " + 格式化时刻(组.起始ts);
    头div.appendChild(刻);

    // 累计 token + 耗时
    var 累 = document.createElement("span");
    累.className = "轮次累计";
    var 累tok = 0, 累耗 = 0;
    for (var i = 0; i < 组.行们.length; i++) {
      var r = 组.行们[i];
      累tok += r.token.输入 + r.token.输出 + r.token.推理;
      if (r.耗时ms) 累耗 += r.耗时ms;
    }
    var tokspan = document.createElement("span"); tokspan.className = "轮次token"; tokspan.textContent = 格式化token(累tok) + " tok";
    var 耗span = document.createElement("span"); 耗span.className = "轮次耗时"; 耗span.textContent = 格式化耗时(累耗);
    累.appendChild(tokspan); 累.appendChild(耗span);
    头div.appendChild(累);

    // 点组头切换折叠
    头div.addEventListener("click", function (e) {
      e.stopPropagation();
      if (状态.折叠模式 === "无" || 状态.折叠模式 === "按消息") {
        状态.折叠模式 = "按轮次";
      }
      if (状态.折叠集.轮次.has(组.轮次)) {
        状态.折叠集.轮次.delete(组.轮次);
      } else {
        状态.折叠集.轮次.add(组.轮次);
      }
      写折叠hash();
      渲染表格();
    });

    组div.appendChild(头div);

    // 组身
    var 身div = document.createElement("div");
    身div.className = "轮次组身";
    if (折叠) 组div.classList.add("折叠");

    for (var j = 0; j < 组.行们.length; j++) {
      var 行 = 组.行们[j];
      身div.appendChild(创建行元素(行));
      // 按消息折叠：message 行后到下一 message 前的 tool/subtool 折叠
      if (行.类型 === "message" && 消息后是否折叠(行)) {
        // 跳过后续 tool/subtool（直到下一 message 或组结束）
        while (j + 1 < 组.行们.length && (组.行们[j + 1].类型 === "tool" || 组.行们[j + 1].类型 === "subtool")) {
          j++;
        }
      }
    }
    组div.appendChild(身div);
    return 组div;
  }

  // ===== 渲染：全量渲染表格 =====
  function 渲染表格() {
    var 分组 = 计算轮次分组();
    var 总行数 = 状态.事件们.length;
    虚拟化启用 = 总行数 > 虚拟化阈值;

    // 清空
    元素.行区.innerHTML = "";

    if (总行数 === 0) {
      var 空 = document.createElement("div");
      空.style.cssText = "padding:40px;text-align:center;color:var(--弱);font-size:13px;";
      空.textContent = "暂无事件，等待 信源 推送……";
      元素.行区.appendChild(空);
      更新汇总();
      return;
    }

    if (虚拟化启用) {
      渲染虚拟(分组);
    } else {
      渲染全量(分组);
    }
    更新汇总();
  }

  function 渲染全量(分组) {
    元素.虚拟顶.style.height = "0px";
    元素.虚拟底.style.height = "0px";
    var 片段 = document.createDocumentFragment();
    for (var i = 0; i < 分组.length; i++) {
      片段.appendChild(创建轮次组元素(分组[i]));
    }
    元素.行区.appendChild(片段);
  }

  // 虚拟滚动渲染：只渲染可见区 ± 缓冲行
  function 渲染虚拟(分组) {
    var 容器高 = 元素.表格容器.clientHeight;
    var 滚动顶 = 元素.表格容器.scrollTop;

    // 计算总高（组头 + 行）
    var 总高 = 0;
    var 项列表 = [];  // { 类型: "组头"/"行", 组索引, 行索引, 高, 偏移 }
    for (var i = 0; i < 分组.length; i++) {
      var 组 = 分组[i];
      var 折叠 = 轮次是否折叠(组.轮次);
      项列表.push({ 类型: "组头", 组索引: i, 高: 组头高, 偏移: 总高 });
      总高 += 组头高;
      if (!折叠) {
        for (var j = 0; j < 组.行们.length; j++) {
          项列表.push({ 类型: "行", 组索引: i, 行索引: j, 高: 行高, 偏移: 总高 });
          总高 += 行高;
        }
      }
    }

    // 找可见范围
    var 起y = 滚动顶 - 缓冲行数 * 行高;
    var 止y = 滚动顶 + 容器高 + 缓冲行数 * 行高;
    var 起位置 = 0, 止位置 = 项列表.length - 1;
    for (var k = 0; k < 项列表.length; k++) {
      if (项列表[k].偏移 + 项列表[k].高 >= 起y) { 起位置 = k; break; }
    }
    for (var k2 = 项列表.length - 1; k2 >= 0; k2--) {
      if (项列表[k2].偏移 <= 止y) { 止位置 = k2; break; }
    }
    可见起 = 起位置;
    可见止 = 止位置;

    // 顶部占位
    var 顶高 = 项列表[起位置].偏移;
    元素.虚拟顶.style.height = 顶高 + "px";

    // 渲染可见项（按组聚合）
    var 片段 = document.createDocumentFragment();
    var 当前组位置 = -1;
    var 当前组div = null;
    var 当前身div = null;
    for (var m = 起位置; m <= 止位置; m++) {
      var 项 = 项列表[m];
      if (项.类型 === "组头") {
        当前组位置 = 项.组索引;
        当前组div = 创建轮次组元素(分组[当前组位置]);
        当前身div = 当前组div.querySelector(".轮次组身");
        // 若起位置 不是组头，需清空组身再按可见行重建
        if (m !== 起位置 || 项.组索引 !== 项列表[起位置].组索引) {
          // 不重建（已含全行），但虚拟化下需裁剪到可见行
        }
        片段.appendChild(当前组div);
      } else if (项.类型 === "行") {
        if (项.组索引 !== 当前组位置) {
          // 跨组：重建组
          当前组位置 = 项.组索引;
          当前组div = 创建轮次组元素(分组[当前组位置]);
          当前身div = 当前组div.querySelector(".轮次组身");
          当前身div.innerHTML = "";
          片段.appendChild(当前组div);
        }
        // 若是组的第一行且组头未渲染，跳过（组头已在组创建时含）
        var 行 = 分组[项.组索引].行们[项.行索引];
        // 避免重复添加（创建轮次组元素 已含全行）
        // 虚拟化下重建：清组身只放可见行
      }
    }

    // 简化虚拟化：直接重建可见组（组头 + 该组可见行）
    片段 = document.createDocumentFragment();
    var 已渲染组 = new Set();
    for (var n = 起位置; n <= 止位置; n++) {
      var 项n = 项列表[n];
      if (项n.类型 === "组头") {
        // 渲染整组（组头 + 全行）——虚拟化下组内行少，整组渲染可接受
        if (!已渲染组.has(项n.组索引)) {
          片段.appendChild(创建轮次组元素(分组[项n.组索引]));
          已渲染组.add(项n.组索引);
        }
      } else if (项n.类型 === "行") {
        if (!已渲染组.has(项n.组索引)) {
          片段.appendChild(创建轮次组元素(分组[项n.组索引]));
          已渲染组.add(项n.组索引);
        }
      }
    }
    元素.行区.innerHTML = "";
    元素.行区.appendChild(片段);

    // 底部占位
    var 末项 = 项列表[止位置];
    var 底高 = 总高 - (末项.偏移 + 末项.高);
    元素.虚拟底.style.height = Math.max(0, 底高) + "px";
  }

  // ===== token 累计汇总（§13.f.7 · 5 分量 + 总计 · 范围切换） =====
  // 范围：总计 / 当前轮次 / 选中范围
  function 算汇总(范围) {
    var s = { 输入: 0, 输出: 0, 缓存读: 0, 缓存写: 0, 推理: 0, 总计: 0 };
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      if (范围 === "当前轮次" && 状态.选中轮次 != null && 行.轮次 !== 状态.选中轮次) continue;
      if (范围 === "选中范围" && 状态.时间范围 && (行.时间戳 < 状态.时间范围.since || 行.时间戳 > 状态.时间范围.until)) continue;
      var t = 行.token;
      s.输入 += t.输入; s.输出 += t.输出;
      s.缓存读 += t.缓存读; s.缓存写 += t.缓存写;
      s.推理 += t.推理;
    }
    s.总计 = s.输入 + s.输出 + s.缓存读 + s.缓存写 + s.推理;
    return s;
  }

  function 更新汇总() {
    var 范围 = (元素.汇总范围 && 元素.汇总范围.value) || "总计";
    var s = 算汇总(范围);
    状态.token汇总 = s;
    $("累计输入").textContent = 格式化token(s.输入);
    $("累计输出").textContent = 格式化token(s.输出);
    $("累计缓存读").textContent = 格式化token(s.缓存读);
    $("累计缓存写").textContent = 格式化token(s.缓存写);
    $("累计推理").textContent = 格式化token(s.推理);
    $("累计总计").textContent = 格式化token(s.总计);
  }

  // ===== 选中行 =====
  function 选中行(行) {
    状态.选中id = 行.id;
    状态.选中轮次 = 行.轮次;
    // 更新 DOM 选中态
    var 旧 = 元素.行区.querySelector(".事件行.选中");
    if (旧) 旧.classList.remove("选中");
    var 新 = 元素.行区.querySelector('.事件行[data-id="' + 行.id + '"]');
    if (新) 新.classList.add("选中");
    // 发详情事件
    document.dispatchEvent(new CustomEvent("轨迹:选中", { detail: 行 }));
    // 默认 L1 重点字段；若已拉全量则保持 L2
    if (行.详略层级 === 0) 行.详略层级 = 1;
    渲染详情面板(行);
    // 后台拉全量（§13.f.11 GET /api/trajectory/event/{id}）
    if (!行.已拉全量) 拉单事件详情(行);
    // 选中轮次变 → 重算汇总（若范围=当前轮次）
    更新汇总();
    // 重渲时间线（选中态高亮）
    渲染时间线();
  }

  // 拉单事件详情全量
  function 拉单事件详情(行) {
    fetch("/api/trajectory/event/" + encodeURIComponent(行.id))
      .then(function (r) { return r.json(); })
      .then(function (详情) {
        // 合并全量字段
        if (详情.inputDetail) 行.输入详情 = 详情.inputDetail;
        if (详情.promptDetail) 行.提示词详情 = 详情.promptDetail;
        if (详情.previousPromptDetail) 行.旧提示词详情 = 详情.previousPromptDetail;
        if (详情.outputDetail) 行.输出详情 = 详情.outputDetail;
        // §13.f.7a 兼容 Rust 端 serde rename "思考链"
        if (详情.thinkingDetail) 行.思考详情 = 详情.thinkingDetail;
        else if (详情.思考链) 行.思考详情 = 详情.思考链;
        if (详情.sourceBlocks) 行.源块们 = 详情.sourceBlocks;
        if (详情.outputBlocks) 行.输出块们 = 详情.outputBlocks;
        if (详情.schemaDetail) 行.模式详情 = 详情.schemaDetail;
        if (详情.assistantMetrics) 行.助手指标 = 详情.assistantMetrics;
        if (详情.result) 行.结果 = 详情.result;
        if (详情.供应者) 行.供应者 = 详情.供应者;
        if (详情.模型) 行.模型 = 详情.模型;
        if (详情.retry != null) 行.重试次 = 详情.retry;
        if (详情.maxRetries != null) 行.最大重试 = 详情.maxRetries;
        if (详情.retryDelayMs != null) 行.重试延迟ms = 详情.retryDelayMs;
        行.已拉全量 = true;
        // 仅当该行仍选中时重渲
        if (状态.选中id === 行.id) 渲染详情面板(行);
      })
      .catch(function () {
        // 端点不存在则用白箱六字段已有的详情字段，标记已拉避免重试
        行.已拉全量 = true;
      });
  }

  // ===== 详情面板（§13.f.3 + §13.f.9 LOD 三级） =====
  // 按事件类型选择性展示字段（对齐 dsh detailTabs）
  function 详情可见字段(行) {
    var t = 行.类型;
    var 字段 = [];
    // 通用：基本信息
    字段.push("基本信息");
    if (行.输入详情) 字段.push("输入原文");
    if (行.提示词详情 && (t === "message" || t === "tool")) 字段.push("提示词");
    if (行.旧提示词详情 && (t === "context" || t === "compacted" || t === "system")) 字段.push("上一提示词");
    if (行.旧提示词详情 && 行.提示词详情) 字段.push("提示词diff");
    if (行.输出详情) 字段.push("输出原文");
    if (行.思考详情 && t === "message") 字段.push("思考原文");
    if (行.源块们 && t === "message") 字段.push("源块");
    if (行.输出块们 && (t === "tool" || t === "subtool")) 字段.push("输出块");
    if (行.模式详情 && (t === "tool" || t === "subtool")) 字段.push("schema");
    if (行.助手指标 && t === "message") 字段.push("指标");
    if (行.结果 && (t === "tool" || t === "subtool")) 字段.push("结果");
    if (行.供应者 || 行.模型) 字段.push("模型");
    if (行.重试次 != null || 行.最大重试 != null) 字段.push("重试");
    if (行.是否错误) 字段.push("错误");
    if (行.影响) 字段.push("影响");
    // L2 全量载荷
    字段.push("全量载荷");
    return 字段;
  }

  function 渲染详情面板(行) {
    元素.详情面板.innerHTML = "";
    var 卡 = document.createElement("div");
    卡.className = "详情卡";

    // 头部：序号 + 类型 + LOD 切换
    var 头 = document.createElement("div");
    头.className = "详情头";
    var 标 = document.createElement("div");
    标.className = "详情标";
    标.textContent = "#" + 行.序号 + " · " + (类型标签[行.类型] || 行.类型);
    if (行.是否错误) {
      var 错标 = document.createElement("span");
      错标.className = "详情错标";
      错标.textContent = "错误";
      标.appendChild(错标);
    }
    头.appendChild(标);

    // LOD 切换（§13.f.9）：L1 重点 / L2 全量
    var 详略组 = document.createElement("div");
    详略组.className = "详情lod";
    var l1 = document.createElement("button");
    l1.type = "button"; l1.className = "lod按钮" + (行.详略层级 === 1 ? " 激活" : "");
    l1.textContent = "重点"; l1.dataset.lod = "1";
    var 层2 = document.createElement("button");
    层2.type = "button"; 层2.className = "lod按钮" + (行.详略层级 === 2 ? " 激活" : "");
    层2.textContent = "全量"; 层2.dataset.lod = "2";
    l1.addEventListener("click", function () { 行.详略层级 = 1; 渲染详情面板(行); });
    层2.addEventListener("click", function () { 行.详略层级 = 2; 渲染详情面板(行); });
    详略组.appendChild(l1); 详略组.appendChild(层2);
    头.appendChild(详略组);
    卡.appendChild(头);

    // 基本信息（始终展示）
    卡.appendChild(渲染详情节("基本信息", [
      ["序号", "#" + 行.序号],
      ["类型", 类型标签[行.类型] || 行.类型],
      ["轮次", 行.轮次 != null ? "Turn " + 行.轮次 : ""],
      ["角色", 行.角色],
      ["时刻", 格式化时刻(行.时间戳)],
      ["耗时", 格式化耗时(行.耗时ms)],
      ["源", 行.源],
      ["动作", 行.动作],
      ["输入 token", 格式化token(行.token.输入)],
      ["输出 token", 格式化token(行.token.输出)],
      ["缓存读", 格式化token(行.token.缓存读)],
      ["缓存写", 格式化token(行.token.缓存写)],
      ["推理 token", 格式化token(行.token.推理)]
    ], false));

    // L1 重点字段
    if (行.详略层级 >= 1) {
      if (行.输入详情) {
        卡.appendChild(渲染原文节("输入原文", 行.输入详情, "输入"));
      }
      if (行.提示词详情 && (行.类型 === "message" || 行.类型 === "tool")) {
        卡.appendChild(渲染原文节("提示词", 行.提示词详情, "提示词"));
      }
      if (行.旧提示词详情 && (行.类型 === "context" || 行.类型 === "compacted" || 行.类型 === "system")) {
        卡.appendChild(渲染原文节("上一提示词", 行.旧提示词详情, "上提示词"));
      }
      // diff（previousPromptDetail vs promptDetail）
      if (行.旧提示词详情 && 行.提示词详情) {
        卡.appendChild(渲染diff节("提示词diff", 行.旧提示词详情, 行.提示词详情));
      }
      if (行.输出详情) {
        卡.appendChild(渲染原文节("输出原文", 行.输出详情, "输出"));
      }
      if (行.思考详情 && 行.类型 === "message") {
        卡.appendChild(渲染原文节("思考原文", 行.思考详情, "思考", true));
      }
      if (行.源块们 && 行.类型 === "message") {
        卡.appendChild(渲染块节("源块", 行.源块们));
      }
      if (行.输出块们 && (行.类型 === "tool" || 行.类型 === "subtool")) {
        卡.appendChild(渲染块节("输出块", 行.输出块们));
      }
      if (行.模式详情 && (行.类型 === "tool" || 行.类型 === "subtool")) {
        卡.appendChild(渲染原文节("schema", 行.模式详情, "schema"));
      }
      if (行.助手指标 && 行.类型 === "message") {
        卡.appendChild(渲染指标节(行.助手指标));
      }
      if (行.结果 && (行.类型 === "tool" || 行.类型 === "subtool")) {
        卡.appendChild(渲染详情节("结果", [["result", 行.结果]], false));
      }
      if (行.供应者 || 行.模型) {
        卡.appendChild(渲染详情节("模型", [
          ["供应者", 行.供应者],
          ["模型", 行.模型]
        ], false));
      }
      if (行.重试次 != null || 行.最大重试 != null) {
        卡.appendChild(渲染详情节("重试", [
          ["重试次数", 行.重试次 != null ? 行.重试次 : ""],
          ["最大重试", 行.最大重试 != null ? 行.最大重试 : ""],
          ["重试延迟", 行.重试次DelayMs != null ? 格式化耗时(行.重试次DelayMs) : ""]
        ], false));
      }
      if (行.是否错误) {
        卡.appendChild(渲染详情节("错误", [["错误标记", "是"]], false));
      }
      if (行.影响) {
        卡.appendChild(渲染原文节("影响", typeof 行.影响 === "string" ? 行.影响 : JSON.stringify(行.影响, null, 2), "影响"));
      }
    }

    // L2 全量载荷：原始 JSON pretty
    if (行.详略层级 >= 2) {
      var 全量 = {};
      for (var k in 行) {
        if (k === "详略层级" || k === "已拉全量") continue;
        全量[k] = 行[k];
      }
      卡.appendChild(渲染原文节("全量载荷", JSON.stringify(全量, null, 2), "全量"));
    }

    元素.详情面板.appendChild(卡);
  }

  // 详情分节（键值对）
  function 渲染详情节(标题, 字段们, 默认折叠) {
    var 节 = document.createElement("section");
    节.className = "详情节";
    var 头 = document.createElement("div");
    头.className = "详情节头";
    var 折 = document.createElement("span");
    折.className = "详情节箭";
    折.textContent = "▼";
    头.appendChild(折);
    var 标 = document.createElement("span");
    标.className = "详情节标";
    标.textContent = 标题;
    头.appendChild(标);
    节.appendChild(头);
    var 身 = document.createElement("div");
    身.className = "详情节身";
    if (默认折叠) { 节.classList.add("折叠"); 身.style.display = "none"; }
    for (var i = 0; i < 字段们.length; i++) {
      var k = 字段们[i][0], v = 字段们[i][1];
      if (v === "" || v == null) continue;
      var d = document.createElement("div");
      d.className = "详情字段";
      var 键 = document.createElement("span");
      键.className = "详情键";
      键.textContent = k;
      var 值 = document.createElement("span");
      值.className = "详情值";
      值.textContent = String(v);
      d.appendChild(键); d.appendChild(值);
      身.appendChild(d);
    }
    节.appendChild(身);
    头.addEventListener("click", function () {
      节.classList.toggle("折叠");
      身.style.display = 节.classList.contains("折叠") ? "none" : "block";
    });
    return 节;
  }

  // 原文节（pre-wrap + 搜索高亮 + 可折叠）
  function 渲染原文节(标题, 原文, 类名, 默认折叠) {
    var 节 = document.createElement("section");
    节.className = "详情节 详情原文节";
    节.dataset.类 = 类名;
    var 头 = document.createElement("div");
    头.className = "详情节头";
    var 折 = document.createElement("span");
    折.className = "详情节箭";
    折.textContent = "▼";
    头.appendChild(折);
    var 标 = document.createElement("span");
    标.className = "详情节标";
    标.textContent = 标题;
    头.appendChild(标);
    var 行数估 = 原文.split("\n").length;
    var 估 = document.createElement("span");
    估.className = "详情节估";
    估.textContent = 行数估 + " 行";
    头.appendChild(估);
    节.appendChild(头);
    var 身 = document.createElement("div");
    身.className = "详情节身";
    var pre = document.createElement("pre");
    pre.className = "详情原文";
    pre.appendChild(渲染原文带高亮(原文));
    身.appendChild(pre);
    if (默认折叠) { 节.classList.add("折叠"); 身.style.display = "none"; }
    节.appendChild(身);
    头.addEventListener("click", function () {
      节.classList.toggle("折叠");
      身.style.display = 节.classList.contains("折叠") ? "none" : "block";
    });
    return 节;
  }

  // 原文带搜索高亮（多段高亮，对齐 dsh searchMatchIndexes）
  function 渲染原文带高亮(原文) {
    var 片段 = document.createDocumentFragment();
    var q = 状态.搜索词;
    if (!q || !原文) {
      片段.appendChild(document.createTextNode(原文));
      return 片段;
    }
    var 小写文 = 原文.toLowerCase();
    var 查询小写 = q.toLowerCase();
    var 位置 = 0;
    var 游标 = 小写文.indexOf(查询小写, 位置);
    while (游标 >= 0) {
      if (游标 > 位置) 片段.appendChild(document.createTextNode(原文.slice(位置, 游标)));
      var 命中span = document.createElement("span");
      命中span.className = "命中";
      命中span.textContent = 原文.slice(游标, 游标 + q.length);
      片段.appendChild(命中span);
      位置 = 游标 + q.length;
      游标 = 小写文.indexOf(查询小写, 位置);
    }
    if (位置 < 原文.length) 片段.appendChild(document.createTextNode(原文.slice(位置)));
    return 片段;
  }

  // diff 节（行级 diff，红绿标记，对齐 dsh promptDiffLines）
  function 渲染diff节(标题, 前, 后) {
    var 节 = document.createElement("section");
    节.className = "详情节 详情diff节";
    var 头 = document.createElement("div");
    头.className = "详情节头";
    var 折 = document.createElement("span");
    折.className = "详情节箭";
    折.textContent = "▼";
    头.appendChild(折);
    var 标 = document.createElement("span");
    标.className = "详情节标";
    标.textContent = 标题;
    头.appendChild(标);
    节.appendChild(头);
    var 身 = document.createElement("div");
    身.className = "详情节身";
    var pre = document.createElement("pre");
    pre.className = "详情diff";
    var 行们 = 算diff行(前, 后);
    for (var i = 0; i < 行们.length; i++) {
      var span = document.createElement("span");
      span.className = "diff行 diff-" + 行们[i].类;
      span.textContent = 行们[i].文 + "\n";
      pre.appendChild(span);
    }
    身.appendChild(pre);
    节.appendChild(身);
    头.addEventListener("click", function () {
      节.classList.toggle("折叠");
      身.style.display = 节.classList.contains("折叠") ? "none" : "block";
    });
    return 节;
  }

  // 行级 diff（LCS 简化版，对齐 dsh promptDiffLines 的 meta/context/added/removed 分类）
  function 算diff行(前, 后) {
    var 前行 = String(前).split("\n");
    var 后行 = String(后).split("\n");
    var 果 = [];
    // 简化：逐行比对，公共前缀+公共后缀，中间标 added/removed
    var 公前 = 0;
    while (公前 < 前行.length && 公前 < 后行.length && 前行[公前] === 后行[公前]) 公前++;
    var 公后 = 0;
    while (公后 < 前行.length - 公前 && 公后 < 后行.length - 公前 &&
           前行[前行.length - 1 - 公后] === 后行[后行.length - 1 - 公后]) 公后++;
    // 公共前缀
    for (var i = 0; i < 公前; i++) 果.push({ 类: "context", 文: " " + 前行[i] });
    // 删除行
    for (var j = 公前; j < 前行.length - 公后; j++) 果.push({ 类: "removed", 文: "-" + 前行[j] });
    // 新增行
    for (var k = 公前; k < 后行.length - 公后; k++) 果.push({ 类: "added", 文: "+" + 后行[k] });
    // 公共后缀
    for (var m = 前行.length - 公后; m < 前行.length; m++) 果.push({ 类: "context", 文: " " + 前行[m] });
    return 果;
  }

  // 块节（sourceBlocks / outputBlocks 结构化展示）
  function 渲染块节(标题, 块们) {
    var 节 = document.createElement("section");
    节.className = "详情节 详情块节";
    var 头 = document.createElement("div");
    头.className = "详情节头";
    var 折 = document.createElement("span");
    折.className = "详情节箭";
    折.textContent = "▼";
    头.appendChild(折);
    var 标 = document.createElement("span");
    标.className = "详情节标";
    标.textContent = 标题 + " · " + 块们.length + " 块";
    头.appendChild(标);
    节.appendChild(头);
    var 身 = document.createElement("div");
    身.className = "详情节身";
    for (var i = 0; i < 块们.length; i++) {
      var 块 = 块们[i];
      var 块div = document.createElement("div");
      块div.className = "详情块";
      var 块头 = document.createElement("div");
      块头.className = "详情块头";
      块头.textContent = "[" + i + "] " + (块.type || 块.类型 || "块");
      if (块.toolName || 块.工具名) {
        块头.textContent += " · " + (块.toolName || 块.工具名);
      }
      块div.appendChild(块头);
      var 块文 = 块.content || 块.内容 || "";
      if (块文) {
        var pre = document.createElement("pre");
        pre.className = "详情块文";
        pre.appendChild(渲染原文带高亮(块文));
        块div.appendChild(pre);
      }
      if (块.imageSrc || 块.图源) {
        var img = document.createElement("img");
        img.className = "详情块图";
        img.src = 块.imageSrc || 块.图源;
        img.alt = 块.imageAlt || 块.图注 || "";
        块div.appendChild(img);
      }
      身.appendChild(块div);
    }
    节.appendChild(身);
    头.addEventListener("click", function () {
      节.classList.toggle("折叠");
      身.style.display = 节.classList.contains("折叠") ? "none" : "block";
    });
    return 节;
  }

  // 指标节（TTFT/解码/总/吞吐，对齐 dsh AssistantTimingPanel）
  function 渲染指标节(指标) {
    var 节 = document.createElement("section");
    节.className = "详情节 详情指标节";
    var 头 = document.createElement("div");
    头.className = "详情节头";
    var 折 = document.createElement("span");
    折.className = "详情节箭";
    折.textContent = "▼";
    头.appendChild(折);
    var 标 = document.createElement("span");
    标.className = "详情节标";
    标.textContent = "指标";
    头.appendChild(标);
    节.appendChild(头);
    var 身 = document.createElement("div");
    身.className = "详情节身";
    var 卡 = document.createElement("div");
    卡.className = "指标卡";
    // TTFT = firstTokenTime - stepStartTime
    var ttft = null, 解码 = null, 总 = null, 吞吐 = null;
    var 起 = 指标.stepStartTime || 指标.起始时刻;
    var 首 = 指标.firstTokenTime || 指标.首token时刻;
    var 完 = 指标.completedTime || 指标.完成时刻;
    var 出tok = 指标.outputTokens || 指标.输出token;
    if (起 != null && 首 != null) ttft = 首 - 起;
    if (首 != null && 完 != null) 解码 = 完 - 首;
    if (起 != null && 完 != null) 总 = 完 - 起;
    if (解码 != null && 解码 > 0 && 出tok != null) 吞吐 = (出tok / (解码 / 1000)).toFixed(1) + " tok/s";
    var 项们 = [
      ["TTFT", ttft != null ? 格式化耗时(ttft) : "—"],
      ["解码", 解码 != null ? 格式化耗时(解码) : "—"],
      ["总耗", 总 != null ? 格式化耗时(总) : "—"],
      ["吞吐", 吞吐 != null ? 吞吐 : "—"],
      ["输出 token", 出tok != null ? 格式化token(出tok) : "—"]
    ];
    for (var i = 0; i < 项们.length; i++) {
      var 项 = document.createElement("div");
      项.className = "指标项";
      var k = document.createElement("span");
      k.className = "指标键";
      k.textContent = 项们[i][0];
      var v = document.createElement("span");
      v.className = "指标值";
      v.textContent = 项们[i][1];
      项.appendChild(k); 项.appendChild(v);
      卡.appendChild(项);
    }
    身.appendChild(卡);
    节.appendChild(身);
    return 节;
  }

  // ===== 信源 流式追加（§13.f.2 流式更新） =====
  function 连接信源() {
    if (信源) { 信源.close(); 信源 = null; }
    if (状态.信源暂停) return;
    try {
      信源 = new EventSource("/api/events/stream");
    } catch (e) {
      设状态("败", "信源 不支持");
      return;
    }
    信源.onopen = function () { 设状态("活", "已连接"); };
    信源.on错or = function () { 设状态("败", "连接断开"); };
    信源.addEventListener("tick_event", function (e) {
      try {
        var 数据 = JSON.parse(e.data);
        追加事件(数据);
      } catch (错) { /* 忽略坏包 */ }
    });
    信源.addEventListener("partial", function (e) {
      try {
        var 数据 = JSON.parse(e.data);
        更新流式行(数据);
      } catch (错) { /* 忽略 */ }
    });
    信源.addEventListener("replay", function (e) {
      // 回放事件（§9.2 第 5 事件）：重载
      try {
        var 数据 = JSON.parse(e.data);
        if (数据.since !== undefined || 数据.until !== undefined) {
          状态.时间范围 = { since: 数据.since, until: 数据.until };
          拉取初始();
        }
      } catch (错) { /* 忽略 */ }
    });
  }

  function 追加事件(白箱事件) {
    var 序号 = 状态.事件们.length + 1;
    var 行 = 装配轨迹行(白箱事件, 序号);
    状态.事件们.push(行);

    // 更新最早 ts
    if (状态.最早ts === null || 行.时间戳 < 状态.最早ts) {
      状态.最早ts = 行.时间戳;
    }

    // 更新房间
    var 房间id = 白箱事件.房间id || "默认";
    if (!房间池.has(房间id)) {
      房间池.set(房间id, { id: 房间id, 名: 白箱事件.房间名 || 房间id, 事件数: 0 });
      房间序.push(房间id);
    }
    房间池.get(房间id).事件数++;
    渲染房间列表();
    // 诸圣在位随事件源字段更新
    渲染诸圣列表();
    // 星图指标卡随事件更新
    渲染指标卡();
    // 群聊消息随事件追加（实时过程门面）
    追加群聊消息(行);

    // 增量渲染：若行数少直接 append，否则全量重渲
    if (状态.事件们.length <= 虚拟化阈值) {
      var 分组 = 计算轮次分组();
      var 末组 = 分组[分组.length - 1];
      var 末组div = 元素.行区.querySelector('.轮次组[data-轮次="' + 末组.轮次 + '"]');
      if (末组div && !轮次是否折叠(末组.轮次)) {
        var 身 = 末组div.querySelector(".轮次组身");
        var 新行 = 创建行元素(行);
        新行.classList.add("淡入", "高亮");
        身.appendChild(新行);
        setTimeout(function () { 新行.classList.remove("高亮"); }, 流式高亮ms);
      } else {
        渲染表格();
      }
    } else {
      渲染表格();
    }
    更新汇总();

    // 贴底跟随
    if (状态.跟随) {
      元素.表格容器.scrollTop = 元素.表格容器.scrollHeight;
    }
  }

  function 更新流式行(数据) {
    // partial 流式：逐字追加到正在进行的行
    var id = 数据.id || 状态.流式行id;
    if (!id) return;
    var 行 = null;
    for (var i = 状态.事件们.length - 1; i >= 0; i--) {
      if (状态.事件们[i].id === id) { 行 = 状态.事件们[i]; break; }
    }
    if (!行) return;
    if (数据.摘要增量) 行.摘要 += 数据.摘要增量;
    if (数据.原文增量) 行.完整原文 += 数据.原文增量;
    if (数据.token) {
      行.token.输出 = 数据.token.输出 || 行.token.输出;
      行.token.推理 = 数据.token.推理 || 行.token.推理;
    }
    if (数据.完成) {
      行.流式中 = false;
      状态.流式行id = null;
    } else {
      状态.流式行id = id;
    }
    // 局部更新该行 DOM
    var 行元素 = 元素.行区.querySelector('.事件行[data-id="' + id + '"]');
    if (行元素) {
      var 摘要span = 行元素.querySelector(".行摘要");
      摘要span.textContent = 行.摘要;
    }
    更新汇总();
  }

  function 设状态(态, 文) {
    元素.状态点.className = 态;
    元素.状态文.textContent = 文;
  }

  // ===== 历史加载（§13.f.2 历史加载） =====
  function 拉取初始() {
    console.log("[拉取初始] 开始");
    状态.历史加载中 = true;
    元素.历史加载.hidden = false;
    var 地址 = "/api/events/recent?limit=" + 历史页大小;
    console.log("[拉取初始] 地址=", 地址);
    fetch(地址).then(function (r) { console.log("[拉取初始] 响应", r.status); return r.json(); }).then(function (数组) {
      console.log("[拉取初始] 收到", 数组.length, "条");
      document.title = "收到" + 数组.length + "条";
      状态.事件们 = [];
      状态.最早ts = null;
      for (var i = 0; i < 数组.length; i++) {
        var 行 = 装配轨迹行(数组[i], i + 1);
        状态.事件们.push(行);
        if (状态.最早ts === null || 行.时间戳 < 状态.最早ts) 状态.最早ts = 行.时间戳;
      }
      document.title = "装配" + 状态.事件们.length + "条";
      状态.已到最早 = 数组.length < 历史页大小;
      元素.已到最早.hidden = !状态.已到最早;
      状态.历史加载中 = false;
      元素.历史加载.hidden = true;
      渲染表格();
      渲染诸圣列表();
      渲染世界星图();
      渲染群聊流(); // 历史事件加载后渲染群聊流
      document.title = "渲染" + 状态.事件们.length + "条";
      // 贴底
      元素.表格容器.scrollTop = 元素.表格容器.scrollHeight;
      状态.跟随 = true;
    }).catch(function (e) {
      document.title = "拉取失败:" + e.message;
      状态.历史加载中 = false;
      元素.历史加载.hidden = true;
      设状态("败", "拉取失败");
    });
  }

  function 拉取更早() {
    if (状态.历史加载中 || 状态.已到最早) return;
    if (状态.最早ts === null) return;
    状态.历史加载中 = true;
    元素.历史加载.hidden = false;
    var 地址 = "/api/trajectory?before=" + 状态.最早ts + "&limit=" + 历史页大小;
    fetch(地址).then(function (r) { return r.json(); }).then(function (数组) {
      if (数组.length === 0) {
        状态.已到最早 = true;
        元素.已到最早.hidden = false;
      } else {
        // 保留当前滚动位置（不跳屏）
        var 旧高 = 元素.表格容器.scrollHeight;
        var 旧顶 = 元素.表格容器.scrollTop;

        // 顶部插入 + 序号重编
        var 新行 = [];
        for (var i = 0; i < 数组.length; i++) {
          新行.push(装配轨迹行(数组[i], 0));  // 序号暂 0，后面重编
        }
        // 合并：新行在前，旧行在后，按 ts 升序
        var 合并 = 新行.concat(状态.事件们);
        合并.sort(function (a, b) { return a.时间戳 - b.时间戳; });
        // 序号重编
        for (var j = 0; j < 合并.length; j++) {
          合并[j].序号 = j + 1;
          合并[j].id = 事件id({ 类型: 合并[j].类型, 序号: 合并[j].序号, 时间戳: 合并[j].时间戳 });
        }
        状态.事件们 = 合并;
        状态.最早ts = 合并[0].时间戳;
        状态.已到最早 = 数组.length < 历史页大小;
        元素.已到最早.hidden = !状态.已到最早;

        渲染表格();
        渲染诸圣列表();

        // 还原滚动位置
        var 新高 = 元素.表格容器.scrollHeight;
        元素.表格容器.scrollTop = 旧顶 + (新高 - 旧高);
      }
      状态.历史加载中 = false;
      元素.历史加载.hidden = true;
    }).catch(function () {
      状态.历史加载中 = false;
      元素.历史加载.hidden = true;
    });
  }

  // ===== 滚动处理（虚拟滚动 + 历史加载 + 贴底跟随） =====
  function onScroll() {
    if (滚动rAF !== null) return;
    滚动rAF = requestAnimationFrame(function () {
      滚动rAF = null;
      var 容器 = 元素.表格容器;
      var 距底 = 容器.scrollHeight - 容器.clientHeight - 容器.scrollTop;
      var 距顶 = 容器.scrollTop;

      // 贴底跟随判定
      var 新跟随 = 距底 <= 贴底阈值px;
      if (新跟随 !== 状态.跟随) {
        状态.跟随 = 新跟随;
        元素.脱离最新.hidden = 新跟随;
      }

      // 历史加载触发
      if (距顶 <= 历史加载阈值px && !状态.历史加载中 && !状态.已到最早) {
        拉取更早();
      }

      // 虚拟滚动重渲
      if (虚拟化启用) {
        渲染表格();
      }
    });
  }

  // ===== 诸圣在位（§13.f.10 角色体系） =====
  // 17 角色：天层双神 + 中层五圣 + 底层四大罗金仙 + 底层六准圣
  // 色彩引用 style.css 的 --色-XXX 变量，职能对齐角色本性
  var 诸圣名录 = [
    { 名: "鸿钧",   职能: "道祖·主政",     层: "天", 色变量: "--色-鸿钧" },
    { 名: "天道",   职能: "巡世·世界之眼", 层: "天", 色变量: "--色-天道" },
    { 名: "女娲",   职能: "造化",         层: "圣", 色变量: "--色-女娲" },
    { 名: "老子",   职能: "道德",         层: "圣", 色变量: "--色-老子" },
    { 名: "元始",   职能: "秩序",         层: "圣", 色变量: "--色-元始" },
    { 名: "通天",   职能: "杀伐",         层: "圣", 色变量: "--色-通天" },
    { 名: "后土",   职能: "轮回",         层: "圣", 色变量: "--色-后土" },
    { 名: "多宝",   职能: "代码炼化",     层: "金", 色变量: "--色-多宝" },
    { 名: "白泽",   职能: "前端",         层: "金", 色变量: "--色-白泽" },
    { 名: "龟灵",   职能: "数据库",       层: "金", 色变量: "--色-龟灵" },
    { 名: "玄天",   职能: "监控",         层: "金", 色变量: "--色-玄天" },
    { 名: "红云",   职能: "业务正确性",   层: "准", 色变量: "--色-红云" },
    { 名: "镇元子", 职能: "数据完整性",   层: "准", 色变量: "--色-镇元子" },
    { 名: "鲲鹏",   职能: "性能并发",     层: "准", 色变量: "--色-鲲鹏" },
    { 名: "神农",   职能: "安全副作用",   层: "准", 色变量: "--色-神农" },
    { 名: "冥河",   职能: "异常兼容",     层: "准", 色变量: "--色-冥河" },
    { 名: "轩辕",   职能: "用户体验",     层: "准", 色变量: "--色-轩辕" }
  ];

  // 从源字段推断洪荒角色（源格式：观测/{域}·{角色} 或 事件流）
  // 提示词/回复 → 鸿钧，工具调用/返回 → 多宝，产物判定 → 红云，设计 → 女娲
  function 从源推断角色(源) {
    if (!源) return null;
    if (源.indexOf("提示词") >= 0) return "鸿钧";
    if (源.indexOf("回复思考") >= 0) return "鸿钧";
    if (源.indexOf("回复内容") >= 0) return "鸿钧";
    if (源.indexOf("工具调用") >= 0) return "多宝";
    if (源.indexOf("工具返回") >= 0) return "多宝";
    if (源.indexOf("产物判定") >= 0) return "红云";
    if (源.indexOf("设计") >= 0) return "女娲";
    return null;
  }

  // 从动作字段推断角色（源字段无信息时的fallback）
  function 从动作推断角色(动作) {
    if (!动作) return null;
    if (动作.indexOf("提示词") >= 0) return "鸿钧";
    if (动作.indexOf("回复思考") >= 0 || 动作.indexOf("思考") >= 0) return "鸿钧";
    if (动作.indexOf("回复内容") >= 0 || 动作.indexOf("回复") >= 0) return "鸿钧";
    if (动作.indexOf("工具调用") >= 0 || 动作.indexOf("调用") >= 0) return "多宝";
    if (动作.indexOf("工具返回") >= 0 || 动作.indexOf("返回") >= 0) return "多宝";
    if (动作.indexOf("产物判定") >= 0 || 动作.indexOf("判定") >= 0) return "红云";
    if (动作.indexOf("设计") >= 0) return "女娲";
    return null;
  }

  // 从源/动作推断事件类型（发言/想法/回复/工具/结果/验证/事件）
  function 推断事件类型(源, 动作) {
    if (!源) 源 = "";
    if (!动作) 动作 = "";
    if (源.indexOf("提示词") >= 0) return "发言";
    if (源.indexOf("回复思考") >= 0) return "想法";
    if (源.indexOf("回复内容") >= 0) return "回复";
    if (源.indexOf("工具调用") >= 0) return "工具";
    if (源.indexOf("工具返回") >= 0) return "结果";
    if (源.indexOf("产物判定") >= 0) return "验证";
    if (动作.indexOf("工具调用") >= 0) return "工具";
    if (动作.indexOf("工具返回") >= 0) return "结果";
    return "事件";
  }

  // 从 状态.事件们 的 源 字段提取在位角色
  function 算在位诸圣() {
    var 在位 = new Set();
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 源 = 状态.事件们[i].源 || "";
      // 先直接匹配角色名
      for (var j = 0; j < 诸圣名录.length; j++) {
        var 名 = 诸圣名录[j].名;
        if (源.indexOf(名) >= 0) 在位.add(名);
      }
      // 再从源字段推断角色
      var 推断 = 从源推断角色(源);
      if (!推断) 推断 = 从动作推断角色(状态.事件们[i].动作 || "");
      if (推断) 在位.add(推断);
    }
    return 在位;
  }

  // 从 状态.事件们 按 源 字段分组，取每个角色最新的事件（最后一个匹配的事件）
  // 返回 Map<角色名, {动作, 摘要, 证据, token, 耗时ms, 时间戳, 轮次, 结果, 供应者, 模型}>
  function 提取诸圣最新状态() {
    var 映射 = new Map();
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      var 源文 = 行.源 || "";
      // 先直接匹配角色名
      var 命中 = false;
      for (var j = 0; j < 诸圣名录.length; j++) {
        var 圣名 = 诸圣名录[j].名;
        if (源文.indexOf(圣名) < 0) continue;
        命中 = true;
        映射.set(圣名, {
          动作: 行.动作 || "",
          摘要: 行.摘要 || "",
          证据: 行.完整原文 || "",
          token: 行.token || {},
          耗时ms: 行.耗时ms != null ? 行.耗时ms : null,
          时间戳: 行.时间戳 || null,
          轮次: 行.轮次 || null,
          结果: 行.result || "",
          供应者: 行.供应者 || "",
          模型: 行.模型 || ""
        });
      }
      // 再从源字段推断角色
      if (!命中) {
        var 推断 = 从源推断角色(源文);
        if (!推断) 推断 = 从动作推断角色(行.动作 || "");
        if (推断) {
          映射.set(推断, {
            动作: 行.动作 || "",
            摘要: 行.摘要 || "",
            证据: 行.完整原文 || "",
            token: 行.token || {},
            耗时ms: 行.耗时ms != null ? 行.耗时ms : null,
            时间戳: 行.时间戳 || null,
            轮次: 行.轮次 || null,
            结果: 行.result || "",
            供应者: 行.供应者 || "",
            模型: 行.模型 || ""
          });
        }
      }
    }
    return 映射;
  }

  // 截断文本到指定字数，超长加省略号
  function 截断文(文, 上限) {
    if (!文) return "";
    var 字 = String(文);
    if (字.length <= 上限) return 字;
    return 字.slice(0, 上限) + "…";
  }

  // 渲染诸圣列表：每角色一条，含色点/名/职能/在位标
  function 渲染诸圣列表() {
    if (!元素.诸圣列表) return;
    元素.诸圣列表.innerHTML = "";
    var 在位 = 算在位诸圣();
    var 片段 = document.createDocumentFragment();
    for (var i = 0; i < 诸圣名录.length; i++) {
      var 圣 = 诸圣名录[i];
      var 条 = document.createElement("div");
      条.className = "角色条";
      条.dataset.名 = 圣.名;
      条.dataset.层 = 圣.层;
      if (在位.has(圣.名)) 条.classList.add("在位");

      // 色点（引用 CSS 变量 --色-XXX）
      var 点 = document.createElement("span");
      点.className = "角色色点";
      点.style.background = "var(" + 圣.色变量 + ")";
      条.appendChild(点);

      // 角色名
      var 名span = document.createElement("span");
      名span.className = "角色名";
      名span.textContent = 圣.名;
      条.appendChild(名span);

      // 职能
      var 职span = document.createElement("span");
      职span.className = "角色职能";
      职span.textContent = 圣.职能;
      条.appendChild(职span);

      // 在位标记
      if (在位.has(圣.名)) {
        var 标 = document.createElement("span");
        标.className = "角色在位标";
        标.textContent = "在位";
        条.appendChild(标);
      }

      片段.appendChild(条);
    }
    元素.诸圣列表.appendChild(片段);
  }

  // ===== 房间列表 =====
  function 渲染房间列表() {
    元素.房间列表.innerHTML = "";
    var 片段 = document.createDocumentFragment();
    for (var i = 0; i < 房间序.length; i++) {
      var 房 = 房间池.get(房间序[i]);
      var div = document.createElement("div");
      div.className = "房间条";
      if (房.id === 当前房间id) div.classList.add("激活");
      div.dataset.id = 房.id;
      var 名 = document.createElement("span");
      名.className = "房间名";
      名.textContent = 房.名;
      var 数 = document.createElement("span");
      数.className = "房间数";
      数.textContent = 房.事件数;
      div.appendChild(名); div.appendChild(数);
      div.addEventListener("click", function (id) {
        return function () {
          当前房间id = id;
          渲染房间列表();
        };
      }(房.id));
      片段.appendChild(div);
    }
    元素.房间列表.appendChild(片段);
  }

  // ===== 搜索（§13.f.5 · 全文索引 · 节流 3s · 高亮 · 导航） =====
  // 对齐 dsh TrajectorySearchIndex：增量索引，3s 节流，多关键词空格分隔
  var 搜索定时 = null;
  var 搜索索引 = new Map();      // id → 文本（小写）
  var 搜索命中序 = [];           // 命中 id 列表（按事件顺序）
  var 搜索当位置 = -1;            // 当前高亮命中 位置

  function 搜索输入时() {
    var q = 元素.搜索输入.value.trim();
    状态.搜索词 = q;
    if (搜索定时) clearTimeout(搜索定时);
    搜索定时 = setTimeout(function () {
      执行搜索(q);
    }, 搜索节流ms);
  }

  // 建索引（对齐 dsh TrajectorySearchIndex.update）
  function 建搜索索引() {
    搜索索引.clear();
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      var 文本们 = [
        行.摘要, 行.源, 行.动作, 行.完整原文,
        行.输入详情, 行.提示词详情, 行.旧提示词详情,
        行.输出详情, 行.思考详情, 行.模式详情, 行.结果,
        行.供应者, 行.模型
      ];
      if (行.影响) 文本们.push(typeof 行.影响 === "string" ? 行.影响 : JSON.stringify(行.影响));
      if (行.源块们) {
        for (var j = 0; j < 行.源块们.length; j++) {
          文本们.push(行.源块们[j].type || "", 行.源块们[j].content || "");
        }
      }
      if (行.输出块们) {
        for (var k = 0; k < 行.输出块们.length; k++) {
          文本们.push(行.输出块们[k].type || "", 行.输出块们[k].content || "");
        }
      }
      var 合 = 文本们.join("\n").toLowerCase();
      搜索索引.set(行.id, 合);
    }
  }

  function 执行搜索(q) {
    状态.搜索命中 = new Set();
    搜索命中序 = [];
    搜索当位置 = -1;
    if (!q) {
      元素.搜索计数.textContent = "";
      渲染表格();
      渲染时间线();
      return;
    }
    // 建索引（事件可能新增）
    建搜索索引();
    // 多关键词空格分隔（对齐 dsh search：terms.every(includes)）
    var 词们 = q.toLowerCase().split(/\s+/).filter(Boolean);
    // 范围：当前轮次 / 全部
    var 限轮次 = (元素.搜索范围 && 元素.搜索范围.value === "当前轮次") ? 状态.选中轮次 : null;
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      if (限轮次 != null && 行.轮次 !== 限轮次) continue;
      var 文 = 搜索索引.get(行.id);
      if (!文) continue;
      var 全命中 = true;
      for (var j = 0; j < 词们.length; j++) {
        if (文.indexOf(词们[j]) < 0) { 全命中 = false; break; }
      }
      if (全命中) {
        状态.搜索命中.add(行.id);
        搜索命中序.push(行.id);
      }
    }
    var n = 状态.搜索命中.size;
    元素.搜索计数.textContent = n + " 命中";
    // 显示导航按钮
    if (元素.搜索上一个) 元素.搜索上一个.style.display = n > 0 ? "" : "none";
    if (元素.搜索下一个) 元素.搜索下一个.style.display = n > 0 ? "" : "none";
    渲染表格();
    渲染时间线();
    // 自动跳到第一个命中
    if (n > 0) 跳命中(0);
  }

  function 跳命中(位置) {
    if (搜索命中序.length === 0) return;
    搜索当位置 = ((位置 % 搜索命中序.length) + 搜索命中序.length) % 搜索命中序.length;
    var id = 搜索命中序[搜索当位置];
    元素.搜索计数.textContent = (搜索当位置 + 1) + "/" + 搜索命中序.length + " 命中";
    // 选中该行
    for (var i = 0; i < 状态.事件们.length; i++) {
      if (状态.事件们[i].id === id) {
        选中行(状态.事件们[i]);
        var 行元素 = 元素.行区.querySelector('.事件行[data-id="' + id + '"]');
        if (行元素) 行元素.scrollIntoView({ block: "center", behavior: "smooth" });
        break;
      }
    }
  }

  function 搜索上一个() { 跳命中(搜索当位置 - 1); }
  function 搜索下一个() { 跳命中(搜索当位置 + 1); }

  // ===== 时间线（§13.f.4 · 4 模式 · Chrome-Network 风格） =====
  // 三车道（对齐 dsh laneFor）：0=输入(system/user/context/compacted) 1=模型(message) 2=工具(tool/subtool)
  function 时间线车道(类型) {
    if (类型 === "tool" || 类型 === "subtool") return 2;
    if (类型 === "message" || 类型 === "compacted") return 1;
    return 0;
  }

  // 7 种事件类型色（与 CSS --系/--用/--注/--缩/--消/--具/--子 对齐）
  var 时间线类型色 = {
    system: "var(--系)", user: "var(--用)", context: "var(--注)",
    compacted: "var(--缩)", message: "var(--消)", tool: "var(--具)", subtool: "var(--子)"
  };

  // 模式 → 每事件 span 的 {start, end}（对齐 dsh deriveTrajectoryTimeline）
  function 算时间线span们(模式) {
    if (状态.事件们.length === 0) return null;
    var span们 = [];
    if (模式 === "sequence") {
      // 序号模式：每事件等宽 1
      for (var i = 0; i < 状态.事件们.length; i++) {
        var e = 状态.事件们[i];
        span们.push({ 起: i, 止: i + 1, 序号: e.序号, 类型: e.类型, 事件: e, 车: 时间线车道(e.类型) });
      }
      return { 起: 0, 止: 状态.事件们.length, span们: span们 };
    }
    // duration / time / actual 都基于时间戳
    var 用耗时 = (模式 === "duration" || 模式 === "actual");
    var 压空闲 = (模式 === "duration");  // duration 压缩空闲，actual 不压
    var 原始span = [];
    for (var j = 0; j < 状态.事件们.length; j++) {
      var e2 = 状态.事件们[j];
      var st = e2.起始时刻 || e2.时间戳;
      var du = 0;
      if (用耗时 && e2.耗时ms != null && isFinite(e2.耗时ms)) du = Math.max(0, e2.耗时ms);
      // actual 模式：若有 assistantMetrics 的 stepStartTime/completedTime 用之
      if (模式 === "actual" && e2.助手指标) {
        var m = e2.助手指标;
        var s = m.stepStartTime || m.起始时刻;
        var c = m.completedTime || m.完成时刻;
        if (s != null && c != null) { st = s; du = Math.max(0, c - s); }
      }
      原始span.push({ 起: st, 止: st + du, 序号: e2.序号, 类型: e2.类型, 事件: e2, 车: 时间线车道(e2.类型) });
    }
    // 排序
    原始span.sort(function (a, b) { return a.起 - b.起 || a.止 - b.止; });
    // 压缩空闲
    var 偏移表 = new Array(原始span.length).fill(0);
    var 累计删 = 0;
    var 覆盖止 = null;
    for (var k = 0; k < 原始span.length; k++) {
      if (压空闲 && 覆盖止 != null && 原始span[k].起 > 覆盖止) 累计删 += 原始span[k].起 - 覆盖止;
      偏移表[k] = 累计删;
      覆盖止 = 覆盖止 == null ? 原始span[k].止 : Math.max(覆盖止, 原始span[k].止);
    }
    var 投影 = [];
    for (var n = 0; n < 原始span.length; n++) {
      投影.push({
        起: 原始span[n].起 - 偏移表[n],
        止: (用耗时 ? 原始span[n].止 : 原始span[n].起) - 偏移表[n],
        序号: 原始span[n].序号, 类型: 原始span[n].类型, 事件: 原始span[n].事件, 车: 原始span[n].车
      });
    }
    if (投影.length === 0) return null;
    var 域起 = Infinity, 域止 = -Infinity;
    for (var p = 0; p < 投影.length; p++) {
      if (投影[p].起 < 域起) 域起 = 投影[p].起;
      if (投影[p].止 > 域止) 域止 = 投影[p].止;
    }
    return { 起: 域起, 止: 域止, span们: 投影 };
  }

  function 渲染时间线() {
    var 模式 = 状态.时间线模式;
    元素.时间线色块.innerHTML = "";
    if (状态.事件们.length === 0) return;

    var 模型 = 算时间线span们(模式);
    if (!模型) return;
    var 域宽 = Math.max(1, 模型.止 - 模型.起);
    var 容器宽 = 元素.时间线色块.clientWidth || 100;
    var 片段 = document.createDocumentFragment();

    // 三车道背景
    var 道们 = document.createElement("div");
    道们.className = "时间线道们";
    for (var d = 0; d < 3; d++) {
      var 道 = document.createElement("div");
      道.className = "时间线道";
      道.dataset.道 = d;
      道们.appendChild(道);
    }
    片段.appendChild(道们);

    // span 色块
    for (var i = 0; i < 模型.span们.length; i++) {
      var s = 模型.span们[i];
      var 块 = document.createElement("div");
      块.className = "时间线块";
      块.dataset.类型 = s.类型;
      块.dataset.序号 = s.序号;
      块.dataset.道 = s.车;
      var 左比 = (s.起 - 模型.起) / 域宽;
      var 宽比 = Math.max(0.002, (s.止 - s.起) / 域宽);  // 最小可见宽 0.2%
      块.style.left = (左比 * 100) + "%";
      块.style.width = (宽比 * 100) + "%";
      块.style.background = 时间线类型色[s.类型] || "var(--弱)";
      // 命中搜索高亮
      if (状态.搜索词 && 状态.搜索命中.has(s.事件.id)) 块.classList.add("命中");
      // 选中
      if (状态.选中id === s.事件.id) 块.classList.add("选中");
      // 时间线范围筛选内
      if (状态.时间范围 && s.事件.时间戳 >= 状态.时间范围.since && s.事件.时间戳 <= 状态.时间范围.until) {
        块.classList.add("范围内");
      }
      // tooltip（对齐 dsh timelineTooltipLabel）
      块.title = 算时间线tooltip(s.事件, 模式);
      // 点击跳转到对应表格行
      块.addEventListener("click", function (事件引用) {
        return function (e) {
          e.stopPropagation();
          选中行(事件引用);
          var 行元素 = 元素.行区.querySelector('.事件行[data-id="' + 事件引用.id + '"]');
          if (行元素) 行元素.scrollIntoView({ block: "center", behavior: "smooth" });
        };
      }(s.事件));
      道们.appendChild(块);
    }

    // 选区高亮
    if (状态.时间范围) {
      var 选 = document.createElement("div");
      选.className = "时间线选区";
      var 起比 = Math.max(0, Math.min(1, (状态.时间范围.since - 模型.起) / 域宽));
      var 止比 = Math.max(0, Math.min(1, (状态.时间范围.until - 模型.起) / 域宽));
      选.style.left = (起比 * 100) + "%";
      选.style.width = ((止比 - 起比) * 100) + "%";
      片段.appendChild(选);
    }

    元素.时间线色块.appendChild(片段);
  }

  // 时间线 tooltip（对齐 dsh timelineTooltipLabel）
  function 算时间线tooltip(事件, 模式) {
    var 标 = 类型标签[事件.类型] || 事件.类型;
    var 行 = [
      "#" + 事件.序号 + " · " + 标,
      "时刻 " + 格式化时刻(事件.时间戳)
    ];
    if (事件.耗时ms != null) 行.push("耗时 " + 格式化耗时(事件.耗时ms));
    if (事件.助手指标) {
      var m = 事件.助手指标;
      var 起 = m.stepStartTime || m.起始时刻;
      var 首 = m.firstTokenTime || m.首token时刻;
      var 完 = m.completedTime || m.完成时刻;
      if (起 != null && 首 != null) 行.push("TTFT " + 格式化耗时(首 - 起));
      if (首 != null && 完 != null) 行.push("解码 " + 格式化耗时(完 - 首));
    }
    if (事件.供应者) 行.push("供应者 " + 事件.供应者);
    if (事件.模型) 行.push("模型 " + 事件.模型);
    return 行.join("\n");
  }

  // ===== 折叠模式切换（§13.f.6 · URL hash 记忆） =====
  function 切折叠模式() {
    var 序 = ["无", "按轮次", "按消息", "全部"];
    var 位置 = 序.indexOf(状态.折叠模式);
    状态.折叠模式 = 序[(位置 + 1) % 序.length];
    if (状态.折叠模式 === "全部") {
      for (var i = 0; i < 状态.事件们.length; i++) {
        if (状态.事件们[i].轮次 != null) 状态.折叠集.轮次.add(状态.事件们[i].轮次);
      }
    } else if (状态.折叠模式 === "无") {
      状态.折叠集.轮次.clear();
      状态.折叠集.消息.clear();
    }
    写折叠hash();
    渲染表格();
  }

  // 折叠状态入 URL hash（§13.f.6：#trajectory?collapse=turn:3,5;msg:7）
  function 写折叠hash() {
    var 段 = [];
    if (状态.折叠集.轮次.size > 0) {
      段.push("turn:" + Array.from(状态.折叠集.轮次).join(","));
    }
    if (状态.折叠集.消息.size > 0) {
      段.push("msg:" + Array.from(状态.折叠集.消息).join(","));
    }
    if (段.length === 0) {
      history.replaceState(null, "", location.pathname + location.search);
    } else {
      history.replaceState(null, "", "#trajectory?collapse=" + 段.join(";"));
    }
  }

  function 读折叠hash() {
    var h = location.hash;
    if (!h || h.indexOf("collapse=") < 0) return;
    var q = h.split("collapse=")[1];
    var 段 = q.split(";");
    for (var i = 0; i < 段.length; i++) {
      var p = 段[i].split(":");
      if (p[0] === "turn") {
        var id列表 = p[1] ? p[1].split(",") : [];
        for (var j = 0; j < id列表.length; j++) {
          var n = parseInt(id列表[j], 10);
          if (!isNaN(n)) 状态.折叠集.轮次.add(n);
        }
      } else if (p[0] === "msg") {
        var id列表2 = p[1] ? p[1].split(",") : [];
        for (var k = 0; k < id列表2.length; k++) {
          if (id列表2[k]) 状态.折叠集.消息.add(id列表2[k]);
        }
      }
    }
    if (状态.折叠集.轮次.size > 0 || 状态.折叠集.消息.size > 0) {
      状态.折叠模式 = 状态.折叠集.轮次.size > 0 ? "按轮次" : "按消息";
    }
  }

  // ===== 顶栏时刻 + 运行时长 =====
  function 更新时刻() {
    var d = new Date();
    元素.时刻.textContent = 格式化时刻(d.getTime());
    if (服务启动时刻ms) {
      var ms = Date.now() - 服务启动时刻ms;
      var s = Math.floor(ms / 1000);
      var h = Math.floor(s / 3600);
      var m = Math.floor((s % 3600) / 60);
      var ss = s % 60;
      元素.运行时长.textContent = (h > 0 ? h + "h " : "") + (m > 0 ? m + "m " : "") + ss + "s";
    }
  }

  // ===== 键盘 =====
  function onKey(e) {
    if (e.key === "p" || e.key === "P") { 切暂停(); return; }
    if (e.key === "f" || e.key === "F") { 切折叠模式(); return; }
    if (e.key === "/") { e.preventDefault(); 开搜索(); return; }
    if (e.key === "?") { $("快捷键浮层").hidden = false; return; }
    if (e.key === "Escape") {
      $("快捷键浮层").hidden = true;
      元素.搜索条.hidden = true;
      清选中();
      return;
    }
    if (e.key === " ") { e.preventDefault(); 回最新(); return; }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); 移选(e.key === "ArrowDown" ? 1 : -1); return; }
    if (e.key === "Enter") { 展开选中(); return; }
  }

  function 切暂停() {
    状态.信源暂停 = !状态.信源暂停;
    var 按钮 = $("暂停信源");
    if (状态.信源暂停) {
      按钮.classList.add("激活");
      按钮.querySelector(".工具文").textContent = "继续";
      if (信源) { 信源.close(); 信源 = null; }
      设状态("静", "已暂停");
    } else {
      按钮.classList.remove("激活");
      按钮.querySelector(".工具文").textContent = "暂停";
      连接信源();
    }
  }

  function 开搜索() {
    元素.搜索条.hidden = false;
    元素.搜索输入.focus();
  }

  function 清选中() {
    状态.选中id = null;
    状态.选中轮次 = null;
    var 旧 = 元素.行区.querySelector(".事件行.选中");
    if (旧) 旧.classList.remove("选中");
    元素.详情面板.innerHTML = '<div class="详情占位"><p>点任意事件行</p><p>看完整原文</p></div>';
    更新汇总();
    渲染时间线();
  }

  function 回最新() {
    元素.表格容器.scrollTop = 元素.表格容器.scrollHeight;
    状态.跟随 = true;
    元素.脱离最新.hidden = true;
  }

  function 移选(方向) {
    if (!状态.选中id) {
      if (状态.事件们.length > 0) 选中行(状态.事件们[状态.事件们.length - 1]);
      return;
    }
    var 位置 = -1;
    for (var i = 0; i < 状态.事件们.length; i++) {
      if (状态.事件们[i].id === 状态.选中id) { 位置 = i; break; }
    }
    var 新位置 = 位置 + 方向;
    if (新位置 < 0 || 新位置 >= 状态.事件们.length) return;
    选中行(状态.事件们[新位置]);
    // 滚到可见
    var 行元素 = 元素.行区.querySelector('.事件行[data-id="' + 状态.事件们[新位置].id + '"]');
    if (行元素) 行元素.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }

  function 展开选中() {
    if (!状态.选中id) return;
    document.dispatchEvent(new CustomEvent("轨迹:展开", { detail: 状态.选中id }));
  }

  // ===== 初始化 =====
  function 初始化() {
    缓存元素();

    // 诸圣在位（初始渲染，事件到达后再更新）
    渲染诸圣列表();

    // 三栏拖拽
    装分隔条(元素.左分隔, "左");
    装分隔条(元素.右分隔, "右");
    应用三栏宽();

    // 左/右栏收起按钮
    $("左栏收起").addEventListener("click", function () { 左栏收起 = !左栏收起; 应用三栏宽(); });
    $("右栏收起").addEventListener("click", function () { 右栏收起 = !右栏收起; 应用三栏宽(); });

    // 滚动
    元素.表格容器.addEventListener("scroll", onScroll, { passive: true });

    // 顶栏按钮
    $("暂停信源").addEventListener("click", 切暂停);
    $("折叠按钮").addEventListener("click", 切折叠模式);
    $("搜索按钮").addEventListener("click", 开搜索);
    $("快捷键按钮").addEventListener("click", function () { $("快捷键浮层").hidden = false; });
    $("快捷键关闭").addEventListener("click", function () { $("快捷键浮层").hidden = true; });

    // 搜索
    元素.搜索输入.addEventListener("input", 搜索输入时);
    $("搜索关闭").addEventListener("click", function () {
      元素.搜索条.hidden = true;
      元素.搜索输入.value = "";
      状态.搜索词 = "";
      状态.搜索命中 = new Set();
      搜索命中序 = [];
      搜索当位置 = -1;
      元素.搜索计数.textContent = "";
      渲染表格();
      渲染时间线();
    });

    // 搜索导航按钮 + 范围切换（任务131 注入）
    注入搜索控件();
    // 汇总范围切换（任务131 注入）
    注入汇总控件();

    // 回最新
    $("回最新").addEventListener("click", 回最新);

    // 时间线模式
    var 模式按钮 = 元素.时间线模式.querySelectorAll("button");
    for (var i = 0; i < 模式按钮.length; i++) {
      模式按钮[i].addEventListener("click", function (按钮) {
        return function () {
          for (var k = 0; k < 模式按钮.length; k++) {
            模式按钮[k].classList.remove("激活");
            模式按钮[k].setAttribute("aria-checked", "false");
          }
          按钮.classList.add("激活");
          按钮.setAttribute("aria-checked", "true");
          状态.时间线模式 = 按钮.dataset.模式;
          渲染时间线();
        };
      }(模式按钮[i]));
    }

    // 时间线拖拽选范围
    装时间线拖拽();

    // 清除范围
    元素.清除范围.addEventListener("click", function () {
      状态.时间范围 = null;
      元素.清除范围.hidden = true;
      拉取初始();
    });

    // 键盘
    document.addEventListener("keydown", onKey);

    // 时刻
    更新时刻();
    setInterval(更新时刻, 1000);

    // 拉服务启动时刻 + 定期轮询快照（实时过程门面，5 秒一刷中心层与指标卡）
    fetch("/api/snapshot").then(function (r) { return r.json(); }).then(function (快照) {
      if (快照 && 快照.启动时刻) 服务启动时刻ms = 快照.启动时刻;
      状态.快照 = 快照;
      渲染世界星图();
      更新鸿钧中心状态();
    }).catch(function () { /* 端点不存在不影响 */ });
    setInterval(拉取快照, 5000);

    // 拉初始轨迹
    读折叠hash();
    拉取初始();

    // 连 信源
    连接信源();

    // 窗口缩放重渲时间线
    window.addEventListener("resize", function () {
      渲染时间线();
      if (虚拟化启用) 渲染表格();
    });

    // ===== 任务146：四视图切换 + 深空背景 + 星系/星图/私聊 初始化 =====
    初始化深空背景();
    绑定视图切换();
    渲染群聊流();
    渲染世界星图();
    渲染私聊会话列表();
    // 默认视图是群聊，无需星系动画
    if (当前视图 === "星系") 启动星系动画();
  }

  function 装时间线拖拽() {
    var 拖中 = false;
    var 起x = 0;
    var 起ts = 0;
    元素.时间线色块.addEventListener("pointerdown", function (e) {
      if (状态.事件们.length === 0) return;
      拖中 = true;
      起x = e.clientX;
      var 矩形 = 元素.时间线色块.getBoundingClientRect();
      var 比 = (e.clientX - 矩形.left) / 矩形.width;
      var 起位置 = Math.floor(比 * 状态.事件们.length);
      起ts = 状态.事件们[Math.max(0, Math.min(状态.事件们.length - 1, 起位置))].时间戳;
    });
    元素.时间线色块.addEventListener("pointermove", function (e) {
      if (!拖中) return;
      var 矩形 = 元素.时间线色块.getBoundingClientRect();
      var 比起 = (起x - 矩形.left) / 矩形.width;
      var 比止 = (e.clientX - 矩形.left) / 矩形.width;
      var 左 = Math.min(比起, 比止);
      var 右 = Math.max(比起, 比止);
      // 实时高亮选区
      var 旧选 = 元素.时间线色块.querySelector(".时间线选区");
      if (旧选) 旧选.remove();
      var 选 = document.createElement("div");
      选.className = "时间线选区";
      选.style.left = (左 * 矩形.width) + "px";
      选.style.width = ((右 - 左) * 矩形.width) + "px";
      元素.时间线色块.appendChild(选);
    });
    元素.时间线色块.addEventListener("pointerup", function (e) {
      if (!拖中) return;
      拖中 = false;
      var 矩形 = 元素.时间线色块.getBoundingClientRect();
      var 比起 = (起x - 矩形.left) / 矩形.width;
      var 比止 = (e.clientX - 矩形.left) / 矩形.width;
      var 起位置 = Math.floor(Math.min(比起, 比止) * 状态.事件们.length);
      var 止位置 = Math.floor(Math.max(比起, 比止) * 状态.事件们.length);
      起位置 = Math.max(0, Math.min(状态.事件们.length - 1, 起位置));
      止位置 = Math.max(0, Math.min(状态.事件们.length - 1, 止位置));
      状态.时间范围 = {
        since: 状态.事件们[起位置].时间戳,
        until: 状态.事件们[止位置].时间戳
      };
      元素.清除范围.hidden = false;
      拉取初始();
    });
  }

  // ===== 任务131：注入搜索导航 + 范围切换 =====
  function 注入搜索控件() {
    // 在搜索条中插入：范围切换 + 上一个 + 下一个
    var 条 = 元素.搜索条;
    // 范围切换
    var 范围 = document.createElement("select");
    范围.id = "搜索范围";
    范围.className = "搜索范围";
    范围.innerHTML = '<option value="全部">全部</option><option value="当前轮次">当前轮次</option>';
    范围.value = "全部";
    条.insertBefore(范围, 元素.搜索计数);
    元素.搜索范围 = 范围;
    范围.addEventListener("change", function () {
      if (状态.搜索词) 执行搜索(状态.搜索词);
    });
    // 上一个
    var 上 = document.createElement("button");
    上.id = "搜索上一个";
    上.type = "button";
    上.className = "搜索导航";
    上.textContent = "‹";
    上.title = "上一个命中";
    上.style.display = "none";
    条.insertBefore(上, 元素.搜索计数);
    元素.搜索上一个 = 上;
    上.addEventListener("click", 搜索上一个);
    // 下一个
    var 下 = document.createElement("button");
    下.id = "搜索下一个";
    下.type = "button";
    下.className = "搜索导航";
    下.textContent = "›";
    下.title = "下一个命中";
    下.style.display = "none";
    条.insertBefore(下, 元素.搜索计数);
    元素.搜索下一个 = 下;
    下.addEventListener("click", 搜索下一个);
  }

  function 注入汇总控件() {
    // 在汇总条左侧插入范围切换
    var 条 = 元素.汇总条;
    var 范围 = document.createElement("select");
    范围.id = "汇总范围";
    范围.className = "汇总范围";
    范围.innerHTML = '<option value="总计">总计</option><option value="当前轮次">当前轮次</option><option value="选中范围">选中范围</option>';
    范围.value = "总计";
    条.insertBefore(范围, 条.firstChild);
    元素.汇总范围 = 范围;
    范围.addEventListener("change", function () { 更新汇总(); });
  }

  // ============================================================
  // 任务146：四视图切换 + 深空背景 + 星系放射图 + 世界星图 + 私聊屏
  // 设计：融合蓝图 §13.f.10 · 参考 v6+ 蓝本 · 复用诸圣名录（17角色）
  // 原则：保留既有逻辑只增不改，洪荒中文变量，SSE 不重连
  // ============================================================

  var 当前视图 = "群聊";
  var 星系动画帧 = null;
  var 深空动画帧 = null;
  var 星图流星帧 = null;
  var 行星位置 = [];
  var 星系起始时刻 = 0;
  var 流星起始 = 0;
  var 流星们 = [];
  var 深空星点们 = [];

  // 星系配置：鸿钧为中心，16行星分4轨道运转
  var 星系配置 = {
    中心: { x: 0, y: 0 },
    鸿钧半径: 32,
    行星半径: 18,
    轨道半径: [130, 200, 270, 340],
    行星速度: [0.00018, -0.00014, 0.00011, -0.00009],
    初始角度: [0, Math.PI / 2, Math.PI, Math.PI * 1.5],
    粒子数: 3
  };

  function 创建SVG(标签) {
    return document.createElementNS("http://www.w3.org/2000/svg", 标签);
  }

  // ===== 视图切换 =====
  function 绑定视图切换() {
    var 标签们 = document.querySelectorAll(".视图标签");
    for (var i = 0; i < 标签们.length; i++) {
      标签们[i].addEventListener("click", function (标签) {
        return function () { 切视图(标签.dataset.视图); };
      }(标签们[i]));
    }
    // 读 URL hash 恢复视图
    var hash = window.location.hash.slice(1);
    if (hash === "时序" || hash === "群聊" || hash === "星图" || hash === "私聊") {
      切视图(hash);
    }
    // 内心世界关闭按钮
    var 关闭按钮 = document.getElementById("内心世界关闭");
    if (关闭按钮) 关闭按钮.addEventListener("click", 关闭内心世界);
    // 群聊流智能滚动跟随：用户向上滚动时暂停自动滚动，滚回底部恢复
    var 群聊流 = document.getElementById("群聊流");
    if (群聊流) {
      群聊流.addEventListener("scroll", function () {
        var 距底 = 群聊流.scrollHeight - 群聊流.clientHeight - 群聊流.scrollTop;
        状态.群聊跟随 = 距底 <= 2;
      }, { passive: true });
    }
  }

  function 切视图(视图名) {
    if (当前视图 === 视图名) return;
    当前视图 = 视图名;
    // 切标签激活态
    var 标签们 = document.querySelectorAll(".视图标签");
    for (var i = 0; i < 标签们.length; i++) {
      var 是 = 标签们[i].dataset.视图 === 视图名;
      标签们[i].classList.toggle("激活", 是);
      标签们[i].setAttribute("aria-checked", 是 ? "true" : "false");
    }
    // 切视图页显隐
    var 页们 = document.querySelectorAll(".视图页");
    for (var j = 0; j < 页们.length; j++) {
      var 匹配 = 页们[j].dataset.视图 === 视图名;
      if (匹配) {
        页们[j].removeAttribute("hidden");
        页们[j].classList.add("激活");
      } else {
        页们[j].setAttribute("hidden", "");
        页们[j].classList.remove("激活");
      }
    }
    // 更新 URL hash
    if (window.location.hash.slice(1) !== 视图名) {
      window.location.hash = 视图名;
    }
    // 星系视图启停动画
    if (视图名 === "星系") {
      启动星系动画();
    } else {
      停止星系动画();
    }
    // 星图视图启停流星
    if (视图名 === "星图") {
      启动星图流星();
    } else {
      停止星图流星();
    }
    // SSE 不重连，信源保持原有连接
  }

  // ===== 深空星点背景 =====
  function 初始化深空背景() {
    var 画布 = document.getElementById("深空背景");
    if (!画布) return;
    var 上下文 = 画布.getContext("2d");

    function 调整尺寸() {
      画布.width = window.innerWidth;
      画布.height = window.innerHeight;
      生成星点();
    }
    function 生成星点() {
      var 数 = Math.floor((画布.width * 画布.height) / 6000);
      深空星点们 = [];
      for (var i = 0; i < 数; i++) {
        深空星点们.push({
          x: Math.random() * 画布.width,
          y: Math.random() * 画布.height,
          r: Math.random() * 1.4 + 0.3,
          基透明: Math.random() * 0.5 + 0.3,
          相位: Math.random() * Math.PI * 2,
          频率: Math.random() * 0.001 + 0.0003,
          色: Math.random() < 0.15 ? "245, 166, 35" : (Math.random() < 0.3 ? "19, 212, 164" : "255, 255, 255")
        });
      }
    }
    function 深空帧(时刻) {
      上下文.clearRect(0, 0, 画布.width, 画布.height);
      for (var i = 0; i < 深空星点们.length; i++) {
        var 星 = 深空星点们[i];
        var 透明 = 星.基透明 * (0.5 + 0.5 * Math.sin(时刻 * 星.频率 + 星.相位));
        上下文.beginPath();
        上下文.arc(星.x, 星.y, 星.r, 0, Math.PI * 2);
        上下文.fillStyle = "rgba(" + 星.色 + ", " + 透明 + ")";
        上下文.fill();
        if (星.r > 1) {
          上下文.beginPath();
          上下文.arc(星.x, 星.y, 星.r * 3, 0, Math.PI * 2);
          上下文.fillStyle = "rgba(" + 星.色 + ", " + (透明 * 0.15) + ")";
          上下文.fill();
        }
      }
      深空动画帧 = requestAnimationFrame(深空帧);
    }
    调整尺寸();
    window.addEventListener("resize", 调整尺寸);
    深空动画帧 = requestAnimationFrame(深空帧);
  }

  // ===== 群聊视图：所有agent对话流 + 内心世界 =====
  // 设计：融合蓝图 §13.f.10 · 复用诸圣名录 + 从源推断角色 + 推断事件类型
  // 群聊流按时间排列，2秒内为并行（卡牌并排）；点击角色头像打开内心世界

  // 渲染群聊流：所有agent对话消息按时间排列
  function 渲染群聊流() {
    var 容器 = document.getElementById("群聊流");
    if (!容器) return;
    容器.innerHTML = "";
    if (状态.事件们.length === 0) {
      容器.innerHTML = '<div class="群聊占位">诸圣交流中……</div>';
      return;
    }

    // 过滤空壳事件：事件流源的工具调用事件缺少原文/结果，不显示
    var 有内容事件 = [];
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      var 是事件流 = 行.源 && 行.源.indexOf("事件流") >= 0;
      var 有内容;
      if (是事件流) {
        // 事件流源事件必须有完整原文或result才显示
        有内容 = (行.完整原文 && 行.完整原文.length > 0) ||
                 (行.result && 行.result.length > 0);
      } else {
        // 观测源事件有摘要或原文或result即显示
        有内容 = (行.摘要 && 行.摘要.length > 0) ||
                 (行.完整原文 && 行.完整原文.length > 0) ||
                 (行.result && 行.result.length > 0) ||
                 (行.动作 && 行.动作.length > 0);
      }
      if (有内容) 有内容事件.push(行);
    }
    if (有内容事件.length === 0) {
      容器.innerHTML = '<div class="群聊占位">诸圣交流中……</div>';
      return;
    }

    // 按时间戳升序排列（旧→新，群聊从上到下）
    有内容事件.sort(function (a, b) { return (a.时间戳 || 0) - (b.时间戳 || 0); });

    // 合并同时间戳的工具调用+工具返回为一条消息，减少噪音
    var 合并后 = [];
    var i = 0;
    while (i < 有内容事件.length) {
      var 当前行 = 有内容事件[i];
      var 当前源 = 当前行.源 || "";
      // 如果是工具调用，找同时间戳的工具返回合并
      if (当前源.indexOf("工具调用") >= 0) {
        var 合并行 = 当前行;
        // 向后找同时间戳的工具返回
        for (var j = i + 1; j < 有内容事件.length; j++) {
          var 下行 = 有内容事件[j];
          if ((下行.时间戳 || 0) - (当前行.时间戳 || 0) > 2000) break;
          if ((下行.源 || "").indexOf("工具返回") >= 0) {
            // 合并：工具调用的内容+工具返回的结果
            合并行 = {
              源: 当前行.源,
              动作: 当前行.动作,
              摘要: 当前行.摘要,
              完整原文: 当前行.完整原文,
              result: 下行.result || 下行.完整原文 || "",
              token: 当前行.token,
              耗时ms: 当前行.耗时ms,
              时间戳: 当前行.时间戳,
              轮次: 当前行.轮次,
              供应者: 当前行.供应者,
              模型: 当前行.模型
            };
            i = j; // 跳过工具返回
            break;
          }
        }
        合并后.push(合并行);
      } else {
        合并后.push(当前行);
      }
      i++;
    }

    // 单列渲染：过滤无实质信息的消息，从上往下按执行顺序排列
    for (var k = 0; k < 合并后.length; k++) {
      var 行 = 合并后[k];
      // 质量过滤：证据太短（<3字）且result也短 → 无实质信息，跳过
      var 证据长 = (行.完整原文 || "").trim().length;
      var result长 = (行.result || "").trim().length;
      if (证据长 < 3 && result长 < 3) continue;
      var 消息 = 创建群聊消息(行, false);
      var 组div = document.createElement("div");
      组div.className = "群聊组";
      组div.appendChild(消息);
      容器.appendChild(组div);
    }

    // 智能滚动：只在跟随状态下滚到底部
    if (状态.群聊跟随) {
      容器.scrollTop = 容器.scrollHeight;
    }
  }

  // 创建单条群聊消息DOM
  function 创建群聊消息(行, 并行) {
    var 角色 = 从源推断角色(行.源 || "");
    if (!角色) 角色 = 从动作推断角色(行.动作 || "");
    if (!角色) 角色 = "未知";
    var 类型 = 推断事件类型(行.源, 行.动作);
    var 圣 = null;
    for (var k = 0; k < 诸圣名录.length; k++) {
      if (诸圣名录[k].名 === 角色) { 圣 = 诸圣名录[k]; break; }
    }
    var 色变量 = 圣 ? 圣.色变量 : "--色-静";
    var 职能 = 圣 ? 圣.职能 : "";

    var 消息 = document.createElement("div");
    消息.className = "群聊消息" + (并行 ? " 卡牌" : "");
    消息.dataset.角色 = 角色;
    消息.dataset.类型 = 类型;

    // 头像（色点+角色名首字）
    var 头像 = document.createElement("div");
    头像.className = "消息头像";
    头像.style.background = "var(" + 色变量 + ")";
    头像.textContent = 角色.charAt(0);
    头像.style.cursor = "pointer";
    头像.addEventListener("click", function () { 打开内心世界(角色); });
    消息.appendChild(头像);

    // 消息体
    var 体 = document.createElement("div");
    体.className = "消息体";

    // 头部：角色名+类型标签+时间
    var 头 = document.createElement("div");
    头.className = "消息头";
    var 名span = document.createElement("span");
    名span.className = "消息角色";
    名span.textContent = 角色;
    名span.style.color = "var(" + 色变量 + ")";
    名span.style.cursor = "pointer";
    名span.addEventListener("click", function () { 打开内心世界(角色); });
    头.appendChild(名span);
    var 类型span = document.createElement("span");
    类型span.className = "消息类型 类型-" + 类型;
    类型span.textContent = 类型;
    头.appendChild(类型span);
    var 时间span = document.createElement("span");
    时间span.className = "消息时间";
    时间span.textContent = 格式化时刻(行.时间戳);
    头.appendChild(时间span);
    体.appendChild(头);

    // 内容摘要：从证据（完整原文）提取实质内容，避免只显示动作名
    var 内容 = document.createElement("div");
    内容.className = "消息内容";
    var 摘要文 = "";
    if (类型 === "工具") {
      // 工具调用：证据里有文件路径或搜索关键词
      摘要文 = 行.完整原文 || 行.摘要 || "";
    } else if (类型 === "想法") {
      摘要文 = 行.完整原文 || 行.摘要 || "";
    } else if (类型 === "结果") {
      摘要文 = 行.result || 行.完整原文 || "";
    } else if (类型 === "发言") {
      摘要文 = 行.完整原文 || 行.摘要 || "";
    } else {
      摘要文 = 行.摘要 || 行.完整原文 || 行.动作 || "";
    }
    内容.textContent = 截断文(摘要文, 200);
    体.appendChild(内容);

    // 底部：耗时+token
    var 底 = document.createElement("div");
    底.className = "消息底";
    var 底文 = [];
    if (行.耗时ms != null) 底文.push((行.耗时ms / 1000).toFixed(1) + "s");
    var tk = 行.token || {};
    if (tk.输入) 底文.push("提" + tk.输入);
    if (tk.输出) 底文.push("出" + tk.输出);
    底.textContent = 底文.join(" · ");
    体.appendChild(底);

    消息.appendChild(体);
    return 消息;
  }

  // SSE收到新事件时追加消息到群聊流底部
  function 追加群聊消息(行) {
    if (当前视图 !== "群聊") return;
    var 容器 = document.getElementById("群聊流");
    if (!容器) return;
    // 移除占位
    var 占位 = 容器.querySelector(".群聊占位");
    if (占位) 占位.remove();
    // 创建消息（非并行）
    var 消息 = 创建群聊消息(行, false);
    消息.classList.add("淡入");
    var 组div = document.createElement("div");
    组div.className = "群聊组";
    组div.appendChild(消息);
    容器.appendChild(组div);
    // 智能滚动：只在跟随状态下滚到底部
    if (状态.群聊跟随) {
      容器.scrollTop = 容器.scrollHeight;
    }
  }

  // 打开内心世界：显示该角色全部事件的全量记录
  function 打开内心世界(角色名) {
    var 面板 = document.getElementById("内心世界");
    if (!面板) return;
    // 设置头部
    var 色点 = document.getElementById("内心世界色点");
    var 名元 = document.getElementById("内心世界角色名");
    var 职元 = document.getElementById("内心世界职能");
    var 圣 = null;
    for (var k = 0; k < 诸圣名录.length; k++) {
      if (诸圣名录[k].名 === 角色名) { 圣 = 诸圣名录[k]; break; }
    }
    if (色点) 色点.style.background = "var(" + (圣 ? 圣.色变量 : "--色-静") + ")";
    if (名元) 名元.textContent = 角色名;
    if (职元) 职元.textContent = 圣 ? 圣.职能 : "";

    // 筛选该角色的所有事件
    var 体 = document.getElementById("内心世界体");
    if (!体) return;
    体.innerHTML = "";

    var 记录数 = 0;
    for (var i = 0; i < 状态.事件们.length; i++) {
      var 行 = 状态.事件们[i];
      var 推断 = 从源推断角色(行.源 || "");
      if (推断 !== 角色名) continue;
      记录数++;
      var 类型 = 推断事件类型(行.源, 行.动作);
      var 记录 = document.createElement("div");
      记录.className = "内心记录 类型-" + 类型;

      // 记录头：类型+时间
      var 记头 = document.createElement("div");
      记头.className = "内心记录头";
      var 类型标 = document.createElement("span");
      类型标.className = "内心记录类型 类型-" + 类型;
      类型标.textContent = 类型;
      记头.appendChild(类型标);
      var 时间 = document.createElement("span");
      时间.className = "内心记录时间";
      时间.textContent = 格式化时刻(行.时间戳);
      记头.appendChild(时间);
      if (行.耗时ms != null) {
        var 耗 = document.createElement("span");
        耗.className = "内心记录耗时";
        耗.textContent = (行.耗时ms / 1000).toFixed(1) + "s";
        记头.appendChild(耗);
      }
      记录.appendChild(记头);

      // 记录内容：全量原文
      var 记内容 = document.createElement("div");
      记内容.className = "内心记录内容";
      var 文 = "";
      if (类型 === "工具") {
        文 = "动作：" + (行.动作 || "") + "\n";
        文 += "摘要：" + (行.摘要 || "") + "\n";
        if (行.完整原文) 文 += "原文：\n" + 行.完整原文;
      } else if (类型 === "想法") {
        文 = 行.完整原文 || 行.摘要 || "";
      } else if (类型 === "结果") {
        文 = 行.result || 行.完整原文 || "";
      } else if (类型 === "发言") {
        文 = 行.摘要 || "";
        if (行.完整原文 && 行.完整原文.length > 行.摘要.length) 文 += "\n\n原文：\n" + 行.完整原文;
      } else {
        文 = 行.摘要 || 行.动作 || "";
        if (行.完整原文 && 行.完整原文.length > 50) 文 += "\n\n原文：\n" + 行.完整原文;
      }
      记内容.textContent = 文;
      记录.appendChild(记内容);

      // token
      var tk = 行.token || {};
      if (tk.输入 || tk.输出 || tk.推理) {
        var tkdiv = document.createElement("div");
        tkdiv.className = "内心记录token";
        var tk文 = [];
        if (tk.输入) tk文.push("提示词 " + tk.输入);
        if (tk.输出) tk文.push("输出 " + tk.输出);
        if (tk.缓存读) tk文.push("缓存读 " + tk.缓存读);
        if (tk.推理) tk文.push("推理 " + tk.推理);
        tkdiv.textContent = tk文.join(" · ");
        记录.appendChild(tkdiv);
      }

      体.appendChild(记录);
    }

    if (记录数 === 0) {
      体.innerHTML = '<div class="内心占位">该角色暂无记录</div>';
    }

    // 打开面板：先 hidden=false 再加 .滑入 触发 translateX(0)→过渡
    // 同时收缩群聊流 margin-right，避免被 fixed 面板遮挡
    面板.classList.remove("滑出");
    面板.hidden = false;
    // 强制 reflow，确保从 translateX(100%) 起始过渡到 translateX(0)
    void 面板.offsetWidth;
    面板.classList.add("滑入");
    var 群聊容器 = document.getElementById("群聊流");
    if (群聊容器) 群聊容器.style.marginRight = "var(--内心世界宽)";
    // 默认滚到底部，展示最新记录
    体.scrollTop = 体.scrollHeight;
  }

  function 关闭内心世界() {
    var 面板 = document.getElementById("内心世界");
    if (!面板) return;
    // 先做滑出动画：translateX(0)→translateX(100%)，300ms 后再 hidden=true
    面板.classList.remove("滑入");
    面板.classList.add("滑出");
    // 立即恢复群聊流宽度
    var 群聊容器 = document.getElementById("群聊流");
    if (群聊容器) 群聊容器.style.marginRight = "0";
    // 与 CSS --慢 (0.3s) 同步，动画结束后隐藏
    setTimeout(function () {
      面板.hidden = true;
      面板.classList.remove("滑出");
    }, 300);
  }

  // ===== 星系放射图：鸿钧为中心，16行星运转 =====
  function 渲染星系图() {
    var 轨道层 = document.getElementById("轨道层");
    if (!轨道层) return; // 星系视图未挂载
    var 鸿钧 = 诸圣名录[0];
    var 子们 = [];
    for (var i = 1; i < 诸圣名录.length; i++) {
      子们.push(诸圣名录[i]);
    }
    绘制轨道层(子们.length);
    绘制中心层(鸿钧);
    绘制行星层(子们);
    绘制连线层(子们);
    绘制粒子层(子们);
    // 用已有事件初始化行星状态（实时过程门面）
    更新星系行星状态();
  }

  function 绘制轨道层(数) {
    var 层 = document.getElementById("轨道层");
    层.innerHTML = "";
    var 轨道数 = Math.min(数, 星系配置.轨道半径.length);
    for (var i = 0; i < 轨道数; i++) {
      var 圆 = 创建SVG("circle");
      圆.setAttribute("cx", 0);
      圆.setAttribute("cy", 0);
      圆.setAttribute("r", 星系配置.轨道半径[i]);
      圆.setAttribute("class", "轨道圆" + (i === 0 ? " 主" : ""));
      层.appendChild(圆);
    }
  }

  function 绘制中心层(鸿钧) {
    var 层 = document.getElementById("中心层");
    层.innerHTML = "";
    // 外层光晕
    var 晕外 = 创建SVG("circle");
    晕外.setAttribute("cx", 0); 晕外.setAttribute("cy", 0);
    晕外.setAttribute("r", 80);
    晕外.setAttribute("fill", "url(#鸿钧晕)");
    晕外.setAttribute("class", "鸿钧晕外");
    层.appendChild(晕外);
    // 中层光晕
    var 晕中 = 创建SVG("circle");
    晕中.setAttribute("cx", 0); 晕中.setAttribute("cy", 0);
    晕中.setAttribute("r", 55);
    晕中.setAttribute("fill", "url(#鸿钧晕)");
    晕中.setAttribute("class", "鸿钧晕中");
    层.appendChild(晕中);
    // 旋转光环
    var 环 = 创建SVG("circle");
    环.setAttribute("cx", 0); 环.setAttribute("cy", 0);
    环.setAttribute("r", 48);
    环.setAttribute("fill", "none");
    环.setAttribute("stroke", "rgba(245, 166, 35, 0.4)");
    环.setAttribute("stroke-width", 1);
    环.setAttribute("stroke-dasharray", "4 8");
    环.setAttribute("class", "鸿钧环");
    层.appendChild(环);
    var 环反 = 创建SVG("circle");
    环反.setAttribute("cx", 0); 环反.setAttribute("cy", 0);
    环反.setAttribute("r", 42);
    环反.setAttribute("fill", "none");
    环反.setAttribute("stroke", "rgba(19, 212, 164, 0.3)");
    环反.setAttribute("stroke-width", 1);
    环反.setAttribute("stroke-dasharray", "2 6");
    环反.setAttribute("class", "鸿钧环 反");
    层.appendChild(环反);
    // 鸿钧核
    var 核 = 创建SVG("circle");
    核.setAttribute("cx", 0); 核.setAttribute("cy", 0);
    核.setAttribute("r", 星系配置.鸿钧半径);
    核.setAttribute("fill", "url(#鸿钧光)");
    核.setAttribute("class", "鸿钧核");
    核.setAttribute("filter", "url(#强光)");
    层.appendChild(核);
    // 鸿钧名
    var 名 = 创建SVG("text");
    名.setAttribute("x", 0); 名.setAttribute("y", 4);
    名.setAttribute("class", "行星标签");
    名.setAttribute("fill", "#0d0f0e");
    名.setAttribute("font-weight", "700");
    名.setAttribute("font-size", "14");
    名.textContent = 鸿钧.名;
    层.appendChild(名);
    var 职 = 创建SVG("text");
    职.setAttribute("x", 0); 职.setAttribute("y", 56);
    职.setAttribute("class", "行星标签");
    职.setAttribute("fill", "#f5a623");
    职.setAttribute("font-size", "11");
    职.textContent = 鸿钧.职能 || "";
    层.appendChild(职);
    // 当前阶段（从 状态.快照 读，实时过程门面）
    var 快照 = 状态.快照 || {};
    var 阶段文 = 创建SVG("text");
    阶段文.setAttribute("x", 0); 阶段文.setAttribute("y", 72);
    阶段文.setAttribute("class", "行星子标 鸿钧阶段文");
    阶段文.setAttribute("fill", "#13d4a4");
    阶段文.textContent = 快照.当前阶段 ? "· " + 快照.当前阶段 : "";
    层.appendChild(阶段文);
    // 当前想法截断（小字，实时过程门面）
    var 想法文 = 创建SVG("text");
    想法文.setAttribute("x", 0); 想法文.setAttribute("y", 86);
    想法文.setAttribute("class", "行星子标 鸿钧想法文");
    想法文.setAttribute("fill", "#a8b0ac");
    想法文.textContent = 快照.当前想法 ? 截断文(快照.当前想法, 24) : "";
    层.appendChild(想法文);
    // 标记鸿钧名，便于 tooltip 与详情面板识别
    核.setAttribute("data-名", "鸿钧");
    // 交互
    核.style.cursor = "pointer";
    核.addEventListener("mouseenter", function (e) { 显示星系tooltip(e, 鸿钧); });
    核.addEventListener("mouseleave", 隐藏星系tooltip);
    核.addEventListener("click", function (e) { 显示星系详情面板(鸿钧); });
  }

  function 绘制行星层(子们) {
    var 层 = document.getElementById("行星层");
    层.innerHTML = "";
    行星位置 = [];
    var 在位 = 算在位诸圣();
    var 最新映射 = 提取诸圣最新状态();
    for (var i = 0; i < 子们.length; i++) {
      var 子 = 子们[i];
      var 轨道索引 = Math.min(i, 星系配置.轨道半径.length - 1);
      var 半径 = 星系配置.轨道半径[轨道索引];
      var 角度 = 星系配置.初始角度[i % 星系配置.初始角度.length] || 0;
      var x = Math.cos(角度) * 半径;
      var y = Math.sin(角度) * 半径;
      var 运转中 = 在位.has(子.名);
      var 最新 = 最新映射.get(子.名) || null;

      行星位置.push({
        索引: i,
        名: 子.名,
        轨道索引: 轨道索引,
        半径: 半径,
        角度: 角度,
        x: x, y: y,
        状态: 运转中 ? "运转中" : "待命",
        最新状态: 最新
      });

      var 组 = 创建SVG("g");
      组.setAttribute("class", "行星组" + (运转中 ? " 运转中" : ""));
      组.setAttribute("data-索引", i);
      组.setAttribute("data-名", 子.名);
      组.setAttribute("transform", "translate(" + x + ", " + y + ")");

      // 光晕
      var 晕 = 创建SVG("circle");
      晕.setAttribute("cx", 0); 晕.setAttribute("cy", 0);
      晕.setAttribute("r", 32);
      晕.setAttribute("fill", "url(#行星晕-运转)");
      晕.setAttribute("class", "行星晕 " + (运转中 ? "运转" : ""));
      if (!运转中) 晕.setAttribute("opacity", "0.15");
      组.appendChild(晕);

      // 行星核
      var 核 = 创建SVG("circle");
      核.setAttribute("cx", 0); 核.setAttribute("cy", 0);
      核.setAttribute("r", 星系配置.行星半径);
      核.setAttribute("fill", 运转中 ? "url(#行星光-运转)" : "url(#行星光-待命)");
      核.setAttribute("class", "行星核");
      if (运转中) 核.setAttribute("filter", "url(#柔光)");
      组.appendChild(核);

      // 行星名
      var 名 = 创建SVG("text");
      名.setAttribute("x", 0); 名.setAttribute("y", 4);
      名.setAttribute("class", "行星标签");
      名.setAttribute("fill", 运转中 ? "#e8ece9" : "#6b736f");
      名.textContent = 子.名;
      组.appendChild(名);

      // 职能副标
      var 职 = 创建SVG("text");
      职.setAttribute("x", 0); 职.setAttribute("y", 32);
      职.setAttribute("class", "行星子标");
      职.textContent = 子.职能 || "";
      组.appendChild(职);

      // 动作摘要（运转中才显示，实时过程门面）
      var 摘要 = 创建SVG("text");
      摘要.setAttribute("x", 0); 摘要.setAttribute("y", 46);
      摘要.setAttribute("class", "行星动作摘要");
      摘要.textContent = 最新 ? 截断文(最新.摘要 || 最新.动作, 20) : "";
      组.appendChild(摘要);

      // 状态点
      var 状态点 = 创建SVG("circle");
      状态点.setAttribute("cx", 14); 状态点.setAttribute("cy", -14);
      状态点.setAttribute("r", 3);
      状态点.setAttribute("fill", 运转中 ? "#13d4a4" : "#6b736f");
      if (运转中) 状态点.setAttribute("filter", "url(#柔光)");
      组.appendChild(状态点);

      // 交互
      组.addEventListener("mouseenter", function (s) { return function (ev) { 显示星系tooltip(ev, s); }; }(子));
      组.addEventListener("mouseleave", 隐藏星系tooltip);
      组.addEventListener("click", function (s) { return function () { 显示星系详情面板(s); }; }(子));
      层.appendChild(组);
    }
  }

  // SSE 收到新事件时，更新对应行星的视觉状态（实时过程门面）
  // 事件参数可选：传入时只更新该事件涉及的角色；不传时用所有在位角色初始化
  function 更新星系行星状态(事件) {
    if (当前视图 !== "星系") return;
    var 层 = document.getElementById("行星层");
    if (!层) return;
    var 最新映射 = 提取诸圣最新状态();
    // 确定要更新的角色名集合
    var 待更新名 = [];
    if (事件 && 事件.源) {
      var 源文 = 事件.源;
      for (var j = 0; j < 诸圣名录.length; j++) {
        if (源文.indexOf(诸圣名录[j].名) >= 0) 待更新名.push(诸圣名录[j].名);
      }
    } else {
      var 在位 = 算在位诸圣();
      在位.forEach(function (n) { 待更新名.push(n); });
    }
    for (var m = 0; m < 待更新名.length; m++) {
      var 圣名 = 待更新名[m];
      var 组 = 层.querySelector('g[data-名="' + 圣名 + '"]');
      if (!组) continue;
      var 最新 = 最新映射.get(圣名) || null;
      // 行星组加运转中 class（触发脉冲动画）
      组.classList.add("运转中");
      // 光晕加运转 class
      var 晕 = 组.querySelector(".行星晕");
      if (晕) {
        晕.classList.add("运转");
        晕.setAttribute("opacity", "1");
      }
      // 行星核切运转色
      var 核 = 组.querySelector(".行星核");
      if (核) {
        核.setAttribute("fill", "url(#行星光-运转)");
        核.setAttribute("filter", "url(#柔光)");
      }
      // 动作摘要文本更新
      var 摘要文 = 组.querySelector(".行星动作摘要");
      if (摘要文 && 最新) {
        摘要文.textContent = 截断文(最新.摘要 || 最新.动作, 20);
      }
      // 行星位置[i].最新状态 同步
      for (var k = 0; k < 行星位置.length; k++) {
        if (行星位置[k].名 === 圣名) {
          行星位置[k].最新状态 = 最新;
          行星位置[k].状态 = "运转中";
          break;
        }
      }
    }
  }

  function 绘制连线层(子们) {
    var 层 = document.getElementById("连线层");
    层.innerHTML = "";
    // 鸿钧到每个子agent
    for (var i = 0; i < 子们.length; i++) {
      var 线 = 创建SVG("line");
      线.setAttribute("x1", 0); 线.setAttribute("y1", 0);
      线.setAttribute("x2", 0); 线.setAttribute("y2", 0);
      线.setAttribute("class", "连线");
      线.setAttribute("data-类型", "主");
      线.setAttribute("data-索引", i);
      层.appendChild(线);
    }
    // 子agent之间的协作连线
    for (var j = 0; j < 子们.length; j++) {
      var k = (j + 1) % 子们.length;
      var 线2 = 创建SVG("line");
      线2.setAttribute("x1", 0); 线2.setAttribute("y1", 0);
      线2.setAttribute("x2", 0); 线2.setAttribute("y2", 0);
      线2.setAttribute("class", "连线 协作");
      线2.setAttribute("data-类型", "协作");
      线2.setAttribute("data-从", j);
      线2.setAttribute("data-到", k);
      层.appendChild(线2);
    }
  }

  function 绘制粒子层(子们) {
    var 层 = document.getElementById("粒子层");
    层.innerHTML = "";
    // 主连线粒子
    for (var i = 0; i < 子们.length; i++) {
      for (var p = 0; p < 星系配置.粒子数; p++) {
        var 粒 = 创建SVG("circle");
        粒.setAttribute("r", 2);
        粒.setAttribute("class", "粒子");
        粒.setAttribute("data-类型", "主");
        粒.setAttribute("data-索引", i);
        粒.setAttribute("data-序", p);
        粒.setAttribute("cx", 0); 粒.setAttribute("cy", 0);
        层.appendChild(粒);
      }
    }
    // 协作连线粒子
    for (var j = 0; j < 子们.length; j++) {
      var k = (j + 1) % 子们.length;
      for (var q = 0; q < 2; q++) {
        var 粒2 = 创建SVG("circle");
        粒2.setAttribute("r", 1.5);
        粒2.setAttribute("class", "粒子");
        粒2.setAttribute("data-类型", "协作");
        粒2.setAttribute("data-从", j);
        粒2.setAttribute("data-到", k);
        粒2.setAttribute("data-序", q);
        粒2.setAttribute("cx", 0); 粒2.setAttribute("cy", 0);
        粒2.setAttribute("fill", "#13d4a4");
        层.appendChild(粒2);
      }
    }
  }

  // 星系运转动画
  function 启动星系动画() {
    停止星系动画();
    if (行星位置.length === 0) return;
    星系起始时刻 = performance.now();
    for (var i = 0; i < 行星位置.length; i++) {
      行星位置[i].角度 = 星系配置.初始角度[行星位置[i].索引 % 星系配置.初始角度.length] || 0;
    }
    星系动画帧 = requestAnimationFrame(星系帧);
  }
  function 停止星系动画() {
    if (星系动画帧) {
      cancelAnimationFrame(星系动画帧);
      星系动画帧 = null;
    }
  }
  function 星系帧(时刻) {
    var 经过 = 时刻 - 星系起始时刻;
    var 层 = document.getElementById("行星层");
    if (!层) { 停止星系动画(); return; }
    // 更新行星位置
    for (var i = 0; i < 行星位置.length; i++) {
      var p = 行星位置[i];
      var 速度 = 星系配置.行星速度[p.轨道索引] || 0.0001;
      p.角度 = (星系配置.初始角度[p.索引 % 星系配置.初始角度.length] || 0) + 速度 * 经过;
      p.x = Math.cos(p.角度) * p.半径;
      p.y = Math.sin(p.角度) * p.半径;
      var 组 = 层.querySelector('g[data-索引="' + p.索引 + '"]');
      if (组) 组.setAttribute("transform", "translate(" + p.x.toFixed(2) + ", " + p.y.toFixed(2) + ")");
    }
    // 更新连线
    var 连线层 = document.getElementById("连线层");
    if (连线层) {
      var 主线们 = 连线层.querySelectorAll('line[data-类型="主"]');
      for (var a = 0; a < 主线们.length; a++) {
        var 线 = 主线们[a];
        var idx = parseInt(线.dataset.索引, 10);
        var pp = 行星位置[idx];
        if (pp) {
          线.setAttribute("x2", pp.x.toFixed(2));
          线.setAttribute("y2", pp.y.toFixed(2));
        }
      }
      var 协作线们 = 连线层.querySelectorAll('line[data-类型="协作"]');
      for (var b = 0; b < 协作线们.length; b++) {
        var 线2 = 协作线们[b];
        var fi = parseInt(线2.dataset.从, 10);
        var ti = parseInt(线2.dataset.到, 10);
        var pa = 行星位置[fi];
        var pb = 行星位置[ti];
        if (pa && pb) {
          线2.setAttribute("x1", pa.x.toFixed(2));
          线2.setAttribute("y1", pa.y.toFixed(2));
          线2.setAttribute("x2", pb.x.toFixed(2));
          线2.setAttribute("y2", pb.y.toFixed(2));
        }
      }
    }
    // 更新粒子
    var 粒子层 = document.getElementById("粒子层");
    if (粒子层) {
      var 主粒们 = 粒子层.querySelectorAll('circle[data-类型="主"]');
      for (var c = 0; c < 主粒们.length; c++) {
        var 粒 = 主粒们[c];
        var pi = parseInt(粒.dataset.索引, 10);
        var pk = parseInt(粒.dataset.序, 10);
        var pp2 = 行星位置[pi];
        if (pp2) {
          var 周期 = 4000;
          var 偏移 = (pk / 星系配置.粒子数 + (经过 / 周期) % 1) % 1;
          粒.setAttribute("cx", (pp2.x * 偏移).toFixed(2));
          粒.setAttribute("cy", (pp2.y * 偏移).toFixed(2));
          粒.setAttribute("opacity", Math.sin(偏移 * Math.PI).toFixed(2));
        }
      }
      var 协作粒们 = 粒子层.querySelectorAll('circle[data-类型="协作"]');
      for (var d = 0; d < 协作粒们.length; d++) {
        var 粒2 = 协作粒们[d];
        var ci = parseInt(粒2.dataset.从, 10);
        var cj = parseInt(粒2.dataset.到, 10);
        var ck = parseInt(粒2.dataset.序, 10);
        var pca = 行星位置[ci];
        var pcb = 行星位置[cj];
        if (pca && pcb) {
          var 周期2 = 5000;
          var 偏移2 = (ck / 2 + (经过 / 周期2) % 1) % 1;
          var px = pca.x + (pcb.x - pca.x) * 偏移2;
          var py = pca.y + (pcb.y - pca.y) * 偏移2;
          粒2.setAttribute("cx", px.toFixed(2));
          粒2.setAttribute("cy", py.toFixed(2));
          粒2.setAttribute("opacity", Math.sin(偏移2 * Math.PI).toFixed(2));
        }
      }
    }
    星系动画帧 = requestAnimationFrame(星系帧);
  }

  // 星系 tooltip：悬停显示角色完整实时信息（实时过程门面）
  function 显示星系tooltip(事件, 子) {
    var 提示 = document.getElementById("星系tooltip");
    if (!提示) return;
    var 最新映射 = 提取诸圣最新状态();
    var 最新 = 最新映射.get(子.名);
    var 快照 = 状态.快照 || {};
    var 行 = [];
    行.push('<div class="tooltip名">' + 转义圣名(子.名) + "</div>");
    行.push('<div class="tooltip行">' + 转义圣名(子.职能 || "") + " · 层 " + 转义圣名(子.层 || "") + "</div>");
    // 鸿钧中心：显示当前想法/要求/阶段
    if (子.名 === "鸿钧") {
      if (快照.当前阶段) 行.push('<div class="tooltip行">阶段：' + 转义圣名(快照.当前阶段) + "</div>");
      if (快照.当前想法) 行.push('<div class="tooltip行">想法：' + 转义圣名(截断文(快照.当前想法, 40)) + "</div>");
      if (快照.当前要求) 行.push('<div class="tooltip行">要求：' + 转义圣名(截断文(快照.当前要求, 40)) + "</div>");
    }
    if (最新) {
      var 动作文 = 最新.摘要 || 最新.动作 || "";
      if (动作文) 行.push('<div class="tooltip行">动作：' + 转义圣名(截断文(动作文, 50)) + "</div>");
      var 模型文 = (最新.供应者 || "") + (最新.供应者 && 最新.模型 ? " " : "") + (最新.模型 || "");
      if (模型文) 行.push('<div class="tooltip行">模型：' + 转义圣名(模型文) + "</div>");
      var 证据文 = 最新.证据 || 最新.结果 || "";
      if (证据文) 行.push('<div class="tooltip行">结果：' + 转义圣名(截断文(证据文, 100)) + "</div>");
      if (最新.耗时ms != null) 行.push('<div class="tooltip行">耗时：' + 格式化耗时(最新.耗时ms) + "</div>");
      var tk = 最新.token || {};
      var tk段 = [];
      if (tk.输入) tk段.push("提" + 格式化token(tk.输入));
      if (tk.输出) tk段.push("出" + 格式化token(tk.输出));
      if (tk.缓存读) tk段.push("读" + 格式化token(tk.缓存读));
      if (tk.推理) tk段.push("推" + 格式化token(tk.推理));
      if (tk段.length) 行.push('<div class="tooltip行">token：' + tk段.join(" ") + "</div>");
      if (最新.时间戳) 行.push('<div class="tooltip行">时刻：' + 格式化时刻(最新.时间戳) + "</div>");
      if (最新.轮次 != null) 行.push('<div class="tooltip行">轮次：' + 转义圣名(最新.轮次) + "</div>");
    } else {
      行.push('<div class="tooltip行">待命中</div>');
    }
    提示.innerHTML = 行.join("");
    var 舞台 = document.querySelector(".星系舞台");
    if (!舞台) return;
    var 矩 = 舞台.getBoundingClientRect();
    提示.style.left = (事件.clientX - 矩.left + 14) + "px";
    提示.style.top = (事件.clientY - 矩.top + 14) + "px";
    提示.classList.add("显");
  }
  function 隐藏星系tooltip() {
    var 提示 = document.getElementById("星系tooltip");
    if (提示) 提示.classList.remove("显");
  }
  function 转义圣名(文) {
    if (文 == null) return "";
    var d = document.createElement("div");
    d.textContent = String(文);
    return d.innerHTML;
  }

  // 星系详情面板：点击行星显示角色完整实时信息（实时过程门面）
  function 显示星系详情面板(子) {
    var 面板 = document.getElementById("星系详情面板");
    if (!面板) return;
    var 最新映射 = 提取诸圣最新状态();
    var 最新 = 最新映射.get(子.名);
    var 快照 = 状态.快照 || {};
    var 段 = [];
    段.push('<div class="详情头">');
    段.push('<span class="详情名">' + 转义圣名(子.名) + "</span>");
    段.push('<span class="详情职">' + 转义圣名(子.职能 || "") + " · 层 " + 转义圣名(子.层 || "") + "</span>");
    段.push('<button class="详情关" type="button" aria-label="关闭">×</button>');
    段.push("</div>");
    // 鸿钧中心：当前想法/要求/阶段
    if (子.名 === "鸿钧") {
      段.push('<div class="详情节">');
      段.push('<div class="详情节标">当前阶段</div>');
      段.push('<div class="详情节文">' + 转义圣名(快照.当前阶段 || "—") + "</div>");
      段.push("</div>");
      段.push('<div class="详情节">');
      段.push('<div class="详情节标">当前想法</div>');
      段.push('<div class="详情节文">' + 转义圣名(快照.当前想法 || "—") + "</div>");
      段.push("</div>");
      段.push('<div class="详情节">');
      段.push('<div class="详情节标">当前要求</div>');
      段.push('<div class="详情节文">' + 转义圣名(快照.当前要求 || "—") + "</div>");
      段.push("</div>");
    }
    if (最新) {
      段.push('<div class="详情节">');
      段.push('<div class="详情节标">当前动作</div>');
      段.push('<div class="详情节文">' + 转义圣名(最新.摘要 || 最新.动作 || "—") + "</div>");
      段.push("</div>");
      var 模型文 = (最新.供应者 || "") + (最新.供应者 && 最新.模型 ? " " : "") + (最新.模型 || "");
      if (模型文) {
        段.push('<div class="详情节">');
        段.push('<div class="详情节标">模型</div>');
        段.push('<div class="详情节文">' + 转义圣名(模型文) + "</div>");
        段.push("</div>");
      }
      段.push('<div class="详情节">');
      段.push('<div class="详情节标">结果原文</div>');
      段.push('<div class="详情节文 详情原文">' + 转义圣名(最新.证据 || 最新.结果 || "—") + "</div>");
      段.push("</div>");
      // 耗时与时刻
      var 元行 = [];
      if (最新.耗时ms != null) 元行.push("耗时 " + 格式化耗时(最新.耗时ms));
      if (最新.时间戳) 元行.push("时刻 " + 格式化时刻(最新.时间戳));
      if (最新.轮次 != null) 元行.push("轮次 " + 转义圣名(最新.轮次));
      if (元行.length) {
        段.push('<div class="详情元">' + 元行.join(" · ") + "</div>");
      }
      // token 用量
      var tk = 最新.token || {};
      var tk行 = [];
      if (tk.输入) tk行.push("提示词 " + 格式化token(tk.输入));
      if (tk.输出) tk行.push("输出 " + 格式化token(tk.输出));
      if (tk.缓存读) tk行.push("缓存读 " + 格式化token(tk.缓存读));
      if (tk.缓存写) tk行.push("缓存写 " + 格式化token(tk.缓存写));
      if (tk.推理) tk行.push("推理 " + 格式化token(tk.推理));
      if (tk行.length) {
        段.push('<div class="详情节">');
        段.push('<div class="详情节标">token 用量</div>');
        段.push('<div class="详情节文">' + tk行.join(" · ") + "</div>");
        段.push("</div>");
      }
    } else {
      段.push('<div class="详情节文">待命中</div>');
    }
    面板.innerHTML = 段.join("");
    面板.hidden = false;
    // 关闭按钮
    var 关钮 = 面板.querySelector(".详情关");
    if (关钮) 关钮.addEventListener("click", 隐藏星系详情面板);
  }
  function 隐藏星系详情面板() {
    var 面板 = document.getElementById("星系详情面板");
    if (面板) 面板.hidden = true;
  }

  // 更新鸿钧中心层文本（快照刷新时调用，不重建 DOM 避免动画闪烁）
  function 更新鸿钧中心状态() {
    var 快照 = 状态.快照 || {};
    var 阶段文 = document.querySelector("#星系图 .鸿钧阶段文");
    if (阶段文) 阶段文.textContent = 快照.当前阶段 ? "· " + 快照.当前阶段 : "";
    var 想法文 = document.querySelector("#星系图 .鸿钧想法文");
    if (想法文) 想法文.textContent = 快照.当前想法 ? 截断文(快照.当前想法, 24) : "";
  }

  // 拉取快照并刷新中心层与指标卡（实时过程门面）
  function 拉取快照() {
    fetch("/api/snapshot").then(function (r) { return r.json(); }).then(function (快照) {
      if (!快照) return;
      if (快照.启动时刻) 服务启动时刻ms = 快照.启动时刻;
      状态.快照 = 快照;
      更新鸿钧中心状态();
      渲染指标卡();
    }).catch(function () { /* 端点不存在不影响 */ });
  }

  // ===== 世界星图 =====
  function 渲染世界星图() {
    渲染指标卡();
    渲染想法星座();
    渲染要求推进();
    渲染事件流星();
    渲染格位星云();
    渲染世界星图SVG();
  }

  function 渲染指标卡() {
    var 想法数 = document.getElementById("指标-想法数");
    var 要求数 = document.getElementById("指标-要求数");
    var 智能体数 = document.getElementById("指标-智能体数");
    var 事件数 = document.getElementById("指标-事件数");
    if (智能体数) 智能体数.textContent = String(诸圣名录.length);
    var 快照 = 状态.快照 || {};
    if (事件数) 事件数.textContent = 快照.最近事件数 != null ? String(快照.最近事件数) : String(状态.事件们.length);
    if (想法数) 想法数.textContent = 快照.当前想法 ? "1" : "0";
    var 未处理想法元 = document.getElementById("指标-未处理想法");
    if (未处理想法元) 未处理想法元.textContent = "未处理 " + (快照.当前想法 ? "1" : "0");
    if (要求数) 要求数.textContent = 快照.当前要求 ? "1" : "0";
    var 要求细分元 = document.getElementById("指标-要求细分");
    if (要求细分元) 要求细分元.textContent = 快照.当前阶段 || "--";
    var 在位 = 算在位诸圣();
    var 运转中元 = document.getElementById("指标-运转中");
    if (运转中元) 运转中元.textContent = "运转中 " + 在位.size;
    var 最近时刻元 = document.getElementById("指标-最近时刻");
    if (最近时刻元 && 状态.事件们.length > 0) {
      最近时刻元.textContent = 格式化时刻(状态.事件们[状态.事件们.length - 1].时间戳);
    } else if (最近时刻元 && 快照.最近事件ts) {
      最近时刻元.textContent = 格式化时刻(快照.最近事件ts);
    }
  }

  function 渲染想法星座() {
    var 容器 = document.getElementById("想法星座");
    if (!容器) return;
    容器.innerHTML = "";
    var 快照 = 状态.快照 || {};
    if (快照.当前想法) {
      var 项 = document.createElement("div");
      项.className = "星座项";
      var 星 = document.createElement("span");
      星.className = "星座点";
      var 文 = document.createElement("span");
      文.className = "星座文";
      文.textContent = 快照.当前想法.substring(0, 60) + (快照.当前想法.length > 60 ? "…" : "");
      项.appendChild(星);
      项.appendChild(文);
      容器.appendChild(项);
    } else {
      var 提示 = document.createElement("div");
      提示.className = "星座项";
      提示.textContent = "想法队列待接天道";
      容器.appendChild(提示);
    }
  }

  function 渲染要求推进() {
    var 容器 = document.getElementById("要求推进");
    if (!容器) return;
    容器.innerHTML = "";
    var 快照 = 状态.快照 || {};
    if (快照.当前要求) {
      var 项 = document.createElement("div");
      项.className = "要求项";
      var 箭头 = document.createElement("span");
      箭头.className = "要求箭";
      箭头.textContent = "→";
      var 文 = document.createElement("span");
      文.className = "要求文";
      文.textContent = 快照.当前要求.substring(0, 60) + (快照.当前要求.length > 60 ? "…" : "");
      var 阶 = document.createElement("span");
      阶.className = "要求阶段";
      阶.textContent = 快照.当前阶段 || "";
      项.appendChild(箭头);
      项.appendChild(文);
      项.appendChild(阶);
      容器.appendChild(项);
    } else {
      var 提示 = document.createElement("div");
      提示.className = "要求项";
      提示.textContent = "要求队列待接天道";
      容器.appendChild(提示);
    }
  }

  function 渲染事件流星() {
    var 容器 = document.getElementById("事件流星");
    if (!容器) return;
    容器.innerHTML = "";
    var 事件们 = 状态.事件们 || [];
    var 显示数 = Math.min(事件们.length, 50);
    for (var i = 事件们.length - 显示数; i < 事件们.length; i++) {
      var 事 = 事件们[i];
      var 项 = document.createElement("div");
      项.className = "事件项";
      var 时刻 = document.createElement("span");
      时刻.className = "事件时刻";
      时刻.textContent = 格式化时刻(事.时间戳);
      var 类型 = document.createElement("span");
      类型.className = "事件类型";
      类型.textContent = 派生事件类型(事);
      var 参数 = document.createElement("span");
      参数.className = "事件参数";
      参数.textContent = 提炼摘要(事.证据 || 事.动作 || "");
      项.appendChild(时刻);
      项.appendChild(类型);
      项.appendChild(参数);
      容器.appendChild(项);
    }
    if (事件们.length === 0) {
      var 空 = document.createElement("div");
      空.className = "事件项";
      空.textContent = "尚无事件";
      容器.appendChild(空);
    }
  }

  function 渲染格位星云() {
    var 容器 = document.getElementById("格位星云");
    if (!容器) return;
    容器.innerHTML = "";
    // 用诸圣名录作为格位
    for (var i = 0; i < 诸圣名录.length; i++) {
      var 圣 = 诸圣名录[i];
      var 项 = document.createElement("span");
      项.className = "格位项";
      项.textContent = 圣.名;
      项.title = 圣.职能 || "";
      容器.appendChild(项);
    }
  }

  function 渲染世界星图SVG() {
    var svg = document.getElementById("世界星图");
    if (!svg) return;
    svg.innerHTML = "";
    var 宽 = 800, 高 = 460;

    // defs 渐变
    var defs = 创建SVG("defs");
    defs.innerHTML =
      '<radialGradient id="星云光" cx="50%" cy="50%" r="50%">' +
        '<stop offset="0%" stop-color="#13d4a4" stop-opacity="0.4"/>' +
        '<stop offset="100%" stop-color="#13d4a4" stop-opacity="0"/>' +
      '</radialGradient>' +
      '<linearGradient id="流星渐" x1="0%" y1="0%" x2="100%" y2="0%">' +
        '<stop offset="0%" stop-color="#f5a623" stop-opacity="0"/>' +
        '<stop offset="80%" stop-color="#f5a623" stop-opacity="0.8"/>' +
        '<stop offset="100%" stop-color="#fff7cf" stop-opacity="1"/>' +
      '</linearGradient>';
    svg.appendChild(defs);

    // 背景星点
    var 背景组 = 创建SVG("g");
    for (var b = 0; b < 60; b++) {
      var 星 = 创建SVG("circle");
      星.setAttribute("cx", Math.random() * 宽);
      星.setAttribute("cy", Math.random() * 高);
      星.setAttribute("r", Math.random() * 0.8 + 0.2);
      星.setAttribute("fill", "#c0c9e0");
      星.setAttribute("opacity", Math.random() * 0.5 + 0.2);
      背景组.appendChild(星);
    }
    svg.appendChild(背景组);

    // 格位星云（诸圣17角色）
    var 星云组 = 创建SVG("g");
    for (var i = 0; i < 诸圣名录.length; i++) {
      var 圣 = 诸圣名录[i];
      var 角 = (i / 诸圣名录.length) * Math.PI * 2;
      var 半径 = 150 + (i % 3) * 50;
      var cx = 宽 / 2 + Math.cos(角) * 半径 * 0.85;
      var cy = 高 / 2 + Math.sin(角) * 半径 * 0.55;
      // 星云团
      var 团 = 创建SVG("circle");
      团.setAttribute("cx", cx); 团.setAttribute("cy", cy);
      团.setAttribute("r", 18 + (i % 3) * 4);
      团.setAttribute("class", "星云团");
      团.setAttribute("fill", "url(#星云光)");
      星云组.appendChild(团);
      // 星云核
      var 核 = 创建SVG("circle");
      核.setAttribute("cx", cx); 核.setAttribute("cy", cy);
      核.setAttribute("r", 2);
      核.setAttribute("class", "星云核");
      星云组.appendChild(核);
      // 散点
      for (var k = 0; k < 5; k++) {
        var 散 = 创建SVG("circle");
        散.setAttribute("cx", cx + (Math.random() - 0.5) * 30);
        散.setAttribute("cy", cy + (Math.random() - 0.5) * 30);
        散.setAttribute("r", Math.random() * 1.2 + 0.3);
        散.setAttribute("class", "星云核");
        散.setAttribute("opacity", Math.random() * 0.6 + 0.2);
        星云组.appendChild(散);
      }
      // 名
      var 文 = 创建SVG("text");
      文.setAttribute("x", cx); 文.setAttribute("y", cy + 28);
      文.setAttribute("class", "星云名");
      文.textContent = 圣.名;
      星云组.appendChild(文);
    }
    svg.appendChild(星云组);

    // 流星组（动画绘制）
    var 流星组 = 创建SVG("g");
    流星组.setAttribute("id", "流星组");
    svg.appendChild(流星组);
  }

  // 星图流星动画
  function 启动星图流星() {
    停止星图流星();
    生成流星们();
    流星起始 = performance.now();
    星图流星帧 = requestAnimationFrame(流星帧);
  }
  function 停止星图流星() {
    if (星图流星帧) {
      cancelAnimationFrame(星图流星帧);
      星图流星帧 = null;
    }
  }
  function 生成流星们() {
    流星们 = [];
    var 事件数 = 状态.事件们.length;
    var 数 = Math.min(8, Math.max(3, Math.floor(事件数 / 10)));
    for (var i = 0; i < 数; i++) {
      流星们.push({
        起始: Math.random() * 5000,
        周期: 4000 + Math.random() * 3000,
        x1: Math.random() * 800,
        y1: Math.random() * 460,
        角: Math.PI / 4 + (Math.random() - 0.5) * 0.6,
        长: 80 + Math.random() * 60
      });
    }
  }
  function 流星帧(时刻) {
    var 组 = document.getElementById("流星组");
    if (!组) { 停止星图流星(); return; }
    组.innerHTML = "";
    var 经过 = 时刻 - 流星起始;
    for (var i = 0; i < 流星们.length; i++) {
      var 流 = 流星们[i];
      var 周期位 = ((经过 + 流.起始) % 流.周期) / 流.周期;
      if (周期位 < 0 || 周期位 > 1) continue;
      var 前进 = 周期位;
      var 头x = 流.x1 + Math.cos(流.角) * 流.长 * 前进;
      var 头y = 流.y1 + Math.sin(流.角) * 流.长 * 前进;
      var 尾x = 头x - Math.cos(流.角) * 30;
      var 尾y = 头y - Math.sin(流.角) * 30;
      var 透明 = Math.sin(周期位 * Math.PI);
      // 尾迹
      var 尾 = 创建SVG("line");
      尾.setAttribute("x1", 尾x); 尾.setAttribute("y1", 尾y);
      尾.setAttribute("x2", 头x); 尾.setAttribute("y2", 头y);
      尾.setAttribute("stroke", "url(#流星渐)");
      尾.setAttribute("stroke-width", "1.5");
      尾.setAttribute("opacity", 透明);
      组.appendChild(尾);
      // 头
      var 头 = 创建SVG("circle");
      头.setAttribute("cx", 头x); 头.setAttribute("cy", 头y);
      头.setAttribute("r", 2);
      头.setAttribute("fill", "#fff7cf");
      头.setAttribute("opacity", 透明);
      头.setAttribute("filter", "drop-shadow(0 0 3px #f5a623)");
      组.appendChild(头);
    }
    星图流星帧 = requestAnimationFrame(流星帧);
  }

  // ===== 私聊屏：5角色会话列表 =====
  function 渲染私聊会话列表() {
    var 容器 = document.getElementById("私聊会话列");
    if (!容器) return;
    容器.innerHTML = "";
    var 五圣 = ["鸿钧", "女娲", "多宝", "红云", "玄天"];
    var 职能映射 = {};
    for (var i = 0; i < 诸圣名录.length; i++) {
      职能映射[诸圣名录[i].名] = 诸圣名录[i].职能;
    }
    var 摘要映射 = {
      鸿钧: "界主与鸿钧一对一对话",
      女娲: "初稿设计与方案",
      多宝: "代码与产物落地",
      红云: "质量与边界审验",
      玄天: "世界巡查与发现"
    };
    for (var j = 0; j < 五圣.length; j++) {
      var 名 = 五圣[j];
      var 项 = document.createElement("div");
      项.className = "私聊会话项" + (j === 0 ? " 选中" : "");
      项.dataset.会话 = 名;
      // 头像
      var 头像 = document.createElement("div");
      头像.className = "会话头像";
      头像.textContent = 名[0];
      头像.style.background = "var(--色-" + 名 + ")";
      头像.style.color = 名 === "鸿钧" || 名 === "玄天" ? "var(--底)" : "#fff";
      // 信息
      var 信息 = document.createElement("div");
      信息.className = "会话信息";
      var 名div = document.createElement("div");
      名div.className = "会话名";
      名div.textContent = 名 + " · " + (职能映射[名] || "");
      var 摘 = document.createElement("div");
      摘.className = "会话摘";
      摘.textContent = 摘要映射[名] || "";
      信息.appendChild(名div);
      信息.appendChild(摘);
      // 状态点
      var 态 = document.createElement("div");
      态.className = "会话态" + (j === 0 ? " 活" : "");
      项.appendChild(头像);
      项.appendChild(信息);
      项.appendChild(态);
      // 点击切换会话
      项.addEventListener("click", function (圣名) {
        return function () {
          var 项们 = 容器.querySelectorAll(".私聊会话项");
          for (var m = 0; m < 项们.length; m++) {
            项们[m].classList.remove("选中");
          }
          项.classList.add("选中");
          更新私聊头(圣名);
        };
      }(名));
      容器.appendChild(项);
    }
    // 默认显示鸿钧
    更新私聊头("鸿钧");
    // 输入栏占位（不接后端，仅壳）
    var 发送钮 = document.getElementById("私聊发送钮");
    var 输入框 = document.getElementById("私聊输入框");
    if (发送钮 && 输入框) {
      发送钮.addEventListener("click", function () {
        var 文 = 输入框.value.trim();
        if (!文) return;
        追加私聊消息("界主", 文);
        输入框.value = "";
      });
      输入框.addEventListener("keydown", function (e) {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          发送钮.click();
        }
      });
    }
  }

  function 更新私聊头(名) {
    var 头像 = document.getElementById("私聊头像");
    var 头名 = document.getElementById("私聊头名");
    var 头职 = document.getElementById("私聊头职");
    if (头像) {
      头像.textContent = 名[0];
      头像.style.background = "var(--色-" + 名 + ")";
      头像.style.color = 名 === "鸿钧" || 名 === "玄天" ? "var(--底)" : "#fff";
    }
    if (头名) 头名.textContent = 名;
    var 职能 = "";
    for (var i = 0; i < 诸圣名录.length; i++) {
      if (诸圣名录[i].名 === 名) { 职能 = 诸圣名录[i].职能; break; }
    }
    if (头职) 头职.textContent = 职能 + " · 接收界主指令";
  }

  function 追加私聊消息(角色, 文) {
    var 流 = document.getElementById("对话流");
    if (!流) return;
    var 消息 = document.createElement("div");
    消息.className = "私聊消息 " + (角色 === "界主" ? "界主" : "圣");
    消息.textContent = 文;
    流.appendChild(消息);
    流.scrollTop = 流.scrollHeight;
  }

  // 启动
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", 初始化);
  } else {
    初始化();
  }

  // 暴露 状态 供外部对接
  window.轨迹状态 = 状态;
})();
