/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 设置面板 实现
   职责：
   1. 监听「乾坤图标」事件（键=设置）→ 展开/收起；关闭时发「乾坤图标复位」清高亮；
   2. 构建居中设置面板，内含 模型（真实接口）+ 智能体（独立模型绑定）；
   3. 只读写自身 DOM，不触碰布局结构；经事件与图标轨解耦。
   ═══════════════════════════════════════════════════════════ */
window.乾坤设置 = (function(){
  // ── 后端地址：由页面模板注入 window.乾坤配置；缺省同源 ──
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }
  // ── 文本转义：接口数据进 DOM 前一律过一遍 ──
  function 名(s){
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  // ── 构建面板 DOM ──
  function 构建(){
    var 托盘 = document.createElement("div");
    托盘.className = "设置-托盘";

    var 面板 = document.createElement("div");
    面板.className = "设置-面板";

    // 头部
    var 头 = document.createElement("div");
    头.className = "设置-头";
    头.innerHTML = "<b>设置</b><span class=\"设置-副\">道基 · 模型与守卫</span>";
    var 关 = document.createElement("button");
    关.className = "设置-关";
    关.setAttribute("aria-label", "关闭设置");
    关.innerHTML = "<svg viewBox=\"0 0 24 24\" width=\"16\" height=\"16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\"><path d=\"M6 6l12 12M18 6L6 18\"/></svg>";
    头.appendChild(关);
    面板.appendChild(头);

    // ── 模型区块（接真实 /api/llm/status · /models · /select）──
    var 模型 = 模型区块();
    面板.appendChild(模型.块);

    // ── 智能体区块（每身份独立模型绑定，真实接口）──
    var 智块 = 区块("智能体");
    智块.appendChild(乾坤接入.智能体区块());
    面板.appendChild(智块);

    托盘.appendChild(面板);

    // ── 交互：关闭（三路关闭统一复位图标高亮）──
    function 收(){
      托盘.classList.remove("开");
      document.dispatchEvent(new CustomEvent("乾坤图标复位", { detail: { 键: "设置" } }));
    }
    关.addEventListener("click", 收);
    托盘.addEventListener("click", function(e){ if (e.target === 托盘){ 收(); } });
    document.addEventListener("keydown", function(e){ if (e.key === "Escape" && 托盘.classList.contains("开")){ 收(); } });

    return { 托盘: 托盘, 刷新模型: 模型.刷新 };
  }

  // ── 模型区块：当前选择 + 供应商模型清单 + 点击切换 + 密钥配置 + 接入向导 ──
  function 模型区块(){
    var 块 = 区块("模型");
    var 当前 = document.createElement("div");
    当前.className = "设置-当前模型";
    当前.innerHTML = "<span class=\"点\"></span><b>未加载</b>";
    var 列表 = document.createElement("div");
    列表.className = "设置-模型列表";
    块.appendChild(当前);
    块.appendChild(列表);
    // 接入入口：展开接入向导（模板/自定义 → 密钥 → 获取模型 → 接入）
    块.appendChild(乾坤接入.接入入口(块, 刷新));

    var 现选 = { 供应商: "", 模型: "" };
    var 切换中 = false; // 防并发切换锁
    // 供应商信息表（名称 → {名称/地址/模型}，密钥行保存时回填 connect 参数）
    var 供应商信息表 = {};

    function 画当前(d){
      if (d && d.配置 && d.当前选择){
        现选.供应商 = d.当前选择.供应商 || "";
        现选.模型 = d.当前选择.模型 || "";
        当前.innerHTML = "<span class=\"点 活\"></span><b>" + 名(现选.模型) + "</b><em>" + 名(现选.供应商) + " · 已接入</em>";
      } else {
        当前.innerHTML = "<span class=\"点 死\"></span><b>LLM 池未装配</b><em>请配置供应商密钥</em>";
      }
      // 供应商信息表（供密钥行回填 connect 参数）
      供应商信息表 = {};
      ((d && d.供应商) || []).forEach(function(s){ 供应商信息表[s.名称] = s; });
    }
    function 画列表(结果){
      列表.innerHTML = "";
      (结果 || []).forEach(function(项){
        var 组 = document.createElement("div");
        组.className = "设置-供应商组";
        var 题 = document.createElement("h6");
        题.innerHTML = "<span>" + 名(项.供应商) + "</span>";
        // 组头密钥按钮：展开密钥行（discover 校验 + connect 保存）
        var 信息 = 供应商信息表[项.供应商];
        if (信息){ 题.appendChild(乾坤接入.密钥按钮(信息, 组, 刷新)); }
        组.appendChild(题);
        if (项.模型 && 项.模型.length){
          项.模型.forEach(function(m){
            var b = document.createElement("button");
            b.className = "设置-模型项" + (项.供应商 === 现选.供应商 && m.id === 现选.模型 ? " 当前" : "");
            b.textContent = m.id;
            b.addEventListener("click", function(){ 选择(项.供应商, m.id); });
            组.appendChild(b);
          });
        } else {
          var 错 = document.createElement("span");
          错.className = "设置-模型错误";
          错.textContent = 项.错误 || "无可用模型";
          组.appendChild(错);
        }
        列表.appendChild(组);
      });
    }
    function 选择(供应商, 模型){
      if (切换中){ return; }
      切换中 = true;
      fetch(后端() + "/api/llm/select", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ 供应商: 供应商, 模型: 模型 })
      })
      .then(function(r){
        if (!r.ok){ throw new Error("HTTP " + r.status); }
        return r.json();
      })
      .then(function(){ 现选 = { 供应商: 供应商, 模型: 模型 }; 刷新(); })
      .catch(function(){
        当前.innerHTML = "<span class=\"点 死\"></span><b>切换失败</b><em>" + 名(供应商) + " · " + 名(模型) + "</em>";
        // 清除列表中所有高亮，避免与顶部状态矛盾
        var 按钮们 = 列表.querySelectorAll(".设置-模型项.当前");
        for (var i = 0; i < 按钮们.length; i++){ 按钮们[i].classList.remove("当前"); }
      })
      .finally(function(){ 切换中 = false; });
    }
    var 刷新中 = false; // 防并发刷新锁
    function 刷新(){
      if (刷新中){ return; }
      刷新中 = true;
      // 先 status（填供应商信息表）再 models（画清单挂密钥按钮）——串行消除竞态
      fetch(后端() + "/api/llm/status")
        .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
        .then(function(d){ 画当前(d); })
        .catch(function(){
          当前.innerHTML = "<span class=\"点 死\"></span><b>数据服务未在线</b><em>需启动后端 127.0.0.1:8321</em>";
        })
        .then(function(){
          fetch(后端() + "/api/llm/models")
            .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
            .then(function(d){ 画列表(d.结果); })
            .catch(function(){
              列表.innerHTML = "<span class=\"设置-模型错误\">模型清单拉取失败（供应商端点不可达）</span>";
            });
        })
        .finally(function(){ 刷新中 = false; });
    }

    return { 块: 块, 刷新: 刷新 };
  }

  // ── 新区块 ──
  function 区块(名){
    var 块 = document.createElement("section");
    块.className = "设置-区块";
    var h = document.createElement("h5");
    h.textContent = 名;
    块.appendChild(h);
    return 块;
  }

  // 托盘实例化后挂到 body（覆盖层），初始收起
  var 实例 = 构建();
  var 托盘 = 实例.托盘;
  document.body.appendChild(托盘);

  // ── 监听图标事件：键=设置 → 展开/收起；展开时拉取真实模型状态，收起时复位图标高亮 ──
  document.addEventListener("乾坤图标", function(e){
    if (e.detail && e.detail.键 === "设置"){
      var 开 = 托盘.classList.toggle("开");
      if (开){ 实例.刷新模型(); }
      else { document.dispatchEvent(new CustomEvent("乾坤图标复位", { detail: { 键: "设置" } })); }
    }
  });

  return { 托盘: 托盘 };
})();
