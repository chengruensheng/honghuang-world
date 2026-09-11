/* ============================================================
   观星呈现-域 · 观测台面-府 · 天机事件-殿 · 帧编排
   把 AG-UI 协议帧编排进「棒 → 条」的层级：收帧 / 落帧 / 分段 / 收尾 / 重绘。
   本模块管「帧怎么归位」，画法交给 分层绘现-阁。
   ============================================================ */

import { $, 运行时, 过筛, 渲染天机空态, 总线, 广播 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 建棒, 建条, 收尾棒, 记动作, 刷棒头, 跟随, 清事件 } from "/观测台面-府/天机事件-殿/分层绘现-阁/棒条-渲染-园/棒条渲染.js";
import { 标工具引, 落结果 } from "/观测台面-府/天机事件-殿/分层绘现-阁/变更-渲染-园/变更渲染.js";
import { 追加结论, 渲染对话 } from "/观测台面-府/万仙对话-殿/消息渲染-阁/对话-渲染-园/对话渲染.js";
import { 结论摘要, 聚棒动作 } from "/观测台面-府/天机事件-殿/结论析出-阁/摘要-逻辑-园/摘要逻辑.js";
import { 动词表 } from "/契约词表-府/常量定义-殿/事件词表-阁/词表-数据-园/契约词表.js";

/** 收帧：全部入缓冲，命中筛选才落屏；结论宣告同步进左栏（与右栏同源） */
export function 收帧(ev){
  运行时.事件缓冲.push(ev);
  if(!过筛(ev)) return;
  落帧(ev);
  if(ev.type === "TEXT_MESSAGE_CONTENT"){
    const 流 = $("#对话流");
    const 动作 = (聚棒动作(运行时.事件缓冲).get(运行时.当前run) || {}).动作 || [];
    追加结论(流, ev.角色 || "系统", ev.delta, 运行时.当前run, 结论摘要(ev.delta, 动作));
    流.parentElement.scrollTop = 流.parentElement.scrollHeight;
  }
}

/** 收尾帧 → 终态语。落帧各分支与「暂存后补收」共用同一口径，避免两处措辞漂移。 */
function 收尾语(ev){
  if(ev.type === "STEP_FINISHED") return {语: ev.stepName ? `已提交 · ${ev.stepName}` : "已提交", 败: false};
  if(ev.type === "RUN_FINISHED")  return {语: ev.result ? `${ev.result.新状态} · 产出 ${ev.result.产出}` : "运行结束", 败: false};
  const 因 = `${ev.message || "未知错误"}${ev.code ? ` [${ev.code}]` : ""}`;
  return {语: `运行错误 · ${因}`, 败: true};
}

/** 该角色第一根尚未收尾的棒。
    同一角色可有多根（回退重跑时大罗金仙会再来一轮）；阶段帧在其角色内按序号到达，
    与棒的建立顺序一一对应——取最近一根会把早先几轮的终态全挤到最后一根上。 */
function 取待收尾棒(角){
  const 列 = 运行时.角色棒.get(角) || [];
  return 列.find(棒=>!棒.数据.完成) || null;
}

/** 收尾帧 = 阶段流来的「这段任期结束了」：STEP_FINISHED / RUN_FINISHED / RUN_ERROR。
    它携带的角色是「已经结束的那一段任期」，不是「当前活动的角色」——
    拿它去和 当前角色 比较、进而切段，会为已结束的角色凭空再造一根棒。 */
const 是收尾帧 = (ev)=> ev.type === "STEP_FINISHED" || ev.type === "RUN_FINISHED" || ev.type === "RUN_ERROR";

