/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   棒条渲染 用例：右栏「棒 → 条」的绘制口径。
   锁三件容易退化的硬约束：
   · 收尾幂等只对真终态生效——占位语必须能被真终态覆盖（缺陷 D1 的防线）；
   · 三态（进行中 / 完成 / 中断）不得互相混；
   · 任务锚点「取不到就不编」——对不上看板就只显示任务号，不挂链接。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 运行时, 总线, 监听, 渲染天机空态 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 建棒, 建条, 收尾棒, 记动作, 摘文, 刷棒头, 折棒, 跳段, 清事件 }
  from "/观测台面-府/天机事件-殿/分层绘现-阁/棒条-渲染-园/棒条渲染.js";

let 档;
beforeEach(()=>{
  ({档} = 装台面());
  运行时.棒表.clear(); 运行时.角色棒.clear(); 运行时.待收尾.clear();
  运行时.展开棒.clear(); 运行时.展开条.clear();
  运行时.任务集 = []; 运行时.筛选 = "全部";
});

test("建棒：一行一棒、默认展开、两表登记，空态随之让位", ()=>{
  渲染天机空态("等待事件…");
  assert.notEqual($("#事件流").querySelector(".空"), null);

  const 记 = 建棒("run-31-1", "圣人");
  const 棒 = $("#事件流").querySelector(".棒");
  assert.equal(棒.id, "段-run-31-1");
  assert.equal(棒.querySelector(".角").textContent, "圣人");
  assert.ok(棒.classList.contains("开"), "进行中默认展开——「正在做什么」直接可见");
  assert.equal($("#事件流").querySelector(".空"), null, "棒来了空态必须让位，否则同屏两套状态自相矛盾");
  assert.equal(运行时.棒表.get("run-31-1"), 记);
  assert.deepEqual(运行时.角色棒.get("圣人").map(r=>r.数据.runId), ["run-31-1"]);
  assert.ok(运行时.展开棒.has("run-31-1"));
});

test("建条：三类型归位（推理 / 工具 / 状态），条数计数", ()=>{
  const 记 = 建棒("r", "圣人");
  const 条 = 建条(记, "工具", "工 具 · 读文件", "状态枚举.rs", "读取成功，共 42 行。");
  assert.equal(条.dataset.类, "工具");
  assert.equal(条.querySelector("b").textContent, "工 具 · 读文件");
  assert.equal(条.querySelector(".引").textContent, "状态枚举.rs");
  assert.equal(条.querySelector(".条文").textContent, "读取成功，共 42 行。");
  assert.equal(建条(记, "状态", "状 态", "", "x").dataset.类, "状态");
  assert.equal(建条(记, "推理", "推 理", "", "y").dataset.类, "推理");
  assert.equal(记.数据.条数, 3);
});

test("收尾棒：真终态幂等，占位语可被真终态覆盖——但反向不行", ()=>{
  const 记 = 建棒("r", "圣人");
  const 态 = ()=>记.dom.querySelector(".态").textContent;

  收尾棒(记, "已交接下一棒", false, true);   // 占位：由「下一棒已开头」反推
  assert.equal(记.数据.完成, true);
  assert.equal(记.数据.占位, true);
  assert.equal(态(), "完成");
  assert.ok(!记.dom.classList.contains("开"), "任期结束就该折起");
  assert.ok(!运行时.展开棒.has("r"));

  收尾棒(记, "运行结束", false);            // 真终态后到
  assert.equal(记.数据.终态, "运行结束", "占位语不是结局，真终态必须能覆盖它（缺陷 D1）");
  assert.equal(记.数据.占位, false);

  收尾棒(记, "运行错误", true);             // 已是真终态
  assert.equal(记.数据.终态, "运行结束", "真终态只认第一次，后续收尾不得改写");
  assert.equal(记.数据.失败, false);

  收尾棒(null, "运行结束", false);          // 无棒不失守
});

test("刷棒头：三态分明、条数如实、「→ 下一步」只在收尾后露面", ()=>{
  const 记 = 建棒("r", "圣人");
  const 态 = ()=>记.dom.querySelector(".态").textContent;
  刷棒头(记);
  assert.equal(态(), "进行中");
  assert.equal(记.dom.querySelector(".棒计").textContent, "0 条");

  建条(记, "状态", "状 态", "", "x");
  刷棒头(记);
  assert.equal(记.dom.querySelector(".棒计").textContent, "1 条");

  // 交接是「这一棒交出的结果」：未收尾时不得提前露面
  记.数据.交接 = "待大罗金仙实现";
  刷棒头(记);
  assert.equal(记.交.textContent, "", "任期内的状态由棒身「状态」条表达，棒头不预告交接");

  收尾棒(记, "已提交", false);
  assert.equal(记.交.textContent, "→ 待大罗金仙实现");
  assert.equal(态(), "完成");

  // 中断是另一根棒的结局：错误终止不得混进「完成」，否则界面上熔断被读成顺利结束
  const 断记 = 建棒("r-断", "圣人");
  收尾棒(断记, "运行错误 · 熔断 [E511]", true);
  assert.equal(断记.dom.querySelector(".态").textContent, "中断");
  assert.equal(断记.dom.querySelector(".棒头").title, "运行错误 · 熔断 [E511]");
});

