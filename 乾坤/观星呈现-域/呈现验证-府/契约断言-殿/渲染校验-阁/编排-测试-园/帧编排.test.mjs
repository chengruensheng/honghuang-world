/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   帧编排 用例：AG-UI 帧 → 「棒 → 条」的归位规则。
   这是两栏同源的枢纽：右栏棒、左栏结论、阶段条都从这里分派。
   锁的硬约束：
   · 收尾帧按角色归位到「该角色当前那一段任期」，不为已结束的角色再造棒；
   · STATE_DELTA 的 /status 流转是「→ 下一步」的唯一权威（模型自述不算）；
   · 筛选只影响落屏，不丢缓冲——切筛选要靠缓冲整体重建。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 渲染天机空态 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 收帧, 落帧, 显阶段, 重绘天机, 复位呈现, 播放脚本 }
  from "/观测台面-府/天机事件-殿/帧流归位-阁/编排-逻辑-园/帧编排.js";
import { 事件脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";

let 档;
beforeEach(()=>{
  ({档} = 装台面());
  运行时.事件缓冲 = []; 运行时.接待记录 = []; 运行时.任务集 = [];
  运行时.棒表.clear(); 运行时.角色棒.clear(); 运行时.待收尾.clear();
  运行时.展开棒.clear(); 运行时.展开条.clear();
  运行时.筛选 = "全部"; 运行时.模式 = "演示";
  运行时.当前run = null; 运行时.当前角色 = ""; 运行时.段序号 = 0;
  运行时.待确认 = false; 运行时.播放计时 = [];
});

test("RUN_STARTED 建棒：当前棒锚定，任务号从 input 取（旧契约形状）", ()=>{
  收帧({type:"RUN_STARTED", runId:"run-31-1", 角色:"圣人", input:{任务id:31, 角色:"圣人"}});
  assert.equal(运行时.当前run, "run-31-1");
  assert.equal(运行时.当前角色, "圣人");
  assert.equal(运行时.棒表.size, 1);
  assert.equal(运行时.棒表.get("run-31-1").数据.任务id, 31, "演示语料把任务号放在 input 里");
  assert.equal($("#事件流").querySelectorAll(".棒").length, 1);
});

test("STATE_DELTA：/status 流转记作交接，补丁逐条成「状态」条（移除也要有着落）", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"STATE_DELTA", 角色:"圣人", delta:[
    {op:"replace", path:"/任务/31/status", value:"待大罗金仙实现"},
    {op:"replace", path:"/任务/31/当前承接人", value:"大罗金仙"},
    {op:"remove", path:"/任务/31/临时标记"},
  ]});
  const 记 = 运行时.棒表.get("r1");
  assert.equal(记.数据.交接, "待大罗金仙实现", "「→ 下一步」的权威只认状态机流转");
  const 条 = 记.dom.querySelector(".条.状态 .引");
  assert.equal(条.textContent, "/任务/31/status → 待大罗金仙实现；/任务/31/当前承接人 → 大罗金仙；/任务/31/临时标记（移除）",
    "补丁逐条列出；op=remove 没有 value，照实写「移除」，不给界面留 undefined");
});

test("STEP_FINISHED 收尾：无交接时只说「已提交」，stepName 进阶段条", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"STEP_FINISHED", stepName:"圣人 · 边界契约设计", 角色:"圣人"});
  const 记 = 运行时.棒表.get("r1");
  assert.equal(记.数据.完成, true);
  assert.equal(记.数据.终态, "已提交", "stepName 是「角色 · 职责」，不是交接目标，不能拿来充数");
  assert.equal($("#阶段条").querySelector("b").textContent, "圣人 · 边界契约设计");
  assert.equal($("#阶段条").style.display, "flex");
});

test("RUN_ERROR 收尾为中断，错误原文成条落屏", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"RUN_ERROR", 角色:"圣人", message:"熔断", code:"E511"});
  const 记 = 运行时.棒表.get("r1");
  assert.equal(记.数据.失败, true);
  assert.equal(记.数据.终态, "运行错误 · 熔断 [E511]");
  assert.notEqual(记.dom.querySelector(".条.工具.败"), null);
  assert.ok(记.dom.querySelector(".条.工具.败 .条文").textContent.includes("熔断"));
});

