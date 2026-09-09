/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 主区对话流 实现
   职责：渲染用户/智能体消息（含头像）、工具卡、运行测试卡、审批区、
   系统消息与输入条，经挂载契约进入主区槽位；支持发送新消息。
   ═══════════════════════════════════════════════════════════ */
window.乾坤对话 = (function(){
  // ── 会话状态：空态启动，所有内容由后续真实交互（用户发送 / 智能体回复 / 看板过程事件）填充 ──
  var 数据 = [];

  // ── SVGs ──
  var I = {
    查: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>',
    编: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg>',
    智: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><rect x="4" y="7" width="16" height="12" rx="2"/><path d="M12 7V4M8 4h8"/><circle cx="9" cy="13" r="1"/><circle cx="15" cy="13" r="1"/><path d="M9 16.5h6"/></svg>',
    发: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12l18-9-6 18-3-7z"/></svg>',
    附: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M21 11l-9 9a5 5 0 1 1-7-7l10-10a3.5 3.5 0 0 1 5 5L10 18"/></svg>',
    具: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M14 3l7 7-4 4-7-7zM3 21l8-4-4-4z"/></svg>',
    引: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M7 7h4v4H7zM13 7h4v4h-4zM7 13h4v4H7zM13 13h4v4h-4z"/></svg>',
  };

  // ── 快捷指令注册表（可扩展）：内置 /清空、/刷新 两条；外部可继续 register_command ──
  var 指令表 = {
    "清空": function(ctx){ return Promise.resolve(cmd_clear(ctx)); },
    "刷新": function(ctx){ return cmd_refresh(ctx); }
  };

  // 注册自定义快捷指令（用于扩展更多 / 指令）
  function register_command(名, 处理器){
    if (!名 || typeof 处理器 !== "function"){ return; }
    指令表[String(名).replace(/^\//, "")] = 处理器;
  }

  // 解析快捷指令：/开头文本拆为 { name, args }；不匹配返回 null
  function parse_command(输入){
    var 文 = String(输入 == null ? "" : 输入).trim();
    if (!文 || 文.charAt(0) !== "/"){ return null; }
    var 段 = 文.replace(/^\/+/, "").split(/\s+/);
    var 名 = 段.shift();
    if (!名){ return null; }
    return { name: 名, args: 段 };
  }

  // /清空：清空当前会话的消息列表渲染与数据；保留会话标识与会话元数据
  function cmd_clear(ctx){
    ctx = ctx || {};
    // 会话标识：当前会话ID（来自外部依赖或顶部状态），存在则保留以便恢复上下文
    var 会话id = (ctx.会话id != null ? ctx.会话id : (window.乾坤对话.会话id || ""));
    var 会话元数据 = window.乾坤对话.会话元数据 || null;
    数据.length = 0;
    var 流 = ctx.流 || document.querySelector(".对话外壳 .对话流");
    if (流){
      流.innerHTML = "";
      流.classList.add("空");
      流.classList.remove("有内容");
      render_welcome_panel(流);
      流.scrollTop = 0;
    }
    // 会话标识与会话元数据保留（仅清空消息列表渲染与数据）
    window.乾坤对话.会话id = 会话id;
    window.乾坤对话.会话元数据 = 会话元数据;
  }

  // /刷新：触发当前会话消息的重新拉取；保持渲染与原发送链路一致
  function cmd_refresh(ctx){
    ctx = ctx || {};
    var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
    var 会话id = (ctx.会话id != null ? ctx.会话id : (window.乾坤对话.会话id || ""));
    // 取最近一次驱动会话ID（看板驱动台保留的当前会话）作为真实拉取目标
    var 真实会话id = 会话id || (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.当前会话id) || "";
    // 不引入新依赖：复用道祖已有 /api/dev/chat 回放历史会话（看板驱动台记录）
    var 端点 = 后端 + "/api/dev/sessions/" + encodeURIComponent(真实会话id || "current");
    return fetch(端点)
      .then(function(r){ return r.ok ? r.json() : { 事件: [] }; })
      .then(function(d){
        数据.length = 0;
        var 事件 = d.事件 || [];
        // 把事件流折叠为对话消息（仅文本类）
        事件.forEach(function(ev){
          if (ev.内容 && (ev.类型 === "任务答复" || ev.类型 === "思考" || ev.类型 === "工具结果")){
            数据.push({ 类:"讯息", 方:"对", 名:ev.角色 || "系统", 时:"刚刚", 文:ev.内容.replace(/</g,"&lt;") });
          }
        });
        var 流 = ctx.流 || document.querySelector(".对话外壳 .对话流");
        if (流){
          流.innerHTML = "";
          if (数据.length){
            流.classList.remove("空");
            流.classList.add("有内容");
            数据.forEach(function(m){ 流.appendChild(渲染(m)); });
            流.scrollTop = 流.scrollHeight;
          } else {
            流.classList.add("空");
            流.classList.remove("有内容");
            render_welcome_panel(流);
          }
        }
      })
      .catch(function(){
        // 静默降级：不抛错，不影响后续输入；聊天面板保持可见
      });
  }

  // 未知指令：在对话区给出友好提示，不抛错、不影响后续输入
  function on_unknown_command(名, 流){
    if (流){
      流.classList.remove("空");
      流.classList.add("有内容");
      var 泡 = document.createElement("div");
      泡.className = "讯";
      // 净化指令名后再拼接，避免 XSS（用户输入的命令名不可信）
      var 转义 = 净化(String(名 || ""));
      var 时间 = document.createElement("span");
      时间.textContent = "系统 · 刚刚";
      泡.appendChild(时间);
      泡.appendChild(document.createTextNode("未知指令 /"));
      var 加粗 = document.createElement("b");
      加粗.textContent = 转义;
      泡.appendChild(加粗);
      泡.appendChild(document.createTextNode("，已忽略。可用：/清空、/刷新"));
      流.appendChild(泡);
      流.scrollTop = 流.scrollHeight;
    }
  }

  // 指令分发：解析 → 路由到内置或注册处理器；统一异常捕获与提示反馈
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

  // 首屏欢迎语：消息列表为空时调用；展示引导文案与快捷指令按钮
  function render_welcome_panel(容器){
    if (!容器){ return; }
    容器.innerHTML = "";
    容器.classList.add("空");
    容器.classList.remove("有内容");
    var 欢迎 = document.createElement("div");
    欢迎.className = "欢迎面板";
    欢迎.innerHTML =
      '<div class="欢迎-顶">' +
        '<span class="欢迎-徽">紫霄宫</span>' +
        '<span class="欢迎-名">洪荒·世界</span>' +
      '</div>' +
      '<div class="欢迎-标">欢迎来到乾坤界面</div>' +
      '<div class="欢迎-次">与道祖对话，发布任务进入五行流转；也可使用下方快捷指令。</div>' +
      '<div class="快捷面板">' +
        '<button class="快捷-项" data-cmd="清空"><span class="快捷-名">/清空</span><span class="快捷-描">清空当前会话</span></button>' +
        '<button class="快捷-项" data-cmd="刷新"><span class="快捷-名">/刷新</span><span class="快捷-描">重新拉取最新消息</span></button>' +
      '</div>';
    容器.appendChild(欢迎);
    // 绑定点击 → 写入输入框并触发分发
    // 注意：textarea 引用需在挂载到主区后才能稳定查询；首次空态渲染时输入框可能未挂载，按需重查
    var 框 = ctx_取输入框();
    var 流 = 容器;
    function ctx_取输入框(){
      return document.querySelector(".对话外壳 .对话输入 textarea");
    }
    Array.prototype.forEach.call(欢迎.querySelectorAll(".快捷-项"), function(btn){
      btn.addEventListener("click", function(){
        var 名 = btn.getAttribute("data-cmd") || "";
        var 框2 = ctx_取输入框();
        if (框2){ 框2.value = "/" + 名; 框2.focus(); }
        dispatch_command({ name: 名, args: [] }, { 流: 流 });
      });
    });
  }

  // ── 渲染入口：挂到主区 ──
  function 建(){
    var 外 = document.createElement("div");
    外.className = "对话外壳 主区视图";
    外.setAttribute("data-视图", "对话");

    // 标题行（无当前任务时显示空态提示）
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
    // 空态：调用欢迎面板组件；非空态：渲染历史
    if (!数据.length){
      render_welcome_panel(流);
    } else {
      流.classList.add("有内容");
      流.classList.remove("空");
      数据.forEach(function(m){ 流.appendChild(渲染(m)); });
    }
    外.appendChild(流);
    流.scrollTop = 流.scrollHeight;

    // 输入条
    var 输 = document.createElement("div");
    输.className = "对话输入";
    输.innerHTML =
      '<div class="行">' +
        '<button class="ico" title="附件">' + I.附 + '</button>' +
        '<button class="ico" title="工具">' + I.具 + '</button>' +
        '<button class="ico" title="引用">' + I.引 + '</button>' +
        '<span class="ctx" id="对话-ctx"><i class="点"></i>未发布任务</span>' +
        '<span style="margin-left:auto"></span>' +
        '<button class="迷你 选">先审后写</button><button class="迷你">自动测试</button><button class="迷你">自动提交</button>' +
      '</div>' +
      '<div class="区"><textarea rows="2" placeholder="描述你的需求 · Enter 发送 · Shift+Enter 换行"></textarea><button class="发">发送' + I.发 + '</button></div>' +
      '<div class="足"><span>⌘↵ 发送 · Esc 取消</span></div>';
    外.appendChild(输);

    // 发送逻辑：真实调用后端 道祖对话 /api/dev/chat
    var 框 = 输.querySelector("textarea");
    var 发 = 输.querySelector(".发");
    function 发送(){
      var 文 = 框.value.trim();
      if (!文){ return; }

      // 识别 / 指令：分发到内置或注册指令处理器
      var 解析 = parse_command(文);
      if (解析){
        dispatch_command(解析, { 流: 流 });
        框.value = "";
        流.scrollTop = 流.scrollHeight;
        return;
      }

      流.appendChild(渲染({ 类:"讯息", 方:"己", 名:"道祖", 字:"九", 时:"刚刚", 文:文.replace(/</g,"&lt;") }));
      框.value = "";
      流.scrollTop = 流.scrollHeight;

      // 加载占位
      var 载 = 流.appendChild(加载中());
      流.scrollTop = 流.scrollHeight;

      // 调接口：流式优先（POST /api/dev/chat/stream 打字机式），失败降级原同步 /api/dev/chat
      var 后端 = (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
      流式对话(后端, 文, 流, 载);
    }
    发.addEventListener("click", 发送);
    框.addEventListener("keydown", function(e){
      if (e.key === "Enter" && !e.shiftKey){ e.preventDefault(); 发送(); }
    });

    return 外;
  }

  // ── 流式对话：POST /api/dev/chat/stream，按帧解析 SSE，打字机式追加 ──
  function 流式对话(后端, 文, 流, 载){
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
      var 泡 = null;       // 回复气泡 DOM（首 delta 到达时建立）
      var p = null;        // 气泡内 <p> 文本容器
      var 回复文本 = "";
      var 任务id = null;
      var 阶段 = "接待中";   // RUN_FINISHED 携带的会话阶段：接待中 / 待确认

      function 建泡(){
        if (泡) return;
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
        if (!泡 && !任务id){
          流.appendChild(渲染({ 类:"讯息", 方:"对", 名:"道祖", 时:"刚刚", 文:"（无回复）" }));
        }
        if (任务id){
          var 卡 = 建任务卡(任务id);
          流.appendChild(卡);
          流.scrollTop = 流.scrollHeight;
          轮询任务卡(卡, 任务id);
        } else if (阶段 === "待确认"){
          // 道祖已对齐需求，处于待确认：建确认卡，5 秒无人确认则自动确认发布建任务卡
          var 确认卡 = 建待确认卡(后端, 流);
          流.appendChild(确认卡);
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
            var 数据 = "";
            帧.split("\n").forEach(function(行){
              if (行.indexOf("data:") === 0){ 数据 += 行.slice(5).trim(); }
            });
            if (!数据) return;
            try {
              var ev = JSON.parse(数据);
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
      // 降级：回退原同步 /api/dev/chat（整体返回）
      同步对话(后端, 文, 流, 载);
    });
  }

  // ── 同步对话：原 /api/dev/chat 整体返回（流式不可用时的降级路径） ──
  function 同步对话(后端, 文, 流, 载){
    fetch(后端 + "/api/dev/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ 消息: 文 })
    })
    .then(function(r){ if (!r.ok){ throw new Error("HTTP " + r.status); } return r.json(); })
    .then(function(d){
      载.remove();
      流.appendChild(渲染({ 类:"讯息", 方:"对", 名:"道祖", 时:"刚刚", 文:净化(d.回复 || "（无回复）") }));
      流.scrollTop = 流.scrollHeight;
      if (d.任务id){
        var 卡 = 建任务卡(d.任务id);
        流.appendChild(卡);
        流.scrollTop = 流.scrollHeight;
        轮询任务卡(卡, d.任务id);
      } else if (d.阶段 === "待确认"){
        var 确认卡 = 建待确认卡(后端, 流);
        // 建待确认卡 内含 appendChild；此处理 5 秒自动确认
      }
    })
    .catch(function(){
      载.remove();
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
    容.innerHTML = '<div class="谁"><b class="名">道祖</b><span class="时">思考中…</span></div><div class="气泡 载">· · ·</div>';
    return 容;
  }

  // ── 净化回复：转义 HTML + 折叠 <think> 思考段 + 换行转 <br> ──
  function 净化(文本){
    var 转义 = String(文本)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    转义 = 转义.replace(/&lt;think&gt;([\s\S]*?)&lt;\/think&gt;/g, function(全, 内){
      return '<details class="思"><summary>思考过程</summary><div>' + 内.replace(/\n/g, "<br>") + '</div></details>';
    });
    转义 = 转义.replace(/\n/g, "<br>");
    return 转义;
  }

  // ── 单条消息渲染 ──
  function 渲染(m){
    if (m.类 === "讯"){
      var s = document.createElement("div");
      s.className = "讯";
      s.innerHTML = '<span>' + m.时 + '</span>' + m.文本;
      return s;
    }
    // 讯息
    var 容 = document.createElement("div");
    容.className = "讯息 " + (m.方 === "己" ? "己" : "对");

    var 谁 = document.createElement("div");
    谁.className = "谁";
    var 像 = document.createElement("span");
    像.className = "像";
    像.innerHTML = m.方 === "己" ? (m.字 || "道") : I.智;
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
      谁.innerHTML += '<span class="徽">火</span>';
    }
    容.appendChild(谁);

    var 泡 = document.createElement("div");
    泡.className = "气泡";
    var p = document.createElement("p");
    p.innerHTML = m.文;
    泡.appendChild(p);

    if (m.文追){
      var p2 = document.createElement("p");
      p2.innerHTML = m.文追;
      泡.appendChild(p2);
    }

    if (m.工具){
      m.工具.forEach(function(t){
        var 卡 = document.createElement("div");
        卡.className = "工具";
        var 头 = document.createElement("header");
        var 图 = t.名 === "编辑文件" ? I.编 : (t.名 === "运行测试" ? I.查 : I.查);
        头.innerHTML = 图 + '<span class="名">' + t.名 + '</span>';
        var 状 = document.createElement("span");
        状.className = "状 " + (t.状类 || "");
        状.innerHTML = (t.状类 === "行" ? '<i class="轮"></i>' : '') + t.状;
        头.appendChild(状);
        var 耗 = document.createElement("span");
        耗.className = "ms";
        耗.textContent = (t.耗 || "");
        头.appendChild(耗);
        卡.appendChild(头);

        if (t.测){
          var ul = document.createElement("ul");
          ul.className = "测";
          t.测.forEach(function(行){
            var li = document.createElement("li");
            li.innerHTML = '<i></i>' + 行;
            ul.appendChild(li);
          });
          卡.appendChild(ul);
        } else {
          var pre = document.createElement("pre");
          pre.innerHTML = t.码;
          卡.appendChild(pre);
        }
        泡.appendChild(卡);
      });
    }

    if (m.审批){
      var 审 = document.createElement("div");
      审.className = "审批";
      审.innerHTML = '<header><span class="叹">!</span>需要决策者确认 · Diff 在右侧呈现</header><div class="描">' + m.审批.描 + '</div><div class="排"><button class="准">批准执行</button><button class="否">驳回</button><button>查看 Diff →</button></div>';
      泡.appendChild(审);
    }

    容.appendChild(泡);
    return 容;
  }

  // ── 五行任务进度卡 ──
  // 状态→五行阶段映射：木(道祖) 火(圣人) 土(大罗) 金(准圣) 水(太乙)
  var 阶段表 = [
    { 五行:"木", 名:"道祖", 状态:["待受理","待道祖澄清","道祖澄清中"] },
    { 五行:"火", 名:"圣人", 状态:["待圣人设计","圣人设计中","待重新设计"] },
    { 五行:"土", 名:"大罗", 状态:["待大罗金仙实现","大罗金仙实现中","待修复","待重新实现"] },
    { 五行:"金", 名:"准圣", 状态:["待准圣验收","准圣验收中","待道祖终审","道祖终审中","待重新验收","已完成"] },
    { 五行:"水", 名:"太乙", 状态:["待清理","清理中","待重新清理","清理完成"] },
  ];
  var 五行色 = { 木:"#6fbf73", 火:"#e87a4a", 土:"#d8b36a", 金:"#c8c8d4", 水:"#6fb0d3" };

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

  // ── 待确认卡片：道祖已对齐需求，等待用户确认（批准/驳回 + 5 秒自动确认） ──
  // 无人操作时 5 秒后自动调 /api/dev/chat/confirm 确认发布建任务卡（默认确认）。
  function 建待确认卡(后端, 流){
    var 卡 = document.createElement("div");
    卡.className = "任务卡 待确认";
    var 秒 = 5;
    卡.innerHTML =
      '<div class="头"><b>道祖已对齐需求</b><span class="状">待确认 '+秒+'s</span></div>'+
      '<div class="描">确认后即安排圣人设计、大罗金仙实现、准圣验收。</div>'+
      '<div class="排"><button class="准">确认发布</button><button class="否">驳回</button></div>';
    流.appendChild(卡);

    // 确认发布：调 confirm 接口，成功后建任务卡并轮询
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
          新卡.style.display = "none";
          流.appendChild(新卡);
          新卡.style.display = "";
          轮询任务卡(新卡, d.任务id);
          流.scrollTop = 流.scrollHeight;
          // 需求已确认发布，看板同步刷新（若界面在看板页）
          if (window.乾坤界面 && window.乾坤界面.切换) { /* 看板页自行刷新 */ }
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
      var 色 = 五行色[s.五行];
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
    // 过程事件游标：SSE 与短轮询共用，序号单调递增，重连时据此去重
    var 事件游标 = 0;
    var 过程源 = null;      // EventSource 句柄（SSE 路径）
    var 事件计时器 = null;  // 短轮询回退句柄
    var 过程区 = 卡.querySelector(".过程");

    // 渲染一条过程事件：按任务过滤 + 序号去重（防 SSE 重连重复推送）
    function 渲染过程行(ev){
      if (!过程区) return;
      if (ev.任务id && ev.任务id !== 任务id) return;
      if (ev.序号 <= 事件游标) return;
      事件游标 = ev.序号;
      var 行 = document.createElement("div");
      行.className = "过程行 " + (ev.类型 || "");
      var 角色 = ev.角色 || "";
      var 工具 = ev.工具名 ? " · " + ev.工具名 : "";
      var 内容 = (ev.内容 || "").replace(/</g,"&lt;").replace(/\n/g,"<br>");
      行.innerHTML = '<span class="标">'+角色+工具+'</span><span class="内">'+内容+'</span>';
      过程区.appendChild(行);
    }

    // 短轮询回退：SSE 不可用或失败时按 2s 增量拉取
    function 回退过程轮询(){
      if (事件计时器) return;
      事件计时器 = setInterval(function(){
        fetch(后端 + "/api/dev/pilot/process?since=" + 事件游标)
          .then(function(r){ if(!r.ok) throw 0; return r.json(); })
          .then(function(d){
            (d.事件 || []).forEach(渲染过程行);
            if (过程区) 过程区.scrollTop = 过程区.scrollHeight;
          })
          .catch(function(){});
      }, 2000);
    }

    // AG-UI 渲染器：按 type 分发，三段式聚合为思考块/工具卡/答复块（消费 /api/dev/stream/agui 标准词）
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

    // 订阅过程事件 SSE 长连接（AG-UI 标准词）；失败降级短轮询（中文词）
    function 订阅过程流(){
      if (typeof EventSource === "undefined"){ 回退过程轮询(); return; }
      var 渲染agui = 建agui渲染器(过程区);
      过程源 = new EventSource(后端 + "/api/dev/stream/agui?since=" + 事件游标);
      过程源.onmessage = function(e){
        try {
          渲染agui(JSON.parse(e.data));
          if (过程区) 过程区.scrollTop = 过程区.scrollHeight;
        } catch(_){}
      };
      过程源.onerror = function(){
        if (过程源){ 过程源.close(); 过程源 = null; }
        回退过程轮询();
      };
    }

    // 1. 轮询任务状态（每3秒）：驱动五行阶段卡（保留任务详情兜底，阶段推进本身为秒级）
    var 状态计时器 = setInterval(function(){
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
            if (头) 头.textContent = "✅ " + 状态;
            clearInterval(状态计时器);
            if (过程源){ 过程源.close(); 过程源 = null; }
            if (事件计时器){ clearInterval(事件计时器); 事件计时器 = null; }
          }
        })
        .catch(function(){});
    }, 3000);

    // 2. 订阅执行过程事件：优先 SSE 长连接，降级短轮询
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