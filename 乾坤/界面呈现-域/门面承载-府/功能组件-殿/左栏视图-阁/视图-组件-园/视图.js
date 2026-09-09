/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 左栏视图 实现
   职责：
   1. 监听「乾坤图标」事件：键=记忆/图谱 → 左抽屉展开并拉取真实数据；键=对话 → 收起；
   2. 记忆：GET /api/memories → 列表（内容 · 标签 · 归档徽标 · 时间）；
   3. 图谱：GET /api/cognition/graph → 概览计数 + 按所属模块聚合 + 节点清单（前 60）；
   4. 只读写自身 DOM；数据一律来自 fetch，失败可见降级。
   ═══════════════════════════════════════════════════════════ */
window.乾坤视图 = (function(){
  // ── 后端地址：由页面模板注入 window.乾坤配置；缺省同源 ──
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }
  function 转(s){
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
  function 时戳(秒){
    if (!秒){ return ""; }
    var d = new Date(秒 * 1000);
    function p(n){ return (n < 10 ? "0" : "") + n; }
    return d.getFullYear() + "-" + p(d.getMonth() + 1) + "-" + p(d.getDate()) + " " + p(d.getHours()) + ":" + p(d.getMinutes());
  }

  // ── 抽屉骨架 ──
  var 抽屉 = document.createElement("aside");
  抽屉.className = "视图-抽屉";
  抽屉.innerHTML =
    '<header class="视图-头"><b class="视图-题">视图</b>' +
    '<span class="视图-数"></span>' +
    '<button class="视图-关" aria-label="关闭视图"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg></button></header>' +
    '<div class="视图-体"></div>';
  document.body.appendChild(抽屉);

  // ── 阅读覆盖层：点开文件后全屏预览内容 ──
  var 阅读 = document.createElement("div");
  阅读.className = "视图-阅读";
  阅读.innerHTML =
    '<div class="视图-阅读-面">' +
    '<header class="视图-阅读-头"><b class="视图-阅读-题">预览</b>' +
    '<button class="视图-阅读-关" aria-label="关闭预览"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg></button></header>' +
    '<pre class="视图-阅读-体"></pre></div>';
  document.body.appendChild(阅读);
  var 阅读题 = 阅读.querySelector(".视图-阅读-题");
  var 阅读体 = 阅读.querySelector(".视图-阅读-体");
  阅读.querySelector(".视图-阅读-关").addEventListener("click", 关阅读);
  阅读.addEventListener("click", function(e){ if (e.target === 阅读){ 关阅读(); } });
  document.addEventListener("keydown", function(e){ if (e.key === "Escape"){ 关阅读(); } });
  function 关阅读(){ 阅读.classList.remove("开"); }

  var 题 = 抽屉.querySelector(".视图-题");
  var 数 = 抽屉.querySelector(".视图-数");
  var 体 = 抽屉.querySelector(".视图-体");
  抽屉.querySelector(".视图-关").addEventListener("click", 收起);

  function 收起(){ 抽屉.classList.remove("开"); }
  function 打开(标题){ 题.textContent = 标题; 数.textContent = ""; 抽屉.classList.add("开"); }

  // ── 记忆视图 ──
  function 载记忆(){
    打开("记忆库");
    体.innerHTML = '<div class="视图-载">读取中…</div>';
    fetch(后端() + "/api/memories")
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(列表){
        数.textContent = (列表.length || 0) + " 条";
        if (!列表.length){
          体.innerHTML = '<div class="视图-空">记忆库为空 · 五行运转产出后自动沉淀</div>';
          return;
        }
        体.innerHTML = "";
        列表.forEach(function(m){
          var 条 = document.createElement("div");
          条.className = "视图-记忆" + (m.归档 ? " 归档" : "");
          条.innerHTML =
            '<p class="视图-记忆-内">' + 转(m.内容) + '</p>' +
            '<p class="视图-记忆-元"><span class="视图-签">' + 转(m.标签 || "无签") + '</span>' +
            (m.归档 ? '<span class="视图-徽 警">待归档</span>' : "") +
            '<em>' + 时戳(m.created_at) + '</em></p>';
          体.appendChild(条);
        });
      })
      .catch(function(){
        体.innerHTML = '<div class="视图-错">记忆库读取失败 · 需数据服务在线（127.0.0.1:8321）</div>';
      });
  }

  // ── 图谱视图：真实结构 = 符号集 / 模块集 / 依赖集 / 技术栈 ──
  function 载图谱(){
    打开("世界态图谱");
    体.innerHTML = '<div class="视图-载">扫描中…</div>';
    fetch(后端() + "/api/cognition/graph")
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(g){
        var 符号 = g.符号集 || [], 模块 = g.模块集 || [], 依赖 = g.依赖集 || [];
        数.textContent = 符号.length + " 符号 · " + 模块.length + " 模块 · " + 依赖.length + " 依赖";
        // 符号按所属模块聚合
        var 组 = {};
        符号.forEach(function(n){
          var k = n.所属模块 || "未归";
          (组[k] = 组[k] || []).push(n);
        });
        体.innerHTML = "";
        Object.keys(组).sort().forEach(function(名){
          var 块 = document.createElement("details");
          块.className = "视图-模块";
          块.innerHTML = "<summary>" + 转(名) + " <em>" + 组[名].length + "</em></summary>";
          var 限 = 60, 截 = 组[名].length > 限;
          组[名].slice(0, 限).forEach(function(n){
            var 行 = document.createElement("div");
            行.className = "视图-节点";
            行.innerHTML = '<span class="视图-种 ' + 转(n.种类 || "") + '">' + 转(n.种类 || "?") + '</span><b>' + 转(n.名称) + '</b>';
            if (n.签名){ 行.title = n.签名; }
            块.appendChild(行);
          });
          if (截){
            var 更 = document.createElement("div");
            更.className = "视图-截";
            更.textContent = "… 另有 " + (组[名].length - 限) + " 项";
            块.appendChild(更);
          }
          体.appendChild(块);
        });
        if (g.技术栈 && g.技术栈.length){
          var 栈 = document.createElement("div");
          栈.className = "视图-栈";
          栈.innerHTML = "<h6>技术栈</h6>" + g.技术栈.map(function(t){ return "<span>" + 转(t) + "</span>"; }).join("");
          体.appendChild(栈);
        }
      })
      .catch(function(){
        体.innerHTML = '<div class="视图-错">图谱读取失败 · 需数据服务在线（127.0.0.1:8321）</div>';
      });
  }

  // ── 事件路由：图标键 → 视图动作 ──
  document.addEventListener("乾坤图标", function(e){
    var 键 = e.detail && e.detail.键;
    if (键 === "记忆"){ 载记忆(); }
    else if (键 === "图谱"){ 载图谱(); }
    else if (键 === "传承殿"){ 载文件("传承殿", "传承殿"); }
    else if (键 === "门禁"){ 载文件("门禁", "门禁"); }
    else if (键 === "架构"){ 载文件("架构", "架构"); }
    else { 收起(); }
  });

  // ── 文件视图：传承殿（文档）/ 门禁（-门禁）/ 架构（架构图）──
  function 载文件(类名, 标题){
    打开(标题);
    体.innerHTML = '<div class="视图-载">读取中…</div>';
    fetch(后端() + "/api/files")
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(g){
        var 列表 = (g && g[类名]) || [];
        数.textContent = 列表.length + " 个文件";
        if (!列表.length){
          体.innerHTML = '<div class="视图-空">暂无' + 转(标题) + '文件</div>';
          return;
        }
        体.innerHTML = "";
        // 按顶层目录分组
        var 组 = {};
        列表.forEach(function(f){
          var 顶 = f.路径.split("/")[0] || "根";
          (组[顶] = 组[顶] || []).push(f);
        });
        Object.keys(组).sort().forEach(function(名){
          var 块 = document.createElement("details");
          块.className = "视图-文件组";
          块.open = true; // 默认展开，确保文件按钮直接可见可点
          块.innerHTML = "<summary>" + 转(名) + " <em>" + 组[名].length + "</em></summary>";
          组[名].forEach(function(f){
            var 行 = document.createElement("button");
            行.className = "视图-文件";
            行.innerHTML = "<b>" + 转(f.名称) + "</b><em>" + 转(f.路径) + "</em>";
            行.title = f.路径;
            行.addEventListener("click", function(){ 读文件(f.路径); });
            块.appendChild(行);
          });
          体.appendChild(块);
        });
      })
      .catch(function(){
        体.innerHTML = '<div class="视图-错">文件清单读取失败 · 需数据服务在线（127.0.0.1:8321）</div>';
      });
  }

  // ── 读取并预览单个文件内容 ──
  function 读文件(路径){
    阅读题.textContent = 路径;
    阅读体.textContent = "读取中…";
    阅读.classList.add("开");
    fetch(后端() + "/api/files/content?路径=" + encodeURIComponent(路径))
      .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function(d){ 阅读体.textContent = d.内容 || "（空文件）"; })
      .catch(function(){ 阅读体.textContent = "读取失败：文件不可达或超出大小上限"; });
  }

  return { 抽屉: 抽屉, 载记忆: 载记忆, 载图谱: 载图谱, 载文件: 载文件, 收起: 收起 };
})();