test("记动作：同一动作只记一次；去重不看位置", ()=>{
  const 记 = 建棒("r", "圣人");
  记动作(记, "读 状态枚举.rs");
  记动作(记, "读 状态枚举.rs");
  记动作(记, "写 a.md");
  记动作(记, "读 状态枚举.rs");
  记动作(记, "");
  assert.deepEqual(记.数据.动作, ["读 状态枚举.rs", "写 a.md"]);
});

test("摘文：动作优先（最多 3 项 + …+N），无动作时把首句折成人话", ()=>{
  assert.equal(摘文({动作:["读 a.rs","写 b.md","跑 cargo test"], 首句:""}), "读 a.rs · 写 b.md · 跑 cargo test");
  assert.equal(摘文({动作:["读 a.rs","写 b.md","跑 cargo test","删 d.bak"], 首句:""}), "读 a.rs · 写 b.md · 跑 cargo test …+1");
  assert.equal(摘文({动作:[], 首句:"设计完成。接口：POST /api/board/{id}/suspend。"}), "设计完成。接口：POST /api/board/{id}/suspend。",
    "人话原文不截——摘要只针对模型产出的结构化自述");
  assert.equal(摘文({动作:[], 首句:""}), "");
});

test("折棒：开合往复与「展开集」同步，未知棒不炸", ()=>{
  const 记 = 建棒("r", "圣人");
  折棒("r");
  assert.ok(!记.dom.classList.contains("开"));
  assert.ok(!运行时.展开棒.has("r"));
  折棒("r");
  assert.ok(记.dom.classList.contains("开"));
  assert.ok(运行时.展开棒.has("r"));
  折棒("不存在的棒");
});

test("清事件：右栏与全部棒记录一次清空", ()=>{
  建棒("r1", "圣人");
  建棒("r2", "准圣");
  运行时.展开条.add("t-1");
  清事件();
  assert.equal($("#事件流").innerHTML, "");
  assert.equal(运行时.棒表.size, 0);
  assert.equal(运行时.角色棒.size, 0);
  assert.equal(运行时.待收尾.size, 0);
  assert.equal(运行时.展开棒.size, 0);
  assert.equal(运行时.展开条.size, 0);
});

test("待收尾暂存于建棒时补收：阶段帧先到时终态语不跨过建棒这一刻丢失", ()=>{
  运行时.待收尾.set("圣人", {语:"已提交", 败:false});
  const 记 = 建棒("r", "圣人");
  assert.equal(记.数据.完成, true);
  assert.equal(记.数据.终态, "已提交");
  assert.equal(运行时.待收尾.has("圣人"), false);
});

test("任务锚点：看板里有这张卡才挂链接，点了广播定位", ()=>{
  运行时.任务集 = [{id:41, title:"任务看板新增「挂起 / 恢复」状态"}];
  const 记 = 建棒("run-41-1", "圣人", 41);
  assert.equal(记.行名.textContent, "任务 #41 · 任务看板新增「挂起 / 恢复」状态");
  assert.ok(记.行名.classList.contains("链"));

  let 收到 = null;
  监听(总线.请求看板定位, d=>{收到 = d;});
  记.行名.click();
  assert.deepEqual(收到, {任务id:41});
});

test("看板里查无此任务：只给任务号、不挂链接；单取成功后才补标题与锚点", async ()=>{
  const 调用 = [];
  globalThis.fetch = async (路径)=>{ 调用.push(路径); return {ok:true, json:async()=>({title:"远端标题"})}; };

  运行时.任务集 = [];
  const 记 = 建棒("run-42-1", "准圣", 42);
  assert.equal(记.行名.textContent, "任务 #42", "取不到标题时不得编，只给任务号");
  assert.ok(!记.行名.classList.contains("链"), "对不上看板就不挂链接");

  await new Promise(r=>setTimeout(r,0));   // 等 fetch → json 两跳微任务
  assert.deepEqual(调用, ["/api/board/42"]);
  assert.equal(记.行名.textContent, "任务 #42 · 远端标题");
  assert.ok(记.行名.classList.contains("链"), "标题取到即对上了看板，锚点此时才成立");
});

test("跳段：定位已存在的棒并打闪烁标；未知棒静默", ()=>{
  const 记 = 建棒("r", "圣人");
  折棒("r");           // 先折起，跳段须自动展开
  跳段("r");
  assert.ok(记.dom.classList.contains("开"));
  assert.ok(记.dom.classList.contains("闪"));
  跳段("不存在的棒");   // 取不到就不编，也不假装找到了
});
