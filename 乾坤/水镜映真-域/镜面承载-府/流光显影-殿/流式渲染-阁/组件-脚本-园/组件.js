/* ═══════════════════════════════════════════════════════════
   水镜 · 镜面（显影层组件注册表 + 页面装配）
   规约：组件不私藏业务状态——一切显示来自长河归约视图；
        动作按名注册（TokUI 安全事件思路），DSL/组件只引用动作名。
   ═══════════════════════════════════════════════════════════ */
window.水镜镜面 = (function(){
  "use strict";

  var 组件表 = {};
  var 动作表 = {};
  var 已建消息 = {};   // 消息id → { 节点, 流光, 收过 }
  var 已建任务 = {};   // 任务id → { 节点, 态点, 过程点 }
  var 待确认节点 = null;
  var 待确认计时 = null;
  var 流点 = null;
  var 态点 = null;

  function 注册组件(名, fn){ if (名 && typeof fn === "function"){ 组件表[名] = fn; } }
  function 注册动作(名, fn){ if (名 && typeof fn === "function"){ 动作表[名] = fn; } }
  function 调动作(名, 参数){ var f = 动作表[名]; return f ? f(参数) : undefined; }

  function 后端址(){
    return (typeof window.水镜配置 !== "undefined" && window.水镜配置.道祖) || "";
  }

  // ── 启：装配输入条、订阅长河、接入河口、初始回放 ──
  function 启(){
    流点 = document.querySelector("#镜流");
    态点 = document.querySelector("#镜态");
    var 输 = document.querySelector("#镜入");
    var 发 = document.querySelector("#镜发");

    window.水镜长河.订事件(河事件);
    window.水镜长河.订状态(河状态);

    if (发){ 发.addEventListener("click", 发送); }
    if (输){
      输.addEventListener("keydown", function(e){
        if (e.key === "Enter" && !e.shiftKey){ e.preventDefault(); 发送(); }
      });
    }

    注册动作("确认发布", 确认发布);
    注册动作("驳回发布", 驳回发布);

    window.水镜河口.接入();
    window.水镜河口.最新会话回放();
  }

  function 发送(){
    var 输 = document.querySelector("#镜入");
    var 文 = (输.value || "").trim();
    if (!文){ return; }
    输.value = "";
    if (文 === "/清空"){
      window.水镜长河.派发({ 类型: "清河" });
      清挂点();
      return;
    }
    if (文 === "/刷新"){
      清挂点();
      window.水镜河口.最新会话回放();
      return;
    }
    appendUser(文);
    window.水镜河口.发消息(文);
  }

  // ── 事件订户：流式增量直通上屏（增量有界）──
  function 河事件(ev){
    if ((ev.类型 === "回复增量" || ev.类型 === "思考增量") && 已建消息[ev.消息id]){
      var 记 = 已建消息[ev.消息id];
      if (ev.类型 === "思考增量"){ 记.思喂 += ev.文 || ""; }
      else { 记.流光.喂(ev.文 || ""); }
    }
    if (ev.类型 === "河错" && 流点){ 流点.appendChild(错误条(ev.来源, ev.文)); 卷底(); }
  }

  // ── 状态订户：结构 diff（消息/任务/待确认/态条）──
  function 河状态(s){
    if (态点){
      var 文 = (s.连接 === "离线" ? "空闲" : s.连接) + (s.阶段 && s.阶段 !== "空闲" ? " · " + s.阶段 : "");
      if (态点.textContent !== 文){ 态点.textContent = 文; }
    }
    if (!流点){ return; }
    var i;
    for (i = 0; i < s.消息.length; i++){ 保消息(s.消息[i]); }
    Object.keys(s.任务).forEach(function(id){ 保任务卡(id, s.任务[id]); });
    保待确认(s.阶段);
    卷底();
  }

  function 保消息(m){
    var 记 = 已建消息[m.消息id];
    if (!记){
      if (m.方向 === "己方"){ 记 = { 节点: appendUser已有(m), 流光: null, 思喂: "" }; }
      else { 记 = 建对方泡(m); }
      已建消息[m.消息id] = 记;
      // 中途入河（回放/晚连）：把已有段一次性喂入（受同一渲染器约束，天然安全）
      if (m.方向 !== "己方" && m.段.length){ 初喂段(记, m.段); }
    }
    if (记.流光 && !m.在流 && !记.收过){ 记.流光.收尾(); 记.收过 = true; }
  }

  function 初喂段(记, 段们){
    for (var i = 0; i < 段们.length; i++){
      var 段 = 段们[i];
      if (段.类 === "思考"){ 记.流光.喂("<think>\n" + 段.文 + "\n</think>\n"); }
      else if (段.类 === "正文"){ 记.流光.喂(段.文 + "\n"); }
      else if (段.类 === "工具"){ 记.流光.喂("[p ⚙ " + 段.名称 + " → " + (段.结果 || "执行中") + "]\n"); }
    }
    记.思喂 = "";
  }

  function 建对方泡(m){
    var 容 = el("div", "镜-讯 对");
    var 头 = el("div", "镜-谁");
    头.appendChild(el("b", "镜-名", m.角色 || "道祖"));
    容.appendChild(头);
    var 泡 = el("div", "镜-泡");
    var 体 = el("div", "镜-流体");
    泡.appendChild(体);
    容.appendChild(泡);
    流点.appendChild(容);
    return { 节点: 容, 流光: window.镜流光.建(体, 动作表), 思喂: "", 收过: false };
  }

  function appendUser(文){
    var 容 = el("div", "镜-讯 己");
    var 头 = el("div", "镜-谁");
    头.appendChild(el("b", "镜-名", "界主"));
    容.appendChild(头);
    var 泡 = el("div", "镜-泡 镜-己泡");
    泡.textContent = 文;              // 界主输入同样零 innerHTML
    容.appendChild(泡);
    流点.appendChild(容);
    卷底();
    return 容;
  }

  function appendUser已有(m){
    var 容 = el("div", "镜-讯 己");
    var 头 = el("div", "镜-谁");
    头.appendChild(el("b", "镜-名", m.角色 || "界主"));
    容.appendChild(头);
    var 泡 = el("div", "镜-泡 镜-己泡");
    var 文 = "";
    m.段.forEach(function(段){ if (段.类 === "正文"){ 文 += 段.文; } });
    泡.textContent = 文;
    容.appendChild(泡);
    流点.appendChild(容);
    return 容;
  }

  // 任务卡：状态词直显（状态词表唯一来源=后端；前端不做字符串猜谜）
  function 保任务卡(id, t){
    var 记 = 已建任务[id];
    if (!记){
      var 卡 = el("div", "镜-任务卡");
      var 头 = el("div", "镜-任务头");
      头.appendChild(el("b", null, "任务 #" + id));
      var 态 = el("span", "镜-任务态");
      头.appendChild(态);
      卡.appendChild(头);
      var 过 = el("div", "镜-任务过程");
      卡.appendChild(过);
      流点.appendChild(卡);
      记 = { 节点: 卡, 态点: 态, 过程点: 过, 过程数: 0 };
      已建任务[id] = 记;
    }
    if (记.态点.textContent !== (t.状态 || "")){ 记.态点.textContent = t.状态 || ""; }
    if (t.过程.length > 记.过程数){
      记.过程数 = t.过程.length;
      记.过程点.textContent = "";
      t.过程.slice(-5).forEach(function(条){
        var 行 = el("div", "镜-过程行");
        行.textContent = (条.角色 || "") + (条.工具名 ? " · " + 条.工具名 : "") + "：" + 条.内容;
        记.过程点.appendChild(行);
      });
    }
  }

  // 待确认卡：5 秒自动确认（动作按名引用）
  function 保待确认(阶段){
    if (阶段 === "待确认" && !待确认节点){ 待确认节点 = 建待确认(); }
    if (阶段 !== "待确认" && 待确认节点 && 待确认节点.dataset.态 === "开"){
      待确认节点.dataset.态 = "收";
      待确认节点.querySelector(".镜-确认态").textContent = "已处理";
      停倒计时();
      待确认节点 = null;
    }
  }

  function 建待确认(){
    var 卡 = el("div", "镜-任务卡 镜-确认卡");
    卡.dataset.态 = "开";
    var 头 = el("div", "镜-任务头");
    头.appendChild(el("b", null, "道祖已对齐需求"));
    头.appendChild(el("span", "镜-确认态", "待确认 5s"));
    卡.appendChild(头);
    卡.appendChild(el("div", "镜-确认描", "确认后即安排圣人设计、大罗金仙实现、准圣验收。"));
    var 排 = el("div", "镜-确认排");
    var 准 = el("button", "dsl-btn", "确认发布");
    var 否 = el("button", "dsl-btn 镜-否", "驳回");
    准.type = "button"; 否.type = "button";
    排.appendChild(准); 排.appendChild(否);
    卡.appendChild(排);
    流点.appendChild(卡);
    var 秒 = 5;
    待确认计时 = setInterval(function(){
      秒 -= 1;
      var 状 = 卡.querySelector(".镜-确认态");
      if (秒 <= 0){ 停倒计时(); 调动作("确认发布", 卡); }
      else if (状){ 状.textContent = "待确认 " + 秒 + "s"; }
    }, 1000);
    准.addEventListener("click", function(){ 停倒计时(); 调动作("确认发布", 卡); });
    否.addEventListener("click", function(){ 停倒计时(); 调动作("驳回发布", 卡); });
    return 卡;
  }

  function 停倒计时(){ if (待确认计时){ clearInterval(待确认计时); 待确认计时 = null; } }

  function 确认发布(卡){
    var 态 = 卡 && 卡.querySelector(".镜-确认态");
    if (态){ 态.textContent = "发布中…"; }
    fetch(后端址() + "/api/dev/chat/confirm", { method: "POST" })
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(d){
        if (态){ 态.textContent = "已发布 · 任务 #" + (d.任务id != null ? d.任务id : "?"); }
        var 排 = 卡 && 卡.querySelector(".镜-确认排");
        if (排 && 排.parentNode){ 排.parentNode.removeChild(排); }
        if (d.任务id != null){
          window.水镜长河.派发({ 类型: "任务状态", 任务id: d.任务id, 状态: "启动中" });
          window.水镜河口.轮任务(d.任务id);
        }
      })
      .catch(function(){ if (态){ 态.textContent = "发布失败"; } });
  }

  function 驳回发布(卡){
    var 态 = 卡 && 卡.querySelector(".镜-确认态");
    if (态){ 态.textContent = "已驳回"; }
    var 排 = 卡 && 卡.querySelector(".镜-确认排");
    if (排 && 排.parentNode){ 排.parentNode.removeChild(排); }
  }

  function 错误条(来源, 文){
    var n = el("div", "镜-河错");
    n.textContent = "〔" + (来源 || "河") + "〕" + (文 || "");
    return n;
  }

  function 清挂点(){
    if (!流点){ return; }
    while (流点.firstChild){ 流点.removeChild(流点.firstChild); }
    Object.keys(已建消息).forEach(function(k){ delete 已建消息[k]; });
    Object.keys(已建任务).forEach(function(k){ delete 已建任务[k]; });
    待确认节点 = null;
    停倒计时();
  }

  function 卷底(){ if (流点){ 流点.scrollTop = 流点.scrollHeight; } }

  function el(标, 类名, 文本){
    var n = document.createElement(标);
    if (类名){ n.className = 类名; }
    if (文本 != null){ n.textContent = 文本; }
    return n;
  }

  return {
    启: 启, 注册组件: 注册组件, 注册动作: 注册动作, 调动作: 调动作,
    清挂点: 清挂点, 当前会话id: ""
  };
})();
