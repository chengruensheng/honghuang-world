/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 对话流快捷指令 验收脚本
   职责：通过 window.乾坤对话.快捷指令验收 = { 场景列表, 取消息数,
   取会话元数据, 运行场景, 渲染验收面板 } 对外暴露纯只读探针为主，
   含一条受控的 cmd_clear / cmd_refresh 调用路径用于演示与断言。
   依赖：window.乾坤对话.cmd_clear / cmd_refresh / register_command /
   dispatch_command / on_unknown_command / parse_command；既有
   /api/dev/sessions/{id} 后端契约（不新增、不修改）。
   本文件仅做验收探针，不修改任何生产模块运行时行为，不联动真实用户
   会话数据，不注册新指令名，不污染 指令表 内容。
   ═══════════════════════════════════════════════════════════ */
(function(){
  if (typeof window === "undefined" || !window.乾坤对话){ return; }
  if (window.乾坤对话.快捷指令验收){ return; } // 幂等：避免重复挂载

  // ── 内部只读 DOM 探针：纯查询，不写入、不联动真实会话数据 ──
  function 取流容器(){
    return document.querySelector(".对话外壳 .对话流");
  }
  function 取流类名(){
    var 流 = 取流容器();
    return 流 ? 流.className : "";
  }

  // 统计当前流容器内可见消息节点数量（.讯息 与 .讯 都算作消息）
  function 取消息数(){
    var 流 = 取流容器();
    if (!流){ return 0; }
    var 讯息们 = 流.querySelectorAll(".讯息");
    var 讯们 = 流.querySelectorAll(".讯");
    return (讯息们 && 讯息们.length || 0) + (讯们 && 讯们.length || 0);
  }

  // 取欢迎面板节点（如果当前为空态）
  function 取欢迎面板(){
    var 流 = 取流容器();
    if (!流){ return null; }
    return 流.querySelector(".欢迎面板") || 流.querySelector(".欢迎区");
  }

  // 会话元数据：来自 window.乾坤对话 上挂载的会话标识与会话元数据
  function 取会话元数据(){
    var k = window.乾坤对话 || {};
    return {
      会话id: k.会话id != null ? k.会话id : "",
      会话元数据: k.会话元数据 != null ? k.会话元数据 : null,
      数据长度: Array.isArray(k.数据) ? k.数据.length : -1
    };
  }

  // ── 场景列表：覆盖空态/有内容/未知指令三类上下文 ──
  function 场景列表(){
    return [
      {
        编号: "S1",
        标题: "空态下 /清空：保留空态 + 保留会话元数据",
        步骤: [
          "确保当前流为空态（无 .讯息/.讯 节点，且 .欢迎面板 存在）",
          "读取调用前的 消息数 与 会话元数据",
          "调用 cmd_clear({})",
          "再次读取 消息数 与 会话元数据"
        ],
        期望: "调用后 消息数 仍为 0（保持空态/欢迎面板），会话id 与 会话元数据 与调用前一致"
      },
      {
        编号: "S2",
        标题: "有内容下 /清空：消息清零 + 保留会话元数据",
        步骤: [
          "模拟有内容（直接 appendChild 若干 .讯/.讯息 节点，绕过真实数据流）",
          "读取调用前的 消息数 与 会话元数据",
          "调用 cmd_clear({})",
          "再次读取 消息数 与 会话元数据"
        ],
        期望: "调用后 消息数 降为 0（流回到欢迎面板），会话id 与 会话元数据 与调用前一致"
      },
      {
        编号: "S3",
        标题: "/刷新：发起 /api/dev/sessions/{id} 拉取（带断网降级）",
        步骤: [
          "读取调用前的 消息数 与 流类名",
          "调用 cmd_refresh({ 会话id: \"current\" })（后端不存在时静默降级）",
          "等待 fetch promise 落地（约 1.5s）",
          "再次读取 消息数 与 流类名"
        ],
        期望: "调用不抛错到全局；后端不可达时流仍为空态或保持上一态，无未捕获异常；会话元数据保留"
      },
      {
        编号: "S4",
        标题: "未知指令：on_unknown_command 走系统提示分支不抛错",
        步骤: [
          "读取调用前的 消息数 与 流类名",
          "调用 dispatch_command({ name: \"不存在的指令\", args: [] }, { 流: 流 })",
          "再次读取 消息数 与 流类名"
        ],
        期望: "调用不抛错；流新增一条 .讯 系统提示；流类名 出现 \"有内容\" 标记；不影响会话元数据"
      },
      {
        编号: "S5",
        标题: "解析入口：parse_command 正确拆分 /清空 /刷新 /未知",
        步骤: [
          "调用 parse_command(\"/清空\")",
          "调用 parse_command(\"/刷新\")",
          "调用 parse_command(\"/未知\")",
          "调用 parse_command(\"普通文本\")"
        ],
        期望: "前两者 name=清空/刷新；第三个 name=未知；第四个返回 null"
      },
      {
        编号: "S6",
        标题: "欢迎面板快捷按钮：data-cmd 仍指向清空/刷新两条",
        步骤: [
          "读取空态下的 .快捷-项 节点列表",
          "遍历其 data-cmd 属性"
        ],
        期望: "data-cmd 仅包含 清空、刷新 两条；无其它指令注册"
      }
    ];
  }

  // ── 受控调用路径：仅用于演示与断言，不污染真实会话数据 ──
  function 受控调用_clear(){
    if (typeof window.乾坤对话.cmd_clear !== "function"){
      return Promise.reject(new Error("cmd_clear 未暴露"));
    }
    try {
      return Promise.resolve(window.乾坤对话.cmd_clear({})).then(function(){
        return { 通道: "cmd_clear", 调用: "ok" };
      });
    } catch (e){
      return Promise.resolve({ 通道: "cmd_clear", 调用: "异常", 错误: String(e && e.message || e) });
    }
  }
  function 受控调用_refresh(会话id){
    if (typeof window.乾坤对话.cmd_refresh !== "function"){
      return Promise.reject(new Error("cmd_refresh 未暴露"));
    }
    try {
      var p = window.乾坤对话.cmd_refresh({ 会话id: 会话id || "current" });
      return Promise.resolve(p).then(function(){
        return { 通道: "cmd_refresh", 调用: "ok" };
      }).catch(function(e){
        return { 通道: "cmd_refresh", 调用: "降级", 错误: String(e && e.message || e) };
      });
    } catch (e){
      return Promise.resolve({ 通道: "cmd_refresh", 调用: "异常", 错误: String(e && e.message || e) });
    }
  }
  function 受控调用_dispatch_unknown(){
    if (typeof window.乾坤对话.dispatch_command !== "function"){
      return Promise.reject(new Error("dispatch_command 未暴露"));
    }
    var 流 = 取流容器();
    try {
      var p = window.乾坤对话.dispatch_command({ name: "不存在的指令", args: [] }, { 流: 流 });
      return Promise.resolve(p).then(function(){
        return { 通道: "dispatch_unknown", 调用: "ok" };
      }).catch(function(e){
        return { 通道: "dispatch_unknown", 调用: "降级", 错误: String(e && e.message || e) };
      });
    } catch (e){
      return Promise.resolve({ 通道: "dispatch_unknown", 调用: "异常", 错误: String(e && e.message || e) });
    }
  }

  // ── 单场景执行器：返回 { 编号, 通过, 详情, 前, 后 } ──
  function 执行单场景(场景){
    var 前 = {
      消息数: 取消息数(),
      流类名: 取流类名(),
      会话元数据: 取会话元数据()
    };
    var 详情 = { 前: 前, 步: [] };

    function 步(标签, 值){ 详情.步.push({ 标签: 标签, 值: 值 }); }

    if (场景.编号 === "S1"){
      步("调用 cmd_clear", "before=" + 前.消息数 + "/" + 前.流类名);
      return 受控调用_clear().then(function(r){
        步("调用结果", r);
        var 后 = { 消息数: 取消息数(), 流类名: 取流类名(), 会话元数据: 取会话元数据() };
        步("调用后", 后);
        var 通过 = (后.消息数 === 0) &&
          (后.会话元数据.会话id === 前.会话元数据.会话id) &&
          (后.会话元数据.会话元数据 === 前.会话元数据.会话元数据);
        return { 编号: 场景.编号, 通过: 通过, 详情: 详情 };
      });
    }

    if (场景.编号 === "S2"){
      // 受控注入三条 .讯 节点作为“有内容”上下文（仅 DOM 探针层面，不动 数据）
      var 流 = 取流容器();
      if (流){
        for (var i = 0; i < 3; i++){
          var 泡 = document.createElement("div");
          泡.className = "讯";
          var 时间 = document.createElement("span");
          时间.textContent = "受控注入 · " + i;
          泡.appendChild(时间);
          泡.appendChild(document.createTextNode("验证脚本受控注入 #" + i));
          流.appendChild(泡);
        }
      }
      步("受控注入", "3 条 .讯");
      var 前二 = { 消息数: 取消息数(), 流类名: 取流类名(), 会话元数据: 取会话元数据() };
      详情.前 = 前二;
      步("调用 cmd_clear", "before=" + 前二.消息数);
      return 受控调用_clear().then(function(r){
        步("调用结果", r);
        var 后 = { 消息数: 取消息数(), 流类名: 取流类名(), 会话元数据: 取会话元数据() };
        步("调用后", 后);
        var 通过 = (后.消息数 === 0) &&
          (后.会话元数据.会话id === 前二.会话元数据.会话id) &&
          (后.会话元数据.会话元数据 === 前二.会话元数据.会话元数据);
        return { 编号: 场景.编号, 通过: 通过, 详情: 详情 };
      });
    }

    if (场景.编号 === "S3"){
      步("调用 cmd_refresh", "会话id=current");
      return 受控调用_refresh("current").then(function(r){
        步("调用结果", r);
        // 等 1.5s 让 fetch promise 落地
        return new Promise(function(resolve){
          setTimeout(function(){
            var 后 = { 消息数: 取消息数(), 流类名: 取流类名(), 会话元数据: 取会话元数据() };
            步("调用后", 后);
            var 通过 = (后.会话元数据.会话id === 前.会话元数据.会话id) &&
              (后.会话元数据.会话元数据 === 前.会话元数据.会话元数据);
            resolve({ 编号: 场景.编号, 通过: 通过, 详情: 详情 });
          }, 1500);
        });
      });
    }

    if (场景.编号 === "S4"){
      步("调用 dispatch_command({name:不存在的指令})", "流=" + (取流容器() ? "存在" : "缺失"));
      return 受控调用_dispatch_unknown().then(function(r){
        步("调用结果", r);
        var 后 = { 消息数: 取消息数(), 流类名: 取流类名(), 会话元数据: 取会话元数据() };
        步("调用后", 后);
        // 流类名 应出现 有内容；消息数 应大于调用前
        var 通过 = (后.消息数 > 前.消息数) && /有内容/.test(后.流类名);
        return { 编号: 场景.编号, 通过: 通过, 详情: 详情 };
      });
    }

    if (场景.编号 === "S5"){
      if (typeof window.乾坤对话.parse_command !== "function"){
        return Promise.resolve({ 编号: 场景.编号, 通过: false, 详情: { 错: "parse_command 未暴露" } });
      }
      var r1 = window.乾坤对话.parse_command("/清空");
      var r2 = window.乾坤对话.parse_command("/刷新");
      var r3 = window.乾坤对话.parse_command("/未知");
      var r4 = window.乾坤对话.parse_command("普通文本");
      步("parse /清空", r1);
      步("parse /刷新", r2);
      步("parse /未知", r3);
      步("parse 普通文本", r4);
      var 通过 = r1 && r1.name === "清空" &&
        r2 && r2.name === "刷新" &&
        r3 && r3.name === "未知" &&
        r4 === null;
      return Promise.resolve({ 编号: 场景.编号, 通过: 通过, 详情: 详情 });
    }

    if (场景.编号 === "S6"){
      var 欢迎 = 取欢迎面板();
      if (!欢迎){
        步("欢迎面板", "未挂载（可能当前非空态）");
        return Promise.resolve({ 编号: 场景.编号, 通过: false, 详情: 详情 });
      }
      var btns = 欢迎.querySelectorAll(".快捷-项");
      var names = [];
      for (var j = 0; j < btns.length; j++){
        names.push(btns[j].getAttribute("data-cmd") || "");
      }
      步("快捷按钮 data-cmd", names);
      var 通过 = names.length === 2 &&
        names.indexOf("清空") >= 0 &&
        names.indexOf("刷新") >= 0;
      return Promise.resolve({ 编号: 场景.编号, 通过: 通过, 详情: 详情 });
    }

    return Promise.resolve({ 编号: 场景.编号, 通过: false, 详情: { 错: "未知场景编号" } });
  }

  // ── 聚合运行：执行全部场景，按列表返回通过/失败清单 ──
  function 运行场景(编号){
    var 列表 = 场景列表();
    var 目标 = (编号 != null) ? 列表.filter(function(s){ return s.编号 === 编号; }) : 列表;
    var 链 = Promise.resolve([]);
    目标.forEach(function(场景){
      链 = 链.then(function(acc){
        return 执行单场景(场景).then(function(r){
          acc.push(r);
          return acc;
        });
      });
    });
    return 链.then(function(详情列表){
      var 通过数 = 详情列表.filter(function(r){ return r.通过; }).length;
      var 结果 = {
        通过: 通过数 === 详情列表.length,
        通过数: 通过数,
        总数: 详情列表.length,
        详情: 详情列表
      };
      try {
        if (typeof console !== "undefined" && console.table){
          console.groupCollapsed("[快捷指令验收] 运行场景 " + (编号 || "全部") + " · " + 通过数 + "/" + 详情列表.length);
          console.table(详情列表.map(function(r){ return { 编号: r.编号, 通过: r.通过, 步数: (r.详情 && r.详情.步 && r.详情.步.length) || 0 }; }));
          console.groupEnd();
        }
      } catch (e){ /* 控制台不可用时静默 */ }
      return 结果;
    });
  }

  // ── 渲染验收面板：可选挂载点；不传则挂到 .对话外壳 末尾 ──
  function 渲染验收面板(挂载点){
    var 宿主 = 挂载点 || document.querySelector(".对话外壳") || document.body;
    if (!宿主){ return null; }
    var 旧 = 宿主.querySelector(".快捷指令-验收面板");
    if (旧){ 旧.remove(); }

    var 面板 = document.createElement("div");
    面板.className = "快捷指令-验收面板";
    面板.setAttribute("data-验收", "快捷指令");

    var 头 = document.createElement("div");
    头.className = "头";
    头.textContent = "快捷指令验收 · 受控探针";
    面板.appendChild(头);

    var 列 = document.createElement("div");
    列.className = "列表";
    var 列表 = 场景列表();
    列表.forEach(function(场景){
      var 行 = document.createElement("div");
      行.className = "行";
      行.setAttribute("data-编号", 场景.编号);
      var 标 = document.createElement("span");
      标.className = "标";
      标.textContent = 场景.编号 + " · " + 场景.标题;
      行.appendChild(标);
      var 态 = document.createElement("span");
      态.className = "态";
      态.textContent = "待跑";
      行.appendChild(态);
      列.appendChild(行);
    });
    面板.appendChild(列);

    var 控 = document.createElement("div");
    控.className = "控";
    var 全跑 = document.createElement("button");
    全跑.className = "全跑";
    全跑.textContent = "运行全部";
    全跑.addEventListener("click", function(){
      运行场景().then(function(结果){
        摘要.textContent = 结果.通过
          ? ("通过 " + 结果.通过数 + "/" + 结果.总数)
          : ("未通过 " + (结果.总数 - 结果.通过数) + "/" + 结果.总数);
        列表.forEach(function(场景){
          var 行 = 列.querySelector('[data-编号="' + 场景.编号 + '"]');
          if (!行) { return; }
          var r = 结果.详情.find(function(x){ return x.编号 === 场景.编号; });
          var 态 = 行.querySelector(".态");
          if (r){
            态.textContent = r.通过 ? "✓" : "✗";
            态.setAttribute("data-通过", r.通过 ? "1" : "0");
          }
        });
      });
    });
    控.appendChild(全跑);
    var 摘要 = document.createElement("span");
    摘要.className = "摘要";
    摘要.textContent = "未运行";
    控.appendChild(摘要);
    面板.appendChild(控);

    宿主.appendChild(面板);
    return 面板;
  }

  // ── 对外暴露（只读探针为主，含一条受控调用路径）──
  window.乾坤对话.快捷指令验收 = {
    场景列表: 场景列表,
    取消息数: 取消息数,
    取会话元数据: 取会话元数据,
    取流类名: 取流类名,
    取欢迎面板: 取欢迎面板,
    受控调用_clear: 受控调用_clear,
    受控调用_refresh: 受控调用_refresh,
    受控调用_dispatch_unknown: 受控调用_dispatch_unknown,
    运行场景: 运行场景,
    渲染验收面板: 渲染验收面板
  };
})();