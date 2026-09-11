/* ============================================================
   观星呈现-域 · 观测台面-府 · 万仙对话-殿 · 对话渲染
   左栏「万仙对话」：接待往返（演示语料 / 实时记录）+ 五层接力的结论宣告。
   结论与右栏读同一份事件缓冲，两栏同源。
   ============================================================ */

import { $, 运行时, 过筛, 转义 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 角色色 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";
import { 对话脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";
import { 结论摘要, 聚棒动作, 职名 } from "/观测台面-府/天机事件-殿/结论析出-阁/摘要-逻辑-园/摘要逻辑.js";
import { 跳段 } from "/观测台面-府/天机事件-殿/分层绘现-阁/棒条-渲染-园/棒条渲染.js";
import { 落确认钮, 发布 } from "/观测台面-府/万仙对话-殿/确认绑定-阁/发布-逻辑-园/发布逻辑.js";

/** 起一条消息外壳，返回可继续追加正文的气泡（流式答复边收边写） */
export function 起消息(流, 角, 时){
  const 色 = 角色色[角] || "var(--字二)";
  const 元 = document.createElement("div");
  元.className = "消息";
  元.innerHTML = `<div class="名"><span class="署" style="color:${色}">${角}</span><span class="时">${时||""}</span></div><div class="气泡"></div>`;
  const 泡 = 元.querySelector(".气泡");
  泡.style.borderLeftColor = 色;
  流.appendChild(元);
  return {元, 泡};
}

/** 追加一条消息；runId 存在时挂「证据锚点」，点击定位到右栏该棒的完整过程 */
export function 追加消息(流, 角, 时, 文, 摘要, runId){
  // 空态是「一条消息都还没有」的表达；消息来了它就得让位，
  // 否则界面上一边写「暂无发言」一边列着发言——那是界面在自述假话。
  流.querySelector(":scope > .空")?.remove();
  const {元, 泡} = 起消息(流, 角, 时);
  泡.textContent = 文;
  if(runId){
    const 锚 = document.createElement("button");
    锚.className = "锚";
    锚.textContent = "⟵ 过程";
    锚.title = "这句话的证据，在天机里这一棒的完整过程中";
    锚.addEventListener("click", ()=>跳段(runId));
    元.querySelector(".名").appendChild(锚);
  }

  if(摘要){
    const 卡 = document.createElement("div");
    卡.className = "摘要卡";
    卡.innerHTML = `
      <div class="卡头">需求摘要 · 待确认</div>
      <div class="卡身">
        <div class="字段"><span class="键">标题</span><span class="值">${转义(摘要.标题)}</span></div>
        <div class="字段"><span class="键">描述</span><span class="值">${转义(摘要.描述)}</span></div>
        <div class="字段"><span class="键">场景</span><span class="值"><span class="签">${转义(摘要.场景)}</span></span></div>
        <div class="字段"><span class="键">优先级</span><span class="值"><span class="签 木">${转义(摘要.优先级)}</span></span></div>
        <button class="确认按钮" id="发布钮">确 认 发 布 · 落 看 板</button>
      </div>`;
    流.appendChild(卡);
  }
}

/** 追加一条「接力结论」：署名带职责，正文是人话摘要，模型原文折叠在后。
    原文必须留——摘要只负责好读，证据不能因为摘要好看就丢掉。 */
export function 追加结论(流, 角, 原文, runId, 摘要){
  流.querySelector(":scope > .空")?.remove();
  const {元, 泡} = 起消息(流, 角, "");
  const 责 = 职名(角);
  if(责) 元.querySelector(".名 .署").textContent = `${角} · ${责}`;
  const 句 = document.createElement("div");
  句.className = "结句";
  句.textContent = 摘要 || 原文;
  泡.appendChild(句);
  // 摘要与原文一致（纯文本没折过）就不再叠一层折叠——同一句话摆两遍是纯噪音。
  if((摘要 || 原文) !== 原文){
    const 详 = document.createElement("details");
    详.className = "原文";
    详.innerHTML = `<summary>原文 · 模型答复</summary>`;
    const 身 = document.createElement("div");
    身.className = "原body";
    身.textContent = 原文;
    详.appendChild(身);
    泡.appendChild(详);
  }
  if(runId){
    const 锚 = document.createElement("button");
    锚.className = "锚";
    锚.textContent = "⟵ 过程";
    锚.title = "这一结论的证据，在天机里这一棒的完整过程中";
    锚.addEventListener("click", ()=>跳段(runId));
    元.querySelector(".名").appendChild(锚);
  }
  return {元, 泡};
}

export function 渲染对话(){
  const 流 = $("#对话流");
  流.innerHTML = "";
  // 一、接待阶段：演示模式铺契约样例语料；实时模式铺真实收到的接待往返。
  //    接待走 /api/dev/chat/stream，与天机流不同源，所以单独存一份 接待记录 供重绘。
  const 接待 = 运行时.模式 === "演示" ? 对话脚本 : 运行时.接待记录;
  接待.filter(m=>运行时.筛选 === "全部" || m.角色 === 运行时.筛选 || m.角色 === "来客")
      .forEach(条=>追加消息(流, 条.角色, 条.时, 条.文, 条.摘要));
  // 二、五层接力结论：由 TEXT_MESSAGE_CONTENT 生成——与右栏读的是同一份事件缓冲
  let 游run = null;
  const 动表 = 聚棒动作(运行时.事件缓冲);   // 结论摘要要用，与右栏棒头同源
  运行时.事件缓冲.forEach(ev=>{
    if(ev.type === "RUN_STARTED") 游run = ev.runId;
    if(ev.type === "TEXT_MESSAGE_CONTENT" && 过筛(ev)){
      const 动作 = (动表.get(游run) || {}).动作 || [];
      追加结论(流, ev.角色 || "系统", ev.delta, 游run, 结论摘要(ev.delta, 动作));
    }
  });
  if(运行时.待确认) 落确认钮(流);
  const 发钮 = 流.querySelector("#发布钮");
  if(发钮 && !发钮.dataset.绑){ 发钮.dataset.绑 = "1"; 发钮.addEventListener("click", 发布); }
  if(流.children.length === 0){
    流.innerHTML = `<div class="空" style="height:auto;padding:30px 10px"><div class="符">○</div><div class="语">该角色暂无发言</div></div>`;
  }
}

/* ---------- 接待往返（经总线来自 流式驱动-府） ----------
   气泡的「内容」由 流式驱动-府 决定（它才收得到 SSE），本府只负责画。
   气泡引用按 id 暂存在此处，流式增量才能找到该往哪个气泡里续写。 */
const 接待泡 = new Map();   // 接待 id → {元, 泡}

/** 起一条接待气泡（用户发问后，道祖那一侧的容器先立起来） */
export function 接待开启(d){
  const 流 = $("#对话流");
  流.querySelector(":scope > .空")?.remove();
  const {元, 泡} = 起消息(流, d.角色, d.时);
  接待泡.set(d.id, {元, 泡});
  流.parentElement.scrollTop = 流.parentElement.scrollHeight;
}

/** 流式追加：文本以最新全量为准，重复广播不会叠字 */
export function 接待增量(d){
  const 对 = 接待泡.get(d.id);
  if(!对) return;   // 气泡被整栏重绘顶掉：渲染对话 会按最新记录重建，此处无需补救
  对.泡.textContent = d.文本;
  const 区 = 对.泡.closest("#对话流").parentElement;
  区.scrollTop = 区.scrollHeight;
}

/** 收尾：落定最终文本；道祖若已对齐完需求，顺手把确认发布入口送上 */
export function 接待收尾(d){
  const 对 = 接待泡.get(d.id);
  const 流 = $("#对话流");
  if(对) 对.泡.textContent = d.文本;
  if(d.待确认){
    运行时.待确认 = true;
    落确认钮(流);
    const 钮 = 流.querySelector("#发布钮");
    if(钮 && !钮.dataset.绑){ 钮.dataset.绑 = "1"; 钮.addEventListener("click", 发布); }
  }
  流.parentElement.scrollTop = 流.parentElement.scrollHeight;
}
