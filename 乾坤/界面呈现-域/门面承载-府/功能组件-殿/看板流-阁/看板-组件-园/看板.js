/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 主区任务看板 实现
   职责：拉取 GET /api/board 真实任务，按五行层级（木火土金水）分列渲染；
   卡片点击弹出详情弹层（GET /api/board/{id}）；驱动按钮带加载态与状态条。
   经挂载契约进入主区槽位；监听「乾坤图标」在对话/看板间切换。
   ═══════════════════════════════════════════════════════════ */
window.乾坤看板 = (function(){
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }

  // ── 状态 → 五行 权威映射（与后端 流转逻辑.状态层级标签 一致）──
  var 五行映射 = {
    "待受理":"木", "进行中":"木", "已取消":"木", "待道祖澄清":"木", "道祖澄清中":"木",
    "待圣人设计":"火", "圣人设计中":"火", "待重新设计":"火",
    "待大罗金仙实现":"土", "大罗金仙实现中":"土", "待修复":"土", "待重新实现":"土",
    "待准圣验收":"金", "准圣验收中":"金", "待道祖终审":"金", "道祖终审中":"金", "待重新验收":"金", "已完成":"金",
    "待清理":"水", "清理中":"水", "待重新清理":"水", "清理完成":"水"
  };

  // ── 五行列定义（相生顺序：木→火→土→金→水）──
  var 五行列 = [
    { 层:"木", 名:"木 · 需求", 角:"道祖",     色:"var(--木)" },
    { 层:"火", 名:"火 · 设计", 角:"圣人",     色:"var(--火)" },
    { 层:"土", 名:"土 · 实现", 角:"大罗金仙", 色:"var(--土)" },
    { 层:"金", 名:"金 · 验收", 角:"准圣",     色:"var(--金)" },
    { 层:"水", 名:"水 · 清理", 角:"太乙金仙", 色:"var(--水)" }
  ];

  var 刷 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/></svg>';
  var 驱 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L4.5 13.5H11L10 22l8.5-11.5H12z"/></svg>';
  var 疾 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 5v14M4 7l5 5-5 5M13 7l5 5-5 5"/></svg>';
  var 关图标 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg>';

  var 列体集合 = {};
  var 计数集合 = {};
  var 驱动条 = null;      // 状态条节点
  var 钮集合 = [];        // [刷新钮, 驱动钮, 疾钮]
  var 轮询句柄 = null;    // 驱动状态轮询定时器
  var 数据缓存 = {};      // id → 任务（弹层兜底用）

  function 建(){
    var 外 = document.createElement("div");
    外.className = "看板外壳 主区视图";
    外.setAttribute("data-视图", "看板");
    外.style.display = "none"; // 默认对话流显示，看板隐藏

    // ── 看板头 ──
    var 头 = document.createElement("div");
    头.className = "看板头";
    头.innerHTML = '<span class="标">任务看板</span><span class="次">五行相生 · 任务流转</span>';
    var 刷新钮 = 按钮(刷 + "刷新", 载看板);
    var 驱动钮 = 按钮(驱 + "驱动", function(){ 驱动(false); });
    驱动钮.classList.add("主");
    var 疾钮 = 按钮(疾 + "驱动到空闲", function(){ 驱动(true); });
    疾钮.classList.add("主");
    头.appendChild(刷新钮); 头.appendChild(驱动钮); 头.appendChild(疾钮);
    钮集合 = [刷新钮, 驱动钮, 疾钮];
    外.appendChild(头);

    // ── 驱动状态条 ──
    驱动条 = document.createElement("div");
    驱动条.className = "驱动条";
    驱动条.innerHTML = '<span class="灯"></span><span class="文">正在探测驱动台…</span>';
    外.appendChild(驱动条);

    // ── 列组 ──
    var 列组 = document.createElement("div");
    列组.className = "看板列组";
    五行列.forEach(function(列){
      var 列节点 = document.createElement("section");
      列节点.className = "看板列";
      列节点.style.setProperty("--列色", 列.色);

      var 列头 = document.createElement("header");
      列头.className = "看板列头";
      列头.innerHTML = '<span class="点"></span><span class="名">' + 列.名 + '</span><span class="角">' + 列.角 + '</span>';
      var 计 = document.createElement("span");
      计.className = "计";
      计.textContent = "0";
      列头.appendChild(计);
      计数集合[列.层] = 计;

      var 列体 = document.createElement("div");
      列体.className = "看板列体";
      列体.setAttribute("data-层", 列.层);
      列体集合[列.层] = 列体;

      列节点.appendChild(列头);
      列节点.appendChild(列体);
      列组.appendChild(列节点);
    });
    外.appendChild(列组);

    // ── 主区视图切换：对话/看板 ──
    document.addEventListener("乾坤图标", function(e){
      var 键 = e.detail && e.detail.键;
      if (键 === "看板"){
        切换主区("看板");
        载看板();
      } else if (键 === "对话"){
        切换主区("对话");
      }
    });

    载看板();
    刷状态();
    return 外;
  }

  function 切换主区(目标){
    document.querySelectorAll(".主区视图").forEach(function(v){
      v.style.display = (v.getAttribute("data-视图") === 目标) ? "" : "none";
    });
  }

  function 载看板(){
    fetch(后端() + "/api/board")
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(任务){
        var 组 = { 木:[], 火:[], 土:[], 金:[], 水:[] };
        (任务 || []).forEach(function(t){
          数据缓存[t.id] = t;
          var 层 = 五行映射[t.status] || "木";
          (组[层] = 组[层] || []).push(t);
        });
        五行列.forEach(function(列){
          var 列表 = 组[列.层] || [];
          计数集合[列.层].textContent = String(列表.length);
          var 体 = 列体集合[列.层];
          体.innerHTML = "";
          if (!列表.length){
            var 空 = document.createElement("div");
            空.className = "看板空";
            空.textContent = "暂无任务";
            体.appendChild(空);
            return;
          }
          列表.forEach(function(t){ 体.appendChild(卡片(t, 列)); });
        });
      })
      .catch(function(){
        五行列.forEach(function(列){
          计数集合[列.层].textContent = "—";
          列体集合[列.层].innerHTML = '<div class="看板空">看板读取失败 · 需数据服务在线</div>';
        });
      });
  }

  function 卡片(t, 列){
    var 卡 = document.createElement("button");
    卡.className = "看板卡";
    var 状 = String(t.status || "—");
    var 进行中 = 状.indexOf("中") >= 0;
    卡.innerHTML =
      '<span class="题">' + 转(t.title || "（无标题）") + '</span>' +
      '<span class="元">' +
        '<span class="状' + (进行中 ? " 行" : "") + '">' + 转(状) + '</span>' +
        (t.优先级 ? '<span class="优">' + 转(t.优先级) + '</span>' : '') +
        (t.当前承接人 && t.当前承接人 !== "道祖" ? '<span class="承">' + 转(t.当前承接人) + '</span>' : '') +
      '</span>';
    卡.title = "任务 #" + t.id + " · 点击查看详情";
    卡.addEventListener("click", function(){ 弹层(t.id); });
    return 卡;
  }

  // ── 卡片详情弹层：优先取详情接口，失败回退列表缓存 ──
  function 弹层(id){
    var 罩 = document.createElement("div");
    罩.className = "看板弹罩";
    var 层 = document.createElement("div");
    层.className = "看板弹层";
    层.innerHTML = '<div class="弹头"><span class="弹标">任务 #' + 转(id) + '</span><span class="弹载">加载中…</span></div>';
    罩.appendChild(层);
    document.body.appendChild(罩);

    // 关闭统一收口：摘除 Esc 监听 + 幂等移除节点（避免监听器堆积与重复关闭抛错）
    function esc(e){ if (e.key === "Escape"){ 关(); } }
    function 关(){
      document.removeEventListener("keydown", esc);
      if (罩.parentNode){ 罩.parentNode.removeChild(罩); }
    }
    罩.addEventListener("click", function(e){ if (e.target === 罩){ 关(); } });
    document.addEventListener("keydown", esc);

    var 本地 = 数据缓存[id];
    fetch(后端() + "/api/board/" + id)
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .catch(function(){ if (本地){ return 本地; } throw new Error("无数据"); })
      .then(function(t){ 渲染弹层(层, t, 关); })
      .catch(function(){
        层.innerHTML = '<div class="弹头"><span class="弹标">任务 #' + 转(id) + '</span></div>' +
          '<div class="弹空">详情读取失败 · 需数据服务在线</div>';
      });
  }

  function 渲染弹层(层, t, 关){
    var 状 = String(t.status || "—");
    var 色 = 五行列.filter(function(c){ return c.层 === (五行映射[状] || "木"); })[0].色;
    var 历史 = (t.状态历史 || []).map(function(h){
      return '<div class="史行"><span class="史迁">' + 转(h.原状态) + ' → ' + 转(h.新状态) + '</span>' +
        '<span class="史者">' + 转(h.操作者) + '</span>' +
        '<span class="史时">' + 时间(h.时间) + '</span>' +
        (h.备注 ? '<span class="史注">' + 转(h.备注) + '</span>' : '') + '</div>';
    }).join("");
    var 回退 = t.回退来源
      ? '<div class="弹块"><div class="块题">回退来源</div><div class="回退行">' +
        '<span class="史迁">' + 转(t.回退来源.来源层级) + ' → ' + 转(t.回退来源.目标层级) + '</span>' +
        '<span class="史注">' + 转(t.回退来源.原因) + '</span>' +
        '<span class="史时">' + 时间(t.回退来源.回退时间) + '</span></div></div>'
      : "";
    var 澄清 = t.澄清记录
      ? '<div class="弹块"><div class="块题">澄清记录</div><div class="史注">' + 转(t.澄清记录.结论) +
        ' · ' + (t.澄清记录.继续 ? "继续推进" : "已终止") + ' · ' + 时间(t.澄清记录.时间) + '</div></div>'
      : "";
    var 扫尾 = t.扫尾记录
      ? '<div class="弹块"><div class="块题">交付核验</div><div class="史注">变更 ' + t.扫尾记录.变更总数 +
        ' · 兑现 ' + t.扫尾记录.兑现数 + ' · 未兑现 ' + t.扫尾记录.未兑现数 +
        (t.扫尾记录.说明 ? ' · ' + 转(t.扫尾记录.说明) : '') + '</div></div>'
      : "";
    var 承接 = (t.承接历史 && t.承接历史.length)
      ? '<div class="弹块"><div class="块题">承接历史</div><div class="史注">' +
        t.承接历史.map(转).join(" · ") + '</div></div>'
      : "";

    层.innerHTML =
      '<div class="弹头">' +
        '<span class="弹标">任务 #' + 转(t.id) + '</span>' +
        '<span class="弹状" style="color:' + 色 + ';border-color:' + 色 + '">' + 转(状) + '</span>' +
        '<button class="弹关" title="关闭（Esc）">' + 关图标 + '</button>' +
      '</div>' +
      '<div class="弹体">' +
        '<div class="弹题">' + 转(t.title || "（无标题）") + '</div>' +
        '<div class="弹描">' + (t.description ? 转(t.description) : '<span class="弱">无描述</span>') + '</div>' +
        '<div class="弹表">' +
          元("场景", t.场景) + 元("优先级", t.优先级) +
          元("发起人", t.发起人) + 元("承接人", t.当前承接人) +
          元("总令牌", t.总令牌 ? Number(t.总令牌).toLocaleString() : "—") +
          元("修复轮次", t.修复轮次 || "—") +
          元("创建", 时间(t.created_at)) + 元("更新", 时间(t.updated_at)) +
        '</div>' + 回退 + 澄清 + 扫尾 + 承接 +
        (历史 ? '<div class="弹块"><div class="块题">状态历史</div><div class="弹史">' + 历史 + '</div></div>' : '') +
      '</div>';
    层.querySelector(".弹关").addEventListener("click", 关);
  }

  function 元(名, 值){
    return '<div class="元项"><span class="元名">' + 名 + '</span><span class="元值">' + 转(值 == null || 值 === "" ? "—" : 值) + '</span></div>';
  }

  function 时间(秒){
    if (!秒){ return "—"; }
    try { return new Date(秒 * 1000).toLocaleString(); } catch(e){ return "—"; }
  }

  function 转(s){
    return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  // ── 工具：构建顶栏按钮 ──
  function 按钮(html, 点击){
    var b = document.createElement("button");
    b.className = "钮";
    b.innerHTML = html;
    b.addEventListener("click", 点击);
    return b;
  }

  // ── 驱动：触发看板驱动台；期间按钮进入加载态并轮询状态条 ──
  function 驱动(到空闲){
    if (钮集合[1].disabled){ return; } // 已有驱动进行中
    设加载(true, 到空闲 ? "正在受理驱动到空闲…" : "正在受理驱动…");
    var 地址 = 后端() + (到空闲 ? "/api/dev/pilot/drain" : "/api/dev/pilot");
    var 体 = 到空闲 ? JSON.stringify({ 上限: 20 }) : undefined;
    fetch(地址, { method: "POST", headers: { "Content-Type": "application/json" }, body: 体 })
      .then(function(r){
        if (r.status === 409){ 条文("已有驱动执行中，正在跟随进度…", "讯"); return null; }
        if (r.status === 503){ 条文("驱动台未就绪：需配置 LLM_API_KEY 并开启 run_dev_agent 后重启", "险"); 设加载(false); return null; }
        if (!r.ok){ throw new Error("HTTP " + r.status); }
        条文(到空闲 ? "已受理：驱动到空闲执行中…" : "已受理：驱动一轮执行中…", "行");
        return r.json();
      })
      .then(function(d){ if (d){ 轮状态(); } })
      .catch(function(){ 条文("驱动请求失败 · 需数据服务在线", "险"); 设加载(false); 载看板(); });
  }

  // 每 2 秒刷新状态条与看板，直到驱动空闲后恢复按钮
  function 轮状态(){
    if (轮询句柄){ return; }
    var 步 = function(){
      fetch(后端() + "/api/dev/pilot/status")
        .then(function(r){ return r.json(); })
        .then(function(d){
          if (d && d.运行中){
            条文("驱动执行中" + (d.最近结果 ? " · " + d.最近结果 : ""), "行");
            载看板();
            轮询句柄 = setTimeout(步, 2000);
          } else {
            轮询句柄 = null;
            设加载(false);
            条文(d && d.最近结果 ? "驱动完成 · " + d.最近结果 : "驱动完成 · 驱动台空闲", "成");
            载看板();
          }
        })
        .catch(function(){
          轮询句柄 = null;
          设加载(false);
          条文("驱动状态探测失败 · 需数据服务在线", "险");
        });
    };
    步();
  }

  // 一次性探测驱动台就绪状态（建园时与驱动结束后复用）
  function 刷状态(){
    fetch(后端() + "/api/dev/pilot/status")
      .then(function(r){ return r.json(); })
      .then(function(d){
        if (d && d.运行中){
          设加载(true);
          条文("驱动执行中" + (d.最近结果 ? " · " + d.最近结果 : ""), "行");
          轮状态();
        } else if (d && d.就绪){
          条文("驱动台就绪 · 空闲", "闲");
        } else {
          条文("驱动台未就绪：需配置 LLM_API_KEY 并开启 run_dev_agent", "险");
        }
      })
      .catch(function(){ 条文("驱动状态未知 · 需数据服务在线", "险"); });
  }

  function 设加载(中, 文案){
    钮集合.forEach(function(b, i){
      b.disabled = 中;
      b.classList.toggle("载", 中 && i > 0);
    });
    if (中 && 文案){ 条文(文案, "行"); }
  }

  function 条文(文, 态){
    if (!驱动条){ return; }
    驱动条.className = "驱动条 态-" + (态 || "闲");
    驱动条.querySelector(".文").textContent = 文;
  }

  return { 建: 建 };
})();

// 经挂载契约入园
乾坤界面.挂载("主区", 乾坤看板.建);
