/* ═══════════════════════════════════════════════════════════
   乾坤 · 功能组件-殿 —— 设置面板 接入组件（密钥/接入向导/智能体绑定）
   职责（面板.js 消费本组件 API，故脚本聚合顺序须在本组件之后）：
   1. 密钥按钮：供应商组头展开密钥行 → discover 真实校验 + connect 保存；
   2. 接入入口：模板/自定义 → 密钥 → 获取模型 → 接入；
   3. 智能体区块：每身份绑定独立模型（agents / bind / unbind）。
   铁律：密钥一律 password 输入，绝不回显已存密钥。
   ═══════════════════════════════════════════════════════════ */
window.乾坤接入 = (function(){
  // ── 基础：后端地址 / 转义 / fetch 封装（与面板.js 同模式）──
  function 后端(){
    return (typeof window.乾坤配置 !== "undefined" && window.乾坤配置.道祖) || "";
  }
  function 名(s){
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
  function 请求(路径, 方法, 体){
    return fetch(后端() + 路径, {
      method: 方法,
      headers: { "Content-Type": "application/json" },
      body: 体 === undefined ? undefined : JSON.stringify(体)
    }).then(function(r){
      if (!r.ok){ throw new Error("HTTP " + r.status); }
      return r.json();
    });
  }
  function 反馈写(元素, 文, 坏){
    元素.textContent = 文;
    元素.classList.toggle("坏", !!坏);
  }
  function 下拉填模型(选, 模型们, 选中){
    选.innerHTML = "";
    (模型们 || []).forEach(function(m){
      var o = document.createElement("option");
      o.value = m.id;
      o.textContent = m.id;
      if (m.id === 选中){ o.selected = true; }
      选.appendChild(o);
    });
  }

  /* ═══ 1. 密钥按钮：供应商组头按钮 → 展开密钥行 ═══ */
  function 密钥按钮(信息, 组, 刷新){
    var 钮 = document.createElement("button");
    钮.type = "button";
    钮.className = "设置-密钥钮";
    钮.textContent = "密钥";
    钮.title = "修改该供应商 API 密钥";
    钮.addEventListener("click", function(){
      var 已有 = 组.querySelector(".设置-密钥行");
      if (已有){ 已有.remove(); return; }
      组.appendChild(密钥行(信息, 刷新));
    });
    return 钮;
  }

  // 密钥行：discover 真实校验（失败展示后端错误原文）→ connect 重名覆盖保存
  function 密钥行(信息, 刷新){
    var 行 = document.createElement("div");
    行.className = "设置-密钥行";
    行.innerHTML =
      "<input class=\"设置-密钥输入\" type=\"password\" placeholder=\"API 密钥（env:变量名 引用可重启恢复；明文仅会话内）\">" +
      "<button class=\"设置-密钥保存\" type=\"button\">验证并保存</button>" +
      "<span class=\"设置-密钥反馈\"></span>";
    var 输入 = 行.querySelector(".设置-密钥输入");
    var 保存钮 = 行.querySelector(".设置-密钥保存");
    var 反馈 = 行.querySelector(".设置-密钥反馈");
    保存钮.addEventListener("click", function(){
      var 密钥 = 输入.value.trim();
      if (!密钥){ 反馈写(反馈, "请输入密钥", true); return; }
      保存钮.disabled = true;
      反馈写(反馈, "验证中…");
      // discover：地址省略 → 后端回退池内已存地址
      请求("/api/llm/discover", "POST", { 供应商: 信息.名称, 密钥: 密钥 })
        .then(function(d){
          if (d.错误){ throw new Error(d.错误); }
          反馈写(反馈, "验证通过（" + 名(d.来源) + " · " + (d.模型 || []).length + " 个模型），保存中…");
          // connect：重名覆盖更新池内密钥并选中该供应商
          return 请求("/api/llm/connect", "POST",
            { 名称: 信息.名称, 地址: 信息.地址, 密钥: 密钥, 模型: 信息.模型 });
        })
        .then(function(){
          反馈写(反馈, "已保存（该供应商已选中）");
          输入.value = "";
          if (刷新){ 刷新(); }
        })
        .catch(function(e){ 反馈写(反馈, "失败：" + e.message, true); })
        .then(function(){ 保存钮.disabled = false; });
    });
    return 行;
  }

  /* ═══ 2. 接入入口：向导（模板/自定义 → 密钥 → 获取模型 → 接入）═══ */
  function 接入入口(容器, 刷新){
    var 钮 = document.createElement("button");
    钮.type = "button";
    钮.className = "设置-接入钮";
    钮.textContent = "＋ 接入供应商";
    钮.addEventListener("click", function(){
      var 已有 = 容器.querySelector(".设置-向导");
      if (已有){ 已有.remove(); return; }
      容器.appendChild(向导(刷新));
    });
    return 钮;
  }

  function 向导(刷新){
    var w = document.createElement("div");
    w.className = "设置-向导";
    w.innerHTML =
      "<div class=\"设置-向导头\"><b>接入供应商</b>" +
        "<button class=\"设置-向导关\" type=\"button\" aria-label=\"关闭\">✕</button></div>" +
      "<div class=\"设置-向导行\">" +
        "<select class=\"设置-向导模板\"><option value=\"\">自定义供应商…</option></select>" +
        "<input class=\"设置-向导名\" placeholder=\"供应商名称（自定义时必填）\">" +
      "</div>" +
      "<div class=\"设置-向导行\">" +
        "<input class=\"设置-向导址\" placeholder=\"API 地址（https://... 或 .../chat/completions）\">" +
      "</div>" +
      "<div class=\"设置-向导行\">" +
        "<input class=\"设置-向导钥\" type=\"password\" placeholder=\"API 密钥（env:变量名 引用可重启恢复）\">" +
      "</div>" +
      "<div class=\"设置-向导行\">" +
        "<button class=\"设置-向导取模\" type=\"button\">获取可用模型</button>" +
        "<select class=\"设置-向导模型\"><option value=\"\">— 先获取模型 —</option></select>" +
      "</div>" +
      "<div class=\"设置-向导行\">" +
        "<button class=\"设置-向导接\" type=\"button\">接入</button>" +
        "<span class=\"设置-向导反馈\"></span>" +
      "</div>";
    var 模板选 = w.querySelector(".设置-向导模板");
    var 名入 = w.querySelector(".设置-向导名");
    var 址入 = w.querySelector(".设置-向导址");
    var 钥入 = w.querySelector(".设置-向导钥");
    var 取模钮 = w.querySelector(".设置-向导取模");
    var 模选 = w.querySelector(".设置-向导模型");
    var 接钮 = w.querySelector(".设置-向导接");
    var 反馈 = w.querySelector(".设置-向导反馈");

    // 模板清单（含环境变量提示）；拉取失败时追加提示选项
    请求("/api/llm/templates", "GET").then(function(d){
      (d.模板 || []).forEach(function(t){
        var o = document.createElement("option");
        o.value = t.名称;
        o.textContent = t.显示名 + (t.环境变量 ? "（env:" + t.环境变量 + "）" : "");
        o.setAttribute("data-址", t.地址 || "");
        模板选.appendChild(o);
      });
    }).catch(function(){
      var o = document.createElement("option");
      o.disabled = true;
      o.textContent = "（模板加载失败，请手动填写）";
      模板选.appendChild(o);
    });

    模板选.addEventListener("change", function(){
      var o = 模板选.options[模板选.selectedIndex];
      名入.value = o.value ? o.value : "";
      址入.value = o.value ? (o.getAttribute("data-址") || "") : "";
    });

    // 获取可用模型：模板命中零网络 / 自定义网络探测
    取模钮.addEventListener("click", function(){
      var 供 = 模板选.value;
      var 址 = 址入.value.trim();
      if (!供 && !址){ 反馈写(反馈, "请选择模板或填写 API 地址", true); return; }
      取模钮.disabled = true;
      反馈写(反馈, "获取模型中…");
      请求("/api/llm/discover", "POST", {
        供应商: 供 || undefined,
        地址: 址 || undefined,
        密钥: 钥入.value.trim() || undefined
      })
      .then(function(d){
        if (d.错误){ throw new Error(d.错误); }
        下拉填模型(模选, d.模型, "");
        反馈写(反馈, "已获取 " + 模选.options.length + " 个模型（来源 " + 名(d.来源) + "）");
      })
      .catch(function(e){ 反馈写(反馈, "获取失败：" + e.message, true); })
      .then(function(){ 取模钮.disabled = false; });
    });

    // 接入：注册进池 + 选中；env: 引用落盘（llm-接入.json）重启恢复
    接钮.addEventListener("click", function(){
      var 请求体 = {
        名称: (模板选.value || 名入.value).trim(),
        地址: 址入.value.trim(),
        密钥: 钥入.value.trim(),
        模型: 模选.value
      };
      if (!请求体.名称){ 反馈写(反馈, "请选择模板或填写供应商名称", true); return; }
      if (!请求体.地址){ 反馈写(反馈, "请填写 API 地址", true); return; }
      if (!请求体.密钥){ 反馈写(反馈, "请输入密钥", true); return; }
      if (!请求体.模型){ 反馈写(反馈, "请先获取可用模型并选择", true); return; }
      接钮.disabled = true;
      反馈写(反馈, "接入中…");
      请求("/api/llm/connect", "POST", 请求体)
        .then(function(){
          反馈写(反馈, "已接入并选中（" + 名(请求体.名称) + " · " + 名(请求体.模型) + "）");
          钥入.value = "";
          if (刷新){ 刷新(); }
        })
        .catch(function(e){ 反馈写(反馈, "接入失败：" + e.message, true); })
        .then(function(){ 接钮.disabled = false; });
    });

    w.querySelector(".设置-向导关").addEventListener("click", function(){ w.remove(); });
    return w;
  }

  /* ═══ 3. 智能体绑定区块：身份 × 绑定（绑定 ?? 全局选择）═══ */
  function 智能体区块(){
    var 块 = document.createElement("div");
    块.className = "设置-绑定";
    块.innerHTML = "<div class=\"设置-绑定空\">加载中…</div>";

    function 拉(){
      Promise.all([
        请求("/api/llm/agents", "GET"),
        请求("/api/llm/status", "GET")
      ]).then(function(组){ 画(组[0].智能体 || [], 组[1]); })
      .catch(function(){
        块.innerHTML = "<div class=\"设置-绑定空\">绑定清单拉取失败（数据服务未在线）</div>";
      });
    }

    function 画(清单, 状态){
      块.innerHTML = "";
      if (!清单.length){
        块.innerHTML = "<div class=\"设置-绑定空\">LLM 池未装配，无法绑定</div>";
        return;
      }
      var 全局 = (状态 && 状态.当前选择) || { 供应商: "", 模型: "" };
      var 供应商清单 = (状态 && 状态.供应商) || [];
      清单.forEach(function(项){ 块.appendChild(绑定行(项, 全局, 供应商清单, 拉)); });
    }

    function 绑定行(项, 全局, 供应商清单, 重画){
      var 行 = document.createElement("div");
      行.className = "设置-绑定行";
      var 说明 = 项.名 === "道祖" ? "主控澄清对话" : "开发执行链";
      var 生效 = 项.绑定
        ? (项.绑定.供应商 + " · " + 项.绑定.模型)
        : (全局.供应商 + " · " + 全局.模型 + "（跟随全局）");
      行.innerHTML =
        "<div class=\"设置-绑定头\"><b>" + 名(项.名) + "</b><em>" + 名(说明) + "</em></div>" +
        "<div class=\"设置-绑定生\"><span>当前生效</span><code>" + 名(生效) + "</code></div>" +
        "<div class=\"设置-绑定选\">" +
          "<select class=\"设置-绑定供\"></select>" +
          "<select class=\"设置-绑定模\"><option value=\"\">— 模型 —</option></select>" +
        "</div>" +
        "<div class=\"设置-绑定制\">" +
          "<button class=\"设置-绑钮\" type=\"button\">绑定</button>" +
          (项.绑定 ? "<button class=\"设置-解钮\" type=\"button\">恢复默认</button>" : "") +
        "</div>";
      var 供选 = 行.querySelector(".设置-绑定供");
      var 模选 = 行.querySelector(".设置-绑定模");
      供应商清单.forEach(function(s){
        var o = document.createElement("option");
        o.value = s.名称;
        o.textContent = s.名称;
        if (项.绑定 && 项.绑定.供应商 === s.名称){ o.selected = true; }
        供选.appendChild(o);
      });
      if (项.绑定){ 拉模型(模选, 项.绑定.供应商, 项.绑定.模型); }
      供选.addEventListener("change", function(){ 拉模型(模选, 供选.value, ""); });

      行.querySelector(".设置-绑钮").addEventListener("click", function(){
        if (!供选.value || !模选.value){ 行错(行, "请选择供应商与模型"); return; }
        行错清(行);
        请求("/api/llm/agent/bind", "POST", { 智能体: 项.名, 供应商: 供选.value, 模型: 模选.value })
          .then(function(){ 重画(); })
          .catch(function(e){ 行错(行, "绑定失败：" + e.message); });
      });
      var 解钮 = 行.querySelector(".设置-解钮");
      if (解钮){
        解钮.addEventListener("click", function(){
          行错清(行);
          请求("/api/llm/agent/unbind", "POST", { 智能体: 项.名 })
            .then(function(){ 重画(); })
            .catch(function(e){ 行错(行, "解绑失败：" + e.message); });
        });
      }
      return 行;
    }

    // 模型下拉：按供应商 discover（池内已存密钥回退，零输入）
    function 拉模型(选, 供应商, 选中){
      选.innerHTML = "<option value=\"\">加载中…</option>";
      请求("/api/llm/discover", "POST", { 供应商: 供应商 })
        .then(function(d){
          if (d.错误){ throw new Error(d.错误); }
          下拉填模型(选, d.模型, 选中);
        })
        .catch(function(){ 选.innerHTML = "<option value=\"\">获取失败</option>"; });
    }

    function 行错(行, 文){
      var t = 行.querySelector(".设置-绑定错");
      if (!t){
        t = document.createElement("div");
        t.className = "设置-绑定错";
        行.appendChild(t);
      }
      t.textContent = 文;
    }
    function 行错清(行){
      var t = 行.querySelector(".设置-绑定错");
      if (t){ t.remove(); }
    }

    拉();
    return 块;
  }

  return {
    密钥按钮: 密钥按钮,
    接入入口: 接入入口,
    智能体区块: 智能体区块
  };
})();