/** 落帧：把一帧并入它所属的棒，并压成一条 */
export function 落帧(ev){
  const 角 = ev.角色 || "";
  const 收尾 = 是收尾帧(ev);
  // 开新棒的两种判据（任一成立即开，故同一角色的连续帧不会重复建棒）：
  //   ① 服务端发了 RUN_STARTED —— 新契约的显式开局；
  //   ② 角色字段变了 —— 兼容未发 RUN_STARTED 的服务（角色一换，就是另一段任期）。
  // 收尾帧不参与②：它的角色指向已结束的棒，不是当前活动角色。
  if(ev.type === "RUN_STARTED"){
    收尾棒(运行时.棒表.get(运行时.当前run), "已交接下一棒");   // 下一棒已开头，即上一棒已收尾（由更晚的事实推出）
    运行时.当前run = ev.runId;
    运行时.当前角色 = 角;
    建棒(ev.runId, 角 || "系统");
  }else if(!收尾 && 角 && 角 !== 运行时.当前角色){
    收尾棒(运行时.棒表.get(运行时.当前run), "已交接下一棒");
    运行时.当前角色 = 角;
    运行时.当前run = `段-${++运行时.段序号}-${角}`;
    建棒(运行时.当前run, 角);
  }
  // 收尾帧按角色归位到「该角色第一根未收尾的棒」；其余帧归「当前棒」。
  // 无角色时（演示流 / 旧服务）退回当前棒，行为与修复前一致。
  const 棒 = 收尾 && 角 ? 取待收尾棒(角) : 运行时.棒表.get(运行时.当前run);
  if(!棒){
    // 该角色的棒尚未建出来（回放时阶段流可能先于过程流到达）：暂存终态语，等建棒时补收。
    if(收尾 && 角) 运行时.待收尾.set(角, 收尾语(ev));
    return;
  }
  棒.数据.帧.push(ev);

  switch(ev.type){
    case "RUN_STARTED":
      break;   // 建棒已在上方完成，此处仅标记为已处理
    case "STEP_STARTED":
      棒.数据.标题 = ev.stepName || "";
      棒.行名.textContent = 棒.数据.标题;
      显阶段(ev.stepName);
      break;
    case "REASONING_MESSAGE_CONTENT":
      if(!棒.数据.首句) 棒.数据.首句 = ev.delta;   // 无工具动作时的摘要兜底
      // 连续推理合成一条，避免一棒几十个碎块
      if(棒.当前条 && 棒.当前条.dataset.类 === "推理"){
        棒.当前条.querySelector(".条文").textContent += ev.delta;
      }else{
        棒.当前条 = 建条(棒, "推理", "推 理", "", ev.delta);
      }
      break;
    case "TOOL_CALL_START":{
      const 条 = 建条(棒, "工具", "工 具 · " + ev.toolCallName, "", "");
      条.dataset.工具 = ev.toolCallName;
      条.querySelector(".条头").addEventListener("click", ()=>{
        const 开 = 条.classList.toggle("开");
        if(开) 运行时.展开条.add(ev.toolCallId); else 运行时.展开条.delete(ev.toolCallId);
      });
      棒.工具表.set(ev.toolCallId, 条);
      棒.当前条 = 条;
      break;
    }
    case "TOOL_CALL_ARGS":{
      const 条 = 棒.工具表.get(ev.toolCallId);
      if(条){
        let 参 = 条.querySelector(".参数");
        if(!参){ 参 = document.createElement("div"); 参.className = "参数"; 条.querySelector(".条文").appendChild(参); }
        参.textContent += ev.delta;
        const 引 = 标工具引(条, 参.textContent);
        if(引) 记动作(棒, (动词表[条.dataset.工具] || 条.dataset.工具 || "用") + " " + 引);
      }
      break;
    }
    case "TOOL_CALL_RESULT":{
      const 条 = 棒.工具表.get(ev.toolCallId) || 棒.当前条;
      if(条) 落结果(条, ev.content || "");
      break;
    }
    case "STATE_DELTA":
      建条(棒, "状态", "状 态", (ev.delta||[]).map(d=>`${d.path} → ${d.value}`).join("；"), "");
      棒.当前条 = null;
      break;
    case "STATE_SNAPSHOT":
      建条(棒, "状态", "状 态 快 照", "", "（整体快照）");
      棒.当前条 = null;
      break;
    case "RUN_ERROR":{
      const 终 = 收尾语(ev);
      建条(棒, "工具 败", "运 行 错 误", "", `${ev.message || "未知错误"}${ev.code ? ` [${ev.code}]` : ""}`);
      棒.当前条 = null;
      // 驱动已终止：这根本棒不能再停在「进行中」——那等于界面替一次失败自述成功
      收尾棒(棒, 终.语, 终.败);
      if(运行时.模式 === "实时") 广播(总线.请求拉看板);
      break;
    }
    case "RUN_FINISHED":{
      const 终 = 收尾语(ev);
      收尾棒(棒, 终.语, 终.败);
      if(运行时.模式 === "实时") 广播(总线.请求拉看板);
      break;
    }
    /* 边界标记与结论宣告不成条：起止由棒头表达，结论属于左栏 */
    case "TEXT_MESSAGE_CONTENT":
      if(!棒.数据.首句) 棒.数据.首句 = ev.delta;   // 道祖一类“只说不做”的棒，用结论首句作摘要
      break;
    /* 阶段完成 = 该角色已把任务交给下一个状态，本轮任期到此为止。
       后端可能不发 RUN_FINISHED（实测「空闲」阶段事件缺席），这条是最强的收尾信号，必须用上，
       否则最后一根棒会永远停在「进行中」——界面看起来像卡死，而任务其实早已推进。 */
    case "STEP_FINISHED":{
      const 终 = 收尾语(ev);
      if(ev.stepName) 显阶段(ev.stepName);
      收尾棒(棒, 终.语, 终.败);
      if(运行时.模式 === "实时") 广播(总线.请求拉看板);   // 状态已推进，看板立即跟上真实数据
      break;
    }
    case "TEXT_MESSAGE_START": case "TEXT_MESSAGE_END":
    case "REASONING_MESSAGE_START": case "REASONING_MESSAGE_END": case "TOOL_CALL_END":
      break;
    default:
      console.warn("词表外事件，已忽略：", ev);
  }
  刷棒头(棒);
  跟随();
}

