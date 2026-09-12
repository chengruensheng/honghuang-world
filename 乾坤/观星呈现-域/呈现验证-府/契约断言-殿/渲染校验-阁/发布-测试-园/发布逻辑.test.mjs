/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   发布逻辑 用例：确认发布入口 + 实时/演示两条发布路径。
   锁的硬约束：
   · 发布以「后端确认」为准：实时路径成功才移除确认钮、才拉权威看板；
   · 失败不静默：钮恢复可点、状态语写明原因（界面只说已确认的事实）；
   · 演示路径经总线请顶层重放——下层不反向调用顶层，依赖方向单向。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 总线, 监听 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 落确认钮, 发布 }
  from "/观测台面-府/万仙对话-殿/确认绑定-阁/发布-逻辑-园/发布逻辑.js";

let 档, 流;
beforeEach(()=>{
  ({档} = 装台面());
  流 = 档.getElementById("对话流");
  运行时.模式 = "实时"; 运行时.待确认 = false;
});

test("落确认钮：落一张待确认卡，重复落只认第一次", ()=>{
  落确认钮(流);
  assert.equal(流.querySelectorAll(".摘要卡").length, 1);
  assert.equal(流.querySelector(".卡头").textContent, "道祖已完成需求对齐 · 待确认");
  assert.notEqual(流.querySelector("#发布钮"), null);

  落确认钮(流);
  assert.equal(流.querySelectorAll(".摘要卡").length, 1, "确认入口只有一个——重复落等于给用户两个发布按钮");
});

test("演示路径：钮转已达文案，经总线请顶层重放", async ()=>{
  运行时.模式 = "演示";
  落确认钮(流);
  const 重放 = [];
  监听(总线.请求重放, ()=>重放.push(1));

  await 发布();
  const 钮 = 流.querySelector("#发布钮");
  assert.equal(钮.disabled, true);
  assert.equal(钮.textContent, "已发布 · 任务 #31 已进入五层流转");
  assert.equal($("#连接语").textContent, "演示数据 · 契约对齐");
  assert.equal(重放.length, 1, "经总线发令——下层不 import 顶层");
});

test("实时路径成功：以后端确认落定，抬升权威数据并撤下确认钮", async ()=>{
  运行时.待确认 = true;
  落确认钮(流);
  const 调用 = [];
  globalThis.fetch = async (路径, 参)=>{ 调用.push([路径, 参]); return {ok:true, json:async()=>({任务id:31})}; };
  const 拉 = [];
  监听(总线.请求拉看板, ()=>拉.push(1));

  await 发布();
  assert.equal(调用[0][0], "/api/dev/chat/confirm");
  assert.equal(调用[0][1].method, "POST");
  assert.equal(运行时.待确认, false);
  assert.equal($("#连接语").textContent, "实时 · 已发布 任务 #31");
  assert.equal(流.querySelector("#发布钮"), null, "后端确认落定才撤钮");
  assert.equal(拉.length, 1, "发布后抬升权威看板——经总线，不直连看板府");
});

test("实时路径失败：钮恢复可点，状态语写明原因", async ()=>{
  落确认钮(流);
  globalThis.fetch = async ()=>({ok:false, status:500});

  await 发布();
  const 钮 = 流.querySelector("#发布钮");
  assert.equal($("#连接语").textContent, "实时 · 发布失败：HTTP 500");
  assert.equal(钮.disabled, false, "失败要能重试，不能把用户锁在禁用态");
  assert.equal(钮.textContent, "确 认 发 布 · 落 看 板");
});

test("网络异常同样落失败原因；无钮时不炸", async ()=>{
  globalThis.fetch = async ()=>{ throw new Error("断网"); };
  await 发布();   // 无确认钮：路径本身不得依赖钮存在
  assert.equal($("#连接语").textContent, "实时 · 发布失败：断网");
});