test("角色换人（未发 RUN_STARTED）即切段：旧棒占位收尾、新棒自造 id", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"REASONING_MESSAGE_CONTENT", 角色:"大罗金仙", delta:"开始实现"});
  assert.equal(运行时.段序号, 1);
  assert.equal(运行时.当前run, "段-1-大罗金仙");
  assert.equal(运行时.棒表.size, 2);
  const 旧 = 运行时.棒表.get("r1");
  assert.equal(旧.数据.完成, true);
  assert.equal(旧.数据.占位, true);
  assert.equal(旧.数据.终态, "已交接下一棒");
  const 新 = 运行时.棒表.get("段-1-大罗金仙");
  assert.equal(新.数据.角色, "大罗金仙");
  assert.equal(新.dom.querySelector(".条.推理 .条文").textContent, "开始实现");
});

test("收尾帧按角色归位：迟到者收自己那一段，不为已结束的角色再造棒", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"RUN_STARTED", runId:"r2", 角色:"大罗金仙"});
  收帧({type:"STEP_FINISHED", stepName:"圣人 · 边界契约设计", 角色:"圣人"});
  assert.equal(运行时.棒表.size, 2, "收尾帧的角色指向已结束的棒，不是当前活动角色");
  assert.equal(运行时.棒表.get("r1").数据.终态, "已提交", "真终态覆盖占位语");
  assert.equal(运行时.棒表.get("r2").数据.完成, false, "另一根仍在任期");
});

test("该角色一根棒都没有时终态先暂存，建棒那一刻补收", ()=>{
  收帧({type:"STEP_FINISHED", stepName:"准圣 · 逐项验收", 角色:"准圣"});
  assert.equal(运行时.棒表.size, 0, "无棒可归时不得凭空造棒");
  assert.equal(运行时.待收尾.has("准圣"), true);

  收帧({type:"RUN_STARTED", runId:"r9", 角色:"准圣"});
  const 记 = 运行时.棒表.get("r9");
  assert.equal(记.数据.完成, true, "阶段帧先到、过程帧后到：终态不跨过建棒这一刻丢失");
  assert.equal(运行时.待收尾.has("准圣"), false);
});

test("TEXT_MESSAGE_CONTENT：命中筛选落左栏，结论与右栏同源；不命中只进缓冲", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"TEXT_MESSAGE_CONTENT", 角色:"圣人", delta:"设计完成。接口：POST /api/board/{id}/suspend。"});
  assert.equal($("#对话流").querySelectorAll(".消息").length, 1);
  assert.equal($("#对话流").querySelector(".结句").textContent, "设计完成。接口：POST /api/board/{id}/suspend。");
  assert.equal(运行时.棒表.get("r1").数据.首句, "设计完成。接口：POST /api/board/{id}/suspend。",
    "首句是棒头摘要的来源——只认正文，不认推理草稿");

 运行时.筛选 = "太乙金仙";
  收帧({type:"TEXT_MESSAGE_CONTENT", 角色:"大罗金仙", delta:"实现完成。"});
  assert.equal($("#对话流").querySelectorAll(".消息").length, 1, "筛选不命中：不落屏");
  assert.equal(运行时.事件缓冲.length, 3, "但必须进缓冲——切筛选要靠它重建");
});

test("重绘天机：按缓冲整体重建，空结果给空态、但缓冲不动", ()=>{
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"RUN_STARTED", runId:"r2", 角色:"大罗金仙"});

  运行时.筛选 = "准圣";
  重绘天机();
  assert.equal($("#事件流").querySelectorAll(".棒").length, 0);
  assert.equal($("#事件流").querySelector(".空 .语").textContent, "该角色暂无事件");

  运行时.筛选 = "大罗金仙";
  重绘天机();
  assert.equal($("#事件流").querySelectorAll(".棒").length, 1);
  assert.equal($("#事件流").querySelector(".棒 .角").textContent, "大罗金仙");
  assert.equal(运行时.事件缓冲.length, 2, "重绘读缓冲，不改缓冲");

  运行时.筛选 = "全部";
  重绘天机();
  assert.equal($("#事件流").querySelectorAll(".棒").length, 2, "回到全部：两根棒都要回来");
});

