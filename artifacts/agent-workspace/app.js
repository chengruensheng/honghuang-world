/* =========================================================================
   洪荒 · IDE 工作台 — app.js
   真实机制版：智能体=独立模型配置(LLM接入) · 记忆=乾坤记忆引擎 ·
   传承殿/门禁/架构 抽屉 · 图谱=三态认知(世界态) · 批准流水线
   ========================================================================= */
(function(){
  "use strict";

  var $  = function(s, c){ return (c||document).querySelector(s); };
  var $$ = function(s, c){ return Array.prototype.slice.call((c||document).querySelectorAll(s)); };
  function now(){
    var d = new Date();
    return [d.getHours(), d.getMinutes(), d.getSeconds()]
      .map(function(n){ return String(n).padStart(2, "0"); }).join(":");
  }

  /* ---------- 右图标轨 → 右面板切换 ---------- */
  var rrailBtns = $$(".id-rbtn");
  var rpanes    = $$(".rpane");

  function showRPane(name){
    rrailBtns.forEach(function(b){ b.classList.toggle("is-on", b.dataset.pane === name); });
    rpanes.forEach(function(p){ p.classList.toggle("is-on", p.dataset.pane === name); });
  }
  rrailBtns.forEach(function(b){
    b.addEventListener("click", function(){ showRPane(b.dataset.pane); });
  });

  /* ---------- 左轨 + 左抽屉（5 个真实面板） ---------- */
  var drawer   = $("#ldrawer");
  var ldTitle  = $("#ld-title");
  var ldSub    = $("#ld-sub");
  var ldBodies = $$(".ld-body", drawer);
  var lrailBtns = $$(".id-rail");
  var currentDrawer = null;

  var DRAWER_META = {
    agents: { title: "智能体", sub: "独立模型配置" },
    memory: { title: "记忆引擎", sub: "乾坤 · 土 · 承载稳定" },
    hall:   { title: "传承殿", sub: "契约 / 文档 / 设计" },
    gate:   { title: "门禁",   sub: "证道 · 验证与守门" },
    arch:   { title: "架构",   sub: "九根六层" }
  };

  function openDrawer(name){
    currentDrawer = name;
    drawer.hidden = false;
    ldTitle.textContent = DRAWER_META[name].title;
    ldSub.textContent   = DRAWER_META[name].sub;
    ldBodies.forEach(function(b){ b.hidden = b.dataset.body !== name; });
    lrailBtns.forEach(function(b){ b.classList.toggle("is-on", b.dataset.pane === name); });
    var inp = drawer.querySelector("input"); if (name === "memory" && inp) inp.focus();
  }
  function closeDrawer(){
    currentDrawer = null;
    drawer.hidden = true;
    lrailBtns.forEach(function(b){ b.classList.toggle("is-on", b.dataset.pane === "chat"); });
  }
  lrailBtns.forEach(function(b){
    b.addEventListener("click", function(){
      var pane = b.dataset.pane;
      if (DRAWER_META[pane]){
        (currentDrawer === pane) ? closeDrawer() : openDrawer(pane);
      } else if (pane === "graph" || pane === "settings"){
        closeDrawer();
        showRPane(pane);        // 图谱 / 设置 → 右面板
      } else {
        closeDrawer();          // 对话
      }
    });
  });
  $("#ld-close").addEventListener("click", closeDrawer);
  document.addEventListener("keydown", function(e){
    if (e.key === "Escape" && currentDrawer) closeDrawer();
  });

  /* ---------- 智能体 = LLM 模型配置 ---------- */
  var llmRows = $$(".llm-row");
  var VENDOR_MODEL = {
    "DeepSeek": "DeepSeek-V3", "智谱 GLM": "GLM-4-Plus", "通义 Qwen": "Qwen-Max",
    "Kimi": "Moonshot-v1-128k", "MiniMax": "MiniMax-M3", "火山方舟": "Doubao-Pro-32k",
    "OpenAI": "GPT-4o", "OpenRouter": "Router-Auto", "Ollama": "qwen2.5:14b(本地)"
  };
  function setCurrentModel(vendor, model){
    $("#llm-cur-name").textContent = model;
    $("#llm-cur-vendor").textContent = vendor + " · 已接入";
    $("#model-top").innerHTML = '<i class="dot dot-fire"></i>' + model;
    $("#model-bot").innerHTML = '<i class="dot dot-fire"></i>' + model;
  }
  llmRows.forEach(function(row){
    row.addEventListener("click", function(){
      llmRows.forEach(function(x){ x.classList.remove("is-cur"); });
      row.classList.add("is-cur");
      var vendor = row.dataset.vendor;
      var model  = VENDOR_MODEL[vendor] || vendor;
      setCurrentModel(vendor, model);
    });
  });
  $("#llm-connect-btn").addEventListener("click", function(){
    var f = $("#llm-form");
    f.hidden = !f.hidden;
  });

  /* ---------- 记忆检索（记忆引擎条目流） ---------- */
  var memSearch = $("#mem-search");
  var memList   = $(".mem-list");
  if (memSearch && memList){
    memSearch.addEventListener("input", function(){
      var q = memSearch.value.trim().toLowerCase();
      memList.classList.toggle("filtered", !!q);
      $$(".mem-item", memList).forEach(function(it){
        it.classList.toggle("hit", !q || it.textContent.toLowerCase().indexOf(q) > -1);
      });
    });
  }

  /* ---------- 阅读覆盖层（传承殿文献 / 架构图） ---------- */
  var reader    = $("#reader");
  var rdTitle   = $("#reader-title");
  var rdPath    = $("#reader-path");
  var rdBody    = $("#reader-body");
  var baseHint  = null;   // 探测出的可读基址

  function esc(s){ return s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;"); }
  function inline(s){
    return esc(s)
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');
  }
  function renderMd(src){
    var lines = src.split(/\r?\n/), out = [], inCode = false, list = null;
    function closeList(){ if (list){ out.push("</" + list + ">"); list = null; } }
    for (var i = 0; i < lines.length; i++){
      var l = lines[i];
      if (/^```/.test(l)){ closeList(); inCode = !inCode; out.push(inCode ? "<pre><code>" : "</code></pre>"); continue; }
      if (inCode){ out.push(esc(l)); continue; }
      var h = l.match(/^(#{1,3})\s+(.*)/);
      if (h){ closeList(); out.push("<h" + h[1].length + ">" + inline(h[2]) + "</h" + h[1].length + ">"); continue; }
      if (/^\s*(-{3,}|\*{3,})\s*$/.test(l)){ closeList(); out.push("<hr>"); continue; }
      var ul = l.match(/^\s*[-*]\s+(.*)/);
      if (ul){ if (list !== "ul"){ closeList(); out.push("<ul>"); list = "ul"; } out.push("<li>" + inline(ul[1]) + "</li>"); continue; }
      var ol = l.match(/^\s*\d+[.、]\s+(.*)/);
      if (ol){ if (list !== "ol"){ closeList(); out.push("<ol>"); list = "ol"; } out.push("<li>" + inline(ol[1]) + "</li>"); continue; }
      var bq = l.match(/^>\s?(.*)/);
      if (bq){ closeList(); out.push("<blockquote>" + inline(bq[1]) + "</blockquote>"); continue; }
      if (/^\s*\|/.test(l)){ closeList(); out.push("<p>" + inline(l.replace(/\|/g," ").trim()) + "</p>"); continue; }
      if (/^\s*$/.test(l)){ closeList(); continue; }
      closeList(); out.push("<p>" + inline(l) + "</p>");
    }
    closeList();
    if (inCode) out.push("</code></pre>");
    return '<div class="md">' + out.join("\n") + "</div>";
  }

  function probeBase(){
    if (baseHint) return Promise.resolve(baseHint);
    var cands = ["../../", "../", "/", "./"];
    var enc = null;
    return new Promise(function(res){
      var i = 0;
      function next(){
        if (i >= cands.length){ res(null); return; }
        var base = cands[i++];
        // 用已知存在的文件探测：README.md
        fetch(base + "README.md", { method: "GET" }).then(function(r){
          if (r.ok){ return r.text(); }
          throw 0;
        }).then(function(t){
          if (t.indexOf("洪荒") > -1 || t.length > 0){ baseHint = base; res(base); }
          else next();
        }).catch(next);
      }
      next();
    });
  }

  function isFileMode(){ return location.protocol === "file:"; }

  // file:// 下从页面路径反推项目根（用于拼启动命令）
  function localProjectRoot(){
    var p = decodeURIComponent(location.pathname).replace(/^\/+/, "").replace(/\//g, "\\");
    var i = p.toLowerCase().indexOf("\\artifacts\\agent-workspace\\");
    return i > -1 ? p.slice(0, i) : null;
  }
  function legacyCopy(t, ok){
    var ta = document.createElement("textarea");
    ta.value = t; ta.style.position = "fixed"; ta.style.opacity = "0";
    document.body.appendChild(ta); ta.select();
    try { document.execCommand("copy"); ok(); } catch(e){}
    document.body.removeChild(ta);
  }
  function readerErrHtml(){
    if (isFileMode()){
      var root = localProjectRoot();
      var cmd  = root ? ('node "' + root + '\\artifacts\\agent-workspace\\预览-服务.js"')
                      : '双击 artifacts\\agent-workspace\\预览.cmd';
      return '<div class="reader__err">'
        + '<p><b>当前以 file:// 方式打开</b> — 浏览器安全策略禁止页面直接读取本地文件，文档无法加载。</p>'
        + '<p>启动预览服务后即可阅读：双击 <code>artifacts\\agent-workspace\\预览.cmd</code>，或复制命令到终端执行：</p>'
        + '<p><code>' + esc(cmd) + '</code></p>'
        + '<div class="reader__acts">'
        + '<a class="ra-btn" href="http://127.0.0.1:8899/artifacts/agent-workspace/index.html">打开 http 预览版 →</a>'
        + '<button class="ra-btn ghost" id="ra-copy">复制启动命令</button>'
        + "</div></div>";
    }
    return '<div class="reader__err">'
      + '<p>内容加载失败 — 请确认预览服务已启动，且以<b>项目根</b>为根目录（README.md 须可经 <code>../../</code> 访问到）。</p>'
      + '<div class="reader__acts"><a class="ra-btn" href="http://127.0.0.1:8000/artifacts/agent-workspace/index.html">打开 http://127.0.0.1:8000 →</a></div>'
      + "</div>";
  }
  rdBody.addEventListener("click", function(e){
    var btn = e.target && e.target.closest ? e.target.closest("#ra-copy") : null;
    if (!btn) return;
    var root = localProjectRoot();
    var cmd  = root ? ('node "' + root + '\\artifacts\\agent-workspace\\预览-服务.js"')
                    : '双击 artifacts\\agent-workspace\\预览.cmd';
    legacyCopy(cmd, function(){
      btn.textContent = "已复制 ✓";
      setTimeout(function(){ btn.textContent = "复制启动命令"; }, 1600);
    });
  });

  function openReader(title, path, kind, src){
    reader.hidden = false;
    rdTitle.textContent = title;
    rdPath.textContent = path || "";
    if (kind === "iframe"){
      if (isFileMode()){
        // file:// 下 iframe 允许加载本地文件，直接走相对路径
        rdBody.innerHTML = "";
        var f0 = document.createElement("iframe");
        f0.src = "../../" + encodeURI(path);
        rdBody.appendChild(f0);
        return;
      }
      probeBase().then(function(base){
        if (!base){ rdBody.innerHTML = '<div class="reader__err">无法定位文件基址（请通过 http:// 访问本页）</div>'; return; }
        rdBody.innerHTML = "";
        var fr = document.createElement("iframe");
        fr.src = base + encodeURI(path);
        rdBody.appendChild(fr);
      });
      return;
    }
    if (isFileMode()){ rdBody.innerHTML = readerErrHtml(); return; }
    probeBase().then(function(base){
      if (!base){ rdBody.innerHTML = readerErrHtml(); return; }
      fetch(base + encodeURI(path)).then(function(r){
        if (!r.ok) throw 0;
        return r.text();
      }).then(function(text){
        if (kind === "md") rdBody.innerHTML = renderMd(text);
        else rdBody.innerHTML = '<div class="plain">' + esc(text) + "</div>";
        rdBody.scrollTop = 0;
      }).catch(function(){
        rdBody.innerHTML = '<div class="reader__err">文件未找到：<code>' + esc(path) + "</code></div>";
      });
    });
  }
  function closeReader(){ reader.hidden = true; }
  $("#reader-close").addEventListener("click", closeReader);
  reader.addEventListener("click", function(e){ if (e.target === reader) closeReader(); });
  document.addEventListener("keydown", function(e){
    if (e.key === "Escape" && !reader.hidden) closeReader();
  });

  /* ---------- 文件面板变体（传承殿=文档 / 门禁=代码 / 架构=图 · 统一标准） ---------- */
  $$(".fv-tree").forEach(function(tree){
    var n = $$(".fv-leaf", tree).length;
    var note = tree.parentElement.querySelector(".fv-count b");
    if (note) note.textContent = n;
  });
  $$(".fv-leaf").forEach(function(leaf){
    leaf.addEventListener("click", function(){
      $$(".fv-leaf").forEach(function(x){ x.classList.remove("hit"); });
      leaf.classList.add("hit");
      openReader(leaf.dataset.title || "", leaf.dataset.src || "", leaf.dataset.kind || "md", null);
    });
  });

  /* ---------- 命令面板 ---------- */
  var bg       = $("#cmdbg");
  var openBtn  = $("#cmd-btn");
  var closeBtn = $("#cmd-close");
  var panel    = bg ? bg.querySelector(".cmdpanel") : null;
  var cmdItems = $$(".cmdlist li");
  var cmdIdx   = -1;

  function paintCmd(){
    cmdItems.forEach(function(li, i){ li.classList.toggle("is-on", i === cmdIdx); });
    var el = cmdItems[cmdIdx]; if (el && el.scrollIntoView) el.scrollIntoView({block:"nearest"});
  }
  function openCmd(){
    bg.hidden = false; cmdIdx = 0; paintCmd();
    var inp = bg.querySelector("input"); if (inp) inp.focus();
    if (openBtn) openBtn.classList.add("is-on");
  }
  function closeCmd(){
    bg.hidden = true;
    if (openBtn) openBtn.classList.remove("is-on");
  }
  if (openBtn)  openBtn.addEventListener("click", function(e){
    e.stopPropagation();
    bg.hidden ? openCmd() : closeCmd();
  });
  if (closeBtn) closeBtn.addEventListener("click", closeCmd);
  if (bg)       bg.addEventListener("click", function(e){ if (e.target === bg) closeCmd(); });
  if (panel)    panel.addEventListener("click", function(e){ e.stopPropagation(); });

  function execCmd(li){
    var cmd = (li.querySelector("span")||{}).textContent || "";
    closeCmd();
    if (cmd === "graph.open")      showRPane("graph");
    if (cmd === "diff.accept.all") acceptAll();
    if (cmd === "test.run")        showRPane("term");
    if (cmd === "memory.recall")   openDrawer("memory");
    if (cmd === "model.switch")    openDrawer("agents");
    if (cmd === "task.new"){ closeDrawer(); var t = $(".id-input textarea"); if (t) t.focus(); }
  }
  cmdItems.forEach(function(li){
    li.addEventListener("click", function(){ execCmd(li); });
  });

  document.addEventListener("keydown", function(e){
    if ((e.metaKey || e.ctrlKey) && (e.key === "k" || e.key === "K")){
      e.preventDefault();
      bg.hidden ? openCmd() : closeCmd();
    }
    if (bg && !bg.hidden){
      if (e.key === "Escape") closeCmd();
      if (e.key === "ArrowDown"){ e.preventDefault(); cmdIdx = (cmdIdx + 1) % cmdItems.length; paintCmd(); }
      if (e.key === "ArrowUp"){ e.preventDefault(); cmdIdx = (cmdIdx - 1 + cmdItems.length) % cmdItems.length; paintCmd(); }
      if (e.key === "Enter" && cmdIdx > -1){ e.preventDefault(); execCmd(cmdItems[cmdIdx]); }
    }
  });

  /* ---------- 终端 ---------- */
  var termBody   = $("#term-body");
  var termPrompt = $("#term-prompt");
  function addTerm(text){
    if (!termBody || !termPrompt) return;
    var line = document.createElement("div");
    line.className = "tline";
    line.style.animation = "msgIn .2s ease-out";
    line.innerHTML = '<span class="t">' + now() + "</span>" + text;
    termBody.insertBefore(line, termPrompt);
    termBody.scrollTop = termBody.scrollHeight;
  }

  /* ---------- 对话流水线（执行者 = 当前模型） ---------- */
  var chat  = $("#chat-scroll");
  var state = $("#task-state");

  function curModel(){ return $("#llm-cur-name").textContent; }
  function scrollChat(){ if (chat) chat.scrollTop = chat.scrollHeight; }
  function mkMsg(html){
    var wrap = document.createElement("div");
    wrap.className = "msg ai";
    wrap.innerHTML = html;
    chat.appendChild(wrap); scrollChat();
    return wrap;
  }
  function mkSys(text){
    var wrap = document.createElement("div");
    wrap.className = "msg sys";
    wrap.innerHTML = '<span class="time">' + now() + "</span><p>" + text + "</p>";
    chat.appendChild(wrap); scrollChat();
    return wrap;
  }
  function agentWho(){
    return '<div class="who">' +
      '<span class="avatar sm agent"><svg viewBox="0 0 24 24"><rect x="4" y="7" width="16" height="12" rx="2"/><path d="M12 7V4M8 4h8"/><circle cx="9" cy="13" r="1"/><circle cx="15" cy="13" r="1"/><path d="M9 16.5h6"/></svg></span>' +
      "<b>" + curModel() + " · 智能体</b>" +
      '<span class="when">' + now() + "</span>" +
      '<span class="pill pill-fire">火</span>' +
    "</div>";
  }
  function setState(running, label){
    if (!state) return;
    state.classList.toggle("is-run", running);
    state.innerHTML = "<i></i>" + label;
  }

  function runPipeline(){
    setState(true, "测试中 · 步骤 5/7");

    setTimeout(function(){
      mkSys("已批准 · 测试流水线启动 · 执行者：" + curModel());
    }, 300);

    setTimeout(function(){
      addTerm("cargo test -p 量劫");
      var m = mkMsg(
        agentWho() +
        '<div class="bubble">' +
          '<div class="tool running">' +
            '<header><i class="ico"><svg viewBox="0 0 24 24"><path d="M4 17l6-6 4 4 6-8"/></svg></i><b>运行测试（证道）</b>' +
            '<span class="run"><i class="spin"></i>运行中</span><span class="ms">cargo test</span></header>' +
            '<ul class="tests">' +
              "<li><i></i>量劫::迭代流转 <em>5 项</em></li>" +
              "<li><i></i>乾坤::记忆写入 <em>3 项</em></li>" +
              "<li><i></i>道韵::规则校验 <em>4 项</em></li>" +
            "</ul>" +
          "</div>" +
        "</div>");
      var tool = m.querySelector(".tool");
      var tt = 0;
      [["pass 太初::任务发布 <span class='ok'>✓</span>"],
       ["pass 量劫::迭代流转 <span class='ok'>✓</span>"],
       ["pass 乾坤::记忆写入 <span class='ok'>✓</span>"],
       ["<span class='ok'>12/12 通过</span> · 覆盖率 94% · <span class='ok'>门禁放行</span>"]
      ].forEach(function(row){
        tt += 550;
        setTimeout(function(){
          addTerm(row[0]);
          if (row[0].indexOf("12/12") > -1){
            tool.classList.remove("running");
            var st = tool.querySelector("header .run");
            st.className = "ok"; st.innerHTML = "完成 ✓";
            tool.querySelector("header .ms").textContent = "6.9s";
          }
        }, tt);
      });
    }, 700);

    setTimeout(function(){
      setState(true, "提交中 · 步骤 6/7");
      addTerm("git commit -m \"feat(量劫): 迭代完成写记忆联动契约 v2.6\"");
      addTerm("git push origin 量劫/联动契约");
      mkMsg(
        agentWho() +
        '<div class="bubble">' +
          '<div class="tool">' +
            '<header><i class="ico"><svg viewBox="0 0 24 24"><circle cx="6" cy="6" r="2.4"/><circle cx="6" cy="18" r="2.4"/><circle cx="18" cy="6" r="2.4"/><path d="M6 8v8"/><path d="M6 12c6 0 6-6 12-6"/></svg></i><b>git 提交</b>' +
            '<span class="ok">完成 ✓</span><span class="ms">a1b2c3d</span></header>' +
            "<pre><code>feat(量劫): 迭代完成写记忆联动契约 v2.6\n\n- 相生信号：迭代完成 → 记忆写入（火生土）\n- 事件去重按类型+载荷合并（土克水）\n+ 新增 3 项联动回归测试（证道）</code></pre>" +
          "</div>" +
          '<div class="prcard">' +
            "<header><b>PR #129</b><span class='n'>量劫/联动契约</span><span class='st'>已创建</span></header>" +
            '<span class="lnk">github.com/honghuang/workspace/pull/129</span>' +
            "<ul>" +
              "<li>测试 12/12 通过（证道门禁放行）</li>" +
              "<li>契约校验通过 · 律校验无违例</li>" +
              "<li>等待道祖合并</li>" +
            "</ul>" +
          "</div>" +
        "</div>");
    }, 3100);

    setTimeout(function(){
      setState(false, "已完成 · 7/7");
      var el = $("#bot-elapsed"); if (el) el.textContent = "04:38";
      mkSys("任务完成 · 火生土：记忆已写入「五行相生联动契约 v2.6」· PR #129 待道祖合并");
    }, 3900);
  }

  function runReplan(){
    setTimeout(function(){
      mkMsg(
        agentWho() +
        '<div class="bubble">' +
          "<p>收到驳回。重新规划：仅保留相生信号主链路，事件去重拆为独立任务（避免扩散影响面）。</p>" +
          '<div class="tool wait">' +
            '<header><i class="ico"><svg viewBox="0 0 24 24"><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1"/></svg></i><b>重新生成 Diff</b>' +
            '<span class="wait">排队</span><span class="ms">scope: 量劫</span></header>' +
          "</div>" +
          "<p>新版 Diff 就绪后会再次请求批准。</p>" +
        "</div>");
      setState(true, "重新规划 · 步骤 4/7");
    }, 400);
  }

  /* ---------- 批准卡 ---------- */
  $$(".approve [data-ok]").forEach(function(btn){
    btn.addEventListener("click", function(){
      var card = btn.closest(".approve");
      card.style.borderColor = "rgba(92,224,160,.4)";
      card.querySelector("header b").textContent = "已批准 · 流水线执行中";
      btn.disabled = true; btn.textContent = "已批准 ✓";
      var no = card.querySelector("[data-no]"); if (no) no.disabled = true;
      showRPane("diff");
      runPipeline();
    });
  });
  $$(".approve [data-no]").forEach(function(btn){
    btn.addEventListener("click", function(){
      var card = btn.closest(".approve");
      card.querySelector("header b").textContent = "已驳回";
      btn.disabled = true; btn.textContent = "已驳回";
      var ok = card.querySelector("[data-ok]"); if (ok) ok.disabled = true;
      setState(false, "已驳回");
      runReplan();
    });
  });
  $$(".approve .gh").forEach(function(btn){
    btn.addEventListener("click", function(){ showRPane("diff"); });
  });

  /* ---------- Diff 审阅 ---------- */
  var diffCode = $(".diff-code");
  var bar      = $(".rev .bar i");
  var cnt      = $(".rev .cnt");
  var total    = diffCode ? $$(".hunk", diffCode).length : 0;
  var reviewed = new Set();

  function refreshProgress(){
    if (!bar || !cnt) return;
    var n = reviewed.size;
    bar.style.width = total ? Math.round(n / total * 100) + "%" : "0%";
    cnt.textContent = n + " / " + total + " hunk 已审";
  }
  if (diffCode){
    $$(".hunk", diffCode).forEach(function(h, i){
      h.style.cursor = "pointer";
      h.title = "点击标记此 hunk 已审";
      h.addEventListener("click", function(){
        if (reviewed.has(i)){ reviewed.delete(i); h.style.opacity = ""; }
        else { reviewed.add(i); h.style.opacity = ".45"; }
        refreshProgress();
      });
    });
  }
  function acceptAll(){
    if (!diffCode) return;
    $$(".hunk", diffCode).forEach(function(h){ h.style.opacity = ".45"; });
    if (total) reviewed = new Set(Array.from({length: total}, function(_, i){ return i; }));
    refreshProgress();
    setState(false, "Diff 已全部接受");
  }
  var acceptBtn = $(".rpane-foot .primary");
  if (acceptBtn) acceptBtn.addEventListener("click", acceptAll);

  var rejectBtn = $(".rpane-foot .ghost");
  if (rejectBtn) rejectBtn.addEventListener("click", function(){
    if (!diffCode) return;
    $$(".ln.add, .ln.del", diffCode).forEach(function(ln){ ln.style.opacity = ".25"; });
    setState(true, "Diff 已驳回 · AI 修改中");
  });

  /* ---------- Diff 文件标签页（切换高亮） ---------- */
  $$(".rpane-head .tabs button").forEach(function(t){
    t.addEventListener("click", function(){
      if (t.classList.contains("more")) return;
      var tabs = $$(".rpane-head .tabs button", t.closest(".rpane-head"));
      tabs.forEach(function(x){ x.classList.remove("is-on"); });
      t.classList.add("is-on");
    });
  });

  /* ---------- 设置开关 / 输入条 mini ---------- */
  $$(".set .sw").forEach(function(sw){
    sw.parentElement.addEventListener("click", function(){ sw.classList.toggle("on"); });
  });
  $$(".id-input .mini").forEach(function(m){
    m.addEventListener("click", function(){ m.classList.toggle("is-on"); });
  });

  /* ---------- 对话输入（本地追加） ---------- */
  var ta = $(".id-input textarea");
  function sendMsg(){
    var v = (ta.value || "").trim();
    if (!v) return;
    var wrap = document.createElement("div");
    wrap.className = "msg user";
    wrap.innerHTML =
      '<div class="who"><span class="avatar sm">九五</span><b>道祖</b>' +
      '<span class="when">' + now() + '</span></div>' +
      '<div class="bubble"></div>';
    wrap.querySelector(".bubble").textContent = v;
    chat.appendChild(wrap);
    ta.value = "";
    scrollChat();
  }
  if (ta){
    ta.addEventListener("keydown", function(e){
      if (e.key === "Enter" && !e.shiftKey){ e.preventDefault(); sendMsg(); }
    });
  }
  var sendBtn = $(".id-input .send");
  if (sendBtn) sendBtn.addEventListener("click", sendMsg);

  /* ---------- 文件树（点击高亮，改动文件跳 Diff） ---------- */
  $$(".t-file").forEach(function(f){
    f.addEventListener("click", function(){
      $$(".t-file").forEach(function(x){ x.style.background = ""; });
      f.style.background = "var(--bg-3)";
      if (f.classList.contains("mod")) showRPane("diff");
    });
  });
})();
