/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 语料校验-阁
   演示语料 用例：它是「契约样例」，不是另一套发明——样例一旦自身不闭合，
   界面在演示模式下照样会出错，而实时模式的缺陷就此被掩盖。
   故此处验证的不是「数据长得对」，而是帧序列能否被 帧编排 正确地消费。
   ============================================================ */

import { test } from "node:test";
import assert from "node:assert/strict";
import { 对话脚本, 演示任务集, 事件脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";
import { 事件词表 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";

test("事件脚本每一帧的 type 都在事件词表内（不得自创事件）", ()=>{
  const 外 = 事件脚本.filter(ev=>!事件词表.includes(ev.type));
  assert.deepEqual(外, [], "样例若含词表外事件，实时流根本不会发它——两套契约当场分家");
});

test("工具帧三件套配对：ARGS / RESULT / END 必有前置同 id 的 START", ()=>{
  const 已开 = new Set();
  事件脚本.forEach((ev, i)=>{
    if(ev.type === "TOOL_CALL_START") 已开.add(ev.toolCallId);
    if(["TOOL_CALL_ARGS","TOOL_CALL_RESULT","TOOL_CALL_END"].includes(ev.type)){
      assert.ok(已开.has(ev.toolCallId), `第 ${i+1} 帧 ${ev.type} 的 ${ev.toolCallId} 无对应 TOOL_CALL_START`);
    }
  });
});

test("每根棒都以 RUN_STARTED 开局、以 STEP_FINISHED 收尾，一一配对", ()=>{
  const 开局 = 事件脚本.filter(ev=>ev.type === "RUN_STARTED").map(ev=>ev.runId);
  const 收尾 = 事件脚本.filter(ev=>ev.type === "STEP_FINISHED").length;
  assert.equal(收尾, 开局.length, "有棒未收尾：界面会永远停在「进行中」，看着像卡死");
  assert.equal(new Set(开局).size, 开局.length, "runId 不得重复");
});

test("RUN_FINISHED 全序列仅一条，且落在最后一棒（逐棒发会让收尾有两种载体）", ()=>{
  const 位 = 事件脚本.map((ev, i)=>ev.type === "RUN_FINISHED" ? i : -1).filter(i=>i >= 0);
  assert.equal(位.length, 1, "RUN_FINISHED 只在整场会话落定时发一条");
  assert.equal(位[0], 事件脚本.length - 1, "它必须是最后一帧");
});

test("每棒的收尾以 STATE_DELTA 的 /status 流转表达「交出了什么」", ()=>{
  const 分段 = [];
  let 段 = null;
  事件脚本.forEach(ev=>{
    if(ev.type === "RUN_STARTED"){ 段 = {runId:ev.runId, 角色:ev.角色, 帧:[]}; 分段.push(段); }
    if(段) 段.帧.push(ev);
  });
  assert.equal(分段.length, 5, "本项目五层接力 = 五根棒");
  assert.deepEqual(分段.map(s=>s.角色), ["圣人","大罗金仙","准圣","道祖","太乙金仙"]);
  分段.forEach(段=>{
    const 状 = 段.帧.filter(ev=>ev.type === "STATE_DELTA")
      .flatMap(ev=>ev.delta || []).filter(d=>typeof d.path === "string" && d.path.endsWith("/status"));
    assert.ok(状.length >= 1, `${段.角色}这一棒缺少 /status 流转——棒头「→ 下一步」会永远空着`);
  });
});

test("每根棒都带 任务id（实时在顶层，样例在 input 内），棒与看板卡锚得上", ()=>{
  const starts = 事件脚本.filter(ev=>ev.type === "RUN_STARTED");
  starts.forEach(ev=>{
    const 号 = ev.任务id || (ev.input && ev.input.任务id);
    assert.equal(号, 31, `${ev.runId} 未锚任务号，棒头点不进看板`);
  });
});

test("对话脚本覆盖接待三方（来客 / 道祖 / 系统），且末条为发布回执", ()=>{
  const 角色集 = [...new Set(对话脚本.map(m=>m.角色))].sort();
  assert.deepEqual(角色集, ["system","来客","道祖"].map(x=>x === "system" ? "系统" : x).sort());
  assert.ok(对话脚本.at(-1).文.includes("需求已发布"), "接待流程须以发布回执收束");
});

test("演示任务集四项任务字段齐备，状态互为不同分层", ()=>{
  assert.equal(演示任务集.length, 4);
  演示任务集.forEach(t=>{
    assert.equal(typeof t.id, "number");
    assert.ok(t.title && t.title.length > 0, "标题不得为空——演示数据也不许占位");
    assert.ok(t.status, "状态缺失则卡片着色无从判断");
    assert.ok(t.角色, "承接角色缺失则泳道归档无处可去");
    assert.ok(t.阶段 && Object.keys(t.阶段).length === 5, "阶段链须覆盖五层");
  });
  assert.equal(new Set(演示任务集.map(t=>t.status)).size, 演示任务集.length, "四种状态应各不相同，演示才看得出分层");
});

test("演示任务集含回退样本，且回退记录字段与后端 回退记录.rs 同名", ()=>{
  const 回 = 演示任务集.filter(t=>t.回退);
  assert.equal(回.length, 1, "须有一个回退样本，否则恢复区在演示下无法呈现");
  assert.deepEqual(Object.keys(回[0].回退).sort(), ["因","由","至","次"].sort());
  assert.equal(回[0].轮次, 3, "该样本轮次为 3（三次修复），对应回退链");
});