/** 阶段条：显示当前推进到哪一层 */
export function 显阶段(名){
  const 条 = $("#阶段条");
  条.style.display = "flex";
  条.innerHTML = `<span class="进">◆</span> 当前阶段：<b style="color:var(--字)"></b>`;
  条.querySelector("b").textContent = 名;
}

/** 切换筛选后按缓冲整体重建（重放落帧，保证 START → ARGS 的配对顺序） */
export function 重绘天机(){
  清事件();
  运行时.当前run = null; 运行时.当前角色 = ""; 运行时.段序号 = 0;   // 重放要从头切段，游标必须归零
  运行时.事件缓冲.filter(过筛).forEach(落帧);
  if($("#事件流").children.length === 0) 渲染天机空态("该角色暂无事件");
}

/** 复位呈现：切模式 / 重放前把左右两栏一起退回初始态。
    两处调用共用这一份口径，免得「清了右栏忘复位阶段条」之类的漂移。 */
export function 复位呈现(空态语){
  清事件();
  运行时.事件缓冲 = [];
  运行时.当前run = null; 运行时.当前角色 = ""; 运行时.段序号 = 0;
  渲染天机空态(空态语 || "");
  $("#阶段条").style.display = "none";   // 复位即隐藏：上一模式的阶段残留不得冒充当前状态
  渲染对话();                            // 左栏同退：清掉上一轮的结论消息
}

/** 演示重放：按 300ms 节拍把契约样例帧逐条喂进来。
    计时句柄入 运行时.播放计时，所以 停() 能一次清光——重放与实时共用同一套刹车。 */
export function 播放脚本(脚本){
  脚本.forEach((ev, i)=>运行时.播放计时.push(setTimeout(()=>收帧(ev), 300*i)));
}
