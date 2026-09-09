/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 主区任务看板 实现
   职责：拉取 GET /api/board 真实任务，按五行层级（木火土金水）分列渲染，
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
    { 层:"木", 名:"木 · 需求", 角:"道祖",     色:"var(--木色)" },
    { 层:"火", 名:"火 · 设计", 角:"圣人",     色:"var(--火色)" },
    { 层:"土", 名:"土 · 实现", 角:"大罗金仙", 色:"var(--土色)" },
    { 层:"金", 名:"金 · 验收", 角:"准圣",     色:"var(--金色)" },
    { 层:"水", 名:"水 · 清理", 角:"太乙金仙", 色:"var(--水色)" }
  ];

  // 刷新图标
  var 刷 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/></svg>';
  // 驱动图标（闪电）
  var 驱 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L4.5 13.5H11L10 22l8.5-11.5H12z"/></svg>';
  // 驱动到空闲图标（双箭头快进）
  var 疾 = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 5v14M4 7l5 5-5 5M13 7l5 5-5 5"/></svg>';

  var 列体集合 = {};
  var 计数集合 = {};

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
    头.appendChild(刷新钮);
    var 驱动钮 = 按钮(驱 + "驱动", function(){ 驱动(false); });
    驱动钮.classList.add("主");
    头.appendChild(驱动钮);
    var 疾钮 = 按钮(疾 + "驱动到空闲", function(){ 驱动(true); });
    疾钮.classList.add("主");
    头.appendChild(疾钮);
    外.appendChild(头);

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

    // 初始载入一次（即便隐藏也预载，切换时数据就绪）
    载看板();
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
        // 按五行归组
        var 组 = { 木:[], 火:[], 土:[], 金:[], 水:[] };
        (任务 || []).forEach(function(t){
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
          列表.forEach(function(t){
            体.appendChild(卡片(t, 列));
          });
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
    卡.title = "任务 #" + t.id;
    return 卡;
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

  // ── 驱动：触发看板驱动台，随后轮询刷新直到空闲 ──
  function 驱动(到空闲){
    var 地址 = 后端() + (到空闲 ? "/api/dev/pilot/drain" : "/api/dev/pilot");
    var 体 = 到空闲 ? JSON.stringify({ 上限: 20 }) : undefined;
    fetch(地址, { method: "POST", headers: { "Content-Type": "application/json" }, body: 体 })
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(){ 轮询驱动(); })
      .catch(function(){ 载看板(); });
  }

  // 每 2 秒刷新看板 + 驱动状态，直到驱动空闲
  function 轮询驱动(){
    载看板();
    fetch(后端() + "/api/dev/pilot/status")
      .then(function(r){ return r.json(); })
      .then(function(d){ if (d && d.运行中){ setTimeout(轮询驱动, 2000); } })
      .catch(function(){});
  }

  return { 建: 建 };
})();

// 经挂载契约入园
乾坤界面.挂载("主区", 乾坤看板.建);
