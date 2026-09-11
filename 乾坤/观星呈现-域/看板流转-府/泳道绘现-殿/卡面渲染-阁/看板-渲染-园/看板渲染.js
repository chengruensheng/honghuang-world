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
    回退: 回 ? {次:回.回退次数, 由:层级角色[回.来源层级] || 回.来源层级, 至:层级角色[回.目标层级] || 回.目标层级, 因:回.原因, 目标层:回.目标层级} : null,
    召回标记: !!t.召回标记,
    当前层级: t.当前层级 || null,
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

/** 定位任务：把该任务那张卡滚进视野并闪一下——从棒头点任务号跳过来时，让眼睛知道是哪张卡。
    找不到就什么都不做：取不到就不编，也不假装找到了。 */
export function 定位任务(任务id){
  const 卡 = document.querySelector(`#泳道区 .卡[data-id="${任务id}"]`);
  if(!卡) return;
  卡.classList.add("开");
  卡.scrollIntoView({behavior:"smooth", block:"center"});
  卡.classList.remove("闪");
  void 卡.offsetWidth;   // 强制重排，让动画可重复触发
  卡.classList.add("闪");
  setTimeout(()=>卡.classList.remove("闪"), 1200);
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
  卡.dataset.id = t.id;   // 天机视图的棒按它定位到这张卡（6.5）
  const 阶段链 = Object.entries(t.阶段).map(([k,v])=>{
    const 色 = v === "✓" ? "var(--木)" : v === "进行" ? "var(--水)" : v === "不通过" ? "var(--火)" : "var(--字三)";
    return `<span style="color:${色}">${转义(k)}${转义(v)}</span>`;
  }).join(" · ");
  卡.innerHTML = `
    <div class="号">#${转义(t.id)}</div>
    <div class="题">${转义(t.title)}</div>
    <div class="底"><span class="态 ${态类(t.status)}">${转义(t.status)}</span>${t.召回标记 ? `<span class="召">⚠ 召回中</span>` : ""}${t.当前层级 ? `<span class="层">${转义(t.当前层级)}层</span>` : ""}<span class="轮">第${转义(t.轮次)}轮</span></div>
    ${t.回退 ? `<div class="回退">↩ 第${转义(t.回退.次)}次回退 · ${转义(t.回退.由)} → ${转义(t.回退.至)}</div>` : ""}
    ${t.status === "待人工验收" ? 审核区() : ""}
    ${t.回退 ? 恢复区(t) : ""}
    <div class="回顾">
      <div class="项"><i>阶段</i><span style="font-family:var(--等);font-size:10.5px">${阶段链}</span></div>
      <div class="项"><i>承接</i><span>${转义((t.承接历史||[]).join(" → "))}</span></div>
      ${t.回退 ? `<div class="项"><i>回退因</i><span style="color:var(--火)">${转义(t.回退.因)}</span></div>` : ""}
      ${(t.动态||[]).map(d=>`<div class="项"><i>${转义(d.时)}</i><span>${转义(d.事)}</span></div>`).join("")}
    </div>`;
  卡.addEventListener("click", ()=>卡.classList.toggle("开"));
  // 最终审核操作（人可看可不看）：仅「待人工验收」任务落通过/驳回按钮；人工覆盖 LLM 自动审核结论
  if(t.status === "待人工验收"){
    const 区 = 卡.querySelector(".审");
    区.addEventListener("click", (e)=>e.stopPropagation());   // 审核操作不触发卡片展开
    区.querySelector(".审通过").addEventListener("click", ()=>审核(t.id, true, null, 区));
    区.querySelector(".审驳回").addEventListener("click", ()=>审核(t.id, false, 区.querySelector(".审因").value, 区));
  }
  // 失败恢复操作区：仅回退过（有回退记录）的任务落「恢复方式四选一」；操作不触发卡片展开
  if(t.回退){
    const 区 = 卡.querySelector(".恢");
    区.addEventListener("click", (e)=>e.stopPropagation());
    区.querySelector(".恢览").addEventListener("click", ()=>预览影响(t.id, 区));
    区.querySelector(".恢执").addEventListener("click", ()=>执行恢复(t.id, 区));
  }
  return 卡;
}

/** 待人工验收任务的审核操作区：通过 / 驳回（驳回原因固定七选一，与后端驳回原因枚举同源） */
function 审核区(){
  const 因 = ["需求不清","设计不符","实现错误","测试不足","产出不完整","扩大范围","缩小范围"];
  const 选项 = 因.map((x,i)=>`<option value="${x}"${i===2?" selected":""}>${x}</option>`).join("");
  return `<div class="审">
    <button class="审通过" type="button">✓ 通过</button>
    <select class="审因">${选项}</select>
    <button class="审驳回" type="button">✗ 驳回</button>
  </div>`;
}

