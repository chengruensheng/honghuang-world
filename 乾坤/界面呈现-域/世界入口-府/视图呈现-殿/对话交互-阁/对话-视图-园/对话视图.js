// 对话视图.js —— 道祖接待（主控澄清：闲聊/识别任务/澄清细节 → 对齐 → 确认发布 → 看板自主流转）

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 加载引擎数据 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';
import { 加载看板数据 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';
import { 道祖对话, 确认发布 } from '../../../运行支撑-殿/数据服务-阁/对话-数据-园/对话数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;

// 轮询间隔与超时上限（毫秒）
const 轮询间隔 = 1000;
const 轮询超时 = 15 * 60 * 1000;

let 主容器 = null;
let 面板容器 = null;
let 轮询器 = null;
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
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>向道祖下达需求，澄清对齐后确认发布，看板由五层协作自主流转</p></div></div><div class="llm-bar" id="llm-bar" style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:8px 12px;margin-bottom:10px;border:1px solid #e4e3dd;border-radius:12px;background:#faf9f5;font-size:13px;"><span style="color:#6b7280;white-space:nowrap;">模型选择</span><select id="llm-provider" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;"></select><select id="llm-model" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;min-width:160px;"></select><span id="llm-hint" style="color:#6b7280;font-size:12px;"></span><button class="btn" id="llm-add" style="margin-left:auto;font-size:12px;padding:3px 10px;">＋ 接入供应商</button></div><div id="llm-panel" style="display:none;margin-bottom:10px;padding:12px;border:1px solid #e4e3dd;border-radius:12px;background:#faf9f5;font-size:13px;flex-direction:column;gap:8px;"><div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;"><span style="color:#6b7280;">模板</span><select id="llm-tpl" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;"></select><button class="btn" id="llm-tpl-fill" style="font-size:12px;padding:3px 10px;">填入模板</button></div><div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;"><span style="color:#6b7280;">名称</span><input id="llm-name" placeholder="供应商名称" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;font-size:13px;width:120px;" /><span style="color:#6b7280;">官网</span><input id="llm-base" placeholder="https://api.xxx.com/v1" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;font-size:13px;flex:1;min-width:220px;" /><span style="color:#6b7280;">密钥</span><input id="llm-key" type="password" placeholder="密钥，或 env:变量名" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;font-size:13px;flex:1;min-width:180px;" /></div><div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;"><button class="btn" id="llm-fetch" style="font-size:12px;padding:3px 10px;">获取可用模型</button><select id="llm-pick" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;min-width:200px;"></select><button class="btn" id="llm-connect" style="font-size:12px;padding:3px 10px;">接入并选择</button></div><div id="llm-panel-hint" style="color:#6b7280;font-size:12px;"></div><pre id="llm-toml" style="display:none;margin:0;padding:8px;background:#f0eee8;border-radius:8px;font-size:12px;overflow:auto;white-space:pre-wrap;"></pre></div><div class="chat" id="chat"></div><div id="confirm-bar" class="confirm-bar" style="display:none"></div><div class="chat-input"><input id="task" placeholder="向道祖下达需求，如：写一个 fibonacci 模块…" /><button class="btn" id="go">发送</button></div>`;
    const 输入框 = 容器.querySelector('#task');
    锁定按钮 = 容器.querySelector('#go');
    锁定按钮.addEventListener('click', () => 发送消息(输入框));
    输入框.addEventListener('keydown', (事件) => {
      if (事件.key === 'Enter') 发送消息(输入框);
    });
    初始化模型选择();
    初始化接入面板();
    欢迎气泡();
  },
  属性(容器) {
    面板容器 = 容器;
    渲染面板();
  },
   卸载() {
     停止轮询();
     运行中 = false;
     对话中 = false;
     待确认需求 = null;
     主容器 = null;
     面板容器 = null;
   },
};

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
    await 加载看板数据();
    启动轮询();
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

