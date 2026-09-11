/* ============================================================
   观星呈现-域 · 流式驱动-府 · 双流接入-殿 · 接待逻辑
   道祖接待：POST /api/dev/chat/stream。EventSource 只支持 GET，
   POST+SSE 只能用 fetch 手动读帧。
   帧序列（后端契约）：RUN_STARTED → TEXT_MESSAGE_CONTENT×N → TEXT_MESSAGE_END → RUN_FINISHED。

   本府只把「收到什么」写进状态并广播出去；气泡怎么画、确认钮怎么落，
   一概归 观测台面-府。故本府不 import 任何其它府的业务代码。
   ============================================================ */

import { 运行时, 连接语, 总线, 广播 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 刷连接语 } from "/流式驱动-府/双流接入-殿/双流拉取-阁/订阅-逻辑-园/订阅逻辑.js";

/** 接待气泡的身份号：观测台面据此把「增量」接到正确的气泡上 */
let 接待序号 = 0;

export async function 问道祖(文){
  if(运行时.接待控) 运行时.接待控.abort();   // 新话盖旧话：同一时刻只留一条接待
  const 控 = new AbortController();
  运行时.接待控 = 控;
  连接语("实时 · 道祖接待中…");
  const id = `接待-${++接待序号}`;
  const 记录 = {id, 角色:"道祖", 时:"主控", 文:""};
  运行时.接待记录.push(记录);
  广播(总线.接待开启, {id, 角色:"道祖", 时:"主控"});
  try{
    const 响应 = await fetch("/api/dev/chat/stream", {
      method:"POST",
      headers:{"Content-Type":"application/json"},
      body: JSON.stringify({消息: 文}),
      signal: 控.signal,
    });
    if(响应.status === 429) throw new Error("接待通道已占用（429），请稍候再发");
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 读 = 响应.body.getReader();
    const 解 = new TextDecoder();
    let 剩 = "";
    for(;;){
      const {value, done} = await 读.read();
      if(done) break;
      剩 += 解.decode(value, {stream:true});
      const 块s = 剩.split("\n\n");
      剩 = 块s.pop();
      for(const 块 of 块s){
        const 行 = 块.split("\n").find(l=>l.startsWith("data:"));
        if(!行) continue;
        let ev;
        try{ ev = JSON.parse(行.slice(5).trim()); }catch{ continue; }
        if(ev.type === "TEXT_MESSAGE_CONTENT" && ev.delta){
          记录.文 += ev.delta;
          广播(总线.接待增量, {id, 文本: 记录.文});
        }else if(ev.type === "RUN_FINISHED"){
          if(ev.阶段 === "待确认") 运行时.待确认 = true;
          if(ev.任务id) 广播(总线.请求拉看板);
        }
      }
    }
    // 收到什么显示什么：没有答复就如实说没有，不替道祖编话
    if(!记录.文) 记录.文 = "（未收到答复：道祖可能未上线）";
    广播(总线.接待收尾, {id, 文本: 记录.文, 待确认: 运行时.待确认});
  }catch(错){
    if(错.name === "AbortError") return;
    记录.文 = "接待失败：" + 错.message;
    广播(总线.接待收尾, {id, 文本: 记录.文, 待确认:false});
  }finally{
    if(运行时.接待控 === 控) 运行时.接待控 = null;
    刷连接语();
  }
}
