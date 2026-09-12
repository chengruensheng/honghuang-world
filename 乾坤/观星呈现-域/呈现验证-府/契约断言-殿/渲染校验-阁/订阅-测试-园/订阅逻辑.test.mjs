/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   订阅逻辑 用例：双流接入 + 演示重放 + 顶栏连接状态语。
   本府只「取数」不碰呈现——收到的帧一律经总线转交，测试据此断言
   「只发令、不越界」。锁的硬约束：
   · 词表外事件不转交（白名单之外的一律拦在门外）；
   · 连接状态语只描述已确认的事实（未连/部分/已连 三态分明）；
   · 停流时先摘 onerror 再关：关闭动作本身不得被写成「断线」。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面, 装事件源桩 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 总线, 监听 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 停, 重放事件, 刷连接语, 订阅, 连实时 }
  from "/流式驱动-府/双流接入-殿/双流拉取-阁/订阅-逻辑-园/订阅逻辑.js";
import { 事件脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";

let 档, 源表;
beforeEach(()=>{
  ({档} = 装台面());
  源表 = 装事件源桩();
  运行时.播放计时 = []; 运行时.已连流.clear();
  运行时.事件源 = null; 运行时.阶段源 = null; 运行时.待确认 = false;
});

test("刷连接语三态：未连 / 部分 / 已连，且右栏空态随之对齐", ()=>{
  刷连接语();
  assert.equal($("#连接语").textContent, "实时 · 未连接");
  assert.equal($("#事件流").querySelector(".空 .语").textContent, "实时流未连接 · 检查后端服务是否在运行");

  运行时.已连流.add("过程流");
  刷连接语();
  assert.equal($("#连接语").textContent, "实时 · 部分连接（过程流）");
  assert.equal($("#事件流").querySelector(".空 .语").textContent, "实时部分连接，等待任务事件…");

  运行时.已连流.add("阶段流");
  刷连接语();
  assert.equal($("#连接语").textContent, "实时 · 已连接 过程流 + 阶段流");
  assert.equal($("#事件流").querySelector(".空 .语").textContent, "实时已连接，等待任务事件…");
});

test("订阅：onopen 记流，onmessage 过词表后经总线转交，词表外与坏帧拦下", ()=>{
  const 收到 = [];
  监听(总线.收到帧, d=>收到.push(d.帧));

  const 源 = 订阅("/api/dev/stream/agui", "过程流");
  assert.equal(源, 源表[0]);
  assert.equal(源.路径, "/api/dev/stream/agui");

  源.onopen();
  assert.equal(运行时.已连流.has("过程流"), true);
  assert.equal($("#连接语").textContent, "实时 · 部分连接（过程流）");

  源.onmessage({data: JSON.stringify({type:"RUN_STARTED", runId:"r1"})});
  assert.equal(收到.length, 1);
  assert.equal(收到[0].runId, "r1");

  源.onmessage({data: JSON.stringify({type:"凭空事件_TYPE"})});
  assert.equal(收到.length, 1, "词表外的帧不得转交——白名单之外一律拦在门外");

  源.onmessage({data: "不是 JSON"});
  assert.equal(收到.length, 1, "坏帧不抛、不转交：一条坏帧不得掀掉整条流");
});

test("订阅：onerror 摘流并把状态语拉回如实态", ()=>{
  const 源 = 订阅("/api/dev/stream/agui", "过程流");
  源.onopen();
  源.onerror();
  assert.equal(运行时.已连流.has("过程流"), false);
  assert.equal($("#连接语").textContent, "实时 · 未连接");
});

test("停()：清重放计时、摘 onerror 再关双流、清已连流", ()=>{
  运行时.播放计时 = [setTimeout(()=>{ throw new Error("重放计时未被停掉"); }, 50)];
  运行时.事件源 = 订阅("/x", "过程流");
  运行时.阶段源 = 订阅("/y", "阶段流");
  运行时.已连流.add("过程流"); 运行时.已连流.add("阶段流");

  停();
  assert.equal(运行时.播放计时.length, 0);
  assert.equal(运行时.事件源, null);
  assert.equal(运行时.阶段源, null);
  assert.equal(源表[0].已关, true);
  assert.equal(源表[1].已关, true);
  assert.equal(源表[0].onerror, null, "先摘 onerror 再关——否则关闭动作本身会被当成断线写进状态语");
  assert.equal(运行时.已连流.size, 0, "关流时已连流一并清空，不留幽灵连接");
});

test("重放事件：停旧流 → 请复位 → 演示状态语 → 播放契约样例", ()=>{
  const 复位 = [], 播放 = [];
  监听(总线.复位呈现, d=>复位.push(d));
  监听(总线.播放脚本, d=>播放.push(d));
  运行时.待确认 = true;

  重放事件();
  assert.equal($("#连接语").textContent, "演示数据 · 契约对齐");
  assert.equal(运行时.待确认, false, "演示不跑确认发布流程");
  assert.equal(复位.length, 1);
  assert.equal(复位[0].空态语, undefined, "重放不给空态语：复位到中立初始态，由播放的帧自己说话");
  assert.equal(播放.length, 1);
  assert.equal(播放[0].脚本, 事件脚本, "重放的正是契约样例脚本——演示与断言的同源");
});

test("连实时：先停后连，两条契约流并行订阅", ()=>{
  const 复位 = [];
  监听(总线.复位呈现, d=>复位.push(d));

  连实时();
  assert.equal(源表.length, 2);
  assert.deepEqual(源表.map(s=>s.路径), ["/api/dev/stream/agui", "/api/dev/stream/agui/state"],
    "过程流 + 阶段流是两条契约流，路径写死在契约里");
  assert.equal(运行时.事件源, 源表[0]);
  assert.equal(运行时.阶段源, 源表[1]);
  assert.equal(复位[0].空态语, "实时 · 连接中…");
  assert.equal($("#连接语").textContent, "实时 · 连接中…");

  源表[0].onopen(); 源表[1].onopen();
  assert.equal($("#连接语").textContent, "实时 · 已连接 过程流 + 阶段流");
});

test("连实时重入：旧流先关，不出现两套流同开", ()=>{
  连实时();
  const 旧过程 = 源表[0], 旧阶段 = 源表[1];
  连实时();
  assert.equal(旧过程.已关, true);
  assert.equal(旧阶段.已关, true, "重连前先停旧流——否则状态语与帧来源都会翻倍");
  assert.equal(源表.length, 4);
});
