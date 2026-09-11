/* ============================================================
   观星呈现-域 · 观测台面-府 · 天机事件-殿 · 棒条渲染
   右栏事件流的骨架：棒（一次 run = 一个角色的一段任期）→ 条（棒内语义单元）。
   本模块只管「怎么画」，不管「帧从哪来、怎么编排」——编排归 帧流归位-阁。
   ============================================================ */

import { $, 运行时 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 角色色 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";
import { 结论摘要 } from "/观测台面-府/天机事件-殿/结论析出-阁/摘要-逻辑-园/摘要逻辑.js";

/** 建棒：一条 run 占一行，进行中默认展开 */
export function 建棒(runId, 角){
  const 色 = 角色色[角] || "var(--字三)";
  const 外 = document.createElement("div");
  外.className = "棒";
  外.id = "段-" + runId;
  外.innerHTML = `
    <div class="棒头">
      <span class="角"></span>
      <span class="棒名"></span>
      <span class="棒摘"></span>
      <span class="态 等"></span>
      <span class="棒计"></span>
      <span class="折">▶</span>
    </div>
    <div class="棒身"></div>`;
  const 角标 = 外.querySelector(".角");
  角标.textContent = 角;
  角标.style.color = 色;
  // 棒名留给 STEP_STARTED 的阶段名，此处不预设「运行中」——
  // 状态由右侧的 .态 独家表达；两处都写状态，收尾时必然对不上（实测「运行中」与「完成」并排）。
  外.querySelector(".棒头").addEventListener("click", ()=>折棒(runId));
  const 记录 = {数据:{runId, 角色:角, 帧:[], 完成:false, 开:false, 标题:"", 终态:"", 条数:0, 动作:[], 首句:""},
                dom:外, 身:外.querySelector(".棒身"), 行名:外.querySelector(".棒名"),
                工具表:new Map(), 当前条:null};
  // 进行中默认展开——「正在做什么」直接可见；收尾时再折起，只留一行摘要
  外.classList.add("开");
  记录.数据.开 = true;
  运行时.展开棒.add(runId);
  // 首棒落地即退出空态：空态是「一根棒都还没有」的表达，棒来了它就必须让位，
  // 否则它会赖在棒的上面，看上去像同一时刻有两套状态在说话。
  const 残留空态 = $("#事件流").querySelector(".空");
  if(残留空态) 残留空态.remove();
  $("#事件流").appendChild(外);
  运行时.棒表.set(runId, 记录);
  if(!运行时.角色棒.has(角)) 运行时.角色棒.set(角, []);
  运行时.角色棒.get(角).push(记录);
  // 阶段帧若先到，终态语已暂存在此——棒一落地就补上，不让它跨过建棒这一刻丢失
  const 挂 = 运行时.待收尾.get(角);
  if(挂){ 运行时.待收尾.delete(角); 收尾棒(记录, 挂.语, 挂.败); }
  return 记录;
}

/** 收尾棒：给一段任期画上句号——标完成、记终态语（折叠后作棒头 title 索引），并折起。
    幂等：STEP_FINISHED 与 RUN_FINISHED 可能先后到达同一棒，只认第一次终态，不重复动作。
    折起不读「开」标志、直接改 DOM：标志与 DOM 类一旦失同步，靠标志判断会漏折——
    任期结束就该收起，这是结论，不是条件。 */
export function 收尾棒(棒, 终态语, 失败){
  if(!棒 || 棒.数据.完成) return;
  棒.数据.完成 = true;
  棒.数据.失败 = !!失败;
  棒.数据.终态 = 终态语 || "";
  刷棒头(棒);
  棒.数据.开 = false;
  棒.dom.classList.remove("开");
  运行时.展开棒.delete(棒.数据.runId);
}

/** 记动作：把「动词 + 目标」攒进棒头摘要（同一动作只记一次） */
export function 记动作(棒, 文){
  if(!文) return;
  const 动 = 棒.数据.动作;
  if(动[动.length-1] !== 文 && !动.includes(文)) 动.push(文);
}

/** 摘要文本：动作优先（最多 3 项，余量以 +N 收尾），无动作时把首句折成人话 */
export function 摘文(d){
  const 动 = d.动作 || [];
  if(动.length){
    const 显 = 动.slice(0,3).join(" · ");
    return 动.length > 3 ? `${显} …+${动.length-3}` : 显;
  }
  // 无工具动作的棒（如道祖只作终审）：首句是模型 JSON，原样铺出来是最难读的一条——
  // 交给 结论摘要 折一次，与左栏同一口径。
  return 结论摘要(d.首句 || "", []);
}

/** 建条：棒内的语义单元 */
export function 建条(棒, 类, 头, 引, 文){
  const 条 = document.createElement("div");
  条.className = "条 " + 类;
  条.dataset.类 = 类.includes("推理") ? "推理" : 类.includes("工具") ? "工具" : "状态";
  条.innerHTML = `<div class="条头"><b></b><span class="引"></span><span class="折">▶</span></div><div class="条文"></div>`;
  条.querySelector("b").textContent = 头;
  条.querySelector(".引").textContent = 引 || "";
  条.querySelector(".条文").textContent = 文 || "";
  棒.身.appendChild(条);
  棒.数据.条数++;
  return 条;
}

/** 刷棒头：三态（中断 / 完成 / 进行中）与摘要、条数
    三态必须分清：中断（错误终止，任期已断但非完成）不能混进「完成」，
    否则熔断终止在界面上会被读成顺利结束。 */
export function 刷棒头(棒){
  const d = 棒.数据;
  棒.dom.classList.toggle("行", !d.完成);
  const 态 = 棒.dom.querySelector(".态");
  态.className = "态 " + (d.失败 ? "败" : d.完成 ? "成" : "行");
  态.textContent = d.失败 ? "中断" : d.完成 ? "完成" : "进行中";
  棒.dom.querySelector(".棒摘").textContent = 摘文(d);
  棒.dom.querySelector(".棒计").textContent = d.条数 + " 条";
  if(d.终态) 棒.dom.querySelector(".棒头").title = d.终态;
}

/** 折棒：手动展开/收起 */
export function 折棒(runId){
  const 棒 = 运行时.棒表.get(runId);
  if(!棒) return;
  const 开 = !棒.数据.开;
  棒.数据.开 = 开;
  棒.dom.classList.toggle("开", 开);
  if(开) 运行时.展开棒.add(runId); else 运行时.展开棒.delete(runId);
}

/** 证据锚点：展开右栏对应那一棒，再定位到它顶端（左栏结论的「⟵ 过程」调它） */
export function 跳段(runId){
  const 棒 = 运行时.棒表.get(runId);
  if(!棒) return;
  if(!棒.数据.开) 折棒(runId);
  const 目标 = 棒.dom;
  目标.scrollIntoView({behavior:"smooth", block:"start"});
  目标.classList.remove("闪");
  void 目标.offsetWidth;   // 强制重排，让动画可重复触发
  目标.classList.add("闪");
  setTimeout(()=>目标.classList.remove("闪"), 1200);
}

/** 自动跟随：用户没手动上翻时才跟着新帧走 */
export function 跟随(){
  const 身 = $("#事件流").parentElement;
  if(身.scrollHeight - 身.scrollTop - 身.clientHeight < 140) 身.scrollTop = 身.scrollHeight;
}

/** 清空右栏与全部棒记录 */
export function 清事件(){
  $("#事件流").innerHTML = "";
  运行时.棒表.clear(); 运行时.角色棒.clear(); 运行时.待收尾.clear();
  运行时.展开棒.clear(); 运行时.展开条.clear();
}
