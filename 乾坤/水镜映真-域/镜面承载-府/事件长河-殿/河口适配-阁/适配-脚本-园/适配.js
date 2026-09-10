/* ═══════════════════════════════════════════════════════════
   水镜 · 河口适配层：长河统一通道（/api/river）直连 + 旧接口降级兜底。
   v1 落地：接待流/过程流切到后端长河事件直通，方言翻译表整体退役；
   旧接口（/api/dev/chat 同步、/api/board 任务态、/api/dev/sessions 回放）保留为兜底/补位。
   ═══════════════════════════════════════════════════════════ */
window.水镜河口 = (function(){
  "use strict";

  var 河 = function(){ return window.水镜长河; };
  var 任务计时 = {};

  function 后端址(){
    return (typeof window.水镜配置 !== "undefined" && window.水镜配置.道祖) || "";
  }

  function 派(ev){ 河().派发(ev); }

  // ── 接待流：POST /api/river/chat（后端长河事件直通，无需方言翻译）──
  // 看门狗（实测依据：后端断流时 read() 挂死不 reject，降级永不触发）：
  //   每次收到帧刷新活动时间；超阈值 → AbortController.abort() 强制断流
  //   （abort 使 read() 必 reject，复用 catch 分支走降级/收尾）。
  //   双阈值：已出正文 15s 无帧判死；未出正文 90s（容忍 LLM 长思考期无帧）。
  function 发消息(文){
    var 局 = { 出过内容: false, 收过尾: false, 活动: Date.now(), 看门狗: null };
    河().置连接("接待中");
    var 控 = new AbortController();
    局.看门狗 = setInterval(function(){
      var 上限 = 局.出过内容 ? 15000 : 90000;
      if (Date.now() - 局.活动 > 上限){ 控.abort(); }
    }, 5000);
    fetch(后端址() + "/api/river/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ 消息: 文 }),
      signal: 控.signal
    })
    .then(function(r){
      if (!r.ok || !r.body){ throw new Error("HTTP " + r.status); }
      var 读 = r.body.getReader();
      var 解码 = new TextDecoder("utf-8");
      var 缓冲 = "";
      function 泵(){
        return 读.read().then(function(果){
          局.活动 = Date.now();               // 有响应即刷新活动时间
          if (果.done){ 派接待尾(局); return; }
          缓冲 += 解码.decode(果.value, { stream: true });
          var 帧们 = 缓冲.split("\n\n");
          缓冲 = 帧们.pop() || "";
          帧们.forEach(function(帧){
            var 数据 = "";
            帧.split("\n").forEach(function(行){
              if (行.indexOf("data:") === 0){ 数据 += 行.slice(5).trim(); }
            });
            if (!数据){ return; }
            try {
              var ev = JSON.parse(数据);       // 长河事件直通：{"类型":...,...}
              if (ev.类型 === "回复增量"){ 局.出过内容 = true; }
              if (ev.类型 === "运行结束"){ 局.收过尾 = true; }
              派(ev);
            } catch(_){}
          });
          return 泵();
        });
      }
      return 泵();
    })
    .catch(function(){
      停看门狗(局);
      if (局.出过内容){ 派接待尾(局); return; } // 已有部分内容：只收尾不重发
      降级同步(文, 局);
    });
  }

  function 停看门狗(局){
    if (局.看门狗){ clearInterval(局.看门狗); 局.看门狗 = null; }
  }

  function 派接待尾(局){
    停看门狗(局);
    河().置连接("空闲");
    if (局.收过尾){ return; }
    派({ 类型: "回复结束", 消息id: "接待" });
    派({ 类型: "运行结束", 阶段: "接待中", 任务id: null });
  }

  // ── 同步降级：POST /api/dev/chat（river 流式不可用时兜底）──
  function 降级同步(文, 局){
    停看门狗(局);
    河().置连接("接待中");
    fetch(后端址() + "/api/dev/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ 消息: 文 })
    })
    .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
    .then(function(d){
      派({ 类型: "己言", 消息id: "己-" + Date.now(), 文: 文 });
      派({ 类型: "回复开始", 消息id: "接待", 角色: "道祖" });
      派({ 类型: "回复增量", 消息id: "接待", 文: d.回复 || "" });
      派({ 类型: "回复结束", 消息id: "接待" });
      派({ 类型: "运行结束", 阶段: d.阶段 || "接待中", 任务id: (d.任务id == null ? null : d.任务id) });
      局.收过尾 = true;
      河().置连接("空闲");
    })
    .catch(function(){
      局.收过尾 = true;
      河().置连接("空闲");
      派({ 类型: "河错", 来源: "道祖", 文: "道祖暂未在线（需后端 run_dev_agent=true 且配置 LLM 密钥）" });
    });
  }

  // ── 过程流：GET /api/river/stream（后端长河事件直通，EventSource 原生重连）──
  function 订过程(){
    if (typeof EventSource === "undefined"){ return; }
    var 源 = new EventSource(后端址() + "/api/river/stream");
    源.onmessage = function(e){
      try { 派(JSON.parse(e.data)); } catch(_){}
    };
    源.onerror = function(){
      派({ 类型: "河错", 来源: "过程流", 文: "过程流中断，等待自动重连" });
    };
  }

  // ── 任务态轮询：/api/board/{id}，终态自停（river 阶段完成推送为补充，此处补全中间态）──
  function 轮任务(任务id){
    var key = String(任务id);
    if (任务计时[key]){ return; }
    任务计时[key] = setInterval(function(){
      fetch(后端址() + "/api/board/" + encodeURIComponent(key))
        .then(function(r){ return r.ok ? r.json() : null; })
        .then(function(t){
          if (!t){ return; }
          var 态 = t.status || t.状态 || ""; // ← 双名回退隔离点
          派({ 类型: "任务状态", 任务id: Number(key), 状态: 态 });
          if (态 === "已完成" || 态 === "清理完成"){ 停任务轮(key); }
        })
        .catch(function(){});
    }, 3000);
  }

  function 停任务轮(key){
    if (任务计时[key]){ clearInterval(任务计时[key]); delete 任务计时[key]; }
  }

  // ── 回放：GET /api/dev/sessions/{id} → 长河事件（与实时同管线）──
  function 回放(会话id){
    return fetch(后端址() + "/api/dev/sessions/" + encodeURIComponent(会话id))
      .then(function(r){ return r.ok ? r.json() : { 事件: [] }; })
      .then(function(d){
        (d.事件 || []).forEach(function(ev, i){
          var id = "回放-" + i;
          if (ev.类型 === "任务答复"){
            派({ 类型: "回复开始", 消息id: id, 角色: ev.角色 || "道祖" });
            派({ 类型: "回复增量", 消息id: id, 文: ev.内容 || "" });
            派({ 类型: "回复结束", 消息id: id });
          } else if (ev.类型 === "思考"){
            派({ 类型: "思考开始", 消息id: id });
            派({ 类型: "思考增量", 消息id: id, 文: ev.内容 || "" });
            派({ 类型: "思考结束", 消息id: id });
          } else if (ev.类型 === "工具结果"){
            派({ 类型: "工具开始", 调用id: id, 名称: "工具" });
            派({ 类型: "工具结果", 调用id: id, 文: ev.内容 || "" });
          }
        });
        河().置连接("空闲");
      })
      .catch(function(){ /* 静默降级：镜面保持可见 */ });
  }

  // ── 自动回放最新会话（解决 API 发布任务后镜面空白）──
  function 最新会话回放(){
    return fetch(后端址() + "/api/dev/sessions")
      .then(function(r){ return r.ok ? r.json() : null; })
      .then(function(d){
        var 列表 = d && (d.会话 || d.sessions || []); // ← 双名回退隔离点
        if (!列表 || !列表.length){ return; }
        var 最新 = 列表[列表.length - 1];
        var sid = 最新.会话id || 最新.id;             // ← 双名回退隔离点
        if (sid){ window.水镜镜面.当前会话id = sid; return 回放(sid); }
      })
      .catch(function(){});
  }

  // ── 接入：过程流 + 运行收尾自动挂任务轮 + 清河清理 ──
  function 接入(){
    订过程();
    河().订事件(function(ev){
      if (ev.类型 === "运行结束" && ev.任务id != null){ 轮任务(ev.任务id); }
      if (ev.类型 === "清河"){
        Object.keys(任务计时).forEach(停任务轮);
      }
    });
  }

  return { 发消息: 发消息, 订过程: 订过程, 轮任务: 轮任务, 回放: 回放, 最新会话回放: 最新会话回放, 接入: 接入 };
})();
