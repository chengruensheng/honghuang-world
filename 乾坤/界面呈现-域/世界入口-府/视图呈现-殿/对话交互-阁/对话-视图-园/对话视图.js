// 对话视图.js —— 对话界面（真实智能体执行：受理 → 事件轮询 → 答复）

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 加载引擎数据 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;

// 轮询间隔与超时上限（毫秒）
const 轮询间隔 = 500;
const 轮询超时 = 15 * 60 * 1000;

let 主容器 = null;
let 面板容器 = null;
let 轮询器 = null;
let 运行中 = false;
let 轮次 = 0;
let 游标 = 0;
let 轮询起始 = 0;
let 锁定按钮 = null;


export const 对话视图 = {
  键: '对话',
  标题: '对话',
  副标题: '与智能体对话，观察它的执行过程',
  分组: '工作台',
  图标,
  挂载(容器) {
    主容器 = 容器;
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>与智能体对话，观察它的执行过程</p></div></div><div class="chat" id="chat"></div><div class="chat-input"><input id="task" placeholder="下达任务，如：在沙箱里写一个 fibonacci 模块…" /><button class="btn" id="go">下达</button></div>`;
    const 输入框 = 容器.querySelector('#task');
    锁定按钮 = 容器.querySelector('#go');
    锁定按钮.addEventListener('click', () => 下达(输入框));
    输入框.addEventListener('keydown', (事件) => {
      if (事件.key === 'Enter') 下达(输入框);
    });
  },
  属性(容器) {
    面板容器 = 容器;
    渲染面板();
  },
  卸载() {
    停止轮询();
    运行中 = false;
    主容器 = null;
    面板容器 = null;
  },
};

/** 下达任务：真实受理后启动事件轮询 */
async function 下达(输入框) {
  if (运行中) return;
  const 文案 = 输入框.value.trim();
  if (!文案) return;
  运行中 = true;
  轮次++;
  锁定按钮.disabled = true;
  输入框.value = '';

  气泡('user', 文案);
  记日志('【下达】', 'act', 文案);
  设置过程('受理中…');

  try {
    const 响应 = await fetch('/api/dev/agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 任务: 文案 }),
    });
    if (响应.ok) {
      const 结果 = await 响应.json();
      气泡('ai', `任务已受理（任务 #${结果.任务id}），智能体开始执行。`);
      设置过程('执行中');
      游标 = 0;
      轮询起始 = Date.now();
      启动轮询();
    } else {
      const 错误 = await 响应.json().catch(() => ({ 错误: `HTTP ${响应.status}` }));
      气泡('ai', `⛔ ${错误.错误 || `受理失败（HTTP ${响应.status}）`}`);
      设置过程('待命');
      记日志('【受阻】', 'warn', 错误.错误 || '受理失败');
      结束执行();
    }
  } catch (错误) {
    气泡('ai', '⛔ 无法连接数据服务，请确认后端已启动。');
    设置过程('待命');
    结束执行();
  }
}

/** 轮询事件流：增量渲染智能体执行过程 */
async function 轮询() {
  if (!运行中) return;
  try {
    const 响应 = await fetch(`/api/dev/events?since=${游标}`);
    if (响应.ok) {
      const 数据 = await 响应.json();
      数据.事件.forEach(渲染事件);
      if (数据.事件.length > 0) {
        游标 = 数据.事件[数据.事件.length - 1].序号;
        渲染面板({ 运行中: 数据.运行中, 工作区: 数据.工作区, 最近结果: 数据.最近结果, 就绪: 数据.就绪 });
      }
      if (!数据.运行中 && 数据.事件.some((事件) => 事件.类型 === '任务答复')) {
        执行结束(数据);
        return;
      }
      if (!数据.运行中 && 游标 === 0) {
        // 受理成功但事件为空且未运行：异常收尾，避免无限轮询
        执行结束(数据);
        return;
      }
    }
  } catch (错误) {
    console.warn('事件轮询失败（后端未就绪？）', 错误);
  }
  if (Date.now() - 轮询起始 > 轮询超时) {
    气泡('ai', '⏱ 执行超时，已停止观察（任务可能仍在后台运行）。');
    记日志('【超时】', 'warn', '事件轮询超时');
    结束执行();
    return;
  }
  轮询器 = setTimeout(轮询, 轮询间隔);
}

function 启动轮询() {
  停止轮询();
  轮询器 = setTimeout(轮询, 轮询间隔);
}

function 停止轮询() {
  if (轮询器) {
    clearTimeout(轮询器);
    轮询器 = null;
  }
}

/** 执行收尾：刷新引擎数据、记日志、解锁 */
async function 执行结束(数据) {
  停止轮询();
  设置过程('任务完成');
  记日志('【完成】', 'ok', 数据.最近结果 || '智能体执行结束');
  await 加载引擎数据();
  渲染面板({ 运行中: false, 工作区: 数据.工作区, 最近结果: 数据.最近结果, 就绪: 数据.就绪 });
  结束执行();
}

function 结束执行() {
  运行中 = false;
  if (锁定按钮) 锁定按钮.disabled = false;
}

/** 按类型渲染一条执行事件 */
function 渲染事件(事件) {
  if (事件.类型 === '思考') {
    气泡('ai', 事件.内容 || '思考中…');
    设置过程(`第 ${事件.轮次 + 1} 轮 · 思考`);
  } else if (事件.类型 === '工具调用') {
    工具调用(事件.工具名, 事件.内容);
    设置过程(`第 ${事件.轮次 + 1} 轮 · ${事件.工具名}`);
  } else if (事件.类型 === '工具结果') {
    工具结果(事件.内容);
  } else if (事件.类型 === '任务答复') {
    气泡('ai', `✅ ${事件.内容}`);
    设置过程('任务完成');
  }
}

