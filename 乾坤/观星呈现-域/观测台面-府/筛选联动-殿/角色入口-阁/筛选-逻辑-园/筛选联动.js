/* ============================================================
   观星呈现-域 · 观测台面-府 · 筛选联动-殿 · 筛选联动
   角色筛选的唯一真源：左栏「角色入口」与右栏「天机筛」共用 运行时.筛选，
   点任一处的角色，左右两栏同时聚焦——两处各存一份筛选状态必然打架。
   ============================================================ */

import { $, 运行时 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 角色 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";
import { 渲染对话 } from "/观测台面-府/万仙对话-殿/消息渲染-阁/对话-渲染-园/对话渲染.js";
import { 重绘天机 } from "/观测台面-府/天机事件-殿/帧流归位-阁/编排-逻辑-园/帧编排.js";

export function 选角色(名){
  运行时.筛选 = 名;
  渲染入口(); 渲染对话(); 渲染天机筛(); 重绘天机();
}

/** 左栏角色入口 */
export function 渲染入口(){
  const 区 = $("#角色入口");
  区.innerHTML = "";
  const 全部 = document.createElement("button");
  全部.className = "全" + (运行时.筛选 === "全部" ? " 选" : "");
  全部.textContent = "全体";
  全部.addEventListener("click", ()=>选角色("全部"));
  区.appendChild(全部);
  角色.forEach(角=>{
    const 钮 = document.createElement("button");
    钮.className = 运行时.筛选 === 角.名 ? "选" : "";
    if(运行时.筛选 === 角.名) 钮.style.borderColor = 角.色;
    钮.innerHTML = `<span class="灯" style="color:${角.色}"></span>${角.名}`;
    钮.addEventListener("click", ()=>选角色(角.名));
    区.appendChild(钮);
  });
}

/** 右栏天机角色过滤条（与左栏同一筛选状态） */
export function 渲染天机筛(){
  const 区 = $("#天机筛");
  区.innerHTML = "";
  const 项 = [{名:"全部", 色:"var(--字二)"}].concat(角色);
  项.forEach(角=>{
    const 钮 = document.createElement("button");
    钮.className = 运行时.筛选 === 角.名 ? "选" : "";
    if(运行时.筛选 === 角.名) 钮.style.borderColor = 角.色;
    钮.innerHTML = `<span class="灯" style="color:${角.色}"></span>${角.名}`;
    钮.addEventListener("click", ()=>选角色(角.名));
    区.appendChild(钮);
  });
}
