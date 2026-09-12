/* ============================================================
   观星呈现-域 · 呈现验证-府 · 契约断言-殿 · 渲染校验-阁
   认知渲染 用例：格位视图（36 格位心智地图 + 详情模态）+ 道规视图（读全文 / 编辑 / 保存）。
   锁的硬约束：
   · 六维度常驻、36 格位名固定（权威源：格位库.rs::维度格位名）——格位名不得随数据漂移；
   · 未填格位如实标「未填」，空载荷字段不列（空白字段列出来只是噪音）；
   · 失败不静默：拉取失败写进对应统计位；保存失败就地提示且不丢编辑内容。
   ============================================================ */

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { 装台面 } from "/呈现验证-府/映射装配-殿/台面装配-阁/台面-模块-园/台面桩.mjs";
import { $ } from "/底座骨架-府/台面框架-殿/视图装配-阁/装配-逻辑-园/运行时中枢.js";
import { 拉格位, 拉道规 }
  from "/认知呈现-府/认知展示-殿/认知绘现-阁/认知-渲染-园/认知渲染.js";

let 档;
beforeEach(()=>{ ({档} = 装台面()); });

const 等到 = ()=>new Promise(r=>setTimeout(r, 5));
const 格位桩 = (格位集)=>{ globalThis.fetch = async ()=>({ok:true, json:async()=>({格位集})}); };

test("拉格位：六维度常驻、命中格位归组、统计如实", async ()=>{
  格位桩([
    {维度:"内部", 格位名:"角色", 摘要:"主控者", 可信度:0.8, 证据引用:["a.md","b.md"]},
    {维度:"内部", 格位名:"能力", 摘要:"", 可信度:0, 证据引用:[]},
    {维度:"规则", 格位名:"门禁", 摘要:"过门禁", 可信度:1, 证据引用:["rules/x.md"]},
  ]);
  await 拉格位();
  const 区 = $("#格位区");
  assert.equal(区.querySelectorAll(".维组").length, 6, "六维度常驻：空的维度组也要在，维度缺失不能被读成界面丢了维度");
  assert.deepEqual([...区.querySelectorAll(".维名")].map(x=>x.textContent),
    ["内部","外在","规则","执行","目标","经历"]);
  // 已填按「摘要非空」算：角色（"主控者"）与门禁（"过门禁"）有摘要，能力摘要是空串即未填
  assert.equal($("#格位统").textContent, "36 格位 · 2 已填 · 34 未填");

  const 内部组 = 区.querySelectorAll(".维组")[0];
  assert.equal(内部组.querySelector(".维数").textContent, "2", "该维度在数据里命中 2 个格位");
  const 卡 = 内部组.querySelectorAll(".格位卡");
  assert.equal(卡[0].querySelector(".格名").textContent, "角色");
  assert.equal(卡[0].querySelector(".格摘要").textContent, "主控者");
  assert.equal(卡[0].querySelector(".信数").textContent, "80%");
  assert.equal(卡[0].querySelector(".证数").textContent, "2 证");
  assert.ok(卡[1].classList.contains("未填"));
  assert.equal(卡[1].querySelector(".格摘要").textContent, "未填");
});

test("拉格位失败：原因落进格位区与统计位", async ()=>{
  globalThis.fetch = async ()=>({ok:false, status:503});
  await 拉格位();
  assert.equal($("#格位区").querySelector(".空 .语").textContent, "格位拉取失败：HTTP 503");
  assert.equal($("#格位统").textContent, "格位拉取失败");
});

