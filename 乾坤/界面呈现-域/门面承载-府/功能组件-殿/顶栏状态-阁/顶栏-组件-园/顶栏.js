/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 顶栏 实现
   职责：构建顶栏节点（logo · 工作区选择器 · 模型标识 · 状态徽标），
   经挂载契约进入顶栏槽位；工作区选择器可切换项目工作区根，模型标识接真实 LLM。
   ═══════════════════════════════════════════════════════════ */
window.乾坤顶栏 = (function(){
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }

  function 建(){
    var 顶 = document.createElement("header");
    顶.className = "组件-顶栏";

    // logo
    var 标 = document.createElement("span");
    标.className = "顶栏-标";
    标.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l3 5 5 1-4 4 1 6-5-3-5 3 1-6-4-4 5-1z"/></svg>洪荒';
    顶.appendChild(标);

    // ── 工作区选择器：定义项目工作区根（文件面板/智能体产出都基于它）──
    顶.appendChild(工作区选择器());

    // 右侧区
    var 右 = document.createElement("div");
    右.className = "顶栏-右";

    // 模型标识（真实 LLM）
    var 模型 = document.createElement("span");
    模型.className = "顶栏-模型";
    模型.innerHTML = '<i class="点"></i><b>…</b>';
    右.appendChild(模型);
    fetch(后端() + "/api/llm/status")
      .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
      .then(function(d){ 模型.querySelector("b").textContent = (d && d.当前选择 && d.当前选择.模型) || "未接入"; })
      .catch(function(){ 模型.querySelector("b").textContent = "未接入"; });

    // 状态徽标
    var 徽 = document.createElement("span");
    徽.className = "顶栏-徽";
    徽.innerHTML = '<i class="点"></i>先审后写';
    右.appendChild(徽);

    顶.appendChild(右);
    return 顶;
  }

  // ── 工作区选择器：按钮 + 内联输入面板 ──
  function 工作区选择器(){
    var 外 = document.createElement("div");
    外.className = "顶栏-工作区";

    var 钮 = document.createElement("button");
    钮.className = "顶栏-工作区-钮";
    钮.type = "button";
    钮.innerHTML =
      '<svg class="夹" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>' +
      '<span class="路">…</span>' +
      '<svg class="下" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 9l6 6 6-6"/></svg>';

    // 输入面板（默认隐藏）
    var 面 = document.createElement("div");
    面.className = "顶栏-工作区-面";
    面.innerHTML =
      '<div class="题">项目工作区根目录</div>' +
      '<input class="入" placeholder="例如 F:\\洪荒 - 世界 或 ./项目" />' +
      '<div class="行">' +
        '<button class="定" type="button">确定</button>' +
        '<button class="消" type="button">取消</button>' +
      '</div>';
    外.appendChild(钮);
    外.appendChild(面);

    var 入 = 面.querySelector(".入");
    var 路 = 钮.querySelector(".路");

    // 打开面板：预填当前值
    function 开(){
      fetch(后端() + "/api/workspace")
        .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
        .then(function(d){ 入.value = d.工作区 || ""; })
        .catch(function(){ 入.value = ""; });
      面.classList.add("开");
      入.focus();
    }
    function 关(){ 面.classList.remove("开"); }

    钮.addEventListener("click", function(){ 面.classList.contains("开") ? 关() : 开(); });
    面.querySelector(".消").addEventListener("click", 关);
    document.addEventListener("keydown", function(e){ if (e.key === "Escape"){ 关(); } });
    document.addEventListener("click", function(e){
      if (!外.contains(e.target)){ 关(); }
    });

    // 确定：POST 设置工作区
    面.querySelector(".定").addEventListener("click", function(){
      var 路径 = 入.value.trim();
      if (!路径){ 入.focus(); return; }
      fetch(后端() + "/api/workspace", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ 工作区: 路径 })
      })
      .then(function(r){ if (!r.ok){ return r.text().then(function(t){ throw new Error(t); }); } return r.json(); })
      .then(function(d){
        路.textContent = d.工作区 || 路径;
        路.title = d.工作区 || 路径;
        关();
      })
      .catch(function(e){
        路.textContent = "目录无效";
        setTimeout(function(){ 载入当前(); }, 1200);
      });
    });

    // 初始载入当前工作区
    载入当前();
    function 载入当前(){
      fetch(后端() + "/api/workspace")
        .then(function(r){ if (!r.ok){ throw new Error(); } return r.json(); })
        .then(function(d){ 路.textContent = d.工作区 || "./"; 路.title = d.工作区 || "./"; })
        .catch(function(){ 路.textContent = "./"; });
    }

    return 外;
  }

  return { 建: 建 };
})();

// 经挂载契约入园
乾坤界面.挂载("顶栏", 乾坤顶栏.建);
