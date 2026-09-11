/* ============================================================
   观星呈现-域 · 观测台面-府 · 万仙对话-殿 · 发布逻辑
   道祖完成需求对齐后的「确认发布」入口，以及实时/演示两种发布路径。
   ============================================================ */

import { $, 运行时, 连接语, 总线, 广播 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";

/** 道祖对齐完成、等用户确认发布——把确认入口落在对话流末尾 */
export function 落确认钮(流){
  if(流.querySelector("#发布钮")) return;
  const 卡 = document.createElement("div");
  卡.className = "摘要卡";
  卡.innerHTML = `<div class="卡头">道祖已完成需求对齐 · 待确认</div>
    <div class="卡身"><button class="确认按钮" id="发布钮">确 认 发 布 · 落 看 板</button></div>`;
  流.appendChild(卡);
}

export async function 发布(){
  const 钮 = $("#发布钮");
  if(运行时.模式 === "演示"){
    if(钮){ 钮.disabled = true; 钮.textContent = "已发布 · 任务 #31 已进入五层流转"; }
    连接语("演示数据 · 契约对齐");
    // 经总线请顶层编排重放演示事件——下层不反向调用顶层，依赖方向保持单向
    广播(总线.请求重放);
    return;
  }
  if(钮){ 钮.disabled = true; 钮.textContent = "确认中…"; }
  try{
    const 响应 = await fetch("/api/dev/chat/confirm", {method:"POST"});
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 数据 = await 响应.json();
    运行时.待确认 = false;
    连接语("实时 · 已发布 任务 #" + (数据.任务id ?? "?"));
    钮?.remove();
    广播(总线.请求拉看板);   // 经总线请看板流转-府取一次权威数据（府间不互相 import）
  }catch(错){
    连接语("实时 · 发布失败：" + 错.message);
    if(钮){ 钮.disabled = false; 钮.textContent = "确 认 发 布 · 落 看 板"; }
  }
}
