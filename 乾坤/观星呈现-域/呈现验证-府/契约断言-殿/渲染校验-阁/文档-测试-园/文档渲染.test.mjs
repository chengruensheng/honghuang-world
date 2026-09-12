/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   文档渲染 用例：架构面板 + 设计面板（/api/files 清单、/api/files/content 只读全文）。
   锁的硬约束：
   · 失败不静默——把原因写进展身，且错误信息进 innerHTML 前必须转义；
   · 设计面板按规则过滤：含「设计」但排除任务级「设计表」（圣人每任务的产物不是设计稿）；
   · 全文只载一次（dataset.载 守住），折叠展开不重复请求。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $ } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 拉架构, 拉设计 } from "/传承呈现-府/文档展示-殿/文档绘现-阁/文档-渲染-园/文档渲染.js";

let 档, 身;
beforeEach(()=>{
  ({档} = 装台面());
  身 = 档.getElementById("展身");
});

const 清单桩 = (数据)=>{ globalThis.fetch = async ()=>({ok:true, json:async()=>数据}); };
const 等到 = ()=>new Promise(r=>setTimeout(r, 5));

test("拉架构：清单落成条目，名与径各自就位", async ()=>{
  清单桩({架构:[
    {名称:"总架构.md", 路径:"架构/总架构.md"},
    {名称:"蓝图.md", 路径:"架构/蓝图.md"},
  ]});
  await 拉架构();
  const 项 = 身.querySelectorAll(".文档项");
  assert.equal(项.length, 2);
  assert.equal(身.querySelector(".文档列").children.length, 2);
  assert.equal(项[0].querySelector(".文档名").textContent, "总架构.md");
  assert.equal(项[0].querySelector(".文档径").textContent, "架构/总架构.md");
});

test("拉架构：空清单如实说「未找到任何架构图」", async ()=>{
  清单桩({架构:[]});
  await 拉架构();
  assert.equal(身.querySelector(".空 .语").textContent, "未找到任何架构图");
});

test("拉架构失败：原因写进展身，且经转义", async ()=>{
  globalThis.fetch = async ()=>({ok:false, status:500});
  await 拉架构();
  assert.equal(身.querySelector(".空 .语").textContent, "架构图拉取失败：HTTP 500");

  globalThis.fetch = async ()=>{ throw new Error("断网 <x>"); };
  await 拉架构();
  assert.equal(身.querySelector(".空 .语").textContent, "架构图拉取失败：断网 <x>");
  assert.equal(身.querySelector("x"), null, "错误信息进 innerHTML 前必须转义");
});

test("拉设计：含「设计」者入选，任务级「设计表」与其他文档排除", async ()=>{
  清单桩({传承殿:[
    {名称:"边界契约设计.md", 路径:"传承殿/设计/边界契约设计.md"},
    {名称:"设计规范.md", 路径:"rules/设计规范.md"},
    {名称:"任务书.md", 路径:"传承殿/设计表/任务书.md"},
    {名称:"设计表-31.md", 路径:"传承殿/任务-31/设计表-31.md"},
    {名称:"交接记录.md", 路径:"传承殿/交接记录.md"},
  ]});
  await 拉设计();
  const 名 = [...身.querySelectorAll(".文档名")].map(x=>x.textContent);
  assert.deepEqual(名, ["边界契约设计.md", "设计规范.md"],
    "「设计表」是圣人每任务一张的产物，不是设计稿——放进来会淹没真正的设计文档");
});

test("条目展开：点开即载全文，收起再点开不重复请求", async ()=>{
  const 调用 = [];
  globalThis.fetch = async (路径)=>{
    调用.push(路径);
    if(路径 === "/api/files") return {ok:true, json:async()=>({架构:[{名称:"总架构.md", 路径:"架构/总架构.md"}]})};
    return {ok:true, json:async()=>({内容:"正文内容\n第二行"})};
  };
  await 拉架构();
  const 项 = 身.querySelector(".文档项");
  const 头 = 项.querySelector(".文档头");

  头.click();
  await 等到();
  assert.ok(项.classList.contains("开"));
  assert.equal(头.getAttribute("aria-expanded"), "true");
  assert.equal(头.querySelector(".文档箭头").textContent, "▾");
  assert.equal(项.querySelector(".文档文").textContent, "正文内容\n第二行");
  assert.equal(调用.at(-1), "/api/files/content?路径=" + encodeURIComponent("架构/总架构.md"),
    "全文路径必须编码后进查询串");

  const 全次数 = 调用.filter(x=>x.startsWith("/api/files/content")).length;
  头.click();   // 收起
  assert.equal(项.classList.contains("开"), false);
  assert.equal(头.querySelector(".文档箭头").textContent, "▸");
  头.click();   // 再开
  await 等到();
  assert.equal(调用.filter(x=>x.startsWith("/api/files/content")).length, 全次数, "全文只载一次");
  assert.equal(项.querySelector(".文档文").textContent, "正文内容\n第二行");
});

test("载全文失败：原因如实落在条目内", async ()=>{
  globalThis.fetch = async (路径)=>{
    if(路径 === "/api/files") return {ok:true, json:async()=>({架构:[{名称:"总架构.md", 路径:"架构/总架构.md"}]})};
    return {ok:false, status:500};
  };
  await 拉架构();
  身.querySelector(".文档头").click();
  await 等到();
  assert.equal(身.querySelector(".文档错").textContent, "内容读取失败：HTTP 500");
});
