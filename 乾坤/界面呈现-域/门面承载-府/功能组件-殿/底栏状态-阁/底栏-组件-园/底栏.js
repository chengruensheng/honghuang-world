/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 底栏 实现
   职责：构建底栏状态栏节点（引擎在线 · 记忆 · 图谱 · 规则 · 事件 · 模型 · 版本），
   经挂载契约进入底栏槽位；计数与模型/版本来自真实接口，失败降级不阻塞。
   ═══════════════════════════════════════════════════════════ */
window.乾坤底栏 = (function(){
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }

  function 建(){
    var 底 = document.createElement("footer");
    底.className = "组件-底栏";

    // ── 左区：引擎在线 + 引擎计数 ──
    底.appendChild(项("<i class=\"点\"></i> 主引擎在线", "底栏-在线"));
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

    // ── 拉取真实数据 ──
    刷新({ 记: 记.querySelector("b"), 谱: 谱.querySelector("b"), 规: 规.querySelector("b"), 事: 事.querySelector("b"), 模: 模型.querySelector("b"), 版: 版 });

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
    数("/api/rules", "规", function(d){ return (d.length || 0) + " 条"; });
    数("/api/events", "事", function(d){ return (d.length || 0) + " 条"; });

    fetch(后端() + "/api/llm/status")
      .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
      .then(function(d){ 槽.模.textContent = (d && d.当前选择 && d.当前选择.模型) || "未接入"; })
      .catch(function(){ 槽.模.textContent = "未接入"; });

    fetch(后端() + "/api/iterations/version")
      .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
      .then(function(d){ 槽.版.textContent = "v" + (d.主 || 0) + "." + (d.次 || 0) + "." + (d.修订 || 0); })
      .catch(function(){ 槽.版.textContent = "v—"; });
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
