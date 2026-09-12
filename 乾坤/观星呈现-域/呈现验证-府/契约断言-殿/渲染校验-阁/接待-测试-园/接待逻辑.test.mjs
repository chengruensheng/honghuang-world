/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   接待逻辑 用例：道祖接待（POST + 手动读 SSE 帧）。
   锁的硬约束：
   · 思考与正文分道——草稿不冒充答复，也不丢（它是过程证据）；
   · 「有没有答复」看可见内容：只回空白也等于没答复，如实说不摆空壳；
   · 新话盖旧话：被盖掉的旧话静默退场，不落半截收尾；
   · 失败不静默：429 / 5xx / 断网各有如实的失败语。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 总线, 监听 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 问道祖 } from "/流式驱动-府/双流接入-殿/接待往返-阁/问答-逻辑-园/接待逻辑.js";

/** 造一条 SSE 响应体：按块表顺序下发，块间由 \n\n 分帧（与后端契约一致） */
function 造流(块表){
  const 编 = new TextEncoder();
  return new ReadableStream({ start(c){ 块表.forEach(b=>c.enqueue(编.encode(b))); c.close(); } });
}

/** 造一条「挂起直到被 abort」的响应体：用于验证新话盖旧话 */
function 可中断流(控){
  return new ReadableStream({
    pull(){
      return new Promise((_, 拒)=>{
        if(控.aborted) return 拒(new DOMException("Aborted", "AbortError"));
        控.addEventListener("abort", ()=>拒(new DOMException("Aborted", "AbortError")), {once:true});
      });
    },
  });
}

let 档, 收件;
beforeEach(()=>{
  ({档} = 装台面());
  运行时.接待记录 = []; 运行时.接待控 = null; 运行时.待确认 = false;
  收件 = {开启:[], 思考:[], 增量:[], 收尾:[], 拉看板:0};
  监听(总线.接待开启, d=>收件.开启.push(d));
  监听(总线.接待思考, d=>收件.思考.push(d));
  监听(总线.接待增量, d=>收件.增量.push(d));
  监听(总线.接待收尾, d=>收件.收尾.push(d));
  监听(总线.请求拉看板, ()=>收件.拉看板++);
});

test("接待往返：开启 → 思考/正文分道下发 → 收尾落待确认", async ()=>{
  const 调用 = [];
  globalThis.fetch = async (路径, 参)=>{ 调用.push([路径, 参]); return {status:200, ok:true, body: 造流([
    'data: {"type":"RUN_STARTED"}\n\n',
    'data: {"type":"REASONING_MESSAGE_CONTENT","delta":"先钉边界。"}\n\n',
    'data: {"type":"TEXT_MESSAGE_CONTENT","delta":"明白。"}\n\n',
    'data: {"type":"TEXT_MESSAGE_CONTENT","delta":"需要确认三点："}\n\n',
    'data: {"type":"RUN_FINISHED","阶段":"待确认"}\n\n',
  ])}; };

  await 问道祖("我要加挂起机制");

  assert.equal(调用[0][0], "/api/dev/chat/stream");
  assert.equal(调用[0][1].method, "POST");
  assert.deepEqual(JSON.parse(调用[0][1].body), {消息:"我要加挂起机制"}, "请求体是契约字段「消息」");
  assert.equal($("#连接语").textContent, "实时 · 未连接", "接待结束回落到刷连接语的口径（此刻无已连流）");

  const id = 运行时.接待记录.at(-1).id;
  assert.equal(收件.开启.length, 1);
  assert.equal(收件.开启[0].id, id);
  assert.equal(收件.开启[0].角色, "道祖");
  assert.deepEqual(收件.思考, [{id, 文本:"先钉边界。"}], "思考走独立通道");
  assert.deepEqual(收件.增量.map(x=>x.文本), ["明白。", "明白。需要确认三点："], "正文按到达累积");
  assert.equal(收件.收尾.length, 1);
  assert.equal(收件.收尾[0].文本, "明白。需要确认三点：");
  assert.equal(收件.收尾[0].待确认, true);
  assert.equal(运行时.待确认, true, "RUN_FINISHED 带「待确认」才点亮待确认态");
  assert.equal(运行时.接待控, null, "无论成败，控都要在 finally 里交还");
});

test("没有可显示答复：如实说没有，不摆空壳气泡", async ()=>{
  globalThis.fetch = async ()=>({status:200, ok:true, body: 造流([
    'data: {"type":"TEXT_MESSAGE_CONTENT","delta":"  \\n "}\n\n',
    'data: {"type":"RUN_FINISHED"}\n\n',
  ])});
  await 问道祖("喂");
  assert.equal(收件.收尾[0].文本, "（道祖未返回可显示的答复）", "只回空白等于没答复——不当成有效答复");
  assert.equal(收件.收尾[0].待确认, false);
});

test("429：接待通道已占用，如实说并保留重试余地", async ()=>{
  globalThis.fetch = async ()=>({status:429, ok:false});
  await 问道祖("喂");
  assert.equal(收件.收尾[0].文本, "接待失败：接待通道已占用（429），请稍候再发");
  assert.equal(收件.收尾[0].待确认, false);
});

test("5xx 与断网：各有如实的失败语", async ()=>{
  globalThis.fetch = async ()=>({status:500, ok:false});
  await 问道祖("喂");
  assert.equal(收件.收尾[0].文本, "接待失败：HTTP 500");

  globalThis.fetch = async ()=>{ throw new Error("断网"); };
  await 问道祖("喂");
  assert.equal(收件.收尾.at(-1).文本, "接待失败：断网");
});

test("RUN_FINISHED 带任务id：抬升权威看板", async ()=>{
  globalThis.fetch = async ()=>({status:200, ok:true, body: 造流([
    'data: {"type":"TEXT_MESSAGE_CONTENT","delta":"已发布。"}\n\n',
    'data: {"type":"RUN_FINISHED","任务id":31}\n\n',
  ])});
  await 问道祖("确认");
  assert.equal(收件.拉看板, 1);
});

test("新话盖旧话：旧话静默退场，不落半截收尾", async ()=>{
  globalThis.fetch = async (路径, 参)=>({status:200, ok:true, body: 可中断流(参.signal)});

  const 甲 = 问道祖("旧话");
  await new Promise(r=>setTimeout(r, 5));   // 让甲推进到读循环（挂起中）
  const 乙 = 问道祖("新话");                 // 触发 abort(甲)
  await 甲;
  assert.equal(收件.收尾.length, 0, "被盖掉的旧话不得收尾——半截气泡的收尾语是噪音");
  assert.equal(收件.开启.length, 2, "两颗气泡都开过场；退场的是内容通道，不是身份");

  const 乙控 = 运行时.接待控;
  assert.notEqual(乙控, null, "甲的 finally 不得误清乙的控");
  乙控.abort();
  await 乙;
  assert.equal(运行时.接待控, null);
  assert.equal(收件.收尾.length, 0, "主动掐断同样静默（AbortError 不是失败）");
});
