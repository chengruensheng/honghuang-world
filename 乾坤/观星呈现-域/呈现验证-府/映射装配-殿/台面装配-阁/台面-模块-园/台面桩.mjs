/* ============================================================
   观星呈现-域 · 呈现验证-府 · 映射装配-殿 · 台面装配-阁 · 台面桩
   呈现层的绘制模块在调用期读全局 document（运行时中枢 的 `$` 即如此），
   node 下没有 document —— 测试必须先把「台面」装起来，被测模块才找得到
   #事件流、建得起棒。骨架取 index.html 的 id 集合，不替它补齐：
   测试缺哪个 id，就让缺的那个在断言里暴露——桩里悄悄补，等于测试自证空气。

   每个测试文件是独立进程，须各自装台面（node --test 默认一文件一进程）。
   用法：
     import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
     const { 档 } = 装台面();
   ============================================================ */
import { Window } from "happy-dom";

/** index.html 的骨架 id 子集：测试真正会碰到的那些，结构与原骨架同形 */
const 骨架 = `
<span class="状态灯"><span class="点"></span><span id="连接语">实时 · 连接中…</span></span>
<span id="底栏当前" class="底当前">空闲 · 等待任务事件</span>
<nav class="基础栏" aria-label="基础导航">
  <button id="轨观星台" class="轨钮 当前"></button>
  <button id="轨看板" class="轨钮"></button>
  <button id="轨格位" class="轨钮"></button>
  <button id="轨道规" class="轨钮"></button>
  <span class="轨隔"></span>
  <!-- 面钮不写在这里：happy-dom 的 HTML 解析器不认非 ASCII 属性名（浏览器支持），
       data-面板 会被截成 data-。改用 setAttribute 建，见 建面钮()。 -->
</nav>
<aside class="展开栏" id="展开栏">
  <div class="展内">
    <div class="展头"><h2 id="展标题">待承纳</h2></div>
    <div class="展身" id="展身"><div class="展空" id="展空">此面板由基础栏图标开启<br>内容待后续承纳</div></div>
  </div>
</aside>
<main class="主区">
  <div class="视图 显 两栏" id="视图-观星台">
    <div id="角色入口" class="角色入口"></div>
    <div class="栏身"><div id="对话流" class="对话流"></div></div>
    <textarea id="输入"></textarea>
    <button class="发送" id="发送"></button>
    <div id="天机筛" class="天机筛"></div>
    <div id="阶段条" class="阶段条" style="display:none"></div>
    <div class="栏身"><div id="事件流" class="事件流"></div></div>
  </div>
  <div class="视图" id="视图-看板"><span class="统" id="看板统"></span><div class="泳道区" id="泳道区"></div></div>
  <div class="视图" id="视图-格位"><span class="统" id="格位统"></span><div class="格位区" id="格位区"></div></div>
  <div class="视图" id="视图-道规"><span class="统" id="道规统"></span><div class="道规区" id="道规区"></div></div>
</main>
`;

/** 装台面：建 happy-dom 窗口，把骨架挂上，并把 window/document/CustomEvent 铺进全局。
    默认 fetch 一律拒绝——测试不触网；需要取数的用例自行替换 globalThis.fetch。 */
export function 装台面({取数, 事件源}={}){
  const 窗 = new Window({url: "http://localhost/"});
  窗.document.body.innerHTML = 骨架;
  建面钮(窗.document);
  globalThis.window = 窗;
  globalThis.document = 窗.document;
  globalThis.CustomEvent = 窗.CustomEvent;   // 广播用 new CustomEvent，模块里读的正是全局
  if(!窗.Element.prototype.scrollIntoView) 窗.Element.prototype.scrollIntoView = function(){};
  globalThis.fetch = 取数 || (async(路径)=>{ throw new Error(`测试未授权网络请求：${路径}`); });
  if(事件源 !== false) 装事件源桩();
  return {窗, 档: 窗.document};
}

/** 面钮用 DOM API 建：happy-dom 的 HTML 解析器不认非 ASCII 属性名（浏览器支持），
    走 innerHTML 会把 data-面板 截成 data-，那样被测模块读到的属性与浏览器不一致。
    属性集对齐 index.html 的五个面钮（title / aria-label / aria-expanded）。 */
function 建面钮(档){
  const 栏 = 档.querySelector(".基础栏");
  ["门禁","传承","记忆","架构","设计"].forEach(名=>{
    const 钮 = 档.createElement("button");
    钮.className = "轨钮 面钮";
    钮.setAttribute("data-面板", 名);
    钮.setAttribute("title", 名);
    钮.setAttribute("aria-label", 名);
    钮.setAttribute("aria-expanded", "false");
    栏.appendChild(钮);
  });
}

/** EventSource 桩：只记录实例与关闭动作，不发任何请求；测试自行触发 onopen/onmessage/onerror */
export function 装事件源桩(){
  const 实例表 = [];
  class 桩源{
    constructor(路径){ this.路径 = 路径; this.已关 = false; 实例表.push(this); }
    close(){ this.已关 = true; }
  }
  globalThis.EventSource = 桩源;
  return 实例表;
}

/** 台面就绪断言：骨架缺 id 时应立即炸在装上，而不是等某条用例莫名其妙地过 */
export function 断言骨架齐备(档){
  const 缺 = ["事件流","对话流","阶段条","角色入口","天机筛","泳道区","看板统","连接语","底栏当前",
    "输入","发送","格位区","格位统","道规区","道规统","展身","展标题","展开栏",
    "视图-观星台","视图-看板","视图-格位","视图-道规","轨观星台","轨看板","轨格位","轨道规"]
    .filter(id=>!档.getElementById(id));
  if(缺.length) throw new Error("台面骨架缺 id：" + 缺.join("、"));
}
