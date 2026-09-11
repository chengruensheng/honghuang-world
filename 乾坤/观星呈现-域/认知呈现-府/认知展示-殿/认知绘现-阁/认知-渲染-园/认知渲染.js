/* ============================================================
   观星呈现-域 · 认知呈现-府 · 认知展示-殿 · 认知渲染
   格位视图（36 格位心智地图，点卡看详情）+ 道规视图（大道/天道规则，可编辑保存）。
   数据取自 /api/cognition/cells（后端权威）；规则全文经 /api/files/content 读、/api/rules/write 写。
   失败不静默——把原因写进统计位，界面只说已确认的事实。
   ============================================================ */

import { $, 转义 } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";

/** 36 固定格位名（6 维度 × 6 名；权威源：格位库.rs::维度格位名） */
const 三十六格位名 = {
  内部: ["角色","能力","职责","状态","记忆","边界"],
  外在: ["定位","结构","依赖","接口","环境","数据"],
  规则: ["禁止","门禁","规范","流程","例外","红线"],
  执行: ["命令","技术栈","步骤","工具","验证","产出"],
  目标: ["初心","现况","愿景","偏移","度量","依赖"],
  经历: ["事件","教训","决策","来源","时间","归档"],
};
const 维度顺序 = ["内部","外在","规则","执行","目标","经历"];
const 层级徽 = { 大道: ["大","红"], 天道: ["天","黄"] };

/** 维度载荷字段 → 中文标签（展示用） */
const 载荷字段名 = {
  层级:"层级", 触发条件:"触发条件", 严重度:"严重度", 例外条款:"例外条款", 历史违反次数:"历史违反次数",
  标准命令:"标准命令", 标准步骤:"标准步骤", 工具清单:"工具清单", 产出契约:"产出契约", 最近实例指针:"最近实例指针",
  初心:"初心", 现况:"现况", 愿景:"愿景", 偏移:"偏移", 度量:"度量", 依赖:"依赖",
  节点路径:"节点路径", 入口:"入口", 数据形状:"数据形状",
  自评语句:"自评语句",
  时间:"时间", 触发器:"触发器", 结果:"结果", 教训要点:"教训要点",
};

const 时间可读 = 秒 => { const n = Number(秒); return n ? new Date(n * 1000).toLocaleString("zh-CN") : "无"; };
const 冷却可读 = 截止 => {
  if(!截止) return "无";
  return Math.floor(Date.now()/1000) < Number(截止) ? `冷却至 ${时间可读(截止)}` : "已解除";
};

/* ---------- 格位视图 ---------- */

/** 拉格位：读 /api/cognition/cells，渲染 6 维度 × 6 格位名 = 36 格位 */
export async function 拉格位(){
  try{
    const 响应 = await fetch("/api/cognition/cells");
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 数据 = await 响应.json();
    渲染格位(数据.格位集 || []);
  }catch(错){
    $("#格位区").innerHTML = 空态("格位拉取失败：" + 转义(错.message));
    $("#格位统").textContent = "格位拉取失败";
  }
}

function 渲染格位(格位集){
  const 区 = $("#格位区");
  区.innerHTML = "";
  let 已填 = 0;
  维度顺序.forEach(维=>{
    const 名表 = 三十六格位名[维] || [];
    const 格 = 名表.map(名=>格位集.find(g=>g.维度===维 && g.格位名===名)).filter(Boolean);
    已填 += 格.filter(g=>(g.摘要||"").length > 0).length;
    区.appendChild(建维组(维, 格));
  });
  $("#格位统").textContent = `36 格位 · ${已填} 已填 · ${36 - 已填} 未填`;
}

function 建维组(维, 格){
  const 组 = document.createElement("section");
  组.className = "维组";
  组.innerHTML = `<div class="维组头"><span class="维名">${转义(维)}</span><span class="维数">${格.length}</span></div><div class="维身"></div>`;
  const 身 = 组.querySelector(".维身");
  格.forEach(g=>身.appendChild(建格卡(g)));
  return 组;
}

