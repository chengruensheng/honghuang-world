/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 词表校验-阁
   契约词表 用例：锁定呈现层「契约单一真相源」的每一项，
   与后端权威源一一对照——词表漂一次，整个界面的语义跟着漂。

   权威源：
   · 事件词表 ← 鸿蒙/基础契约-域/交互协议-府/模块.rs 的 Event 枚举（17 个 variant，
     声明序即此处顺序；serde rename_all = SCREAMING_SNAKE_CASE 自动映射事件词）
   · 层级角色/阶段 ← 太初/…/标识-数据-园/层级记录.rs 的 impl From<AgentRole> for 五行层级
     与 五行层级::阶段名
   · 动词表 ← 鸿蒙/…/循环-模块-园/工具定义.rs 的对外工具名（8 个）
   ============================================================ */

import { test } from "node:test";
import assert from "node:assert/strict";
import { 事件词表, 层级角色, 层级阶段, 角色, 角色色, 动词表 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";

test("事件词表逐项锁定 17 个 AG-UI 事件词，顺序 = 后端 Event 枚举声明序", ()=>{
  assert.deepEqual(事件词表, [
    "RUN_STARTED","RUN_FINISHED","RUN_ERROR","STEP_STARTED","STEP_FINISHED",
    "TEXT_MESSAGE_START","TEXT_MESSAGE_CONTENT","TEXT_MESSAGE_END",
    "TOOL_CALL_START","TOOL_CALL_ARGS","TOOL_CALL_END","TOOL_CALL_RESULT",
    "REASONING_MESSAGE_START","REASONING_MESSAGE_CONTENT","REASONING_MESSAGE_END",
    "STATE_SNAPSHOT","STATE_DELTA",
  ], "事件词表必须与后端 Event 枚举逐项同位——多一个会放行词表外事件，少一个会静默丢帧");
  assert.equal(new Set(事件词表).size, 事件词表.length, "事件词不得重复");
});

test("五层角色与阶段映射与后端 层级记录.rs 同源", ()=>{
  assert.deepEqual(层级角色, {木:"道祖", 火:"圣人", 土:"大罗金仙", 金:"准圣", 水:"太乙金仙"},
    "道祖=木 / 圣人=火 / 大罗金仙=土 / 准圣=金 / 太乙金仙=水");
  assert.deepEqual(层级阶段, {木:"需求", 火:"设计", 土:"实现", 金:"验收", 水:"清理"},
    "阶段名必须与后端 五行层级::阶段名 逐字一致（看板泳道按它与层级历史配对）");
});

test("角色表按五行生序给出五角色，色位取层级变量", ()=>{
  assert.equal(角色.length, 5);
  assert.deepEqual(角色.map(x=>x.名), ["道祖","圣人","大罗金仙","准圣","太乙金仙"]);
  assert.deepEqual(角色.map(x=>x.色), ["var(--木)","var(--火)","var(--土)","var(--金)","var(--水)"]);
  assert.deepEqual(角色.map(x=>x.责), ["需求","设计","实现","验收","清理"]);
});

test("角色表三列与两张映射表互洽，不得各说各话", ()=>{
  const 序 = ["木","火","土","金","水"];
  角色.forEach((角, i)=>{
    assert.equal(角.名, 层级角色[序[i]], `第 ${i+1} 位的角色名须与层级角色映射一致`);
    assert.equal(角.责, 层级阶段[序[i]], `第 ${i+1} 位的职责须与层级阶段映射一致`);
  });
});

test("角色色覆盖来客/系统与五角色，键序即展示序", ()=>{
  assert.deepEqual(Object.keys(角色色), ["来客","系统","道祖","圣人","大罗金仙","准圣","太乙金仙"]);
  assert.equal(角色色["来客"], "var(--字二)");
  assert.equal(角色色["系统"], "var(--字三)");
});

test("动词表覆盖后端全部 8 个对外工具名，动词互不重复", ()=>{
  assert.deepEqual(Object.keys(动词表).sort(), ["写文件","列目录","删除文件","按名找文件","搜索内容","精确编辑","运行命令","读文件"].sort(),
    "工具名须与后端 工具定义.rs 的对外工具名逐字一致，否则摘要会退化成整段 JSON");
  assert.equal(new Set(Object.values(动词表)).size, Object.keys(动词表).length,
    "同一动词不得复用于两个工具——摘要要能一眼区分钟做了什么");
});
