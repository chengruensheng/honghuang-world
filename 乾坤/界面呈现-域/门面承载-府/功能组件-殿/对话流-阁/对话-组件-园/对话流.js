/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 主区对话流 实现
   职责：渲染用户/道祖消息、五行任务进度卡、待确认卡、系统消息与
   输入条，经挂载契约进入主区槽位；支持快捷指令与流式回复。
   ═══════════════════════════════════════════════════════════ */
window.乾坤对话 = (function(){
  // ── 会话状态：空态启动，内容由真实交互填充 ──
  var 数据 = [];
  var 发送中 = false;   // 防重复发送锁
  var 发按钮 = null;    // 发送按钮引用（解锁用）

  // ── SVGs ──
  var I = {
    智: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><rect x="4" y="7" width="16" height="12" rx="2"/><path d="M12 7V4M8 4h8"/><circle cx="9" cy="13" r="1"/><circle cx="15" cy="13" r="1"/><path d="M9 16.5h6"/></svg>',
    发: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12l18-9-6 18-3-7z"/></svg>'
  };

  // ── 快捷指令注册表（可扩展）：内置 /清空、/刷新 两条 ──
  var 指令表 = {
    "清空": function(ctx){ return Promise.resolve(cmd_clear(ctx)); },
    "刷新": function(ctx){ return cmd_refresh(ctx); }
  };

  function register_command(名, 处理器){
    if (!名 || typeof 处理器 !== "function"){ return; }
    指令表[String(名).replace(/^\//, "")] = 处理器;
  }

  function parse_command(输入){
    var 文 = String(输入 == null ? "" : 输入).trim();
    if (!文 || 文.charAt(0) !== "/"){ return null; }
    var 段 = 文.replace(/^\/+/, "").split(/\s+/);
    var 名 = 段.shift();
    if (!名){ return null; }
    return { name: 名, args: 段 };
  }

  // /清空：清空消息渲染与数据；保留会话标识与元数据
  function cmd_clear(ctx){
    ctx = ctx || {};
    var 会话id = (ctx.会话id != null ? ctx.会话id : (window.乾坤对话.会话id || ""));
    var 会话元数据 = window.乾坤对话.会话元数据 || null;
    数据.length = 0;
    var 流 = ctx.流 || document.querySelector(".对话外壳 .对话流");
    if (流){
      流.innerHTML = "";
      render_welcome_panel(流);
      流.scrollTop = 0;
    }
    window.乾坤对话.会话id = 会话id;
    window.乾坤对话.会话元数据 = 会话元数据;
  }

  // /刷新：重新拉取当前会话消息；无会话时直接跳过（避免注定失败的占位请求），失败静默降级
  function cmd_refresh(ctx){
    ctx = ctx || {};
    var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
    var 会话id = (ctx.会话id != null ? ctx.会话id : (window.乾坤对话.会话id || ""));
    var 真实会话id = 会话id || (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.当前会话id) || "";
    if (!真实会话id){ return Promise.resolve(); }
    var 端点 = 后端 + "/api/dev/sessions/" + encodeURIComponent(真实会话id || "current");
    return fetch(端点)
      .then(function(r){ return r.ok ? r.json() : { 事件: [] }; })
      .then(function(d){
        数据.length = 0;
        (d.事件 || []).forEach(function(ev){
          if (ev.内容 && (ev.类型 === "任务答复" || ev.类型 === "思考" || ev.类型 === "工具结果")){
            // 回放内容一律过净化后再入 DOM
            数据.push({ 类:"讯息", 方:"对", 名:ev.角色 || "系统", 时:"刚刚", 文:净化(ev.内容) });
          }
        });
        var 流 = ctx.流 || document.querySelector(".对话外壳 .对话流");
        if (流){
          流.innerHTML = "";
          if (数据.length){
            数据.forEach(function(m){ 流.appendChild(渲染(m)); });
            流.scrollTop = 流.scrollHeight;
          } else {
            render_welcome_panel(流);
          }
        }
      })
      .catch(function(){ /* 静默降级：聊天面板保持可见 */ });
  }

  function on_unknown_command(名, 流){
    if (流){
      var 泡 = document.createElement("div");
      泡.className = "讯";
      泡.appendChild(document.createTextNode("未知指令 /"));
      var 加粗 = document.createElement("b");
      加粗.textContent = String(名 || "");
      泡.appendChild(加粗);
      泡.appendChild(document.createTextNode("，已忽略。可用：/清空、/刷新"));
      流.appendChild(泡);
      流.scrollTop = 流.scrollHeight;
    }
  }

  function dispatch_command(parsed, ctx){
    ctx = ctx || {};
    if (!parsed || !parsed.name){
      on_unknown_command("", ctx.流);
      return Promise.resolve();
    }
    var 处理器 = 指令表[parsed.name];
    if (!处理器){
      on_unknown_command(parsed.name, ctx.流);
      return Promise.resolve();
    }
    try {
      return Promise.resolve(处理器(ctx)).catch(function(){
        on_unknown_command(parsed.name, ctx.流);
      });
    } catch (e){
      on_unknown_command(parsed.name, ctx.流);
      return Promise.resolve();
    }
  }

  // 首屏欢迎面板：引导文案 + 快捷指令
  function render_welcome_panel(容器){
    if (!容器){ return; }
    容器.innerHTML = "";
    var 欢迎 = document.createElement("div");
    欢迎.className = "欢迎面板";
    欢迎.innerHTML =
      '<div class="欢迎-顶">' +
        '<span class="欢迎-徽">紫霄宫</span>' +
        '<span class="欢迎-名">洪荒·世界</span>' +
      '</div>' +
      '<div class="欢迎-标">与道祖对话</div>' +
      '<div class="欢迎-次">描述你的需求，道祖澄清对齐后发布任务，<br>经 木→火→土→金→水 五行流转自主完成。</div>' +
      '<div class="欢迎-行">' +
        '<i style="background:var(--木)"></i><i style="background:var(--火)"></i><i style="background:var(--土)"></i><i style="background:var(--金)"></i><i style="background:var(--水)"></i>' +
      '</div>' +
      '<div class="快捷面板">' +
        '<button class="快捷-项" data-cmd="清空"><span class="快捷-名">/清空</span><span class="快捷-描">清空当前会话</span></button>' +
        '<button class="快捷-项" data-cmd="刷新"><span class="快捷-名">/刷新</span><span class="快捷-描">重新拉取最新消息</span></button>' +
      '</div>';
    容器.appendChild(欢迎);
    function 取输入框(){
      return document.querySelector(".对话外壳 .对话输入 textarea");
    }
    Array.prototype.forEach.call(欢迎.querySelectorAll(".快捷-项"), function(btn){
      btn.addEventListener("click", function(){
        var 名 = btn.getAttribute("data-cmd") || "";
        var 框 = 取输入框();
        // 回显指令便于所见即所发；执行后清空，避免残留已执行的命令文本
        if (框){ 框.value = "/" + 名; 框.focus(); }
        dispatch_command({ name: 名, args: [] }, { 流: 容器 });
        if (框){ 框.value = ""; }
      });
    });
  }

  // ── 净化与轻渲染：先整体转义杜绝注入，再对已转义文本做结构替换 ──
  // 支持：思考折叠、```围栏代码块、`行内码`、**粗体**、# 标题、- 列表
  function 净化(文本){
    var s = String(文本 == null ? "" : 文本)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    // 1. 思考段折叠（内容保留原始换行，交由样式 pre-wrap 呈现）
    s = s.replace(/&lt;think&gt;([\s\S]*?)&lt;\/think&gt;/g, function(全, 内){
      return '<details class="思"><summary>思考过程</summary><div>' + 内 + '</div></details>';
    });
    // 2. 围栏代码块先摘出，避免内部被行内规则改写
    //    语言标记仅参与围栏识别，不落 DOM：净化不转义引号，属性拼接会被 " 逃逸（XSS）
    var 码堆 = [];
    s = s.replace(/```([\w-]*)\n?([\s\S]*?)```/g, function(全, 语言, 码){
      码堆.push(码.replace(/\n$/, ""));
      return "\x00" + (码堆.length - 1) + "\x00";
    });
    // 3. 行内码 / 粗体 / 标题 / 列表
    s = s.replace(/`([^`\n]+)`/g, "<code>$1</code>");
    s = s.replace(/\*\*([^*\n]+)\*\*/g, "<b>$1</b>");
    s = s.replace(/^#{1,4}\s+(.*)$/gm, "<h6>$1</h6>");
    s = s.replace(/^[-*]\s+(.*)$/gm, "<span class='列'>•</span> $1");
    // 4. 换行 → <br>
    s = s.replace(/\n/g, "<br>");
    // 5. 还原代码块（pre 内保留真实换行）
    s = s.replace(/\x00(\d+)\x00/g, function(全, 序){
      return '<pre class="码"><code>' + 码堆[Number(序)] + '</code></pre>';
    });
    return s;
  }

  // ── 渲染入口：挂到主区 ──
  function 建(){
    var 外 = document.createElement("div");
    外.className = "对话外壳 主区视图";
    外.setAttribute("data-视图", "对话");

    // 标题行
    var 标题 = document.createElement("div");
    标题.className = "对话标题";
    标题.innerHTML = '<span class="标">紫霄宫对话</span><span class="次">与道祖对话，发布任务或自由提问</span>';
    var 态 = document.createElement("span");
    态.className = "态";
    态.innerHTML = '<i class="点"></i>空闲';
    标题.appendChild(态);
    外.appendChild(标题);

    // 消息流
    var 流 = document.createElement("div");
    流.className = "对话流";
    if (!数据.length){
      render_welcome_panel(流);
    } else {
      数据.forEach(function(m){ 流.appendChild(渲染(m)); });
      流.scrollTop = 流.scrollHeight;
    }
    外.appendChild(流);

    // 输入条：只保留真实可用的输入与发送
    var 输 = document.createElement("div");
    输.className = "对话输入";
    输.innerHTML =
      '<div class="区">' +
        '<textarea rows="2" placeholder="描述你的需求 · Enter 发送 · Shift+Enter 换行"></textarea>' +
        '<button class="发" title="发送">发送' + I.发 + '</button>' +
      '</div>' +
      '<div class="足"><span>Enter 发送 · Shift+Enter 换行</span><span class="足-右">对话即发布 · 五行自主流转</span></div>';
    外.appendChild(输);

    var 框 = 输.querySelector("textarea");
    var 发 = 输.querySelector(".发");
    发按钮 = 发;
    function 发送(){
      if (发送中){ return; }
      var 文 = 框.value.trim();
      if (!文){ return; }
      var 解析 = parse_command(文);
      if (解析){
        dispatch_command(解析, { 流: 流 });
        框.value = "";
        流.scrollTop = 流.scrollHeight;
        return;
      }
      发送中 = true;
      发.disabled = true;
      设置态("运转中");
      // 用户消息同样过净化：杜绝自输入 HTML 被当标记执行（自注入 XSS）
      流.appendChild(渲染({ 类:"讯息", 方:"己", 名:"界主", 字:"界", 时:"刚刚", 文:净化(文) }));
      框.value = "";
      流.scrollTop = 流.scrollHeight;
      var 载 = 流.appendChild(加载中());
      流.scrollTop = 流.scrollHeight;
      var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
      流式对话(后端, 文, 流, 载);
    }
    发.addEventListener("click", 发送);
    框.addEventListener("keydown", function(e){
      if (e.key === "Enter" && !e.shiftKey){ e.preventDefault(); 发送(); }
    });

    // ── 初始化：自动获取最新会话并回放消息（解决 API 发布任务后对话流空白的问题）──
    (function(){
      var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
      fetch(后端 + "/api/dev/sessions")
        .then(function(r){ return r.ok ? r.json() : null; })
        .then(function(d){
          var 列表 = d && (d.会话 || d.sessions || []);
          if (!列表.length){ return; }
          // 取最新会话
          var 最新 = 列表[列表.length - 1];
          var sid = 最新.会话id || 最新.id;
          if (!sid){ return; }
          window.乾坤对话.会话id = sid;
          // 如果当前没有消息，自动回放
          if (!数据.length){
            cmd_refresh({ 流: 流, 会话id: sid });
          }
        })
        .catch(function(){ /* 静默降级 */ });
    })();

    // ── 驱动状态轮询：标题行实时反映驱动运行/空闲 ──
    (function(){
      var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
      setInterval(function(){
        if (!外.isConnected){ return; }
        fetch(后端 + "/api/dev/pilot/status")
          .then(function(r){ return r.ok ? r.json() : null; })
          .then(function(d){
            if (!d){ return; }
            var 运行 = d.运行中 || d.running;
            设置态(运行 ? "运转中" : "空闲");
          })
          .catch(function(){});
      }, 3000);
    })();

    return 外;
  }

  // ── 标题状态点：发送后运转中，回复收尾后回空闲 ──
  function 设置态(名){
    var 态 = document.querySelector(".对话外壳 .对话标题 .态");
    if (!态){ return; }
    态.innerHTML = '<i class="点"></i>' + 名;
    if (名 !== "空闲"){ 态.classList.add("忙"); } else { 态.classList.remove("忙"); }
    // 回到空闲时自动解锁发送按钮
    if (名 === "空闲"){
      发送中 = false;
      if (发按钮){ 发按钮.disabled = false; }
    }
  }

  // ── 流式对话：POST /api/dev/chat/stream，SSE 打字机式追加 ──
  function 流式对话(后端, 文, 流, 载){
    var 出过内容 = false; // 流中断时区分"零内容可降级重发"与"已有部分内容不可重复"
    fetch(后端 + "/api/dev/chat/stream", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ 消息: 文 })
    })
    .then(function(r){
      if (!r.ok || !r.body){ throw new Error("HTTP " + r.status); }
      var 读 = r.body.getReader();
      var 解码 = new TextDecoder("utf-8");
      var 缓冲 = "";
      var 泡 = null;
      var p = null;
      var 回复文本 = "";
      var 任务id = null;
      var 阶段 = "接待中";

      function 建泡(){
        if (泡) return;
        出过内容 = true;
        泡 = 渲染({ 类:"讯息", 方:"对", 名:"道祖", 时:"刚刚", 文:"" });
        流.appendChild(泡);
        p = 泡.querySelector(".气泡 p");
      }
      function 追加(delta){
        建泡();
        回复文本 += delta;
        if (p){ p.innerHTML = 净化(回复文本); }
        流.scrollTop = 流.scrollHeight;
      }
      function 收尾(){
        if (载 && 载.parentNode){ 载.remove(); }
        设置态("空闲");
        if (!泡 && !任务id){
          流.appendChild(渲染({ 类:"讯息", 方:"对", 名:"道祖", 时:"刚刚", 文:"（无回复）" }));
        }
        if (任务id){
          var 卡 = 建任务卡(任务id);
          流.appendChild(卡);
          流.scrollTop = 流.scrollHeight;
          轮询任务卡(卡, 任务id);
        } else if (阶段 === "待确认"){
          // 建待确认卡 内含 appendChild，此处不再重复挂载
          建待确认卡(后端, 流);
          流.scrollTop = 流.scrollHeight;
        }
      }
      function 泵(){
        return 读.read().then(function(结果){
          if (结果.done){ 收尾(); return; }
          缓冲 += 解码.decode(结果.value, { stream: true });
          var 帧们 = 缓冲.split("\n\n");
          缓冲 = 帧们.pop() || "";
          帧们.forEach(function(帧){
            var 数据帧 = "";
            帧.split("\n").forEach(function(行){
              if (行.indexOf("data:") === 0){ 数据帧 += 行.slice(5).trim(); }
            });
            if (!数据帧) return;
            try {
              var ev = JSON.parse(数据帧);
              if (ev.type === "TEXT_MESSAGE_CONTENT" && ev.delta){ 追加(ev.delta); }
              else if (ev.type === "RUN_FINISHED"){
                任务id = ev.任务id || null;
                阶段 = ev.阶段 || "接待中";
              }
            } catch(_){}
          });
          return 泵();
        });
      }
      return 泵();
    })
    .catch(function(){
      // 已有部分流内容：只收尾不重发，避免同步降级再追加一条重复回复
      if (出过内容){
        if (载 && 载.parentNode){ 载.remove(); }
        设置态("空闲");
        return;
      }
      同步对话(后端, 文, 流, 载);
    });
  }

  // ── 同步对话：流式不可用时的降级路径 ──
  function 同步对话(后端, 文, 流, 载){
    fetch(后端 + "/api/dev/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ 消息: 文 })
    })
    .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
    .then(function(d){
      载.remove();
      设置态("空闲");
      流.appendChild(渲染({ 类:"讯息", 方:"对", 名:"道祖", 时:"刚刚", 文:净化(d.回复 || "（无回复）") }));
      流.scrollTop = 流.scrollHeight;
      if (d.任务id){
        var 卡 = 建任务卡(d.任务id);
        流.appendChild(卡);
        流.scrollTop = 流.scrollHeight;
        轮询任务卡(卡, d.任务id);
      } else if (d.阶段 === "待确认"){
        建待确认卡(后端, 流);
      }
    })
    .catch(function(){
      载.remove();
      设置态("空闲");
      流.appendChild(渲染({
        类:"讯息", 方:"对", 名:"道祖", 时:"刚刚",
        文:"（道祖暂未在线：需后端 run_dev_agent=true 且配置 LLM 密钥）请检查数据服务后重试。"
      }));
      流.scrollTop = 流.scrollHeight;
    });
  }

  // ── 加载占位气泡 ──
  function 加载中(){
    var 容 = document.createElement("div");
    容.className = "讯息 对";
    容.innerHTML = '<div class="谁"><b class="名">道祖</b><span class="时">思考中…</span></div><div class="气泡 载"><span class="跳"><i></i><i></i><i></i></span></div>';
    return 容;
  }

  // ── 单条消息渲染 ──
  function 渲染(m){
    if (m.类 === "讯"){
      var s = document.createElement("div");
      s.className = "讯";
      s.innerHTML = '<span>' + m.时 + '</span>' + m.文本;
      return s;
    }
    var 容 = document.createElement("div");
    容.className = "讯息 " + (m.方 === "己" ? "己" : "对");

    var 谁 = document.createElement("div");
    谁.className = "谁";
    var 像 = document.createElement("span");
    像.className = "像";
    像.innerHTML = m.方 === "己" ? (m.字 || "界") : I.智;
    谁.appendChild(像);
    var 名 = document.createElement("b");
    名.className = "名";
    名.textContent = m.名;
    谁.appendChild(名);
    var 时 = document.createElement("span");
    时.className = "时";
    时.textContent = m.时;
    谁.appendChild(时);
    if (m.方 === "对"){
      var 徽 = document.createElement("span");
      徽.className = "徽";
      徽.textContent = "火";
      谁.appendChild(徽);
    }
    容.appendChild(谁);

    var 泡 = document.createElement("div");
    泡.className = "气泡";
    var p = document.createElement("p");
    p.innerHTML = m.文;
    泡.appendChild(p);
    容.appendChild(泡);
    return 容;
  }

  // ── 五行任务进度卡 ──
  var 阶段表 = [
    { 五行:"木", 名:"道祖", 状态:["待受理","待道祖澄清","道祖澄清中"] },
    { 五行:"火", 名:"圣人", 状态:["待圣人设计","圣人设计中","待重新设计"] },
    { 五行:"土", 名:"大罗", 状态:["待大罗金仙实现","大罗金仙实现中","待修复","待重新实现"] },
    { 五行:"金", 名:"准圣", 状态:["待准圣验收","准圣验收中","待道祖终审","道祖终审中","待重新验收","已完成"] },
    { 五行:"水", 名:"太乙", 状态:["待清理","清理中","待重新清理","清理完成"] },
  ];

  function 状态到阶段(状态){
    for (var i=0; i<阶段表.length; i++){
      if (阶段表[i].状态.indexOf(状态) >= 0) return i;
    }
    return -1;
  }
  function 状态到结果(状态){
    if (状态 === "已完成" || 状态 === "清理完成") return "完成";
    if (状态.indexOf("中") >= 0) return "进行";
    if (状态.indexOf("重新") >= 0) return "重跑";
    return "等待";
  }

  // ── 待确认卡片：批准/驳回 + 5 秒自动确认 ──
  function 建待确认卡(后端, 流){
    var 卡 = document.createElement("div");
    卡.className = "任务卡 待确认";
    var 秒 = 5;
    卡.innerHTML =
      '<div class="头"><b>道祖已对齐需求</b><span class="状">待确认 '+秒+'s</span></div>'+
      '<div class="描">确认后即安排圣人设计、大罗金仙实现、准圣验收。</div>'+
      '<div class="排"><button class="准">确认发布</button><button class="否">驳回</button></div>';
    流.appendChild(卡);

    function 确认发布(){
      卡.querySelector(".状").textContent = "发布中…";
      fetch(后端 + "/api/dev/chat/confirm", { method: "POST" })
        .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
        .then(function(d){
          卡.querySelector(".头 b").textContent = "已确认 · 任务 #" + (d.任务id || "?");
          卡.querySelector(".状").textContent = "已发布";
          var 排 = 卡.querySelector(".排");
          if (排) 排.remove();
          var 新卡 = 建任务卡(d.任务id);
          流.appendChild(新卡);
          轮询任务卡(新卡, d.任务id);
          流.scrollTop = 流.scrollHeight;
        })
        .catch(function(e){
          卡.querySelector(".状").textContent = "发布失败";
          console.error("道祖确认发布失败", e);
        });
    }
    function 驳回(){
      clearInterval(计时器);
      卡.querySelector(".头 b").textContent = "已驳回";
      卡.querySelector(".状").textContent = "未发布";
      var 排 = 卡.querySelector(".排");
      if (排) 排.remove();
    }
    var 计时器 = setInterval(function(){
      秒 -= 1;
      var 状 = 卡.querySelector(".状");
      if (秒 >= 0 && 状 && 状.textContent.indexOf("待确认") === 0){ 状.textContent = "待确认 " + 秒 + "s"; }
      if (秒 <= 0){ clearInterval(计时器); 确认发布(); }
    }, 1000);
    卡.querySelector(".准").addEventListener("click", function(){ clearInterval(计时器); 确认发布(); });
    卡.querySelector(".否").addEventListener("click", 驳回);
    return 卡;
  }

  function 建任务卡(任务id){
    var 卡 = document.createElement("div");
    卡.className = "任务卡";
    卡.setAttribute("data-任务id", 任务id);
    var 行 = 阶段表.map(function(s, i){
      var 色 = "var(--" + s.五行 + ")";
      return '<div class="阶段" data-序="'+i+'">'+
        '<span class="点" style="background:'+色+'"></span>'+
        '<span class="名">'+s.五行+"·"+s.名+'</span>'+
        '<span class="态">等待</span></div>';
    }).join("");
    卡.innerHTML =
      '<div class="头"><b>任务 #'+任务id+'</b><span class="状">启动中…</span></div>'+
      '<div class="流">'+行+'</div>'+
      '<div class="过程"></div>';
    return 卡;
  }

  function 轮询任务卡(卡, 任务id){
    var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
    var 事件游标 = 0;
    var 过程源 = null;      // EventSource 句柄（SSE 路径）
    var 事件计时器 = null;  // 短轮询回退句柄
    var 状态计时器 = null;
    var 过程区 = 卡.querySelector(".过程");

    // 停止一切：卡片已被移除（/清空）或任务终态时调用，杜绝幽灵轮询
    function 全停(){
      if (状态计时器){ clearInterval(状态计时器); 状态计时器 = null; }
      if (事件计时器){ clearInterval(事件计时器); 事件计时器 = null; }
      if (过程源){ 过程源.close(); 过程源 = null; }
    }

    function 渲染过程行(ev){
      if (!过程区) return;
      if (ev.任务id && ev.任务id !== 任务id) return;
      if (ev.序号 <= 事件游标) return;
      事件游标 = ev.序号;
      var 行 = document.createElement("div");
      行.className = "过程行 " + (ev.类型 || "");
      var 角色 = ev.角色 || "";
      var 工具 = ev.工具名 ? " · " + ev.工具名 : "";
      var 内容 = (ev.内容 || "").replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/\n/g,"<br>");
      行.innerHTML = '<span class="标">'+角色+工具+'</span><span class="内">'+内容+'</span>';
      过程区.appendChild(行);
    }

    // 短轮询回退：SSE 不可用或失败时按 2s 增量拉取
    function 回退过程轮询(){
      if (事件计时器) return;
      事件计时器 = setInterval(function(){
        if (!卡.isConnected){ 全停(); return; }
        fetch(后端 + "/api/dev/pilot/process?since=" + 事件游标)
          .then(function(r){ if(!r.ok) throw 0; return r.json(); })
          .then(function(d){
            (d.事件 || []).forEach(渲染过程行);
            if (过程区) 过程区.scrollTop = 过程区.scrollHeight;
          })
          .catch(function(){});
      }, 2000);
    }

    // AG-UI 渲染器：按 type 分发聚合为思考块/工具卡/答复块
    function 建agui渲染器(过程区){
      var 块 = { 思:null, 具:null, 复:null, 复文:"" };
      function 新行(cls, 标){
        var 行 = document.createElement("div");
        行.className = "过程行 " + cls;
        var 签 = document.createElement("span");
        签.className = "标";
        签.textContent = 标;
        行.appendChild(签);
        过程区.appendChild(行);
        return 行;
      }
      return function(ev){
        if (!卡.isConnected){ 全停(); return; }
        switch(ev.type){
          case "REASONING_MESSAGE_START":
            块.思 = 新行("思考", "思考");
            var 折 = document.createElement("details");
            折.className = "思"; 折.open = true;
            折.innerHTML = '<summary>思考过程</summary><div class="内"></div>';
            块.思.appendChild(折);
            break;
          case "REASONING_MESSAGE_CONTENT":
            if (块.思){
              var 思内 = 块.思.querySelector(".内");
              if (思内) 思内.textContent += (ev.delta || "");
            }
            break;
          case "REASONING_MESSAGE_END":
            块.思 = null;
            break;
          case "TOOL_CALL_START":
            块.具 = 新行("工具调用", ev.toolCallName || "工具");
            var 码 = document.createElement("pre");
            码.className = "内";
            块.具.appendChild(码);
            break;
          case "TOOL_CALL_ARGS":
            if (块.具){
              var 码内 = 块.具.querySelector(".内");
              if (码内) 码内.textContent += (ev.delta || "");
            }
            break;
          case "TOOL_CALL_END":
            块.具 = null;
            break;
          case "TOOL_CALL_RESULT":
            var 果行 = 新行("工具结果", "结果");
            var 果 = document.createElement("span");
            果.className = "内";
            果.textContent = (ev.content || "");
            果行.appendChild(果);
            break;
          case "TEXT_MESSAGE_START":
            块.复文 = "";
            块.复 = 新行("任务答复", "答复");
            var 文 = document.createElement("span");
            文.className = "内";
            块.复.appendChild(文);
            break;
          case "TEXT_MESSAGE_CONTENT":
            块.复文 += (ev.delta || "");
            if (块.复){
              var 文内 = 块.复.querySelector(".内");
              if (文内) 文内.innerHTML = 净化(块.复文);
            }
            break;
          case "TEXT_MESSAGE_END":
            块.复 = null;
            break;
        }
      };
    }

    // 订阅过程事件 SSE；失败降级短轮询
    function 订阅过程流(){
      if (typeof EventSource === "undefined"){ 回退过程轮询(); return; }
      var 渲染agui = 建agui渲染器(过程区);
      过程源 = new EventSource(后端 + "/api/dev/stream/agui?since=" + 事件游标);
      过程源.onmessage = function(e){
        if (!卡.isConnected){ 全停(); return; }
        try {
          渲染agui(JSON.parse(e.data));
          if (过程区) 过程区.scrollTop = 过程区.scrollHeight;
        } catch(_){}
      };
      过程源.onerror = function(){
        if (过程源){ 过程源.close(); 过程源 = null; }
        if (卡.isConnected){ 回退过程轮询(); }
      };
    }

    // 1. 轮询任务状态（每3秒）驱动五行阶段卡
    状态计时器 = setInterval(function(){
      if (!卡.isConnected){ 全停(); return; }
      fetch(后端 + "/api/board/" + 任务id)
        .then(function(r){ if(!r.ok) throw 0; return r.json(); })
        .then(function(t){
          var 状态 = t.status || t.状态 || "";
          var 序 = 状态到阶段(状态);
          var 结果 = 状态到结果(状态);
          var 头 = 卡.querySelector(".头 .状");
          if (头) 头.textContent = 状态;
          var 阶段们 = 卡.querySelectorAll(".阶段");
          阶段们.forEach(function(el, i){
            var 态 = el.querySelector(".态");
            if (i < 序) { el.className = "阶段 done"; 态.textContent = "完成"; }
            else if (i === 序) { el.className = "阶段 active"; 态.textContent = 结果 === "进行" ? "进行中" : (结果 === "重跑" ? "重跑" : "进行"); }
            else { el.className = "阶段"; 态.textContent = "等待"; }
          });
          if (状态 === "已完成" || 状态 === "清理完成"){
            if (头) 头.textContent = "✓ " + 状态;
            全停();
          }
        })
        .catch(function(){});
    }, 3000);

    // 2. 订阅执行过程事件
    订阅过程流();
  }

  return {
    建: 建,
    渲染: 渲染,
    render_welcome_panel: render_welcome_panel,
    parse_command: parse_command,
    cmd_clear: cmd_clear,
    cmd_refresh: cmd_refresh,
    register_command: register_command,
    dispatch_command: dispatch_command,
    on_unknown_command: on_unknown_command
  };
})();

// 经挂载契约入园（主区）
乾坤界面.挂载("主区", 乾坤对话.建);