function 建格卡(g){
  const 未填 = !(g.摘要 && g.摘要.length > 0);
  const 信 = Math.round((g.可信度 || 0) * 100);
  const 卡 = document.createElement("div");
  卡.className = "格位卡" + (未填 ? " 未填" : "");
  卡.title = "点击查看详情";
  卡.setAttribute("role", "button");
  卡.tabIndex = 0;
  卡.setAttribute("aria-label", "查看格位 " + g.格位名 + " 详情");
  卡.innerHTML = `
    <div class="格名">${转义(g.格位名)}</div>
    <div class="格摘要">${未填 ? "未填" : 转义(g.摘要)}</div>
    <div class="格底"><span class="信条"><i style="width:${信}%"></i></span><span class="信数">${信}%</span><span class="证数">${(g.证据引用||[]).length} 证</span></div>`;
  卡.addEventListener("click", ()=>开详情(g));
  return 卡;
}

/** 格位详情：点卡弹模态，展示完整载荷/证据/时间/冷却期 */
function 开详情(g){
  document.querySelector(".格遮罩")?.remove();
  const 信 = Math.round((g.可信度 || 0) * 100);
  const 遮 = document.createElement("div");
  遮.className = "格遮罩";
  遮.innerHTML = `
    <div class="格详情卡">
      <div class="格详情头">
        <span class="格详情名">${转义(g.格位名)}</span>
        <span class="格详情维">${转义(g.维度)}</span>
        <button class="格详情关" title="关闭">×</button>
      </div>
      <div class="格详情身">
        <div class="格详情摘要">${转义(g.摘要 || "未填")}</div>
        <div class="格详情信"><span class="信条"><i style="width:${信}%"></i></span><span class="信数">${信}%</span></div>
        ${渲染载荷(g.维度载荷)}
        <div class="格详情节"><div class="节名">证据引用</div>${(g.证据引用||[]).length ? (g.证据引用||[]).map(e=>`<div class="证条">${转义(e)}</div>`).join("") : `<div class="载空">无</div>`}</div>
        <div class="格详情节">
          <div class="节名">时间</div>
          <div class="载条"><span class="载名">最后校验</span><span class="载值">${时间可读(g.最后校验时间)}</span></div>
          <div class="载条"><span class="载名">冷却期</span><span class="载值">${冷却可读(g.冷却期至)}</span></div>
        </div>
      </div>
    </div>`;
  遮.addEventListener("click", e=>{ if(e.target === 遮) 遮.remove(); });
  遮.querySelector(".格详情关").addEventListener("click", ()=>遮.remove());
  document.body.appendChild(遮);
}

/** 渲染维度载荷（外部标签枚举：{ "规则": {...} } 等） */
function 渲染载荷(载荷){
  if(!载荷 || typeof 载荷 !== "object" || Object.keys(载荷).length === 0){
    return `<div class="格详情节"><div class="节名">维度载荷</div><div class="载空">未填载荷</div></div>`;
  }
  const 键 = Object.keys(载荷)[0];
  const 值 = 载荷[键];
  let 条 = `<div class="载条"><span class="载名">载荷类型</span><span class="载值">${转义(键)}</span></div>`;
  if(值 && typeof 值 === "object"){
    for(const [k, v] of Object.entries(值)){
      if(v === null || v === undefined || v === "" || (Array.isArray(v) && v.length === 0)) continue;
      const 名 = 载荷字段名[k] || k;
      const 文 = Array.isArray(v) ? v.map(转义).join("、") : 转义(String(v));
      条 += `<div class="载条"><span class="载名">${转义(名)}</span><span class="载值">${文}</span></div>`;
    }
  }
  return `<div class="格详情节"><div class="节名">维度载荷</div>${条}</div>`;
}

/* ---------- 道规视图 ---------- */

/** 拉道规：读 /api/cognition/cells，筛出规则维度里指向 rules/*.md 的规则种子 */
export async function 拉道规(){
  try{
    const 响应 = await fetch("/api/cognition/cells");
    if(!响应.ok) throw new Error("HTTP " + 响应.status);
    const 数据 = await 响应.json();
    渲染道规(数据.格位集 || []);
  }catch(错){
    $("#道规区").innerHTML = 空态("道规拉取失败：" + 转义(错.message));
    $("#道规统").textContent = "道规拉取失败";
  }
}

