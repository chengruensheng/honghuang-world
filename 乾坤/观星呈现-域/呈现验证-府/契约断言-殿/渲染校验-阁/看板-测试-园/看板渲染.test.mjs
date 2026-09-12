/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   看板渲染 用例：五层泳道 + 任务卡（数据取自 /api/board，后端权威）。
   锁的硬约束：
   · 五条泳道常驻——空的也要在，层级缺失不能被读成界面丢了层；
   · 终态集与后端流转逻辑同源（清理完成 / 已取消）；
   · 拉取失败不静默：失败原因写进界面，只说已确认的事实。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 终态集, 渲染看板, 拉看板, 定位任务 }
  from "/看板流转-府/泳道绘现-殿/卡面渲染-阁/看板-渲染-园/看板渲染.js";

let 档;
beforeEach(()=>{
  ({档} = 装台面());
  运行时.任务集 = [];
});

/** 适配后的任务形状（渲染看板 直接消费 运行时.任务集） */
const 任务 = (覆写={})=>Object.assign({
  id:31, title:"任务看板新增「挂起 / 恢复」状态", status:"清理完成", 角色:"太乙金仙", 轮次:1,
  阶段:{需求:"✓", 设计:"✓", 实现:"✓", 验收:"✓", 终审:"✓"},
  承接历史:["圣人","大罗金仙","准圣","道祖","太乙金仙"], 回退:null, 召回标记:false, 当前层级:null, 动态:[],
}, 覆写);

test("终态集与后端流转逻辑同源：清理完成 / 已取消", ()=>{
  assert.deepEqual(终态集, ["清理完成","已取消"]);
});

test("渲染看板：五泳道常驻，空泳道给「空闲」，统计只数流转中", ()=>{
  运行时.任务集 = [
    任务(),
    任务({id:27, title:"定向回退", status:"待修复", 角色:"大罗金仙", 轮次:3,
      阶段:{需求:"✓", 设计:"✓", 实现:"✓", 验收:"不通过", 终审:"—"},
      回退:{次:1, 由:"准圣", 至:"大罗金仙", 因:"状态历史回退链未记录来源", 目标层:"土"}}),
  ];
  渲染看板();
  const 泳道 = $("#泳道区").querySelectorAll(".泳道");
  assert.equal(泳道.length, 5, "五条泳道常驻——空的也要在，层级缺失不能被读成界面丢了层");
  assert.deepEqual([...泳道].map(x=>x.querySelector(".职").textContent),
    ["道祖","圣人","大罗金仙","准圣","太乙金仙"]);
  assert.equal(泳道[0].querySelector(".泳道身 .空 .语").textContent, "空闲");
  assert.equal($("#看板统").textContent, "共 2 个在办任务 · 1 个流转中", "终态不计入流转中");
});

test("任务卡：号 / 题 / 态类 / 轮次 / 承接链如实落屏", ()=>{
  运行时.任务集 = [任务()];
  渲染看板();
  const 卡 = $("#泳道区").querySelector(".卡[data-id='31']");
  assert.notEqual(卡, null, "卡片带 data-id——天机视图的棒按它定位到这张卡");
  assert.equal(卡.querySelector(".号").textContent, "#31");
  assert.equal(卡.querySelector(".题").textContent, "任务看板新增「挂起 / 恢复」状态");
  assert.ok(卡.querySelector(".态").classList.contains("成"), "终态着「成」色");
  assert.equal(卡.querySelector(".态").textContent, "清理完成");
  assert.equal(卡.querySelector(".轮").textContent, "第1轮");
  assert.ok(卡.querySelector(".回顾").textContent.includes("圣人 → 大罗金仙 → 准圣 → 道祖 → 太乙金仙"));
});

