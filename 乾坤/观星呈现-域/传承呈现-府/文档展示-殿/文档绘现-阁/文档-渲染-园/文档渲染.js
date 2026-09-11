/* ============================================================
   观星呈现-域 · 传承呈现-府 · 文档展示-殿 · 文档渲染
   架构面板（架构/蓝图文档）+ 设计面板（传承殿里含「设计」的文档）。
   数据取自 /api/files（后端权威，三类：传承殿/门禁/架构）；全文经 /api/files/content 只读。
   失败不静默——把原因写进展身，界面只说已确认的事实。
   ============================================================ */

import { $, 转义 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";

function 空态(语){
  return `<div class="空" style="height:auto;padding:30px 10px"><div class="符">○</div><div class="语">${语}</div></div>`;
}

async function 拉清单(){
  const r = await fetch("/api/files");
  if(!r.ok) throw new Error("HTTP " + r.status);
  return r.json();
}

/** 架构面板：/api/files 的「架构」分类（文件名含架构/蓝图） */
export async function 拉架构(){
  const 身 = $("#展身");
  try{
    const 数据 = await 拉清单();
    渲染列表(身, 数据.架构 || [], "架构图");
  }catch(错){
    身.innerHTML = 空态("架构图拉取失败：" + 转义(错.message));
  }
}

/** 设计面板：传承殿里含「设计」的文档，但排除任务级「设计表」（圣人每任务一张的产物，非设计稿） */
export async function 拉设计(){
  const 身 = $("#展身");
  try{
    const 数据 = await 拉清单();
    const 设计 = (数据.传承殿 || []).filter(条 =>
      (条.路径.includes("设计") || 条.名称.includes("设计")) &&
      !条.路径.includes("设计表") && !条.名称.includes("设计表"));
    渲染列表(身, 设计, "设计稿");
  }catch(错){
    身.innerHTML = 空态("设计稿拉取失败：" + 转义(错.message));
  }
}

function 渲染列表(身, 列表, 类型){
  身.innerHTML = "";
  if(!列表.length){ 身.innerHTML = 空态(`未找到任何${类型}`); return; }
  const 列 = document.createElement("div");
  列.className = "文档列";
  列表.forEach(条 => 列.appendChild(建条目(条)));
  身.appendChild(列);
}

function 建条目(条){
  const 项 = document.createElement("div");
  项.className = "文档项";
  项.innerHTML = `
    <div class="文档头" role="button" tabindex="0" aria-expanded="false">
      <span class="文档名">${转义(条.名称)}</span>
      <span class="文档径">${转义(条.路径)}</span>
      <span class="文档箭头">▸</span>
    </div>
    <div class="文档全文"></div>`;
  const 头 = 项.querySelector(".文档头");
  const 全文 = 项.querySelector(".文档全文");
  头.addEventListener("click", async ()=>{
    const 开 = 项.classList.toggle("开");
    头.setAttribute("aria-expanded", 开 ? "true" : "false");
    头.querySelector(".文档箭头").textContent = 开 ? "▾" : "▸";
    if(开 && !全文.dataset.载){
      全文.dataset.载 = "1";
      await 载全文(全文, 条.路径);
    }
  });
  return 项;
}

/** 读文档全文（/api/files/content）——只读展示，不提供编辑（写接口仅限 rules/） */
async function 载全文(全文, 路径){
  全文.innerHTML = `<div class="文档载">载入中…</div>`;
  try{
    const r = await fetch("/api/files/content?路径=" + encodeURIComponent(路径));
    if(!r.ok) throw new Error("HTTP " + r.status);
    const 内容 = await r.json();
    全文.innerHTML = `<pre class="文档文">${转义(内容.内容 || "")}</pre>`;
  }catch(错){
    全文.innerHTML = `<div class="文档错">内容读取失败：${转义(错.message)}</div>`;
  }
}
