// 对话视图.js —— 道祖接待（主控澄清：闲聊/识别任务/澄清细节 → 对齐 → 确认发布 → 看板自主流转）
// 模型选择逻辑已拆至 模型选择-园/模型选择.js（供底部状态栏浮层调用）

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 加载引擎数据 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';
import { 加载看板数据 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';
import { 道祖对话, 确认发布 } from '../../../运行支撑-殿/数据服务-阁/对话-数据-园/对话数据.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';
import { 事件图标, 事件类型类 } from '../../看板观测-阁/事件-呈现-园/事件呈现.js';

const 图标 = `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;

// 轮询间隔与超时上限（毫秒）
const 轮询间隔 = 1000;
const 轮询超时 = 15 * 60 * 1000;

let 主容器 = null;
let 面板容器 = null;
let 轮询器 = null;
let 过程轮询器 = null;
let 过程游标 = 0;
let 过程面板打开 = false;
let 历史面板打开 = false;
let 历史已加载 = false;
let 运行中 = false;
let 对话中 = false;
let 待确认需求 = null;
let 轮询起始 = 0;
let 锁定按钮 = null;
let 确认按钮 = null;
let 游标 = 0;

export const 对话视图 = {
  键: '对话',
  标题: '对话',
  副标题: '道祖接待 → 澄清对齐 → 确认发布 → 看板自主流转',
  分组: '工作台',
  图标,
  挂载(容器) {
    主容器 = 容器;
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>向道祖下达需求，澄清对齐后确认发布，看板由五层协作自主流转</p></div></div><div class="chat" id="chat"></div><div id="confirm-bar" class="confirm-bar" style="display:none"></div><div class="chat-input"><input id="task" placeholder="向道祖下达需求，如：写一个 fibonacci 模块…" /><button class="btn" id="go">发送</button></div><div class="process-panel" id="process-panel"><div class="process-header" id="process-header"><span class="process-title">实时过程</span><span class="process-status" id="process-status">待命</span><span class="process-count" id="process-count"></span><span class="process-chevron">▸</span></div><div class="process-stream" id="process-stream" style="display:none"></div><div class="history-header" id="history-header"><span class="process-title">历史会话</span><span class="history-hint">回放任一次驱动过程</span><span class="history-chevron">▸</span></div><div class="history-body" id="history-body" style="display:none"><div id="history-list"></div><div id="history-replay"></div></div></div>`;
    const 输入框 = 容器.querySelector('#task');
    锁定按钮 = 容器.querySelector('#go');
    锁定按钮.addEventListener('click', () => 发送消息(输入框));
    输入框.addEventListener('keydown', (事件) => {
      if (事件.key === 'Enter') 发送消息(输入框);
    });
    欢迎气泡();
    // 过程面板点击展开/折叠
    const 过程头 = 容器.querySelector('#process-header');
    if (过程头) {
      过程头.addEventListener('click', () => {
        过程面板打开 = !过程面板打开;
        const 流 = 容器.querySelector('#process-stream');
        const 箭头 = 容器.querySelector('.process-chevron');
        if (流) 流.style.display = 过程面板打开 ? 'block' : 'none';
        if (箭头) 箭头.textContent = 过程面板打开 ? '▾' : '▸';
      });
    }
    // 历史会话面板：点击展开/折叠，首次展开时加载会话清单
    const 历史头 = 容器.querySelector('#history-header');
    if (历史头) {
      历史头.addEventListener('click', () => {
        历史面板打开 = !历史面板打开;
        const 体 = 容器.querySelector('#history-body');
        const 箭头 = 容器.querySelector('.history-chevron');
        if (体) 体.style.display = 历史面板打开 ? 'block' : 'none';
        if (箭头) 箭头.textContent = 历史面板打开 ? '▾' : '▸';
        if (历史面板打开 && !历史已加载) {
          历史已加载 = true;
          加载会话列表();
        }
      });
    }
  },
  属性(容器) {
    面板容器 = 容器;
    渲染面板();
  },
   卸载() {
     停止轮询();
     停止过程轮询();
     运行中 = false;
     对话中 = false;
     待确认需求 = null;
     主容器 = null;
     面板容器 = null;
   },
};