test("回退任务：回退语与回退因落卡，态类着「退」", ()=>{
  运行时.任务集 = [任务({id:27, status:"待修复", 角色:"大罗金仙", 回退:{次:2, 由:"准圣", 至:"大罗金仙", 因:"状态历史回退链未记录来源", 目标层:"土"}})];
  渲染看板();
  const 卡 = $("#泳道区").querySelector(".卡");
  assert.equal(卡.querySelector(".回退").textContent, "↩ 第2次回退 · 准圣 → 大罗金仙");
  assert.ok(卡.querySelector(".态").classList.contains("退"));
  assert.ok(卡.querySelector(".回顾").textContent.includes("状态历史回退链未记录来源"),
    "看得到失败还要能看到原因——回退因必须进回顾区");
});

test("待人工验收卡：落「通过 / 驳回」操作区，驳回原因七选一默认「实现错误」", ()=>{
  运行时.任务集 = [任务({id:40, title:"待验收的任务", status:"待人工验收", 角色:"道祖", 轮次:0, 阶段:{}, 承接历史:[]})];
  渲染看板();
  const 卡 = $("#泳道区").querySelector(".卡");
  assert.notEqual(卡.querySelector(".审"), null);
  const 选项 = [...卡.querySelectorAll(".审因 option")];
  assert.deepEqual(选项.map(o=>o.value),
    ["需求不清","设计不符","实现错误","测试不足","产出不完整","扩大范围","缩小范围"], "驳回原因固定七选一，与后端枚举同源");
  const 选中 = 选项.filter(o=>o.hasAttribute("selected"));
  assert.equal(选中.length, 1);
  assert.equal(选中[0].value, "实现错误");
  assert.notEqual(卡.querySelector(".审通过"), null);
  assert.notEqual(卡.querySelector(".审驳回"), null);
});

test("拉看板：成功即用后端数据重建（角色取当前承接人，轮次取修复轮次）", async ()=>{
  globalThis.fetch = async (路径)=>{
    assert.equal(路径, "/api/board");
    return {ok:true, json:async()=>([{
      id:31, title:"任务看板新增「挂起 / 恢复」状态", status:"待清理", 当前承接人:"太乙金仙",
      修复轮次:1, 状态历史:[{原状态:"道祖终审中", 新状态:"待清理", 操作者:"道祖", 时间:1789000000, 备注:"放行"}],
      层级历史:[{层级:"木", 状态:"已完成"},{层级:"火", 状态:"已完成"},{层级:"土", 状态:"已完成"},{层级:"金", 状态:"已完成"}],
      }])};
  };
  await 拉看板();
  assert.equal(运行时.任务集.length, 1);
  assert.equal(运行时.任务集[0].角色, "太乙金仙", "角色取「当前承接人」，没有才退回状态历史末条");
  assert.equal(运行时.任务集[0].轮次, 1);
  assert.equal(运行时.任务集[0].阶段["需求"], "✓");
  assert.equal($("#看板统").textContent, "共 1 个在办任务 · 1 个流转中");
  assert.equal($("#泳道区").querySelectorAll(".卡").length, 1);
});

test("拉看板失败：失败原因写进界面，不静默、不假装有数据", async ()=>{
  globalThis.fetch = async ()=>({ok:false, status:503});
  await 拉看板();
  assert.equal(运行时.任务集.length, 0);
  assert.ok($("#泳道区").textContent.includes("看板拉取失败：HTTP 503"));
  assert.equal($("#看板统").textContent, "看板拉取失败 · 可切回演示查看契约样例");
});

test("拉看板网络异常：同样落失败原因（错误信息经转义）", async ()=>{
  globalThis.fetch = async ()=>{ throw new Error("断网 <x>"); };
  await 拉看板();
  assert.ok($("#泳道区").textContent.includes("看板拉取失败：断网 <x>"));
  assert.equal($("#泳道区").querySelector("x"), null, "错误信息进 innerHTML 前必须转义");
});

test("定位任务：命中卡片即展开并打闪烁标；查无此卡静默", ()=>{
  运行时.任务集 = [任务({id:31})];
  渲染看板();
  定位任务(31);
  const 卡 = $("#泳道区 .卡[data-id='31']");
  assert.ok(卡.classList.contains("开"));
  assert.ok(卡.classList.contains("闪"));
  定位任务(99);   // 取不到就什么都不做，也不假装找到了
});
