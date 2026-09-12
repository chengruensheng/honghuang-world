/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   变更渲染 用例：工具结果的呈现——成败着色、参数引用名、精确编辑的增删对照。
   引用名与棒头摘要同源（摘要逻辑.工具引），两处各算一套必然漂移；
   失败着色不许静默：错误原文必须留在结果位，界面不替失败自述成功。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { 标工具引, 落结果, 渲染变更, 建码行 }
  from "/观测台面-府/天机事件-殿/分层绘现-阁/变更-渲染-园/变更渲染.js";

let 档;
beforeEach(()=>{ ({档} = 装台面()); });

/** 造一条工具条：结构与 棒条渲染.建条 产出同形 */
function 造工具条(){
  const 条 = document.createElement("div");
  条.className = "条 工具";
  条.innerHTML = `<div class="条头"><b>工 具 · 读文件</b><span class="引"></span><span class="折">▶</span></div><div class="条文"></div>`;
  return 条;
}

test("工具引：路径取末段、命令取前两词、模式原样", ()=>{
  const 条 = 造工具条();
  assert.equal(标工具引(条, '{"路径":"太初/任务管理-域/状态-枚举-园/状态枚举.rs"}'), "状态枚举.rs");
  assert.equal(条.querySelector(".引").textContent, "状态枚举.rs");

  assert.equal(标工具引(条, '{"路径":"工作区\\\\任务-31\\\\看板驱动-模块.rs"}'), "看板驱动-模块.rs", "反斜杠路径同等对待");
  assert.equal(标工具引(条, '{"命令":"cargo test -p tc-task -- --nocapture"}'), "cargo test", "命令只取前两词");
  assert.equal(标工具引(条, '{"模式":"**/*.bak"}'), "**/*.bak", "按名找文件只有模式，漏掉它摘要就退化成整段 JSON");
});

test("工具引：入参分片未成形时不写引用名", ()=>{
  const 条 = 造工具条();
  assert.equal(标工具引(条, '{"路径":"太初/任务管理'), "", "分片到达、JSON 未成形：取不到就不写");
  assert.equal(条.querySelector(".引").textContent, "");
  assert.equal(标工具引(条, ""), "");
});

test("落结果：失败关键词着色，结果原文如实落位", ()=>{
  const 条 = 造工具条();
  落结果(条, "读取成功，共 42 行。");
  assert.equal(条.querySelector(".结果").textContent, "读取成功，共 42 行。");
  assert.ok(!条.classList.contains("败"));

  落结果(条, "替换失败：文件被占用");
  assert.ok(条.classList.contains("败"));
  assert.equal(条.querySelector(".结果").textContent, "替换失败：文件被占用", "失败原因必须留在屏上，不静默");
  assert.equal(条.querySelectorAll(".结果").length, 1, "结果位只有一个：重落即覆盖，不叠层");
});

test("渲染变更：入参带「旧 / 新」时画增删对照，减在前加在后", ()=>{
  const 条 = 造工具条();
  const 参 = document.createElement("div");
  参.className = "参数";
  参.textContent = JSON.stringify({路径:"状态枚举.rs", 旧:"    // 待补充：挂起相关状态\n", 新:"    挂起中,\n    待恢复,\n"});
  条.querySelector(".条文").appendChild(参);

  渲染变更(条);
  const 码 = 条.querySelector(".码");
  assert.notEqual(码, null);
  assert.equal(码.querySelectorAll(".码行.减").length, 1, "旧文一行（尾换行不算空行）");
  assert.equal(码.querySelectorAll(".码行.加").length, 2, "新文两行");
  assert.equal(码.querySelector(".码行.减 .标").textContent, "-");
  assert.equal(码.querySelector(".码行.加 .标").textContent, "+");
  assert.equal(码.querySelector(".码行.减 .本").textContent, "    // 待补充：挂起相关状态");
  assert.equal(码.querySelector(".码行.加 .本").textContent, "    挂起中,");

  渲染变更(条);
  assert.equal(条.querySelectorAll(".码").length, 1, "已画过不再画一层");
});

test("渲染变更：参数缺失 / 非 JSON / 无旧新 三种情况都不画对照", ()=>{
  const 无参 = 造工具条();
  渲染变更(无参);
  assert.equal(无参.querySelector(".码"), null);

  const 非JSON = 造工具条();
  非JSON.querySelector(".条文").innerHTML = '<div class="参数">分片{"路径":</div>';
  渲染变更(非JSON);
  assert.equal(非JSON.querySelector(".码"), null);

  const 无旧新 = 造工具条();
  无旧新.querySelector(".条文").innerHTML = '<div class="参数">{"路径":"a.rs"}</div>';
  渲染变更(无旧新);
  assert.equal(无旧新.querySelector(".码"), null, "普通读文件不该被画成变更");
});

test("落结果与变更对照联动：结果落地时一并补画对照", ()=>{
  const 条 = 造工具条();
  const 参 = document.createElement("div");
  参.className = "参数";
  参.textContent = JSON.stringify({旧:"a\n", 新:"b\nc\n"});
  条.querySelector(".条文").appendChild(参);
  落结果(条, "替换成功，原子写。");
  assert.equal(条.querySelectorAll(".码行").length, 3);
});

test("建码行：加减标与原行文本；空行占位不塌陷", ()=>{
  const 加 = 建码行("加", "新增一行");
  assert.equal(加.className, "码行 加");
  assert.equal(加.querySelector(".标").textContent, "+");
  assert.equal(加.querySelector(".本").textContent, "新增一行");

  const 减 = 建码行("减", "");
  assert.equal(减.querySelector(".标").textContent, "-");
  assert.equal(减.querySelector(".本").textContent, " ", "空行也要占位，否则行高塌陷看不出增删位置");
});