test("格位详情：点卡弹模态（载荷/证据/时间/冷却），关钮与遮罩点击都能关", async ()=>{
  格位桩([{维度:"内部", 格位名:"角色", 摘要:"主控者", 可信度:0.8, 证据引用:["a.md","b.md"],
    最后校验时间: 1789000000, 冷却期至: 9999999999}]);
  await 拉格位();
  $("#格位区").querySelector(".格位卡").click();

  const 遮 = document.querySelector(".格遮罩");
  assert.notEqual(遮, null);
  assert.equal(遮.querySelector(".格详情名").textContent, "角色");
  assert.equal(遮.querySelector(".格详情维").textContent, "内部");
  assert.equal(遮.querySelector(".格详情摘要").textContent, "主控者");
  assert.equal(遮.querySelectorAll(".证条").length, 2);
  assert.equal(遮.querySelector(".载空").textContent, "未填载荷");

  const 时 = [...遮.querySelectorAll(".载条")].map(x=>[x.querySelector(".载名").textContent, x.querySelector(".载值").textContent]);
  assert.deepEqual(时[0], ["最后校验", new Date(1789000000 * 1000).toLocaleString("zh-CN")]);
  assert.equal(时[1][0], "冷却期");
  assert.ok(时[1][1].startsWith("冷却至 "), "冷却未过：写明冷却到什么时候");

  // 点内容不关，点遮罩才关
  遮.querySelector(".格详情卡").click();
  assert.notEqual(document.querySelector(".格遮罩"), null, "点卡片内容不该误关");
  遮.click();
  assert.equal(document.querySelector(".格遮罩"), null);

  // 再开一次：关钮可关，且不会叠出第二层遮罩
  $("#格位区").querySelector(".格位卡").click();
  $("#格位区").querySelector(".格位卡").click();
  assert.equal(document.querySelectorAll(".格遮罩").length, 1, "详情模态至多一层");
  document.querySelector(".格详情关").click();
  assert.equal(document.querySelector(".格遮罩"), null);
});

test("维度载荷：中文标签就位，空值字段不列", async ()=>{
  格位桩([{维度:"规则", 格位名:"门禁", 摘要:"x", 可信度:1, 证据引用:[],
    维度载荷:{规则:{层级:"大道", 触发条件:"提交前", 严重度:"高", 例外条款:"", 历史违反次数:3, 空数组:[]}}}]);
  await 拉格位();
  $("#格位区").querySelector(".格位卡").click();
  const 遮 = document.querySelector(".格遮罩");
  assert.equal(遮.querySelector(".格详情节 .载名").textContent, "载荷类型");
  assert.equal(遮.querySelector(".格详情节 .载值").textContent, "规则");
  // 只数「维度载荷」那一节的载条：时间节（最后校验 / 冷却期）也用 .载条，别混进来
  const 载节 = [...遮.querySelectorAll(".格详情节")].find(x=>x.querySelector(".节名").textContent === "维度载荷");
  const 名 = [...载节.querySelectorAll(".载条 .载名")].map(x=>x.textContent);
  assert.deepEqual(名, ["载荷类型","层级","触发条件","严重度","历史违反次数"],
    "空字符串与空数组字段略过——空白字段列出来只是噪音");
  assert.ok(遮.textContent.includes("大道"));
});

test("拉道规：只认规则维度且证据指向 rules/ 的格位，按大道/天道分组（缺层级归天道）", async ()=>{
  格位桩([
    {维度:"规则", 格位名:"缺陷治理", 摘要:"六类缺陷", 证据引用:["rules/AI缺陷治理规则.md"], 维度载荷:{规则:{层级:"天道"}}},
    {维度:"规则", 格位名:"命名规则", 摘要:"命名有据", 证据引用:["rules/命名规则.md"], 维度载荷:{规则:{层级:"大道"}}},
    {维度:"规则", 格位名:"兜底条", 摘要:"无层级载荷", 证据引用:["rules/z.md"], 维度载荷:null},
    {维度:"规则", 格位名:"无关格", 摘要:"证据不指向 rules/", 证据引用:["x.rs"], 维度载荷:{规则:{层级:"天道"}}},
    {维度:"内部", 格位名:"角色", 摘要:"非规则维度", 证据引用:["rules/y.md"], 维度载荷:{}},
  ]);
  await 拉道规();
  assert.equal($("#道规统").textContent, "大道 1 条 · 天道 2 条 · 可编辑");
  const 组 = $("#道规区").querySelectorAll(".道规组");
  assert.equal(组.length, 2);
  assert.equal(组[0].querySelector(".层名").textContent, "大道");
  assert.equal(组[0].querySelector(".层徽").textContent, "大");
  assert.equal(组[1].querySelector(".层名").textContent, "天道");
  assert.equal(组[1].querySelector(".层徽").textContent, "天");
  assert.equal(组[1].querySelectorAll(".道规项").length, 2, "缺层级载荷者归天道——不硬算成大道");
});

test("拉道规无收获：如实说没读到规则种子", async ()=>{
  格位桩([{维度:"内部", 格位名:"角色", 摘要:"x", 证据引用:[], 维度载荷:{}}]);
  await 拉道规();
  assert.equal($("#道规区").querySelector(".空 .语").textContent, "未从格位读到任何 rules/*.md 规则种子");
  assert.equal($("#道规统").textContent, "大道 0 条 · 天道 0 条 · 可编辑");
});

