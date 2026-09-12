/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   对话渲染 用例：左栏「万仙对话」——接待语料铺陈、五层接力结论、接待往返流式追加。
   锁的硬约束（呈现域规则 五·5 同源同口径 / 五·6 轮次可辨）：
   · 轮次标只在换棒时新增——同一根棒的两条结论不能各标一个轮次；
   · 摘要与原文一致时不再叠折叠层；不一致时原文必须留（证据不因摘要好看而丢）；
   · 空态是「一条消息都没有」的表达，消息来了必须让位。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 起消息, 追加消息, 追加结论, 渲染对话, 接待开启, 接待思考, 接待增量, 接待收尾 }
  from "/观测台面-府/万仙对话-殿/消息渲染-阁/对话-渲染-园/对话渲染.js";
import { 对话脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";

let 档, 流;
beforeEach(()=>{
  ({档} = 装台面());
  流 = 档.getElementById("对话流");
  运行时.模式 = "实时"; 运行时.筛选 = "全部";
  运行时.接待记录 = []; 运行时.事件缓冲 = []; 运行时.待确认 = false;
});

test("起消息：署名为角色、时标可空、气泡返回到手", ()=>{
  const {元, 泡} = 起消息(流, "道祖", "主控");
  assert.equal(元.querySelector(".署").textContent, "道祖");
  assert.equal(元.querySelector(".时").textContent, "主控");
  assert.equal(泡, 元.querySelector(".气泡"));
  assert.equal(流.querySelectorAll(".消息").length, 1);
});

test("追加消息：空态让位；正文与需求摘要卡一并落屏", ()=>{
  流.innerHTML = '<div class="空"><div class="符">○</div><div class="语">该角色暂无发言</div></div>';
  追加消息(流, "来客", "14:18", "我要给任务看板加一个「挂起」机制。");
  assert.equal(流.querySelector(".空"), null, "消息来了空态必须让位，否则一边写「暂无发言」一边列着发言");

  追加消息(流, "道祖", "14:21", "已对齐，需求摘要如下。", {标题:"任务看板新增「挂起 / 恢复」状态", 描述:"描述", 场景:"功能开发", 优先级:"中"});
  const 卡 = 流.querySelector(".摘要卡");
  assert.notEqual(卡, null);
  assert.equal(卡.querySelector(".卡头").textContent, "需求摘要 · 待确认");
  assert.equal(卡.querySelector("#发布钮").textContent, "确 认 发 布 · 落 看 板");
  assert.equal(卡.querySelectorAll(".字段").length, 4);
});

test("追加消息：带 runId 时挂证据锚点，点了回右栏定位那一棒", ()=>{
  追加消息(流, "圣人", "", "设计完成。", null, "run-31-1");
  const 锚 = 流.querySelector(".锚");
  assert.equal(锚.textContent, "⟵ 过程");
  锚.click();   // 右栏无此棒：取不到就不编，也不假装找到了——不抛即达标
});

test("追加结论：署名为「角色 · 职责」，轮次标只在换棒时新增", ()=>{
  追加结论(流, "圣人", "原文A", "run-1", "摘要A");
  追加结论(流, "圣人", "原文B", "run-1", "摘要B");
  assert.equal(流.querySelectorAll(".轮标").length, 1, "同一根棒的两条结论属于同一轮，不得各标一个轮次");
  assert.equal(流.querySelector(".轮标").textContent, "第 1 棒 · 圣人 · 设计");
  assert.equal(流.querySelector(".名 .署").textContent, "圣人 · 设计", "署名带职责，读者不看右栏也知道这棒归谁");

  追加结论(流, "大罗金仙", "原文C", "run-2", "摘要C");
  assert.equal(流.querySelectorAll(".轮标").length, 2);
  assert.equal(流.querySelectorAll(".轮标")[1].textContent, "第 2 棒 · 大罗金仙 · 实现");
});

test("追加结论：摘要≠原文时原文折叠保留；一致时不叠噪音层", ()=>{
  追加结论(流, "圣人", "设计完成。接口：POST /api/board/{id}/suspend。", "run-1", "设计完成。接口：POST /api/board/{id}/suspend。");
  assert.equal(流.querySelectorAll("details.原文").length, 0, "同一句话摆两遍是纯噪音");

  追加结论(流, "大罗金仙", '{"通过":true,"动作":["读 a.rs"]}', "run-2", "通过 · 读 a.rs");
  const 详 = 流.querySelector("details.原文");
  assert.notEqual(详, null, "摘要只负责好读，证据不能因为摘要好看就丢掉");
  assert.equal(详.querySelector(".原body").textContent, '{"通过":true,"动作":["读 a.rs"]}');
  assert.equal(流.querySelectorAll(".消息")[1].querySelector(".结句").textContent, "通过 · 读 a.rs");
});

test("渲染对话（演示模式）：契约样例五条接待语料 + 摘要卡发布入口", ()=>{
  运行时.模式 = "演示";
  渲染对话();
  assert.equal(流.querySelectorAll(".消息").length, 对话脚本.length);
  assert.notEqual(流.querySelector("#发布钮"), null, "道祖末条带需求摘要，发布入口随之落屏");
  assert.equal(流.querySelector(".空"), null);
});

test("渲染对话（指名筛选）：来客始终在场——它是对话的另一方", ()=>{
  运行时.模式 = "演示";
  运行时.筛选 = "道祖";
  渲染对话();
  assert.equal(流.querySelectorAll(".消息").length, 4, "道祖 2 条 + 来客 2 条；系统那条不属于本视角");
  assert.deepEqual([...流.querySelectorAll(".署")].map(x=>x.textContent), ["来客","道祖","来客","道祖"]);
});

test("渲染对话（实时模式、无记录、无缓冲）：如实说「该角色暂无发言」", ()=>{
  渲染对话();
  assert.equal(流.querySelector(".空 .语").textContent, "该角色暂无发言");
});

test("渲染对话：该栏已被顶层宣告故障时不重绘——故障说明不得被盖成空态", ()=>{
  流.dataset.降级 = "1";
  流.innerHTML = "对话呈现未就绪（该府装载失败）";
  渲染对话();
  assert.equal(流.innerHTML, "对话呈现未就绪（该府装载失败）");
});

test("接待往返：开启 → 思考（独立通道）→ 增量 → 收尾落确认钮", ()=>{
  渲染对话();
  接待开启({id:"接待-1", 角色:"道祖", 时:"主控"});
  assert.equal(流.querySelectorAll(".消息").length, 1);
  assert.equal(流.querySelector(".空"), null);

  接待思考({id:"接待-1", 文本:"先钉死三条边界。"});
  assert.equal(流.querySelector(".推理条 .推body").textContent, "先钉死三条边界。");
  assert.equal(流.querySelector(".气泡").textContent, "", "思考不进气泡正文——草稿不冒充答复");

  接待增量({id:"接待-1", 文本:"明白。为准确对齐，我需要确认三点："});
  assert.equal(流.querySelector(".气泡").textContent, "明白。为准确对齐，我需要确认三点：");

  接待收尾({id:"接待-1", 文本:"明白。为准确对齐，我需要确认三点：\n① …\n② …\n③ …", 待确认:true});
  assert.equal(运行时.待确认, true);
  assert.notEqual(流.querySelector("#发布钮"), null);

  接待增量({id:"查无此泡", 文本:"x"});   // 气泡被整栏重绘顶掉的情形：不炸
  接待思考({id:"查无此泡", 文本:"x"});
});