// ============ 对话核心逻辑 ============

/** 欢迎语：道祖接待开场 */
function 欢迎气泡() {
  气泡('道祖', '贫道道祖，主理接待与任务澄清。施主有何需求，尽管道来；需求清晰后，贫道自会给出对齐摘要，供施主确认发布。');
}

/** 发送消息给道祖：闲聊/识别任务/澄清细节 → 对齐给出摘要 */
async function 发送消息(输入框) {
  if (运行中 || 对话中) return;
  const 文案 = 输入框.value.trim();
  if (!文案) return;
  对话中 = true;
  锁定按钮.disabled = true;
  输入框.value = '';

  气泡('用户', 文案);
  记日志('【对话】', 'act', 文案);
  设置过程('道祖思量中…');

  try {
    const 结果 = await 道祖对话(文案);
    气泡('道祖', 结果.回复 || '（道祖未置一词）');
    if (结果.阶段 === '待确认' && 结果.需求) {
      待确认需求 = 结果.需求;
      显示确认栏(结果.需求);
      设置过程('待确认：请确认发布或继续补充');
    } else {
      设置过程('接待中');
    }
  } catch (错误) {
    气泡('道祖', `⛔ ${错误.message || '道祖未响应，请确认后端与 LLM 已就绪。'}`);
    设置过程('待命');
    记日志('【受阻】', 'warn', 错误.message || '道祖未响应');
  } finally {
    对话中 = false;
    锁定按钮.disabled = false;
  }
}

/** 显示确认栏：展示对齐需求摘要 + 确认发布/继续补充 */
function 显示确认栏(需求) {
  if (!主容器) return;
  const 栏 = 主容器.querySelector('#confirm-bar');
  if (!栏) return;
  栏.style.display = 'block';
  const 场景 = 需求.场景 ? `<div class="confirm-meta">场景：${转义(需求.场景)}</div>` : '';
  const 优先级 = 需求.优先级 ? `<div class="confirm-meta">优先级：${转义(需求.优先级)}</div>` : '';
  栏.innerHTML = `<div class="confirm-box">
    <div class="confirm-title">已对齐需求：${转义(需求.标题)}</div>
    <div class="confirm-desc">${转义(需求.描述)}</div>${场景}${优先级}
    <div class="confirm-actions">
      <button class="btn-ok" id="confirm-ok">确认发布</button>
      <button class="btn-cancel" id="confirm-cancel">继续补充</button>
    </div>
  </div>`;
  确认按钮 = 栏.querySelector('#confirm-ok');
  栏.querySelector('#confirm-ok').addEventListener('click', () => 确认发布任务());
  栏.querySelector('#confirm-cancel').addEventListener('click', () => {
    待确认需求 = null;
    栏.style.display = 'none';
    气泡('道祖', '请继续补充需求细节，贫道再与你对齐。');
    设置过程('接待中');
  });
}

/** 确认发布：落看板 + 写记忆 + 自动驱动，进入轮询 */
async function 确认发布任务() {
  if (运行中) return;
  if (!待确认需求) return;
  运行中 = true;
  if (确认按钮) 确认按钮.disabled = true;
  设置过程('发布中…');

  try {
    const 结果 = await 确认发布();
    气泡('道祖', `已发布到看板（任务 #${结果.任务id}），五层协作开始自主流转。`);
    待确认需求 = null;
    if (主容器) {
      const 栏 = 主容器.querySelector('#confirm-bar');
      if (栏) 栏.style.display = 'none';
    }
    设置过程('自主流转中');
    轮询起始 = Date.now();
    游标 = 0;
    过程游标 = 0;
    清空过程流();
    await 加载看板数据();
    启动轮询();
    启动过程轮询();
  } catch (错误) {
    气泡('道祖', `⛔ ${错误.message || '发布失败'}`);
    设置过程('待确认');
    记日志('【受阻】', 'warn', 错误.message || '发布失败');
    运行中 = false;
    if (确认按钮) 确认按钮.disabled = false;
  }
}

