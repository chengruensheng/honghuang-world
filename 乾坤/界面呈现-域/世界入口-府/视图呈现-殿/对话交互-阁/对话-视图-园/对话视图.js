// 对话视图.js —— 对话受理（需求 → 发布进看板 → 驱动器自主五层流转）

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 加载引擎数据 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';
import { 加载看板数据 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;

// 轮询间隔与超时上限（毫秒）
const 轮询间隔 = 1000;
const 轮询超时 = 15 * 60 * 1000;

let 主容器 = null;
let 面板容器 = null;
let 轮询器 = null;
let 运行中 = false;
let 轮询起始 = 0;
let 锁定按钮 = null;
let 上次阶段签名 = '';

export const 对话视图 = {
  键: '对话',
  标题: '对话',
  副标题: '下达需求 → 发布进看板 → 驱动器自主五层流转',
  分组: '工作台',
  图标,
  挂载(容器) {
    主容器 = 容器;
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>下达需求，发布到看板由驱动器自主五层流转（圣人设计→大罗金仙实现→准圣验收→道祖终审）</p></div></div><div class="chat" id="chat"></div><div class="chat-input"><input id="task" placeholder="下达需求，如：在沙箱里写一个 fibonacci 模块…" /><button class="btn" id="go">下达</button></div>`;
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

/** 下达需求：受理即发布进看板并自动驱动，轮询驱动器摘要 */
async function 下达(输入框) {
  if (运行中) return;
  const 文案 = 输入框.value.trim();
  if (!文案) return;
  运行中 = true;
  锁定按钮.disabled = true;
  输入框.value = '';
  上次阶段签名 = '';

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
      气泡('ai', `已发布到看板（任务 #${结果.任务id}），驱动器开始自主五层流转。`);
      设置过程('自主流转中');
      轮询起始 = Date.now();
      await 加载看板数据();
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

/** 轮询驱动器摘要：阶段完成/空闲/错误，同步刷新看板 */
async function 轮询() {
  if (!运行中) return;
  try {
    const 状态 = await fetch('/api/dev/pilot/status').then((响应) => 响应.json());
    if (状态.最近阶段) {
      const 阶段 = 状态.最近阶段;
      const 签名 = `${阶段.类型}|${阶段.任务id}|${阶段.新状态}|${阶段.消息 || ''}`;
      if (签名 !== 上次阶段签名) {
        上次阶段签名 = 签名;
        if (阶段.类型 === '空闲') {
          气泡('ai', '驱动器空闲：暂无待承接任务。可在看板发布，或在此继续下达需求。');
          设置过程('空闲');
        } else if (阶段.类型 === '阶段完成') {
          气泡('ai', `✅ 阶段完成：任务 #${阶段.任务id} 已由 ${阶段.角色 || '—'} 推进到「${阶段.新状态 || '—'}」`);
          设置过程('自主流转中');
        } else if (阶段.类型 === '错误') {
          气泡('ai', `⚠️ 驱动出错：${阶段.消息 || '未知错误'}（任务保持待承接，可重试）`);
        }
      }
    }
    await 加载看板数据();
    if (!状态.运行中 && 状态.最近阶段) {
      完成(状态.最近结果);
      return;
    }
  } catch (错误) {
    console.warn('驱动状态轮询失败（后端未就绪？）', 错误);
  }
  if (Date.now() - 轮询起始 > 轮询超时) {
    气泡('ai', '⏱ 驱动轮询超时，已停止观察（任务可能仍在后台流转）。');
    记日志('【超时】', 'warn', '驱动轮询超时');
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

/** 本轮驱动结束：刷新引擎数据、记日志、解锁 */
async function 完成(最近结果) {
  停止轮询();
  设置过程('本轮完成');
  记日志('【完成】', 'ok', 最近结果 || '驱动器本轮完成');
  await 加载引擎数据();
  渲染面板({ 运行中: false, 最近结果 });
  结束执行();
}

function 结束执行() {
  运行中 = false;
  if (锁定按钮) 锁定按钮.disabled = false;
}

function 气泡(角色, 文本) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ' + (角色 === 'user' ? 'user' : 'ai');
  行.innerHTML = `<div class="avatar ${角色 === 'user' ? 'u' : 'ai'}">${角色 === 'user' ? '我' : 'AI'}</div><div class="bubble"><span class="role">${角色 === 'user' ? '用户' : '驱动器'}</span>${转义(文本)}</div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 设置过程(文本) {
  if (!面板容器) return;
  const 流 = 面板容器.querySelector('#p-flow');
  if (流) 流.innerHTML = `<div class="kv"><b>当前动作</b>${转义(文本)}</div>`;
}

/** 渲染属性面板：看板驱动台状态 */
function 渲染面板(状态) {
  if (!面板容器) return;
  const 块 = 状态 || { 运行中, 最近结果: null };
  const 结果文案 = 块.最近结果 ? 转义(块.最近结果) : '—';
  面板容器.innerHTML = `<h3>驱动器状态</h3><div class="prop-group">
    <div class="prop-item"><span class="k">模式</span><span class="v">看板自主流转（需求→发布→五层）</span></div>
    <div class="prop-item"><span class="k">状态</span><span class="v">${块.运行中 ? '自主流转中' : '待命'}</span></div>
    <div class="prop-item"><span class="k">流转</span><span class="v">圣人设计 → 大罗金仙实现 → 准圣验收 → 道祖终审</span></div>
    <div class="prop-item"><span class="k">最近结果</span><span class="v">${结果文案}</span></div>
  </div><h3>执行过程</h3><div id="p-flow"><div class="kv"><b>当前动作</b>${块.运行中 ? '自主流转中' : '待命'}</div></div>
  ${块.运行中 ? '<div style="font-size:12px;color:#64748b;margin-top:6px;">驱动为单轮原子执行，不可中断；可在看板视图查看任务卡片状态。</div>' : ''}`;
}

/** HTML 转义，防止任务文本注入标记 */
function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
