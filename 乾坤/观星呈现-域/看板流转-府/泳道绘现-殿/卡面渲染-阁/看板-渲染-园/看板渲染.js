/* ============================================================
   观星呈现-域 · 看板流转-府 · 泳道绘现-殿 · 看板渲染
   看板全屏视图：五层泳道 + 任务卡。数据取自 /api/board（后端权威）。
   失败时不静默——把失败原因写进统计位，界面只说已确认的事实。
   ============================================================ */

import { $, 运行时, 转义 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 角色, 层级角色, 层级阶段 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";

/** 后端任务的终态集合（权威源：流转逻辑.rs 的状态分层——清理完成是终态，已取消是终态） */
export const 终态集 = ["清理完成","已取消"];

export function 渲染看板(){
  const 区 = $("#泳道区");
  区.innerHTML = "";
  let 总 = 0;
  角色.forEach(角=>{
    const 该角任务 = 运行时.任务集.filter(t=>t.角色 === 角.名);
    总 += 该角任务.length;
    const 泳道 = document.createElement("div");
    泳道.className = "泳道";
    泳道.innerHTML = `
      <div class="泳道头">
        <span class="徽" style="background:${角.色}"></span>
        <span class="职">${角.名}</span><span class="责">${角.责}</span>
        <span class="数">${该角任务.length}</span>
      </div>
      <div class="泳道身"></div>`;
    const 身 = 泳道.querySelector(".泳道身");
    if(该角任务.length === 0){
      身.innerHTML = `<div class="空" style="height:auto;padding:22px 6px"><div class="符">○</div><div class="语">空闲</div></div>`;
    }else{
      该角任务.forEach(t=>身.appendChild(建卡(t)));
    }
    区.appendChild(泳道);
  });
  $("#看板统").textContent = `共 ${总} 个在办任务 · ${运行时.任务集.filter(t=>!终态集.includes(t.status)).length} 个流转中`;
}

/** 后端任务 → 视图卡片。字段全部取自真实契约，缺什么显示什么，不补占位白。 */
function 适配任务(t){
  const 历史 = t.状态历史 || [];
  const 层史 = t.层级历史 || [];
  const 回 = t.回退来源 || null;
  const 阶段 = {};
  Object.entries(层级阶段).forEach(([层, 名])=>{
    const 记 = 层史.filter(x=>x.层级 === 层);
    if(回 && 回.目标层级 === 层) 阶段[名] = "不通过";
    else if(记.length === 0) 阶段[名] = "—";
    else 阶段[名] = 记.some(x=>String(x.状态||"").includes("完成")) ? "✓" : "进行";
  });
  return {
    id: t.id,
    title: t.title,
    status: t.status,
    角色: t.当前承接人 || (历史.length ? 历史[历史.length-1].操作者 : "—"),
    轮次: t.修复轮次 || 0,
    阶段,
    承接历史: t.承接历史 || [],
    回退: 回 ? {次:回.回退次数, 由:层级角色[回.来源层级] || 回.来源层级, 至:层级角色[回.目标层级] || 回.目标层级, 因:回.原因} : null,
    动态: 历史.slice(-5).map(h=>({时:时钟(h.时间), 事:`${h.原状态} → ${h.新状态}（${h.操作者}${h.备注 ? " · " + h.备注 : ""}）`})),
  };
}

/** 秒级时间戳 → HH:MM */
function 时钟(秒){
  if(!秒) return "";
  return new Date(秒 * 1000).toLocaleTimeString("zh-CN", {hour12:false, hour:"2-digit", minute:"2-digit"});
}

/** 看板真实化：读 /api/board（后端权威数据），映射后重绘。 */
export async function 拉看板(){
  try{
    const 响应 = await fetch("/api/board");
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 数据 = await 响应.json();
    运行时.任务集 = 数据.map(适配任务);
    渲染看板();
  }catch(错){
    运行时.任务集 = [];
    渲染看板();
    $("#泳道区").innerHTML = `<div class="空"><div class="符">○</div><div class="语">看板拉取失败：${转义(错.message)}</div></div>`;
    $("#看板统").textContent = "看板拉取失败 · 可切回演示查看契约样例";
  }
}

function 态类(态){
  if(终态集.includes(态) || 态.includes("已完成")) return "成";
  if(/进行中|设计中|实现中|验收中|终审中|清理中/.test(态)) return "行";
  if(态.includes("待修复")) return "退";
  if(态.includes("清理")) return "清";
  return "等";
}

function 建卡(t){
  const 卡 = document.createElement("div");
  卡.className = "卡";
  const 阶段链 = Object.entries(t.阶段).map(([k,v])=>{
    const 色 = v === "✓" ? "var(--木)" : v === "进行" ? "var(--水)" : v === "不通过" ? "var(--火)" : "var(--字三)";
    return `<span style="color:${色}">${转义(k)}${转义(v)}</span>`;
  }).join(" · ");
  卡.innerHTML = `
    <div class="号">#${转义(t.id)}</div>
    <div class="题">${转义(t.title)}</div>
    <div class="底"><span class="态 ${态类(t.status)}">${转义(t.status)}</span><span class="轮">第${转义(t.轮次)}轮</span></div>
    ${t.回退 ? `<div class="回退">↩ 第${转义(t.回退.次)}次回退 · ${转义(t.回退.由)} → ${转义(t.回退.至)}</div>` : ""}
    <div class="回顾">
      <div class="项"><i>阶段</i><span style="font-family:var(--等);font-size:10.5px">${阶段链}</span></div>
      <div class="项"><i>承接</i><span>${转义((t.承接历史||[]).join(" → "))}</span></div>
      ${t.回退 ? `<div class="项"><i>回退因</i><span style="color:var(--火)">${转义(t.回退.因)}</span></div>` : ""}
      ${(t.动态||[]).map(d=>`<div class="项"><i>${转义(d.时)}</i><span>${转义(d.事)}</span></div>`).join("")}
    </div>`;
  卡.addEventListener("click", ()=>卡.classList.toggle("开"));
  return 卡;
}
