/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   筛选联动 用例：角色筛选的唯一真源。
   左栏「角色入口」与右栏「天机筛」共用 运行时.筛选——两处各存一份必然打架，
   本用例锁的就是「点哪一处，两栏一起动」。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 选角色, 渲染入口, 渲染天机筛 }
  from "/观测台面-府/筛选联动-殿/角色入口-阁/筛选-逻辑-园/筛选联动.js";
import { 收帧 } from "/观测台面-府/天机事件-殿/帧流归位-阁/编排-逻辑-园/帧编排.js";

let 档;
beforeEach(()=>{
  ({档} = 装台面());
  运行时.模式 = "实时";   // 演示模式会铺契约样例语料，联动断言用实时模式更干净
  运行时.筛选 = "全部";
  运行时.事件缓冲 = []; 运行时.接待记录 = [];
  运行时.棒表.clear(); 运行时.角色棒.clear();
  运行时.当前run = null; 运行时.当前角色 = ""; 运行时.段序号 = 0;
});

test("角色入口：全体 + 五角色，当前项带选中态", ()=>{
  渲染入口();
  const 钮 = $("#角色入口").querySelectorAll("button");
  assert.equal(钮.length, 6);
  assert.equal(钮[0].textContent, "全体");
  assert.ok(钮[0].classList.contains("选"));
  assert.deepEqual([...钮].slice(1).map(b=>b.textContent), ["道祖","圣人","大罗金仙","准圣","太乙金仙"]);

  运行时.筛选 = "准圣";
  渲染入口();
  const 新钮 = $("#角色入口").querySelectorAll("button");
  assert.ok(新钮[4].classList.contains("选"), "准圣是第 4 个角色（索引 4）");
  assert.ok(!新钮[0].classList.contains("选"), "选中同名互斥：全体不得与单个角色同时亮");
});

test("天机筛：全部 + 五角色，与左栏同一份筛选状态", ()=>{
  运行时.筛选 = "大罗金仙";
  渲染天机筛();
  const 钮 = [...$("#天机筛").querySelectorAll("button")];
  assert.deepEqual(钮.map(b=>b.textContent), ["全部","道祖","圣人","大罗金仙","准圣","太乙金仙"]);
  assert.equal(钮.filter(b=>b.classList.contains("选")).length, 1, "同时只能有一个亮");
  assert.equal(钮[3].textContent, "大罗金仙");
  assert.ok(钮[3].classList.contains("选"));
});

test("选角色：一处选定，两栏入口、右栏事件、左栏对话同时聚焦", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"RUN_STARTED", runId:"r2", 角色:"准圣"});
  assert.equal($("#事件流").querySelectorAll(".棒").length, 2);

  选角色("圣人");
  assert.equal(运行时.筛选, "圣人", "筛选的唯一真源必须先落定");

  const 入口钮 = [...$("#角色入口").querySelectorAll("button")].find(b=>b.textContent === "圣人");
  assert.ok(入口钮.classList.contains("选"));
  const 筛钮 = [...$("#天机筛").querySelectorAll("button")].find(b=>b.textContent === "圣人");
  assert.ok(筛钮.classList.contains("选"), "右栏天机筛与左栏必须在同一状态上");

  assert.equal($("#事件流").querySelectorAll(".棒").length, 1, "右栏按新筛选重建");
  assert.equal($("#事件流").querySelector(".棒 .角").textContent, "圣人");
  assert.notEqual($("#对话流").querySelector(".空"), null, "左栏同退：指名角色时无该角发言就如实说");

  选角色("全部");
  assert.equal($("#事件流").querySelectorAll(".棒").length, 2, "回到全部：缓冲里的两棒都回来");
});

test("左右两处按钮点击走同一份口径", ()=>{
  const 入口 = $("#角色入口");
  渲染入口(); 渲染天机筛();

  [...入口.querySelectorAll("button")].find(b=>b.textContent === "道祖").click();
  assert.equal(运行时.筛选, "道祖");

  [...$("#天机筛").querySelectorAll("button")].find(b=>b.textContent === "太乙金仙").click();
  assert.equal(运行时.筛选, "太乙金仙", "右栏点击同样改写唯一真源");
  const 入口钮 = [...$("#角色入口").querySelectorAll("button")].find(b=>b.textContent === "太乙金仙");
  assert.ok(入口钮.classList.contains("选"), "右栏点的，左栏也要亮");
});
