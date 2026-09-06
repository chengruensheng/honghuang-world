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
let 游标 = 0;

export const 对话视图 = {
  键: '对话',
  标题: '对话',
  副标题: '下达需求 → 发布进看板 → 驱动器自主五层流转',
  分组: '工作台',
  图标,
  挂载(容器) {
    主容器 = 容器;
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>下达需求，发布到看板由驱动器自主五层流转（圣人设计→大罗金仙实现→准圣验收→道祖终审）</p></div></div><div class="llm-bar" id="llm-bar" style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:8px 12px;margin-bottom:10px;border:1px solid #e4e3dd;border-radius:12px;background:#faf9f5;font-size:13px;"><span style="color:#6b7280;white-space:nowrap;">模型选择</span><select id="llm-provider" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;"></select><select id="llm-model" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;min-width:160px;"></select><span id="llm-hint" style="color:#6b7280;font-size:12px;"></span></div><div class="chat" id="chat"></div><div class="chat-input"><input id="task" placeholder="下达需求，如：在沙箱里写一个 fibonacci 模块…" /><button class="btn" id="go">下达</button></div>`;
    const 输入框 = 容器.querySelector('#task');
    锁定按钮 = 容器.querySelector('#go');
    锁定按钮.addEventListener('click', () => 下达(输入框));
    输入框.addEventListener('keydown', (事件) => {
      if (事件.key === 'Enter') 下达(输入框);
    });
    初始化模型选择();
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
  游标 = 0;

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

/** 初始化模型选择条：拉取池状态 → 供应商下拉 → 拉取模型列表 → 模型下拉 → 切换生效 */
async function 初始化模型选择() {
  if (!主容器) return;
  const 供应商框 = 主容器.querySelector('#llm-provider');
  const 模型框 = 主容器.querySelector('#llm-model');
  const 提示 = 主容器.querySelector('#llm-hint');
  if (!供应商框 || !模型框 || !提示) return;
  try {
    const 状态 = await fetch('/api/llm/status').then((响应) => 响应.json());
    if (!状态.配置 || !状态.供应商 || 状态.供应商.length === 0) {
      提示.textContent = '未配置 LLM 池（default.toml [llm] 或环境变量）';
      供应商框.disabled = true;
      模型框.disabled = true;
      return;
    }
    const 当前 = 状态.当前选择 || null;
    for (const 供应商 of 状态.供应商) {
      const 选项 = document.createElement('option');
      选项.value = 供应商.名称;
      选项.textContent = `${供应商.名称}（默认 ${供应商.模型 || '—'}）`;
      供应商框.appendChild(选项);
    }
    if (当前 && 当前.供应商) 供应商框.value = 当前.供应商;
    供应商框.addEventListener('change', () => {
      提示.textContent = '';
      刷新模型列表(供应商框, 模型框, 提示, 当前);
    });
    模型框.addEventListener('change', () => 应用模型选择(供应商框, 模型框, 提示));
    await 刷新模型列表(供应商框, 模型框, 提示, 当前);
  } catch (错误) {
    console.warn('LLM 状态获取失败', 错误);
    提示.textContent = '无法获取 LLM 池状态（后端未就绪？）';
  }
}

/** 拉取当前供应商的可用模型列表并填充下拉 */
async function 刷新模型列表(供应商框, 模型框, 提示, 当前) {
  模型框.innerHTML = '';
  const 选中 = 供应商框.value;
  if (!选中) return;
  try {
    const 响应 = await fetch('/api/llm/models').then((r) => r.json());
    const 条目 = (响应.结果 || []).find((r) => r.供应商 === 选中);
    if (条目 && 条目.模型 && 条目.模型.length > 0) {
      for (const 模型 of 条目.模型) {
        const 选项 = document.createElement('option');
        选项.value = 模型.id;
        选项.textContent = 模型.id;
        模型框.appendChild(选项);
      }
      if (当前 && 当前.供应商 === 选中 && 当前.模型) {
        const 已存在 = [...模型框.options].some((o) => o.value === 当前.模型);
        if (已存在) {
          模型框.value = 当前.模型;
        } else {
          const 补充 = document.createElement('option');
          补充.value = 当前.模型;
          补充.textContent = `${当前.模型}（当前）`;
          模型框.appendChild(补充);
          模型框.value = 当前.模型;
        }
      }
      提示.textContent = '已从 API 获取可用模型';
    } else {
      提示.textContent = '模型列表不可用：' + (条目 ? 条目.错误 || '空列表' : '未知供应商');
    }
  } catch (错误) {
    console.warn('模型列表获取失败', 错误);
    提示.textContent = '模型列表拉取失败';
  }
}

/** 切换模型：POST /api/llm/select 运行时全局生效并落盘 */
async function 应用模型选择(供应商框, 模型框, 提示) {
  const 供应商 = 供应商框.value;
  const 模型 = 模型框.value;
  if (!供应商 || !模型) return;
  try {
    const 响应 = await fetch('/api/llm/select', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 供应商, 模型 }),
    });
    if (响应.ok) {
      提示.textContent = `已切换：${供应商} / ${模型}`;
      记日志('【模型】', 'act', `切换 LLM 选择：${供应商} / ${模型}`);
    } else {
      提示.textContent = '切换失败（非法供应商或模型？）';
    }
  } catch (错误) {
    console.warn('模型切换失败', 错误);
    提示.textContent = '切换失败（后端未就绪？）';
  }
}
