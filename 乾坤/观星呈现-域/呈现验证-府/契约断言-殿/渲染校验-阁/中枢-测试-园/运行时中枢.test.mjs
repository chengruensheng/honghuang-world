/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   运行时中枢 用例：底座骨架-府 的最小一层——选择器 / 转义 / 状态语 /
   空态 / 帧筛选 / 总线。各府崩溃隔离的前提是它可用（呈现域规则 四·2），
   故这里锁的是「谁都依赖的那几条不变量」。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面, 断言骨架齐备 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $, 转义, 运行时, 过筛, 连接语, 渲染天机空态, 刷天机空态, 总线, 广播, 监听 }
  from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";

let 档;
beforeEach(()=>{
  ({档} = 装台面());
  断言骨架齐备(档);
  运行时.筛选 = "全部";
});

test("选择器命中骨架元素，未命中返回 null", ()=>{
  assert.notEqual($("#事件流"), null);
  assert.equal($("#事件流").tagName, "DIV");
  assert.equal($("#不存在的区"), null);
});

test("转义覆盖全部五个注入字符，空值不落 null", ()=>{
  assert.equal(转义('<b class="x">&\'</b>'), "&lt;b class=&quot;x&quot;&gt;&amp;&#39;&lt;/b&gt;");
  assert.equal(转义(null), "", "null 必须落空串——拼进 innerHTML 不能出现 \"null\"");
  assert.equal(转义(undefined), "");
  assert.equal(转义(0), "0");
  assert.equal(转义("普通文本"), "普通文本");
});

test("帧筛选：全部放行；指名筛选只放该角色，无角色帧按「系统」算", ()=>{
  assert.equal(运行时.筛选, "全部");
  assert.equal(过筛({角色:"圣人"}), true);

  运行时.筛选 = "圣人";
  assert.equal(过筛({角色:"圣人"}), true);
  assert.equal(过筛({角色:"大罗金仙"}), false);
  assert.equal(过筛({}), false, "无角色帧默认归「系统」，筛选不是我时不得放行");
  assert.equal(过筛({角色:"系统"}), false);
});

test("状态语只写文本，不落 HTML", ()=>{
  连接语('实时 · <已连接>');
  assert.equal(档.getElementById("连接语").textContent, "实时 · <已连接>");
});

test("天机空态写明「在等什么」，且不盖掉该区的故障宣告", ()=>{
  渲染天机空态("实时流未连接 · 检查后端服务是否在运行");
  const 区 = 档.getElementById("事件流");
  assert.equal(区.querySelector(".空 .语").textContent, "实时流未连接 · 检查后端服务是否在运行");
  assert.equal(区.querySelector(".空 .符").textContent, "○");

  // 故障说明是确认过的事实，空态不得覆盖
  区.dataset.降级 = "1";
  渲染天机空态("等待事件…");
  assert.equal(区.querySelector(".空 .语").textContent, "实时流未连接 · 检查后端服务是否在运行",
    "该区已被标为故障时，改写成「等待事件…」等于界面替坏掉的区域自述正常");
  delete 区.dataset.降级;

  // 空态文案经转义：等的是「什么」由数据决定，不得成为注入面
  渲染天机空态('<img src=x onerror=1>');
  assert.equal(区.querySelector(".空 .语").textContent, '<img src=x onerror=1>');
  assert.equal(区.querySelector("img"), null);
});

test("刷天机空态只在「一根棒都没有」时改写", ()=>{
  const 区 = 档.getElementById("事件流");
  刷天机空态("该角色暂无事件");
  assert.equal(区.querySelector(".空 .语").textContent, "该角色暂无事件");

  // 已有内容（哪怕只剩一根棒）时不插空态：棒还在，界面就不能说「暂无事件」
  区.innerHTML = '<div class="棒"></div>';
  刷天机空态("该角色暂无事件");
  assert.equal(区.querySelector(".空"), null);
});

test("总线事件名同前缀、两两不重名，广播携带 detail 原样到达", ()=>{
  const 名 = Object.values(总线);
  assert.equal(名.length, 10);
  名.forEach(x=>assert.ok(x.startsWith("洪荒:"), `事件名须带统一前缀：${x}`));
  assert.equal(new Set(名).size, 名.length, "同一总线上不得有两个同名事件——一处广播两处乱接");

  let 收到 = null;
  监听("洪荒:测试往返", (d)=>{ 收到 = d; });
  广播("洪荒:测试往返", {帧:{type:"RUN_STARTED", runId:"r-1"}});
  assert.deepEqual(收到, {帧:{type:"RUN_STARTED", runId:"r-1"}});
});

test("运行时状态对象载荷齐备：棒表/角色棒/待收尾为每屏重建容器", ()=>{
  assert.ok(Array.isArray(运行时.事件缓冲));
  assert.ok(Array.isArray(运行时.接待记录));
  assert.ok(Array.isArray(运行时.任务集));
  assert.ok(运行时.棒表 instanceof Map);
  assert.ok(运行时.角色棒 instanceof Map);
  assert.ok(运行时.待收尾 instanceof Map);
  assert.ok(运行时.展开棒 instanceof Set);
  assert.ok(运行时.展开条 instanceof Set);
  assert.ok(运行时.已连流 instanceof Set);
  assert.equal(运行时.筛选, "全部", "唯一筛选真源初始为「全部」");
});