/** 失败恢复操作区：看到失败 → 了解原因（回顾区「回退因」）→ 选择恢复方式（四选一）→ 预览影响范围 → 执行。
    恢复方式与后端「定向回退请求.恢复方式」枚举同源（回退并召回/仅回退/仅标记/取消）。 */
function 恢复区(){
  const 方式 = [
    ["回退并召回", "回退当前 + 召回下游"],
    ["仅回退", "仅回退当前任务"],
    ["仅标记", "仅标记问题，不改状态"],
    ["取消", "取消本次恢复"],
  ];
  const 选项 = 方式.map(([值,名])=>`<option value="${值}">${名}</option>`).join("");
  return `<div class="恢">
    <textarea class="恢因" placeholder="失败原因（错误描述）"></textarea>
    <div class="恢行">
      <select class="恢式">${选项}</select>
      <button class="恢览" type="button">影响预览</button>
      <button class="恢执" type="button">执行恢复</button>
    </div>
    <div class="恢影"></div>
    <div class="恢错"></div>
  </div>`;
}

/** 影响范围预览：GET /api/board/{id}/impact → 只读展示会被连带召回的任务（当前状态 → 将变更为），不执行召回 */
async function 预览影响(任务id, 区){
  const 影位 = 区.querySelector(".恢影");
  影位.textContent = "正在分析影响范围…";
  try{
    const 响应 = await fetch(`/api/board/${任务id}/impact`);
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 数据 = await 响应.json();
    const 影响 = 数据.影响任务 || [];
    影位.textContent = "";
    if(影响.length === 0){
      影位.textContent = "无连带受影响任务";
    }else{
      影响.forEach(x=>{
        const 项 = document.createElement("div");
        项.className = "影项";
        项.textContent = `#${x.任务id} ${x.标题}：${x.当前状态} → ${x.将变更为}`;
        影位.appendChild(项);
      });
    }
  }catch(错){
    影位.textContent = "影响预览失败：" + 错.message;
  }
}

/** 执行恢复：POST /api/board/{id}/rollback（恢复方式四选一）→ 成功后重拉看板；失败原因内联，不静默 */
async function 执行恢复(任务id, 区){
  const 因 = 区.querySelector(".恢因").value.trim();
  const 式 = 区.querySelector(".恢式").value;
  const 控件 = 区.querySelectorAll("button,select,textarea");
  控件.forEach(b=>b.disabled = true);
  const 错位 = 区.querySelector(".恢错");
  错位.textContent = "";
  try{
    const 响应 = await fetch(`/api/board/${任务id}/rollback`, {
      method:"POST",
      headers:{"Content-Type":"application/json"},
      body: JSON.stringify({错误描述: 因, 恢复方式: 式}),
    });
    if(!响应.ok){
      let 说明 = "HTTP " + 响应.status;
      try{
        const 体 = await 响应.json();
        if(typeof 体 === "string" && 体) 说明 = 体;
      }catch(_){}
      throw new Error(说明);
    }
    拉看板();   // 恢复成功：回到后端权威数据（卡片随新状态自然迁移）
  }catch(错){
    错位.textContent = "恢复失败：" + 错.message;
    控件.forEach(b=>b.disabled = false);
  }
}

/** 人工覆盖最终审核结论：POST /api/board/{id}/review → 成功后重拉看板；失败原因内联在操作区，不静默 */
async function 审核(任务id, 通过, 驳回原因, 区){
  区.querySelectorAll("button,select").forEach(b=>b.disabled = true);
  let 错位 = 区.querySelector(".审错");
  if(!错位){ 错位 = document.createElement("div"); 错位.className = "审错"; 区.appendChild(错位); }
  错位.textContent = "";
  try{
    const 响应 = await fetch(`/api/board/${任务id}/review`, {
      method:"POST",
      headers:{"Content-Type":"application/json"},
      body: JSON.stringify({通过, 驳回原因: 通过 ? null : (驳回原因 || null), 评语:""}),
    });
    if(!响应.ok){
      let 说明 = "HTTP " + 响应.status;
      try{
        const 体 = await 响应.json();
        if(typeof 体 === "string" && 体) 说明 = 体;
      }catch(_){}
      throw new Error(说明);
    }
    拉看板();   // 审核成功：回到后端权威数据（卡片随新状态自然迁移）
  }catch(错){
    错位.textContent = "审核失败：" + 错.message;
    区.querySelectorAll("button,select").forEach(b=>b.disabled = false);
  }
}
