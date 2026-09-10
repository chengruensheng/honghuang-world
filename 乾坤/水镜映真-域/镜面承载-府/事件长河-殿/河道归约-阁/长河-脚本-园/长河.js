/* ═══════════════════════════════════════════════════════════
   水镜 · 事件长河（镜面端核心）
   规约：界面 = 对长河事件的纯归约；镜不藏水（组件不私藏业务状态）。
   本文件是 词表.rs 归约的 JS 镜像同构实现（规格 = 设计稿 §4.3）。
   订户两类：状态订户（收归约后视图，渲染结构）；
             事件订户（收原始事件，流式内容直通上屏）。
   ═══════════════════════════════════════════════════════════ */
window.水镜长河 = (function(){
  "use strict";

  var 状态 = 空河();
  var 状态订户 = [];
  var 事件订户 = [];

  function 空河(){
    return { 阶段: "空闲", 消息: [], 任务: {}, 过程游标: 0, 连接: "离线" };
  }

  // ── 归约（镜像 词表.rs）：原地更新，单写者 = 长河 ──
  function 找消息(id){
    for (var i = 0; i < 状态.消息.length; i++){
      if (状态.消息[i].消息id === id){ return 状态.消息[i]; }
    }
    return null;
  }

  function 开消息(id, 向, 角色){
    var m = 找消息(id);
    if (!m){
      m = { 消息id: id, 方向: 向, 角色: 角色, 段: [], 在流: false };
      状态.消息.push(m);
    }
    return m;
  }

  function 尾工具段(m){
    for (var i = m.段.length - 1; i >= 0; i--){
      if (m.段[i].类 === "工具"){ return m.段[i]; }
    }
    return null;
  }

  // 追段文：有对应在流尾段则续写，否则新开段（与 Rust 追段文 同构）
  function 追段文(id, 类, 文){
    var m = 找消息(id);
    if (!m || !文){ return; }
    var 尾 = m.段.length ? m.段[m.段.length - 1] : null;
    if (尾 && 尾.类 === 类 && (类 !== "正文" || m.在流)){
      尾.文 += 文;
    } else {
      m.段.push({ 类: 类, 文: 文 });
    }
  }

  function 归约(ev){
    var s = 状态;
    switch (ev.类型){
      case "回复开始": {
        var m = 开消息(ev.消息id, "对方", ev.角色 || "道祖");
        m.在流 = true;
        break;
      }
      case "回复增量":
        追段文(ev.消息id, "正文", ev.文 || "");
        break;
      case "回复结束": {
        var m1 = 找消息(ev.消息id);
        if (m1){ m1.在流 = false; }
        break;
      }
      case "思考开始": {
        var m2 = 开消息(ev.消息id, "对方", "道祖");
        m2.在流 = true;
        var 有 = m2.段.some(function(x){ return x.类 === "思考"; });
        if (!有){ m2.段.push({ 类: "思考", 文: "" }); }
        break;
      }
      case "思考增量":
        追段文(ev.消息id, "思考", ev.文 || "");
        break;
      case "思考结束": {
        var m3 = 找消息(ev.消息id);
        if (m3){ m3.在流 = false; }
        break;
      }
      case "工具开始": {
        var m4 = 开消息(ev.调用id, "对方", "道祖");
        m4.在流 = true;
        m4.段.push({ 类: "工具", 名称: ev.名称 || "工具", 参数: "", 结果: "" });
        break;
      }
      case "工具参数": {
        var m5 = 找消息(ev.调用id);
        var 段5 = m5 && 尾工具段(m5);
        if (段5){ 段5.参数 += (ev.片段 || ""); }
        break;
      }
      case "工具结束":
        break;
      case "工具结果": {
        var m6 = 找消息(ev.调用id);
        var 段6 = m6 && 尾工具段(m6);
        if (段6){ 段6.结果 = ev.文 || ""; }
        break;
      }
      case "运行结束": {
        s.阶段 = ev.阶段 || "接待中";
        if (ev.任务id != null){ 保任务(ev.任务id); }
        break;
      }
      case "任务状态": {
        var t = 保任务(ev.任务id);
        t.状态 = ev.状态 || "";
        break;
      }
      case "过程事件": {
        if (ev.序号 > s.过程游标){
          s.过程游标 = ev.序号;
          var t2 = 保任务(ev.任务id);
          t2.过程.push({ 序号: ev.序号, 类别: ev.类别 || "", 角色: ev.角色 || "", 工具名: ev.工具名 || null, 内容: ev.内容 || "" });
        }
        break;
      }
      case "河错":
        // 结构不变；错误经事件订户由显影层呈现
        break;
      case "清河": {
        var 净 = 空河();
        s.阶段 = 净.阶段; s.消息 = 净.消息; s.任务 = 净.任务; s.过程游标 = 0;
        break; // 连接态保留
      }
      default:
        break;
    }
  }

  function 保任务(id){
    var key = String(id);
    if (!s任务(key)){ 状态.任务[key] = { 状态: "", 过程: [] }; }
    return 状态.任务[key];
  }
  function s任务(key){ return Object.prototype.hasOwnProperty.call(状态.任务, key); }

  // ── 订户与派发 ──
  function 派发(ev){
    if (!ev || !ev.类型){ return; }
    归约(ev);
    var i;
    for (i = 0; i < 事件订户.length; i++){ 事件订户[i](ev); }
    for (i = 0; i < 状态订户.length; i++){ 状态订户[i](状态); }
  }

  function 订状态(fn){ 状态订户.push(fn); fn(状态); }
  function 订事件(fn){ 事件订户.push(fn); }
  function 当前(){ return 状态; }

  function 置连接(态){
    if (状态.连接 !== 态){
      状态.连接 = 态;
      状态订户.forEach(function(fn){ fn(状态); });
    }
  }

  return { 派发: 派发, 订状态: 订状态, 订事件: 订事件, 当前: 当前, 置连接: 置连接 };
})();