/** 轮询驱动事件流：游标增量拉取，逐条渲染阶段完成/空闲/错误，同步刷新看板 */
async function 轮询() {
  if (!运行中) return;
  try {
    const 状态 = await fetch(`/api/dev/pilot/events?since=${游标}`).then((响应) => 响应.json());
    if (状态.事件 && 状态.事件.length > 0) {
      for (const 事件 of 状态.事件) {
        渲染事件(事件);
        游标 = 事件.序号;
      }
    }
    await 加载看板数据();
    if (!状态.运行中 && 状态.最近结果) {
      完成(状态.最近结果);
      return;
    }
  } catch (错误) {
    console.warn('驱动事件轮询失败（后端未就绪？）', 错误);
  }
  if (Date.now() - 轮询起始 > 轮询超时) {
    气泡('ai', '⏱ 驱动轮询超时，已停止观察（任务可能仍在后台流转）。');
    记日志('【超时】', 'warn', '驱动轮询超时');
    结束执行();
    return;
  }
  轮询器 = setTimeout(轮询, 轮询间隔);
}

/** 渲染一条驱动阶段事件 */
function 渲染事件(事件) {
  if (事件.类型 === '空闲') {
    气泡('ai', '驱动器空闲：暂无待承接任务。可在看板发布，或在此继续下达需求。');
    设置过程('空闲');
  } else if (事件.类型 === '阶段完成') {
    气泡('ai', `✅ 阶段完成：任务 #${事件.任务id} 已由 ${事件.角色 || '—'} 推进到「${事件.新状态 || '—'}」`);
    设置过程('自主流转中');
  } else if (事件.类型 === '错误') {
    气泡('ai', `⚠️ 驱动出错：${事件.消息 || '未知错误'}（任务保持待承接，可重试）`);
  }
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
  停止过程轮询();
  更新过程状态('已完成');
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
  const 是用户 = 角色 === '用户' || 角色 === 'user';
  const 头像 = 是用户 ? '我' : (角色 === '道祖' ? '道' : 'AI');
  const 名称 = 是用户 ? '用户' : (角色 === '道祖' ? '道祖' : '驱动器');
  const 行 = document.createElement('div');
  行.className = 'msg-row ' + (是用户 ? 'user' : 'ai');
  行.innerHTML = `<div class="avatar ${是用户 ? 'u' : 'ai'}">${头像}</div><div class="bubble"><span class="role">${名称}</span>${转义(文本)}</div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 设置过程(文本) {
  if (!面板容器) return;
  const 流 = 面板容器.querySelector('#p-flow');
  if (流) 流.innerHTML = `<div class="kv"><b>当前动作</b>${转义(文本)}</div>`;
}

/** 渲染属性面板：道祖接待 + 看板驱动台状态（复用统一折叠框架） */
function 渲染面板(状态) {
  if (!面板容器) return;
  const 块 = 状态 || { 运行中, 最近结果: null };
  const 结果文案 = 块.最近结果 ? 转义(块.最近结果) : '—';
  const 状态文案 = 块.运行中 ? '自主流转中' : '待命';
  渲染属性面板(面板容器, '道祖接待状态', 状态文案, 块.运行中 ? 'running' : '', `
    <div class="prop-group">
      <div class="prop-item"><span class="k">模式</span><span class="v">道祖接待 → 澄清对齐 → 确认发布 → 五层流转</span></div>
      <div class="prop-item"><span class="k">状态</span><span class="v">${状态文案}</span></div>
      <div class="prop-item"><span class="k">流转</span><span class="v">圣人设计 → 大罗金仙实现 → 准圣验收 → 道祖终审 → 太乙金仙清理</span></div>
      <div class="prop-item"><span class="k">最近结果</span><span class="v">${结果文案}</span></div>
    </div>
    <h3>执行过程</h3><div id="p-flow"><div class="kv"><b>当前动作</b>${状态文案}</div></div>
    ${块.运行中 ? '<div style="font-size:12px;color:#64748b;margin-top:6px;">驱动为单轮原子执行，不可中断；可在看板视图查看任务卡片状态。</div>' : ''}
  `);
}

/** HTML 转义，防止任务文本注入标记 */
function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

// ============ 实时过程流（智能体循环每步事件） ============

function 启动过程轮询() {
  停止过程轮询();
  更新过程状态('运行中');
  过程轮询器 = setTimeout(过程轮询, 800);
}

function 停止过程轮询() {
  if (过程轮询器) {
    clearTimeout(过程轮询器);
    过程轮询器 = null;
  }
}

function 清空过程流() {
  if (!主容器) return;
  const 流 = 主容器.querySelector('#process-stream');
  if (流) 流.innerHTML = '';
  const 计数 = 主容器.querySelector('#process-count');
  if (计数) 计数.textContent = '';
}

function 更新过程状态(文本) {
  if (!主容器) return;
  const 状态 = 主容器.querySelector('#process-status');
  if (状态) {
    状态.textContent = 文本;
    状态.className = 'process-status ' + (文本 === '运行中' ? 'running' : 文本 === '已完成' ? 'done' : '');
  }
}

async function 过程轮询() {
  if (!运行中) return;
  try {
    const 响应 = await fetch(`/api/dev/pilot/process?since=${过程游标}`);
    const 数据 = await 响应.json();
    if (数据.运行中 === false && 数据.事件 && 数据.事件.length === 0) {
      // 驱动器已空闲且无新事件，停止轮询
      更新过程状态('已完成');
      return;
    }
    if (数据.事件 && 数据.事件.length > 0) {
      for (const 事件 of 数据.事件) {
        渲染过程事件(事件);
        过程游标 = 事件.序号;
      }
    }
    更新过程状态(数据.运行中 ? '运行中' : '已完成');
  } catch (错误) {
    // 静默重试，不干扰主轮询
  }
  过程轮询器 = setTimeout(过程轮询, 800);
}

function 渲染过程事件(事件) {
  if (!主容器) return;
  const 流 = 主容器.querySelector('#process-stream');
  if (!流) return;
  渲染事件行(流, 事件);
  流.scrollTop = 流.scrollHeight;
  const 计数 = 主容器.querySelector('#process-count');
  if (计数) 计数.textContent = `${流.children.length} 条`;
}

function 渲染事件行(容器, 事件) {
  const 类型类 = 事件类型类(事件.类型);
  const 类型图标 = 事件图标(事件.类型);
  const 角色 = 事件.角色 || '?';
  const 轮次 = 事件.轮次 != null ? `R${事件.轮次}` : '';
  const 工具名 = 事件.工具名 ? ` <span class="proc-tool">${转义(事件.工具名)}</span>` : '';
  const 内容 = 事件.内容 ? `<div class="proc-content">${转义(事件.内容)}</div>` : '';
  const 项 = document.createElement('div');
  项.className = `proc-item ${类型类}`;
  项.innerHTML = `<div class="proc-meta"><span class="proc-icon">${类型图标}</span><span class="proc-role">${转义(角色)}</span><span class="proc-round">${轮次}</span><span class="proc-type">${转义(事件.类型)}</span>${工具名}</div>${内容}`;
  容器.appendChild(项);
}

// ============ 历史会话回放 ============

async function 加载会话列表() {
  if (!主容器) return;
  try {
    const 响应 = await fetch('/api/dev/sessions');
    const 数据 = await 响应.json();
    const 列表 = 主容器.querySelector('#history-list');
    if (!列表) return;
    if (!数据.会话 || 数据.会话.length === 0) {
      列表.innerHTML = '<div class="history-empty">暂无历史会话（完成一次驱动后出现）</div>';
      return;
    }
    列表.innerHTML = '';
    for (const 会话 of 数据.会话) 列表.appendChild(渲染会话项(会话));
  } catch (错误) {
    // 静默：后端未就绪时不打扰
  }
}

function 渲染会话项(会话) {
  const 行 = document.createElement('div');
  行.className = 'session-item';
  const 时间 = new Date(会话.创建时间 * 1000).toLocaleString('zh-CN', { hour12: false });
  const 结果 = 会话.结果摘要 ? 转义(会话.结果摘要) : '—';
  const 任务id = (会话.任务id列表 && 会话.任务id列表[0]) || null;
  const 可操作 = 任务id !== null;
  行.innerHTML = `
    <div class="session-meta"><b>#${会话.会话id}</b><span>${转义(会话.发起方式 || '驱动')}</span><span>${时间}</span><span class="session-status">${转义(会话.状态 || '—')}</span><span class="session-count">${会话.事件数} 条</span></div>
    <div class="session-result">${结果}</div>
    <div class="session-actions">
      <button class="btn btn-mini session-act" data-act="resume" ${可操作 ? '' : 'disabled'} title="从该会话断点恢复继续（延续 LLM 上下文）">恢复</button>
      <button class="btn btn-mini session-act" data-act="fork" ${可操作 ? '' : 'disabled'} title="从该会话分叉一条新的驱动线">分叉</button>
    </div>`;
  行.addEventListener('click', () => 回放会话(会话.会话id));
  行.querySelectorAll('.session-act').forEach((按钮) => {
    按钮.addEventListener('click', async (事件) => {
      事件.stopPropagation();
      if (!任务id) return;
      const 动作 = 按钮.dataset.act;
      try {
        const 响应 = await fetch(`/api/dev/sessions/${会话.会话id}/${动作}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ 任务id }),
        });
        if (!响应.ok) {
          const 错误 = await 响应.json().catch(() => null);
          alert(动作 === 'resume' ? '恢复失败：' + ((错误 && 错误.错误) || '未知') : '分叉失败：' + ((错误 && 错误.错误) || '未知'));
          return;
        }
        alert(动作 === 'resume' ? '已发起恢复驱动，正在延续上下文…' : '已发起分叉，新驱动线开始…');
        加载会话列表();
      } catch (错误) {
        alert('请求失败，请稍后重试');
      }
    });
  });
  return 行;
}

