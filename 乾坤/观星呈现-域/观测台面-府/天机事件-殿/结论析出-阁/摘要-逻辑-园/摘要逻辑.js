/* ============================================================
   观星呈现-域 · 观测台面-府 · 天机事件-殿 · 摘要逻辑
   「把机器产出折成人话」的唯一口径：棒头摘要与左栏结论都必须走这里。
   两处各算一套，早晚互相漂移。
   ============================================================ */

import { 角色, 动词表 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";

/** 从工具入参里取目标名：路径取末段、命令取前两词、模式原样。
    「按名找文件」的入参只有 模式（没有 路径 / 命令），漏掉它，这一棒的摘要就会退化成整段 JSON。 */
export function 工具引(原文){
  try{
    const 对 = JSON.parse(原文);
    if(对.路径) return String(对.路径).split(/[\/\\]/).pop();
    if(对.命令) return String(对.命令).trim().split(/\s+/).slice(0,2).join(" ");
    if(对.模式) return String(对.模式);
  }catch(e){ /* 入参分片到达，未成形时不管 */ }
  return "";
}

/** 按 run 聚合工具动作，与右栏棒头同一口径（动词表 + 目标名）。
    左栏结论也要说人话，且必须跟右栏说同一个版本。 */
export function 聚棒动作(帧列){
  const 表 = new Map();   // runId → {动作:[], 参:Map(toolCallId → {工具, 文})}
  let 游 = null;
  帧列.forEach(ev=>{
    if(ev.type === "RUN_STARTED") 游 = ev.runId;
    if(!游) return;
    let 集 = 表.get(游);
    if(!集){ 集 = {动作:[], 参:new Map()}; 表.set(游, 集); }
    if(ev.type === "TOOL_CALL_START") 集.参.set(ev.toolCallId, {工具:ev.toolCallName, 文:""});
    else if(ev.type === "TOOL_CALL_ARGS"){
      const 记 = 集.参.get(ev.toolCallId);
      if(记) 记.文 += ev.delta;
    }else if(ev.type === "TOOL_CALL_RESULT"){
      const 记 = 集.参.get(ev.toolCallId);
      if(!记) return;
      const 引 = 工具引(记.文);
      const 文 = (动词表[记.工具] || 记.工具 || "用") + (引 ? " " + 引 : "");
      if(!集.动作.includes(文)) 集.动作.push(文);
    }
  });
  return 表;
}

/** 结论摘要：把一层的答复（模型产出的 JSON）折成人话。
    先「做了什么」（与右栏同源），再补判定词；都没有才退回原文首句。
    判定词只认各层契约里固定的布尔字段——不做语义猜测，猜错就等于界面替模型发言。 */
/** 剥掉 markdown 代码围栏：模型常把自述包在 ```json … ``` 里。
    围栏是给渲染器看的记号，不是内容本体——不剥，JSON 永远解析不出来，
    一整块带围栏的裸文本就铺进了结论栏。 */
function 去皮(文){
  const 围 = 文.match(/^\s*```[A-Za-z]*[ \t]*\r?\n([\s\S]*?)\r?\n?[ \t]*```\s*$/);
  return 围 ? 围[1].trim() : 文.trim();
}

export function 结论摘要(原文, 动作){
  const 文 = String(原文 ?? "");
  const 净 = 去皮(文);
  let 对 = null;
  try{ 对 = JSON.parse(净); }catch(e){ /* 不是 JSON */ }
  if(!对 || typeof 对 !== "object"){
    // 人写的纯文本（演示语料、道祖的对话）原样保留——摘要只针对模型产出的结构化自述；
    // 把本来就好读的话再截一刀，是拿摘要去毁内容。
    // 但「疑似结构化自述」（以 { 或 [ 开头）却解析失败的，不是人话：多半是模型 JSON 被截断
    // 或掺了前后缀。原样铺出来是一屏裸符号，读者一个字的收获也没有。给一句中性说明，原文交折叠区。
    if(/^\s*[{\[]/.test(净)) return "（模型自述结构不完整 · 原文见下）";
    return 文;
  }
  const 判定 = [];
  if(!Array.isArray(对)){
    if(对.通过 === true) 判定.push("通过");
    else if(对.通过 === false) 判定.push("不通过");
    if(对.自检 && 对.自检.通过 === true) 判定.push("自检通过");
    if(对.最终结果 === true) 判定.push("验收通过");
    if(对.归档完成 === true) 判定.push((对.清理项 || []).length ? `归档完成 · 清理 ${对.清理项.length} 项` : "归档完成 · 无残留");
    if(typeof 对.需求满足度 === "number") 判定.push(`需求满足度 ${对.需求满足度}/10`);
  }
  // 判定优先于动作：左栏回答的是「这一棒结论是什么」，不是「这一棒做了什么」——
  // 把二十几项动作铺在最前、判定被挤出屏外，等于结论宣告从不宣告结论。
  const 段 = 判定.concat(动作 || []);
  if(段.length){
    // 截断粒度与右栏棒头一致（最多 3 项 +「…+N」）：同一件事在两栏各截一刀，早晚对不上。
    const 显 = 段.slice(0, 3).join(" · ");
    return 段.length > 3 ? `${显} …+${段.length - 3}` : 显;
  }
  const 首 = 净.replace(/\s+/g, " ").trim();   // 也用去围栏后的文本：首句不该是「```json」
  return 首.length > 42 ? 首.slice(0, 42) + "…" : 首;
}

/** 角色 → 职责（需求 / 设计 / 实现 / 验收 / 清理），署名用 */
export function 职名(名){
  const 项 = 角色.find(x=>x.名 === 名);
  return 项 ? 项.责 : "";
}