test("复位呈现：清右栏、清缓冲、隐阶段条、左栏同退", ()=>{
  运行时.模式 = "实时";   // 演示模式会铺契约样例语料，复位口径用实时模式验证
  收帧({type:"RUN_STARTED", runId:"r1", 角色:"圣人"});
  收帧({type:"TEXT_MESSAGE_CONTENT", 角色:"圣人", delta:"x"});
  显阶段("圣人 · 边界契约设计");

  复位呈现("实时 · 连接中…");
  assert.equal($("#事件流").querySelectorAll(".棒").length, 0);
  assert.equal($("#事件流").querySelector(".空 .语").textContent, "实时 · 连接中…");
  assert.equal(运行时.事件缓冲.length, 0);
  assert.equal($("#阶段条").style.display, "none", "复位即隐藏：上一模式的阶段残留不得冒充当前状态");
  assert.equal($("#对话流").querySelectorAll(".消息").length, 0);
  assert.notEqual($("#对话流").querySelector(".空"), null, "左栏同退到空态");
});

test("显阶段：阶段名落 b 元素，条显示", ()=>{
  显阶段("大罗金仙 · 代码实现与自检");
  assert.equal($("#阶段条").querySelector("b").textContent, "大罗金仙 · 代码实现与自检");
  assert.equal($("#阶段条").style.display, "flex");
});

test("演示重放按 300ms 节拍喂帧，句柄入册供一次刹车", (t)=>{
  t.mock.timers.enable({apis:["setTimeout"]});
  播放脚本([
    {type:"RUN_STARTED", runId:"r1", 角色:"圣人"},
    {type:"STEP_FINISHED", stepName:"圣人 · 边界契约设计", 角色:"圣人"},
  ]);
  assert.equal(运行时.播放计时.length, 2, "计时句柄必须入册——停() 要能一次清光");
  assert.equal(运行时.棒表.size, 0, "未到点不得提前喂帧");

  t.mock.timers.tick(300);
  assert.equal(运行时.棒表.size, 1);
  t.mock.timers.tick(300);
  assert.equal(运行时.棒表.get("r1").数据.完成, true);
  t.mock.timers.reset();
});

test("端到端：契约样例全序列喂完，五棒归位、每棒收尾、左栏同源五条", ()=>{
  事件脚本.forEach(收帧);

  const 列 = [...运行时.棒表.values()];
  assert.deepEqual(列.map(r=>r.数据.角色), ["圣人","大罗金仙","准圣","道祖","太乙金仙"],
    "五层接力：一棒归一角色，顺序即交棒顺序");
  assert.equal(运行时.事件缓冲.length, 事件脚本.length, "缓冲收全量帧，不丢不重");
  列.forEach((r,i)=>assert.equal(r.数据.完成, true, `第 ${i+1} 棒未收尾——界面会把它停在「进行中」`));
  列.forEach(r=>assert.equal(r.数据.任务id, 31, "每根棒都要锚定任务号，否则看板定位无从谈起"));

  assert.equal(列[0].数据.交接, "待大罗金仙实现");
  assert.equal(列[3].数据.交接, "待清理");
  assert.equal(列[4].数据.交接, "清理完成");
  assert.equal(列[4].数据.终态, "已交出 · 清理完成", "末棒由 STEP_FINISHED 落定（交接取自状态机流转）");

  assert.equal($("#事件流").querySelectorAll(".棒").length, 5);
  assert.equal($("#对话流").querySelectorAll(".消息").length, 5,
    "左栏结论由 TEXT_MESSAGE_CONTENT 生成——与右栏读同一份缓冲，两栏同源");
});
