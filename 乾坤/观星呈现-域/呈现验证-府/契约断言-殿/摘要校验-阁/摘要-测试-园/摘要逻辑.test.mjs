/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 摘要校验-阁
   摘要逻辑 用例：它是「把机器产出折成人话」的唯一口径，
   左栏结论与右栏棒头都必须走这里（rules/呈现域规则.md 五·4、五·5）。
   两处各截一刀，早晚互相漂移——本文件把这条硬约束钉在断言里。
   ============================================================ */

import { test } from "node:test";
import assert from "node:assert/strict";
import { 工具引, 聚棒动作, 结论摘要, 职名 } from "/观测台面-府/天机事件-殿/结论析出-阁/摘要-逻辑-园/摘要逻辑.js";
import { 事件脚本 } from "/契约词表-府/常量定义-殿/演示语料-阁/样例-数据-园/演示语料.js";

const 栏 = "`".repeat(3);

test("工具引：路径取末段，Windows / Unix 分隔符同等对待", ()=>{
  assert.equal(工具引('{"路径":"太初/任务管理-域/任务核心-府/状态枚举.rs"}'), "状态枚举.rs");
  assert.equal(工具引('{"路径":"太初\\\\任务管理-域\\\\状态枚举.rs"}'), "状态枚举.rs");
});

test("工具引：命令取前两词（第三个词起是参数噪音），模式原样", ()=>{
  assert.equal(工具引('{"命令":"cargo test -p tc-task -- --nocapture"}'), "cargo test");
  assert.equal(工具引('{"命令":"  cargo   test  "}'), "cargo test");
  assert.equal(工具引('{"模式":"**/*.rs"}'), "**/*.rs");
});

test("工具引：入参分片到达或无可引用字段时返回空串，不得抛错", ()=>{
  assert.equal(工具引('{"路径":"太初/任务管理'), "");
  assert.equal(工具引(""), "");
  assert.equal(工具引(undefined), "");
  assert.equal(工具引('{"内容":"只有内容没有目标"}'), "");
});

test("聚棒动作：按 run 聚合工具动作，动词表 + 目标名（与右栏棒头同口径）", ()=>{
  const 表 = 聚棒动作(事件脚本);
  assert.deepEqual(表.get("run-31-1").动作, ["读 状态枚举.rs", "写 任务挂起恢复-设计.md"]);
  assert.deepEqual(表.get("run-31-3").动作, ["跑 cargo test"]);
});

test("聚棒动作：RUN_STARTED 之前的帧不入任何 run（未开局无棒可归位）", ()=>{
  const 表 = 聚棒动作([
    {type:"TOOL_CALL_START", toolCallId:"t-1", toolCallName:"读文件"},
    {type:"TOOL_CALL_ARGS", toolCallId:"t-1", delta:'{"路径":"a/b.rs"}'},
    {type:"TOOL_CALL_RESULT", toolCallId:"t-1", content:"ok"},
  ]);
  assert.equal(表.size, 0, "无 runId 的孤帧不得凭空造出一根棒");
});

test("聚棒动作：同 run 内重复动作只记一次", ()=>{
  const 表 = 聚棒动作([
    {type:"RUN_STARTED", runId:"r-1", 角色:"圣人"},
    {type:"TOOL_CALL_START", toolCallId:"t-1", toolCallName:"读文件"},
    {type:"TOOL_CALL_ARGS", toolCallId:"t-1", delta:'{"路径":"a/x.rs"}'},
    {type:"TOOL_CALL_RESULT", toolCallId:"t-1", content:"ok"},
    {type:"TOOL_CALL_START", toolCallId:"t-2", toolCallName:"读文件"},
    {type:"TOOL_CALL_ARGS", toolCallId:"t-2", delta:'{"路径":"a/x.rs"}'},
    {type:"TOOL_CALL_RESULT", toolCallId:"t-2", content:"ok"},
  ]);
  assert.deepEqual(表.get("r-1").动作, ["读 x.rs"]);
});

test("结论摘要：先剥 markdown 围栏再解析——围栏是给渲染器的记号，不是内容", ()=>{
  const 原文 = `${栏}json\n{"通过":true}\n${栏}`;
  assert.equal(结论摘要(原文, []), "通过", "不剥围栏则永远解析不出 JSON，整块裸文本铺进结论栏");
});

test("结论摘要：判定词先于动作（结论栏回答的是「这一棒结论是什么」）", ()=>{
  assert.equal(结论摘要('{"通过":false}', ["读 a.rs"]), "不通过 · 读 a.rs");
  assert.equal(结论摘要('{"自检":{"通过":true}}', []), "自检通过");
  assert.equal(结论摘要('{"最终结果":true}', []), "验收通过");
  assert.equal(结论摘要('{"需求满足度":9}', []), "需求满足度 9/10");
});

test("结论摘要：归档完成分「有清理项 / 无残留」两种措辞，不臆造第三种", ()=>{
  assert.equal(结论摘要('{"归档完成":true,"清理项":["a","b","c"]}', []), "归档完成 · 清理 3 项");
  assert.equal(结论摘要('{"归档完成":true}', []), "归档完成 · 无残留");
});

test("结论摘要：截断粒度与右栏一致——最多 3 项 +「…+N」", ()=>{
  // 判定词自身占一个显示位（判定优先于动作，故它必须留在前三项内）
  const 带判定 = 结论摘要('{"通过":true}', ["读 a.rs","写 b.md","改 c.rs","跑 cargo test","删 d.bak"]);
  assert.equal(带判定, "通过 · 读 a.rs · 写 b.md …+3");
  assert.equal(带判定.split(" · ").length, 3, "显示项不得多于 3");
  // 无判定词时，槽位全给动作
  const 纯动作 = 结论摘要("{}", ["读 a.rs","写 b.md","改 c.rs","跑 cargo test","删 d.bak"]);
  assert.equal(纯动作, "读 a.rs · 写 b.md · 改 c.rs …+2");
});

test("结论摘要：人写的纯文本原样保留——摘要不该去毁本来就好读的话", ()=>{
  const 文 = "设计完成。接口：POST /api/board/{id}/suspend。";
  assert.equal(结论摘要(文, []), 文);
});

test("结论摘要：疑似结构化自述却解析失败，给中性说明而非铺一屏裸符号", ()=>{
  assert.equal(结论摘要('{"未闭合":', []), "（模型自述结构不完整 · 原文见下）");
  assert.equal(结论摘要("[1,2,", []), "（模型自述结构不完整 · 原文见下）");
});

test("结论摘要：空答复不抛错，返回空串交由上层兜底", ()=>{
  assert.equal(结论摘要("", []), "");
  assert.equal(结论摘要(null, []), "");
});

test("职名：五角色各有职责，未知角色返回空串（不编造职责）", ()=>{
  assert.equal(职名("道祖"), "需求");
  assert.equal(职名("圣人"), "设计");
  assert.equal(职名("大罗金仙"), "实现");
  assert.equal(职名("准圣"), "验收");
  assert.equal(职名("太乙金仙"), "清理");
  assert.equal(职名("来客"), "");
});
