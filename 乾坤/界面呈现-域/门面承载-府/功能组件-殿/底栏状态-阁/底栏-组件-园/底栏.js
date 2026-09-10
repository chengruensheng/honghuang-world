/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 底栏 实现
   职责：构建底栏状态栏节点（引擎在线 · 记忆 · 图谱 · 规则 · 事件 · 模型 · 版本），
   经挂载契约进入底栏槽位；计数与模型/版本来自真实接口，失败降级不阻塞。
   ═══════════════════════════════════════════════════════════ */
window.乾坤底栏 = (function(){
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }

  // 模块级槽位引用（供事件监听器和定时器使用，避免闭包泄漏）
  var 槽引用 = null;
  var 在线节点 = null;

  function 建(){
    var 底 = document.createElement("footer");
    底.className = "组件-底栏";

    // ── 左区：引擎在线 + 引擎计数 ──
    var 在线项 = 项("<i class=\"点\"></i> <span class=\"在线文\">主引擎在线</span>", "底栏-在线");
    在线节点 = 在线项;
    底.appendChild(在线项);
    底.appendChild(点());
    var 记 = 项("记忆 <b class=\"记\">…</b>");
    var 谱 = 项("图谱 <b class=\"谱\">…</b>");
    var 规 = 项("规则 <b class=\"规\">…</b>");
    var 事 = 项("事件 <b class=\"事\">…</b>");
    底.appendChild(记); 底.appendChild(点());
    底.appendChild(谱); 底.appendChild(点());
    底.appendChild(规); 底.appendChild(点());
    底.appendChild(事);

    // ── 右区：模型 + 版本 ──
    var 右 = document.createElement("div");
    右.className = "底栏-右";
    var 模型 = document.createElement("span");
    模型.className = "底栏-模型";
    模型.innerHTML = "<i class=\"点\"></i><b class=\"模\">…</b>";
    右.appendChild(模型);
    右.appendChild(点());
    var 版 = document.createElement("span");
    版.className = "底栏-版";
    版.textContent = "v…";
    右.appendChild(版);
    底.appendChild(右);

    槽引用 = { 记: 记.querySelector("b"), 谱: 谱.querySelector("b"), 规: 规.querySelector("b"), 事: 事.querySelector("b"), 模: 模型.querySelector("b"), 版: 版 };

    // ── 拉取真实数据 ──
    刷新(槽引用);

    // ── 设置面板关闭事件监听（只注册一次，在IIFE顶层）──
    document.addEventListener("乾坤图标复位", function(e){
      if (e.detail && e.detail.键 === "设置" && 槽引用){
        fetch(后端() + "/api/llm/status")
          .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
          .then(function(d){ 槽引用.模.textContent = (d && d.当前选择 && d.当前选择.模型) || "未接入"; })
          .catch(function(){});
      }
    });

    // ── 定时刷新：每30秒更新计数（保持数据新鲜）──
    setInterval(function(){
      if (槽引用){ 刷新(槽引用); }
    }, 30000);

    return 底;
  }

  // ── 并发拉取记忆/图谱/规则/事件计数 + 模型 + 版本 ──
  function 刷新(槽){
    function 数(url, 键, 取值){
      fetch(后端() + url)
        .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
        .then(function(d){ 槽[键].textContent = 取值(d); })
        .catch(function(){ 槽[键].textContent = "—"; });
    }
    数("/api/memories", "记", function(d){ return (d.length || 0) + " 条"; });
    数("/api/cognition/graph", "谱", function(d){ return (d.符号集 || []).length + " 符号"; });

    // 规则计数 = RuleSet + 心智格位，用 Promise.allSettled 消除竞态闪烁
    Promise.allSettled([
      fetch(后端() + "/api/rules").then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); }),
      fetch(后端() + "/api/cognition/cells").then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
    ]).then(function(结果组){
      var rs = 0, 格位 = 0;
      // rules
      if (结果组[0].status === "fulfilled"){
        var rd = 结果组[0].value;
        rs = rd.length || 0;
      }
      // cells
      if (结果组[1].status === "fulfilled"){
        var cd = 结果组[1].value;
        var 全部 = cd.格位集 || cd.全部 || cd.cells || [];
        for (var i = 0; i < 全部.length; i++){
          var g = 全部[i];
          if ((g.维度 === "规则" || g.dimension === "规则") && g.摘要 && g.摘要.length > 0){
            格位++;
          }
        }
      }
      槽.规.textContent = (rs + 格位) + " 条";
    });

    数("/api/events", "事", function(d){ return (d.length || 0) + " 条"; });

    // 模型状态 + 引擎在线指示器联动
    fetch(后端() + "/api/llm/status")
      .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
      .then(function(d){
        槽.模.textContent = (d && d.当前选择 && d.当前选择.模型) || "未接入";
        设置在线态(true);
      })
      .catch(function(){
        槽.模.textContent = "未接入";
        设置在线态(false);
      });

    fetch(后端() + "/api/iterations/version")
      .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
      .then(function(d){ 槽.版.textContent = "v" + (d.主 || 0) + "." + (d.次 || 0) + "." + (d.修订 || 0); })
      .catch(function(){ 槽.版.textContent = "v—"; });
  }

  // ── 引擎在线状态动态更新 ──
  function 设置在线态(在线){
    if (!在线节点){ return; }
    var 文 = 在线节点.querySelector(".在线文");
    var 点 = 在线节点.querySelector(".点");
    if (文){ 文.textContent = 在线 ? "主引擎在线" : "引擎离线"; }
    if (点){
      if (在线){ 点.classList.remove("死"); } else { 点.classList.add("死"); }
    }
  }

  function 项(html, 类){
    var s = document.createElement("span");
    s.className = 类 ? "底栏-项 " + 类 : "底栏-项";
    s.innerHTML = html;
    return s;
  }

  function 点(){
    var s = document.createElement("span");
    s.className = "底栏-点";
    s.textContent = "·";
    return s;
  }

  return { 建: 建 };
})();

// 经挂载契约入园
乾坤界面.挂载("底栏", 乾坤底栏.建);