async function 回放会话(会话id) {
  if (!主容器) return;
  try {
    const 响应 = await fetch(`/api/dev/sessions/${会话id}`);
    if (!响应.ok) return;
    const 详情 = await 响应.json();
    const 回放容器 = 主容器.querySelector('#history-replay');
    if (!回放容器) return;
    const 列表 = 主容器.querySelector('#history-list');
    if (列表) 列表.querySelectorAll('.session-item').forEach((项) => 项.classList.remove('active'));
    回放容器.innerHTML = '';
    const 摘要头 = document.createElement('div');
    摘要头.className = 'replay-head';
    摘要头.textContent = `会话 #${详情.会话id} · ${详情.发起方式 || '驱动'} · ${详情.状态 || '—'} · ${(详情.事件 || []).length} 条`;
    回放容器.appendChild(摘要头);
    if (!详情.事件 || 详情.事件.length === 0) {
      回放容器.innerHTML += '<div class="history-empty">该会话暂无过程记录</div>';
      return;
    }
    const 事件容器 = document.createElement('div');
    事件容器.className = 'replay-stream';
    回放容器.appendChild(事件容器);
    for (const 事件 of 详情.事件) 渲染事件行(事件容器, 事件);
    事件容器.scrollTop = 事件容器.scrollHeight;
  } catch (错误) {
    // 静默
  }
}
