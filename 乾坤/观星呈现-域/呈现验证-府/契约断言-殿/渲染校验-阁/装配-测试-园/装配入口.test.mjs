/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   装配入口 用例：域层唯一装配点的端到端验证。
   import 本模块即触发 启动()——所有府真实装载、真实接线，
   因此本文件跑的是「整域装配 + 总线转接」的真链路，而非桩件对打。
   锁的硬约束：
   · 首屏直连真实契约（模式=实时、双流订阅、看板拉一次）；
   · 帧经总线转交给观测台面，底栏同步当前任务与角色（同棒只写一次）；
   · 府与府不互相 import：全部跨府协同都经总线接线表走。
   ============================================================ */

import { test } from "node:test";
import assert from "node:assert/strict";
import { 装台面, 装事件源桩, 断言骨架齐备 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 总线, 广播 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 事件脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";

const 调用 = [];
const 造流 = (块表)=>{ const 编 = new TextEncoder(); return new ReadableStream({ start(c){ 块表.forEach(b=>c.enqueue(编.encode(b))); c.close(); } }); };

const 桩取数 = async (路径)=>{
  调用.push(路径);
  if(路径 === "/api/board") return {ok:true, json:async()=>([
    {id:31, title:"任务看板新增「挂起 / 恢复」状态", status:"待清理", 当前承接人:"太乙金仙", 修复轮次:1, 状态历史:[], 层级历史:[]},
  ])};
  if(路径.startsWith("/api/dev/chat/stream")) return {status:200, ok:true, body: 造流([
    'data: {"type":"TEXT_MESSAGE_CONTENT","delta":"收到。"}\n\n',
    'data: {"type":"RUN_FINISHED"}\n\n',
  ])};
  if(路径 === "/api/files") return {ok:true, json:async()=>({架构:[{名称:"总架构.md", 路径:"架构/总架构.md"}], 传承殿:[]})};
  if(路径.startsWith("/api/files/content")) return {ok:true, json:async()=>({内容:"正文"})};
  if(路径.startsWith("/api/cognition/cells")) return {ok:true, json:async()=>({格位集:[]})};
  if(路径 === "/api/dev/chat/confirm") return {ok:true, json:async()=>({任务id:31})};
  return {ok:false, status:404};
};

/** 轮询等待：装配是异步的（逐府动态装载 + 首屏取数），断言前须等条件成立 */
async function 等到(条, 超时 = 3000){
  const 起 = Date.now();
  while(!条()){
    if(Date.now() - 起 > 超时) throw new Error("等待装配超时");
    await new Promise(r=>setTimeout(r, 10));
  }
}

const {档} = 装台面({取数: 桩取数, 事件源: false});
断言骨架齐备(档);
const 源表 = 装事件源桩();
await import("/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/装配入口.js");
await 等到(()=>源表.length >= 2 && 档.getElementById("角色入口").children.length > 0);

test("装配就绪：模式=实时、两栏入口渲染、首屏取数、双流订阅", ()=>{
  assert.equal(运行时.模式, "实时", "界面默认即实时，无演示分支");
  assert.equal($("#角色入口").querySelectorAll("button").length, 6);
  assert.equal($("#天机筛").querySelectorAll("button").length, 6);
  assert.notEqual($("#对话流").querySelector(".空"), null, "首屏无记录：左栏如实说该角色暂无发言");
  assert.equal($("#连接语").textContent, "实时 · 连接中…");
  assert.deepEqual(源表.map(s=>s.路径), ["/api/dev/stream/agui", "/api/dev/stream/agui/state"]);
  assert.ok(调用.includes("/api/board"), "首屏直连真实契约：看板先取一次权威数据");
  assert.equal($("#看板统").textContent, "共 1 个在办任务 · 1 个流转中");
});

test("帧经总线落右栏，底栏同步当前任务与角色（同棒只写一次）", ()=>{
  源表[0].onmessage({data: JSON.stringify({type:"RUN_STARTED", runId:"r1", 角色:"圣人", input:{任务id:31}})});
  assert.equal($("#事件流").querySelectorAll(".棒").length, 1, "帧经总线转交给观测台面——接线表的第一条");
  assert.equal($("#底栏当前").textContent, "任务 #31 · 圣人任期中");

  源表[0].onmessage({data: JSON.stringify({type:"REASONING_MESSAGE_CONTENT", runId:"r1", 角色:"圣人", delta:"想想"})});
  assert.equal($("#底栏当前").textContent, "任务 #31 · 圣人任期中", "同一棒不重复刷新底栏");

  源表[0].onmessage({data: JSON.stringify({type:"STEP_FINISHED", 角色:"圣人", stepName:"圣人 · 设计", 任务id:31})});
  assert.equal($("#底栏当前").textContent, "任务 #31 · 圣人已交接");

  源表[0].onmessage({data: JSON.stringify({type:"RUN_FINISHED", 任务id:31})});
  assert.equal($("#底栏当前").textContent, "任务 #31 · 本轮驱动收尾");

  源表[1].onmessage({data: JSON.stringify({type:"RUN_ERROR", 任务id:31, message:"熔断"})});
  assert.equal($("#底栏当前").textContent, "任务 #31 · 驱动出错");
});

test("发送「呈上」：来客气泡落左栏，道祖接待经总线流式收尾", async ()=>{
  $("#输入").value = "我要加挂起机制";
  $("#发送").click();
  // 气泡先立、答复随流到达：等到道祖那格真的收到正文再断言，
  // 只等「出现两条消息」会把「气泡是空的」也判成通过。
  await 等到(()=>[...$("#对话流").querySelectorAll(".气泡")].some(x=>x.textContent === "收到。"));

  assert.equal($("#输入").value, "", "发送即清空输入框");
  const 消息s = $("#对话流").querySelectorAll(".消息");
  assert.equal(消息s[0].querySelector(".气泡").textContent, "我要加挂起机制", "来客话先落左栏");
  assert.equal(消息s[1].querySelector(".署").textContent, "道祖");
  assert.equal(消息s[1].querySelector(".气泡").textContent, "收到。", "接待答复经总线增量落进道祖气泡");
  assert.equal($("#对话流").querySelector(".空"), null);
});

test("回车发送；输入法选字的 Enter 与 Shift+Enter 一律放过", async ()=>{
  const 条数 = ()=>$("#对话流").querySelectorAll(".消息").length;
  const 前 = 条数();

  // 输入法选字中的 Enter：是「确认候选词」，不是发送
  const 法 = new 档.defaultView.KeyboardEvent("keydown", {key:"Enter", bubbles:true, cancelable:true});
  Object.defineProperty(法, "isComposing", {value:true});
  $("#输入").value = "选字中";
  $("#输入").dispatchEvent(法);
  assert.equal(条数(), 前, "选字 Enter 不得发送");
  assert.equal($("#输入").value, "选字中");

  // Shift+Enter：换行，不发送
  $("#输入").dispatchEvent(new 档.defaultView.KeyboardEvent("keydown", {key:"Enter", shiftKey:true, bubbles:true, cancelable:true}));
  assert.equal(条数(), 前, "Shift+Enter 是换行，不发送");

  // 裸 Enter：发送
  $("#输入").dispatchEvent(new 档.defaultView.KeyboardEvent("keydown", {key:"Enter", bubbles:true, cancelable:true}));
  await 等到(()=>条数() > 前);
  assert.equal($("#输入").value, "");
});

test("切视图：四轨互斥，进视图即取一次权威数据", async ()=>{
  const 看板前 = 调用.filter(p=>p === "/api/board").length;
  $("#轨看板").click();
  assert.ok($("#视图-看板").classList.contains("显"));
  assert.ok(!$("#视图-观星台").classList.contains("显"), "主视图互斥：一次只显一个");
  assert.ok($("#轨看板").classList.contains("当前"));
  assert.ok(!$("#轨观星台").classList.contains("当前"));
  assert.ok(调用.filter(p=>p === "/api/board").length > 看板前, "进看板即取一次权威数据");

  const 格位前 = 调用.filter(p=>p.startsWith("/api/cognition/cells")).length;
  $("#轨格位").click();
  assert.ok($("#视图-格位").classList.contains("显"));
  assert.ok(调用.filter(p=>p.startsWith("/api/cognition/cells")).length > 格位前);

  $("#轨道规").click();
  assert.ok($("#视图-道规").classList.contains("显"));
  await 等到(()=>$("#道规统").textContent !== "");   // 道规取数是异步的：统计位由数据回来才落字
  assert.equal($("#道规统").textContent, "大道 0 条 · 天道 0 条 · 可编辑");

  $("#轨观星台").click();
  assert.ok($("#视图-观星台").classList.contains("显"));
});

test("承面板：架构/设计接真实清单，其余面板如实说待承纳；再点同钮收起", async ()=>{
  // 不用属性选择器：happy-dom 的选择器不认非 ASCII 属性名（浏览器认），
  // 按类名取到钮、再比 data-面板 值，测的是同一件事。
  const 面钮 = (名)=>[...档.querySelectorAll(".面钮")].find(b=>b.getAttribute("data-面板") === 名);
  const 架构钮 = 面钮("架构");
  架构钮.click();
  assert.ok($("#展开栏").classList.contains("开"));
  assert.equal($("#展标题").textContent, "架构");
  assert.equal(架构钮.getAttribute("aria-expanded"), "true");
  await 等到(()=>$("#展身").querySelector(".文档项"));
  assert.equal($("#展身").querySelector(".文档名").textContent, "总架构.md");

  架构钮.click();   // 同钮再点：收起
  assert.ok(!$("#展开栏").classList.contains("开"));
  assert.equal(架构钮.getAttribute("aria-expanded"), "false");

  面钮("设计").click();
  assert.equal($("#展标题").textContent, "设计");
  await 等到(()=>$("#展身").textContent.includes("未找到任何设计稿"));

  面钮("记忆").click();
  assert.ok($("#展身").textContent.includes("内容待后续承纳"), "未承纳面板如实说待承纳，不假装有内容");
});

test("总线：请求看板定位——切看板、抬权威数据、定位到卡", async ()=>{
  广播(总线.请求看板定位, {任务id:31});
  await 等到(()=>$("#泳道区").querySelector(".卡[data-id='31']")?.classList.contains("闪"));
  assert.ok($("#视图-看板").classList.contains("显"), "点任务号即切到看板");
  assert.ok($("#泳道区").querySelector(".卡[data-id='31']").classList.contains("开"));
});

test("总线：请求重放——订阅府停旧流、复位、按节拍喂契约样例", ()=>{
  广播(总线.请求重放);
  assert.equal($("#连接语").textContent, "演示数据 · 契约对齐");
  assert.equal(运行时.播放计时.length, 事件脚本.length, "重放句柄全部入册：停() 一次清光");
  assert.equal(源表[0].已关, true, "重放即停实时流：两套来源不得同时喂右栏");

  // 收尾清理：不让最长 44 秒的重放计时拖住测试进程
  运行时.播放计时.forEach(clearTimeout);
  运行时.播放计时 = [];
});