function 渲染道规(格位集){
  const 区 = $("#道规区");
  区.innerHTML = "";
  const 规 = 格位集.filter(g=>g.维度==="规则" && (g.证据引用||[]).some(证=>证.includes("rules/")));
  const 大道 = 规.filter(g=>取层级(g)==="大道");
  const 天道 = 规.filter(g=>取层级(g)==="天道");
  $("#道规统").textContent = `大道 ${大道.length} 条 · 天道 ${天道.length} 条 · 可编辑`;
  if(大道.length) 区.appendChild(建道规组("大道", 大道));
  if(天道.length) 区.appendChild(建道规组("天道", 天道));
  if(!规.length) 区.innerHTML = 空态("未从格位读到任何 rules/*.md 规则种子");
}

function 取层级(g){
  // 维度载荷序列化形如 { "规则": { "层级": "大道", ... } }（serde 外部标签枚举）
  const 载 = g.维度载荷 && g.维度载荷.规则;
  return 载 && 载.层级 ? 载.层级 : "天道";
}

function 建道规组(层, 列表){
  const 徽 = 层级徽[层] || ["规","黄"];
  const 组 = document.createElement("section");
  组.className = "道规组";
  组.innerHTML = `<div class="道规组头"><span class="层徽 ${徽[1]}">${徽[0]}</span><span class="层名">${转义(层)}</span><span class="层数">${列表.length} 条</span></div><div class="道规身"></div>`;
  const 身 = 组.querySelector(".道规身");
  列表.forEach(g=>身.appendChild(建道规项(g)));
  return 组;
}

function 建道规项(g){
  const 项 = document.createElement("div");
  项.className = "道规项";
  const 证据路径 = (g.证据引用||[]).find(证=>证.includes("rules/")) || "";
  项.innerHTML = `
    <div class="规头" role="button" tabindex="0" aria-expanded="false"><span class="规名">${转义(g.格位名)}</span><span class="规摘要">${转义(g.摘要)}</span><span class="规箭头">▸</span></div>
    <div class="规全文"></div>`;
  const 全文 = 项.querySelector(".规全文");
  const 规头 = 项.querySelector(".规头");
  规头.addEventListener("click", async ()=>{
    const 开 = 项.classList.toggle("开");
    规头.setAttribute("aria-expanded", 开 ? "true" : "false");
    规头.querySelector(".规箭头").textContent = 开 ? "▾" : "▸";
    if(开 && !全文.dataset.载){
      全文.dataset.载 = "1";
      await 载全文(全文, 证据路径);
    }
  });
  return 项;
}

/** 读规则全文（/api/files/content）并渲染只读态 + 编辑入口 */
async function 载全文(全文, 证据路径){
  全文.innerHTML = `<div class="规加载">载入中…</div>`;
  try{
    const r = await fetch("/api/files/content?路径=" + encodeURIComponent(证据路径));
    if(!r.ok) throw new Error("HTTP " + r.status);
    const 内容 = await r.json();
    渲染全文(全文, 证据路径, 内容.内容 || "");
  }catch(错){
    全文.innerHTML = `<div class="规错">规则全文读取失败：${转义(错.message)}</div>`;
  }
}

function 渲染全文(全文, 证据路径, 内容){
  全文.innerHTML = `
    <pre class="规文">${转义(内容)}</pre>
    <div class="规操"><button class="规编辑钮">编辑</button></div>`;
  全文.querySelector(".规编辑钮").addEventListener("click", ()=>{
    全文.innerHTML = `
      <textarea class="规编框">${转义(内容)}</textarea>
      <div class="规操"><button class="规存钮">保存</button><button class="规取钮">取消</button><span class="规提"></span></div>`;
    全文.querySelector(".规取钮").addEventListener("click", ()=>渲染全文(全文, 证据路径, 内容));
    全文.querySelector(".规存钮").addEventListener("click", async ()=>{
      const 提 = 全文.querySelector(".规提");
      const 新内容 = 全文.querySelector(".规编框").value;
      try{
        const r = await fetch("/api/rules/write", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ 路径: 证据路径, 内容: 新内容 }),
        });
        if(!r.ok) throw new Error("HTTP " + r.status);
        提.textContent = "已保存，刷新中…";
        await 载全文(全文, 证据路径);
      }catch(错){
        提.textContent = "保存失败：" + 错.message;
      }
    });
  });
}

function 空态(语){
  return `<div class="空" style="height:auto;padding:30px 10px"><div class="符">○</div><div class="语">${语}</div></div>`;
}