/** 渲染属性面板：道祖接待 + 看板驱动台状态 */
function 渲染面板(状态) {
  if (!面板容器) return;
  const 块 = 状态 || { 运行中, 最近结果: null };
  const 结果文案 = 块.最近结果 ? 转义(块.最近结果) : '—';
  面板容器.innerHTML = `<h3>道祖接待状态</h3><div class="prop-group">
    <div class="prop-item"><span class="k">模式</span><span class="v">道祖接待 → 澄清对齐 → 确认发布 → 五层流转</span></div>
    <div class="prop-item"><span class="k">状态</span><span class="v">${块.运行中 ? '自主流转中' : '待命'}</span></div>
    <div class="prop-item"><span class="k">流转</span><span class="v">圣人设计 → 大罗金仙实现 → 准圣验收 → 道祖终审 → 太乙金仙清理</span></div>
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

/** 初始化接入面板：模板 → 填表单 → 获取可用模型 → 接入并选择（dsh「获取可用模型」式向导） */
async function 初始化接入面板() {
  const 面板 = 主容器.querySelector('#llm-panel');
  const 按钮 = 主容器.querySelector('#llm-add');
  const 模板框 = 主容器.querySelector('#llm-tpl');
  const 名称框 = 主容器.querySelector('#llm-name');
  const 地址框 = 主容器.querySelector('#llm-base');
  const 密钥框 = 主容器.querySelector('#llm-key');
  const 获取 = 主容器.querySelector('#llm-fetch');
  const 选择框 = 主容器.querySelector('#llm-pick');
  const 接入 = 主容器.querySelector('#llm-connect');
  const 提示 = 主容器.querySelector('#llm-panel-hint');
  const toml区 = 主容器.querySelector('#llm-toml');
  if (!面板 || !按钮 || !模板框 || !名称框 || !地址框 || !密钥框 || !获取 || !选择框 || !接入 || !提示 || !toml区) return;
  let 已获取模型 = [];
  let 探测来源 = '';
  按钮.addEventListener('click', () => {
    面板.style.display = 面板.style.display === 'none' ? 'flex' : 'none';
  });
  // 模板下拉（内置目录，无密钥）
  try {
    const 响应 = await fetch('/api/llm/templates').then((r) => r.json());
    (响应.模板 || []).forEach((模板) => {
      const 选项 = document.createElement('option');
      选项.value = 模板.名称;
      选项.textContent = `${模板.显示名}（${模板.地址}）`;
      模板框.appendChild(选项);
    });
  } catch (错误) {
    console.warn('模板拉取失败', 错误);
  }
  // 选模板 → 自动填名称/地址 + 提示 env 变量
  模板框.addEventListener('change', () => {
    const 名 = 模板框.value;
    if (!名) return;
    const 模板 = (JSON.parse(sessionStorage.getItem('llm-模板缓存') || '[]')).find((t) => t.名称 === 名);
    if (模板) {
      名称框.value = 模板.名称;
      地址框.value = 模板.地址;
      密钥框.placeholder = 模板.环境变量 ? `密钥，或 env:${模板.环境变量}` : '本地服务无需密钥，留空探测';
      提示.textContent = 模板.环境变量 ? `模板已填入：密钥建议放 .env 的 ${模板.环境变量}，表单填 env:${模板.环境变量}` : 'Ollama 本地服务：密钥留空即可探测';
    }
  });
  // 缓存模板清单（change 时用）
  fetch('/api/llm/templates').then((r) => r.json()).then((响应) => {
    sessionStorage.setItem('llm-模板缓存', JSON.stringify(响应.模板 || []));
  }).catch(() => {});
  // 获取可用模型：目录命中零网络；否则探测端点（密钥三级复用）
  获取.addEventListener('click', async () => {
    try {
      获取.disabled = true;
      提示.textContent = '获取模型中…';
      选择框.innerHTML = '';
      已获取模型 = [];
      const 名称 = 名称框.value.trim();
      const 地址 = 地址框.value.trim();
      const 密钥 = 密钥框.value.trim();
      const 响应 = await fetch('/api/llm/discover', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 供应商: 名称 || undefined, 地址: 地址 || undefined, 密钥: 密钥 || undefined }),
      });
      const 结果 = await 响应.json();
      if (结果.错误) {
        提示.textContent = `获取失败：${结果.错误}`;
        return;
      }
      探测来源 = 结果.来源 || '';
      已获取模型 = 结果.模型 || [];
      if (已获取模型.length === 0) {
        提示.textContent = '未发现可用模型，可手输模型 ID 后接入';
        return;
      }
      for (const 模型 of 已获取模型) {
        const 选项 = document.createElement('option');
        选项.value = 模型.id;
        选项.textContent = 模型.名称 ? `${模型.id}（${模型.名称}）` : 模型.id;
        选择框.appendChild(选项);
      }
      提示.textContent = `已从${探测来源 === '目录' ? '内置目录' : '端点'}获取 ${已获取模型.length} 个模型，选择后点击接入`;
      if (!名称) 名称框.value = '自定义-' + (地址.match(/[a-z0-9]+/i) || ['svc'])[0];
      记日志('【模型】', 'act', `获取可用模型：${探测来源 === '目录' ? '目录' : 地址} 共 ${已获取模型.length} 个`);
    } catch (错误) {
      console.warn('获取可用模型失败', 错误);
      提示.textContent = '获取失败（后端未就绪？）';
    } finally {
      获取.disabled = false;
    }
  });
  // 接入并选择：注册进池 + 全局选中 + env 引用落盘
  接入.addEventListener('click', async () => {
    const 名称 = 名称框.value.trim();
    const 地址 = 地址框.value.trim();
    const 密钥 = 密钥框.value.trim();
    const 模型 = 选择框.value || 选择框.options[0]?.value || '';
    if (!名称 || !地址 || !模型) {
      提示.textContent = '请先填写名称/官网并获取模型';
      return;
    }
    try {
      接入.disabled = true;
      const 响应 = await fetch('/api/llm/connect', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 名称, 地址, 密钥, 模型 }),
      });
      if (响应.status === 404) {
        提示.textContent = '后端 LLM 池未装配（需 default.toml [llm] 配置或环境密钥），接入不可用';
        return;
      }
      const 结果 = await 响应.json();
      if (!响应.ok || !结果.成功) {
        提示.textContent = '接入失败：' + (结果.错误 || '后端拒绝');
        return;
      }
      提示.textContent = `已接入 ${结果.选择.供应商} / ${结果.选择.模型}。${密钥.startsWith('env:') ? 'env 引用已落盘，重启自动恢复。' : '明文密钥仅本次会话有效，重启后需重新接入。'}可把下方 TOML 贴入 default.toml 永久配置。`;
      if (结果.配置片段) {
        toml区.textContent = 结果.配置片段;
        toml区.style.display = 'block';
      }
      记日志('【模型】', 'act', `接入供应商：${结果.选择.供应商} / ${结果.选择.模型}`);
      // 刷新 顶部 供应商下拉（重新走 初始化模型选择）
      const 条 = 主容器.querySelector('#llm-bar');
      if (条) 条.innerHTML = '<span style="color:#6b7280;white-space:nowrap;">模型选择</span><select id="llm-provider" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;"></select><select id="llm-model" style="padding:4px 8px;border-radius:8px;border:1px solid #d8d5cc;background:#fff;font-size:13px;min-width:160px;"></select><span id="llm-hint" style="color:#6b7280;font-size:12px;"></span><button class="btn" id="llm-add" style="margin-left:auto;font-size:12px;padding:3px 10px;">＋ 接入供应商</button>';
      初始化模型选择();
      初始化接入面板();
    } catch (错误) {
      console.warn('接入失败', 错误);
      提示.textContent = '接入失败（后端未就绪？）';
    } finally {
      接入.disabled = false;
    }
  });
}