test("道规项：展开载全文 → 编辑 → 保存成功后以新文重载；取消恢复只读", async ()=>{
  const 调用 = [];
  globalThis.fetch = async (路径, 参)=>{
    调用.push([路径, 参]);
    if(路径.startsWith("/api/cognition/cells")) return {ok:true, json:async()=>({格位集:[
      {维度:"规则", 格位名:"命名规则", 摘要:"命名有据", 证据引用:["rules/命名规则.md"], 维度载荷:{规则:{层级:"大道"}}},
    ]})};
    if(路径 === "/api/rules/write") return {ok:true, json:async()=>({})};
    return {ok:true, json:async()=>({内容:"规则正文 v1"})};
  };
  await 拉道规();
  const 项 = $("#道规区").querySelector(".道规项");
  const 头 = 项.querySelector(".规头");

  头.click();
  await 等到();
  assert.ok(项.classList.contains("开"));
  assert.equal(头.querySelector(".规箭头").textContent, "▾");
  assert.equal(项.querySelector(".规文").textContent, "规则正文 v1");
  assert.equal(调用.at(-1)[0], "/api/files/content?路径=" + encodeURIComponent("rules/命名规则.md"));

  // 编辑 → 取消：恢复只读，不回写
  项.querySelector(".规编辑钮").click();
  assert.equal(项.querySelector(".规编框").value, "规则正文 v1");
  项.querySelector(".规编框").value = "改了一半";
  项.querySelector(".规取钮").click();
  assert.equal(项.querySelectorAll(".规编框").length, 0);
  assert.equal(项.querySelector(".规文").textContent, "规则正文 v1", "取消即回到服务端的版本，草稿不留在屏上");

  // 编辑 → 保存：写接口 + 以服务端新文重载
  globalThis.fetch = async (路径, 参)=>{
    调用.push([路径, 参]);
    if(路径 === "/api/rules/write") return {ok:true, json:async()=>({})};
    return {ok:true, json:async()=>({内容:"规则正文 v2"})};
  };
  项.querySelector(".规编辑钮").click();
  项.querySelector(".规编框").value = "规则正文 v2";
  项.querySelector(".规存钮").click();
  await 等到(); await 等到();
  const 写 = 调用.find(([p,m])=>p === "/api/rules/write" && m);
  assert.deepEqual(JSON.parse(写[1].body), {路径:"rules/命名规则.md", 内容:"规则正文 v2"});
  assert.equal(项.querySelector(".规文").textContent, "规则正文 v2", "保存成功即用服务端版本重载——屏上是权威文本");
});

test("保存失败：就地提示且不丢编辑内容；载全文失败如实报错", async ()=>{
  globalThis.fetch = async (路径)=>{
    if(路径.startsWith("/api/cognition/cells")) return {ok:true, json:async()=>({格位集:[
      {维度:"规则", 格位名:"命名规则", 摘要:"x", 证据引用:["rules/命名规则.md"], 维度载荷:{规则:{层级:"大道"}}},
    ]})};
    if(路径 === "/api/rules/write") return {ok:false, status:500};
    return {ok:true, json:async()=>({内容:"规则正文"})};
  };
  await 拉道规();
  const 项 = $("#道规区").querySelector(".道规项");
  项.querySelector(".规头").click();
  await 等到();
  项.querySelector(".规编辑钮").click();
  项.querySelector(".规编框").value = "改后正文";
  项.querySelector(".规存钮").click();
  await 等到();
  assert.equal(项.querySelector(".规提").textContent, "保存失败：HTTP 500");
  assert.equal(项.querySelector(".规编框").value, "改后正文", "保存失败不得丢掉用户刚写的内容");

  // 载全文失败（换一条：让 content 接口失败）
  globalThis.fetch = async (路径)=>{
    if(路径.startsWith("/api/cognition/cells")) return {ok:true, json:async()=>({格位集:[
      {维度:"规则", 格位名:"命名规则", 摘要:"x", 证据引用:["rules/命名规则.md"], 维度载荷:{规则:{层级:"大道"}}},
    ]})};
    return {ok:false, status:500};
  };
  await 拉道规();
  $("#道规区").querySelector(".规头").click();
  await 等到();
  assert.equal($("#道规区").querySelector(".规错").textContent, "规则全文读取失败：HTTP 500");
});