function 气泡(角色, 文本) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ' + (角色 === 'user' ? 'user' : 'ai');
  行.innerHTML = `<div class="avatar ${角色 === 'user' ? 'u' : 'ai'}">${角色 === 'user' ? '我' : 'AI'}</div><div class="bubble"><span class="role">${角色 === 'user' ? '用户' : '智能体'}</span>${转义(文本)}</div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 工具调用(工具, 参数) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ai';
  行.innerHTML = `<div class="avatar ai">AI</div><div class="bubble" style="flex:1;max-width:100%;padding:0;background:none;border:none;"><div class="toolcall"><div class="tt"><svg viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"/></svg>调用工具 · <span class="cmd">${转义(工具)}</span></div><div class="tr">${转义(参数)}</div></div></div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 工具结果(文本) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ai';
  行.innerHTML = `<div class="avatar ai">AI</div><div class="bubble" style="flex:1;max-width:100%;padding:0;background:none;border:none;"><div class="toolcall"><div class="tt" style="color:var(--wood)">✓ 工具返回</div><div class="tr">${转义(文本)}</div></div></div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 设置过程(文本) {
  if (!面板容器) return;
  const 流 = 面板容器.querySelector('#p-flow');
  if (流) 流.innerHTML = `<div class="kv"><b>当前动作</b>${转义(文本)}</div>`;
}

/** 渲染属性面板：真实状态 + 中断按钮 + 工作区切换 */
function 渲染面板(状态) {
  if (!面板容器) return;
  const 状态块 = 状态 || { 就绪: null, 运行中, 工作区: '—', 最近结果: null };
  const 就绪文案 = 状态块.就绪 === null ? '待查询' : (状态块.就绪 ? '已上线' : '未上线（需 LLM_API_KEY）');
  const 运行文案 = 状态块.运行中 ? '执行中' : '待命';
  const 结果文案 = 状态块.最近结果 ? 转义(状态块.最近结果) : '—';
  const 工作区只读 = 状态块.运行中 ? 'readonly' : '';
  面板容器.innerHTML = `<h3>智能体状态</h3><div class="prop-group">
    <div class="prop-item"><span class="k">就绪</span><span class="v">${转义(就绪文案)}</span></div>
    <div class="prop-item"><span class="k">状态</span><span class="v">${转义(运行文案)}</span></div>
    <div class="prop-item"><span class="k">工具</span><span class="v">读文件 / 写文件 / 运行命令</span></div>
    <div class="prop-item"><span class="k">工作区</span><span class="v">${转义(状态块.工作区)}</span></div>
    <div class="prop-item" style="flex-direction:column;align-items:stretch;">
      <span class="k" style="margin-bottom:4px;">切换工作区</span>
      <div style="display:flex;gap:4px;">
        <input id="ws-input" value="${转义(状态块.工作区 === '—' ? '' : 状态块.工作区)}" placeholder="输入工作区路径…" style="flex:1;font-size:12px;padding:4px 6px;border:1px solid var(--border);border-radius:4px;" ${工作区只读} />
        <button class="btn" id="ws-btn" style="padding:4px 8px;font-size:12px;" ${工作区只读}>切换</button>
      </div>
    </div>
    <div class="prop-item"><span class="k">最近结果</span><span class="v">${结果文案}</span></div>
  </div><h3>执行过程</h3><div id="p-flow"><div class="kv"><b>当前动作</b>${状态块.运行中 ? '执行中' : '待命'}</div></div>
  ${状态块.运行中 ? '<button class="btn" id="stop" style="margin-top:8px;width:100%;">中断执行</button>' : ''}`;
  const 停止按钮 = 面板容器.querySelector('#stop');
  if (停止按钮) 停止按钮.addEventListener('click', 中断执行);
  const 切换按钮 = 面板容器.querySelector('#ws-btn');
  if (切换按钮) 切换按钮.addEventListener('click', 切换工作区);
}

/** 切换工作区：POST /api/dev/workspace */
async function 切换工作区() {
  const 输入框 = 面板容器.querySelector('#ws-input');
  if (!输入框) return;
  const 新工作区 = 输入框.value.trim();
  if (!新工作区) return;
  try {
    const 响应 = await fetch('/api/dev/workspace', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 工作区: 新工作区 }),
    });
    if (响应.ok) {
      气泡('ai', `工作区已切换到：${新工作区}`);
      记日志('【切换】', 'act', `工作区 → ${新工作区}`);
    } else {
      const 错误 = await 响应.json().catch(() => ({ 错误: `HTTP ${响应.status}` }));
      气泡('ai', `切换失败：${错误.错误 || 响应.status}`);
    }
  } catch (错误) {
    气泡('ai', '无法连接数据服务，请确认后端已启动。');
  }
}

/** 中断：置位后端中断句柄，智能体下一轮停止 */
async function 中断执行() {
  try {
    await fetch('/api/dev/agent/stop', { method: 'POST' });
    气泡('ai', '已请求中断，智能体将在下一轮停止。');
    记日志('【中断】', 'warn', '用户请求中断执行');
  } catch (错误) {
    console.warn('中断请求失败', 错误);
  }
}

/** HTML 转义，防止任务文本注入标记 */
function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
