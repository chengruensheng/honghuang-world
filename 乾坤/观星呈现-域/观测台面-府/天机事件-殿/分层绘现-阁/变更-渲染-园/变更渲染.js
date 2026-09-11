/* ============================================================
   观星呈现-域 · 观测台面-府 · 天机事件-殿 · 变更渲染
   工具结果的呈现：成败着色、参数原文、以及「精确编辑」的增删对照。
   「看得见」最该落地的就是这一块。
   ============================================================ */

import { 工具引 } from "/观测台面-府/天机事件-殿/结论析出-阁/摘要-逻辑-园/摘要逻辑.js";

/** 工具条标题的引用名：与棒头摘要同源，两处不能各算一套 */
export function 标工具引(条, 原文){
  const 名 = 工具引(原文);
  if(名) 条.querySelector(".引").textContent = 名;
  return 名;
}

/** 工具结果：落结果，并把带「旧 / 新」的变更画成增删对照 */
export function 落结果(条, 内容){
  if(/失败|错误|✗|不通过|panic|Error/.test(内容)) 条.classList.add("败");
  let 果 = 条.querySelector(".结果");
  if(!果){ 果 = document.createElement("div"); 果.className = "结果"; 条.querySelector(".条文").appendChild(果); }
  果.textContent = 内容;
  渲染变更(条);
}

/** 入参带「旧 / 新」（精确编辑）时画 - / + 对照 */
export function 渲染变更(条){
  const 参 = 条.querySelector(".参数");
  if(!参 || 条.querySelector(".码")) return;
  let 对; try{ 对 = JSON.parse(参.textContent); }catch(e){ return; }
  if(!对.旧 || !对.新) return;
  const 码 = document.createElement("div");
  码.className = "码";
  String(对.旧).replace(/\n$/, "").split("\n").forEach(行=>码.appendChild(建码行("减", 行)));
  String(对.新).replace(/\n$/, "").split("\n").forEach(行=>码.appendChild(建码行("加", 行)));
  条.querySelector(".条文").appendChild(码);
}

export function 建码行(号, 文本){
  const 行 = document.createElement("div");
  行.className = "码行 " + 号;
  行.innerHTML = `<span class="标">${号 === "加" ? "+" : "-"}</span><span class="本"></span>`;
  行.querySelector(".本").textContent = 文本 || " ";
  return 行;
}
