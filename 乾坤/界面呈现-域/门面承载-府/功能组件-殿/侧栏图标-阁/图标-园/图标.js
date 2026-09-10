/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 侧栏图标轨 实现
   职责：
   1. 数据驱动地构建「图标 + 小字」按钮；
   2. 只经 乾坤界面.挂载("左栏", 节点) 入园，绝不改写布局；
   3. 点击派发「乾坤图标」事件（组件间经事件解耦）；
   4. 监听「乾坤图标复位」事件：设置面板关闭/抽屉收起时清除对应高亮。
   ═══════════════════════════════════════════════════════════ */
window.乾坤组件 = (function(){
  // ── 图标 SVG 内联（沿用参考设计语义）──
  var S = {
    对话:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>',
    看板:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="18" rx="1.5"/><rect x="14" y="3" width="7" height="11" rx="1.5"/></svg>',
    记忆:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="8" ry="2.5"/><path d="M4 5v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5V5"/><path d="M4 11v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5v-6"/></svg>',
    传承殿:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M3 21h18M5 21V8l7-5 7 5v13"/><path d="M9 21v-6h6v6M9 11h.01M15 11h.01"/></svg>',
    门禁:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/><path d="M9 12l2 2 4-4.5"/></svg>',
    架构:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="5" r="2.2"/><circle cx="5" cy="12" r="2.2"/><circle cx="19" cy="12" r="2.2"/><circle cx="12" cy="19" r="2.2"/><path d="M12 7.2v3M7 13.5L10 17M17 13.5L14 17M10.5 5H7M13.5 5H17"/></svg>',
    图谱:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="6" r="2.4"/><circle cx="18" cy="6" r="2.4"/><circle cx="12" cy="18" r="2.4"/><path d="M7.6 7.6L11 16M16.4 7.6L13 16"/></svg>',
    设置:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>'
  };

  // ── 左侧导航：对话/看板为主区双视图，其余为抽屉视图与设置 ──
  var 左导航 = [
    { 键:"对话",   名:"对话",   图:S.对话 },
    { 键:"看板",   名:"看板",   图:S.看板 },
    { 键:"记忆",   名:"记忆",   图:S.记忆 },
    { 键:"传承殿", 名:"传承",   图:S.传承殿 },
    { 键:"门禁",   名:"门禁",   图:S.门禁 },
    { 键:"架构",   名:"架构",   图:S.架构 },
    { 键:"图谱",   名:"图谱",   图:S.图谱 },
    { 键:"设置",   名:"设置",   图:S.设置, 末:true }
  ];

  // ── 构建图标轨 ──
  function 建轨(数据, 默认键){
    var 轨 = document.createElement("div");
    轨.className = "组件-图标轨";
    数据.forEach(function(项){
      var 钮 = document.createElement("button");
      钮.className = "组件-图标" + (项.末 ? " 组件-末" : "");
      钮.setAttribute("data-键", 项.键 || "末");
      钮.setAttribute("title", 项.名 || 项.键);
      钮.innerHTML = (项.图 || S.设置) + "<em>" + (项.名 || "设置") + "</em>";
      if (项.键 === 默认键){
        钮.classList.add("是");
      }
      钮.addEventListener("click", function(){
        轨.querySelectorAll(".组件-图标").forEach(function(x){ x.classList.remove("是"); });
        钮.classList.add("是");
        // 派发图标选择事件，供对话/看板/视图/设置等组件监听（组件间经事件解耦）
        document.dispatchEvent(new CustomEvent("乾坤图标", { detail: { 键: 项.键 } }));
      });
      轨.appendChild(钮);
    });
    // 监听复位事件：面板关闭/抽屉收起时清除对应键的高亮
    document.addEventListener("乾坤图标复位", function(e){
      var 键 = e.detail && e.detail.键;
      if (!键){ return; }
      var 钮 = 轨.querySelector('.组件-图标[data-键="' + 键 + '"]');
      if (钮){ 钮.classList.remove("是"); }
    });
    return 轨;
  }

  // ── 对外：只提供「构建节点」的工厂，经挂载契约入园 ──
  return {
    左: function(){ return 建轨(左导航, "对话"); }
  };
})();

// ── 经挂载契约入园（布局与组件的唯一接口）──
乾坤界面.挂载("左栏", 乾坤组件.左);
