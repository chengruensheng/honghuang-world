/* ═══════════════════════════════════════════════════════════
   水镜 · 流光渲染器：LLM 内容流的增量上屏
   铁律一：内容路径零 innerHTML —— 只用 createElement/textContent，
           XSS 免疫由构造保证，不靠净化纪律。
   铁律二：增量有界 —— 完整行固化后永不重渲；重渲只发生在
           单个"未完成行"尾区，工作量 ∝ 本块增量。
   支持语法（v0 极简集，TokUI 兼容语法族）：
     Markdown-lite：# 标题 / - 列表 / **粗体** / `行内码` / ```围栏```
     思考折叠：<think> ... </think>
     DSL-lite：[card tt:标题] ... [/card]
               [btn clk:动作名]文本[/btn]
               [p 文本] / [h1~h6 文本]
   ═══════════════════════════════════════════════════════════ */
window.镜流光 = (function(){
  "use strict";

  function 建(挂点, 动作表){
    var 固流 = el("div", "流-固");
    var 尾区 = el("div", "流-尾");
    挂点.appendChild(固流);
    挂点.appendChild(尾区);

    var 缓冲 = "";          // 未完成行（尾区独占）
    var 栈 = [];            // 容器栈：card / think / 围码
    var 动作 = 动作表 || {};

    function 容(){
      return 栈.length ? 栈[栈.length - 1].点 : 固流;
    }

    function 喂(增量){
      if (!增量){ return; }
      缓冲 += 增量;
      if (缓冲.indexOf("\n") < 0){ 渲尾(); return; }
      var 行们 = 缓冲.split("\n");
      缓冲 = 行们.pop();          // 未完成行留尾区
      行们.forEach(固化行);
      渲尾();
    }

    // ── 完整行处理：状态机（容器标记 → DSL → Markdown）──
    function 固化行(行){
      var 文 = 行.replace(/\s+$/, "");
      // 1. 思考折叠标记
      if (文 === "<think>"){ 开容器("think"); return; }
      if (文 === "</think>"){ 关容器("think"); return; }
      // 2. 围栏
      var 栅 = 文.match(/^```([\w-]*)\s*$/);
      if (栅){
        if (栈.length && 栈[栈.length-1].类 === "围码"){ 关容器("围码"); }
        else { 开围码(); }
        return;
      }
      if (栈.length && 栈[栈.length-1].类 === "围码"){
        栈[栈.length-1].点.appendChild(document.createTextNode(行 + "\n"));
        return;
      }
      // 3. DSL-lite
      if (试DSL(文)){ return; }
      // 4. Markdown-lite 行
      容().appendChild(文行(文));
    }

    function 试DSL(文){
      var m;
      if ((m = 文.match(/^\[card(?:\s+tt:(.+))?\]\s*$/))){ 开卡(m[1] || ""); return true; }
      if (/^\[\/card\]\s*$/.test(文)){ 关容器("card"); return true; }
      if ((m = 文.match(/^\[btn(?:\s+clk:([\w\u4e00-\u9fa5-]+))?\](.*)\[\/btn\]\s*$/))){
        容().appendChild(建按钮(m[2] || "按钮", m[1] || ""));
        return true;
      }
      if ((m = 文.match(/^\[(p|h[1-6])\s+([^\]]*)\]\s*$/))){
        容().appendChild(文行(m[2], m[1]));
        return true;
      }
      return false;
    }

    function 开卡(标题){
      var 卡 = el("div", "dsl-card");
      if (标题){ 卡.appendChild(el("div", "dsl-card-题", 标题)); }
      var 体 = el("div", "dsl-card-体");
      卡.appendChild(体);
      容().appendChild(卡);
      栈.push({ 类: "card", 点: 体 });
    }

    function 开围码(){
      var pre = document.createElement("pre");
      pre.className = "流-码";
      var 码 = document.createElement("code");
      pre.appendChild(码);
      容().appendChild(pre);
      栈.push({ 类: "围码", 点: 码 });
    }

    function 开容器(类){
      if (类 === "think"){
        var 折 = document.createElement("details");
        折.className = "流-思";
        折.open = true;
        var 摘 = document.createElement("summary");
        摘.textContent = "思考过程";
        var 体 = el("div", "流-思体");
        折.appendChild(摘); 折.appendChild(体);
        容().appendChild(折);
        栈.push({ 类: "think", 点: 体 });
      }
    }

    function 关容器(类){
      for (var i = 栈.length - 1; i >= 0; i--){
        if (栈[i].类 === 类){ 栈.length = i; return; }
      }
    }

    function 建按钮(文本, 动作名){
      var b = document.createElement("button");
      b.className = "dsl-btn";
      b.type = "button";
      b.textContent = 文本;                 // 命名引用：DSL 只带动作名，不带代码
      b.addEventListener("click", function(){
        if (动作名 && typeof 动作[动作名] === "function"){ 动作[动作名](); }
      });
      return b;
    }

    // ── 行渲染：先结构（标题/列表/空段），再内联（粗体/行内码）──
    function 文行(文, 强类型){
      var m;
      if (!文){ return el("div", "流-空行"); }
      if (强类型 && /^h[1-6]$/.test(强类型)){
        var 级 = document.createElement(强类型);
        级.className = "流-题";
        内联(文, 级);
        return 级;
      }
      if ((m = 文.match(/^#{1,4}\s+(.*)$/))){
        var t = el("div", "流-题 流-题-强");
        内联(m[1], t);
        return t;
      }
      if ((m = 文.match(/^[-*]\s+(.*)$/))){
        var 行 = el("div", "流-列");
        行.appendChild(el("span", "流-列点", "•"));
        var 体 = el("span");
        内联(m[1], 体);
        行.appendChild(体);
        return 行;
      }
      var p = el("div", "流-段");
      内联(文, p);
      return p;
    }

    // 内联：**粗体** 与 `行内码` → DOM 节点（零 HTML 拼接）
    function 内联(文, 父){
      var 尾 = 0;
      var re = /\*\*([^*\n]+)\*\*|`([^`\n]+)`/g;
      var m;
      while ((m = re.exec(文)) !== null){
        if (m.index > 尾){ 父.appendChild(document.createTextNode(文.slice(尾, m.index))); }
        if (m[1] != null){
          var b = document.createElement("b");
          b.textContent = m[1];
          父.appendChild(b);
        } else {
          var c = document.createElement("code");
          c.className = "流-内码";
          c.textContent = m[2];
          父.appendChild(c);
        }
        尾 = m.index + m[0].length;
      }
      if (尾 < 文.length){ 父.appendChild(document.createTextNode(文.slice(尾))); }
    }

    // ── 尾区：只渲染未完成行（有界重渲唯一发生地）──
    function 渲尾(){
      尾区.textContent = "";
      if (!缓冲){ return; }
      if (栈.length && 栈[栈.length-1].类 === "围码"){
        栈[栈.length-1].点.appendChild(document.createTextNode(缓冲));
        缓冲 = "";                       // 半行直接落入围码（append-only）
        return;
      }
      var p = el("div", "流-段 流-尾行");
      p.textContent = 缓冲;
      尾区.appendChild(p);
    }

    function 收尾(){
      if (缓冲){ var 残 = 缓冲; 缓冲 = ""; 固化行(残); }
      while (栈.length){ 关容器(栈[栈.length-1].类); }
      尾区.textContent = "";
    }

    return { 喂: 喂, 收尾: 收尾 };
  }

  function el(标, 类名, 文本){
    var n = document.createElement(标);
    if (类名){ n.className = 类名; }
    if (文本 != null){ n.textContent = 文本; }
    return n;
  }

  // 验收样张：注入攻击串 + 富文本语法混排（供页面/测试调用断言）
  function 样张(){
    return [
      "<think>先思考：<img src=x onerror=alert(1)>",
      "得出结论。</think>",
      "# 标题 **加粗** 与 `行内码`",
      "- 列表项 <script>alert(2)</script>",
      "正文段落 \" onmouseover=\"x(1)",
      "[card tt:样例卡]",
      "[p 卡内段落 <b>非标签</b>]",
      "[btn clk:无此动作]点我[/btn]",
      "[/card]",
      "```html",
      "<img src=x onerror=alert(3)>",
      "```"
    ].join("\n");
  }

  return { 建: 建, 样张: 样张 };
})();
