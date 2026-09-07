// 模型选择.js —— 底部状态栏 LLM 模型选择浮层（供应商选择/切换/接入向导）
// 从对话视图.js 拆出：仅依赖「记日志」与 fetch，供状态栏浮层调用，与对话核心无耦合。

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';

const 模型图标 = `<svg viewBox="0 0 24 24"><path d="M12 2a2 2 0 0 1 2 2c0 .74-.4 1.39-1 1.73V7h1a7 7 0 0 1 7 7h1a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-1v1a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-1H2a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h1a7 7 0 0 1 7-7h1V5.73c-.6-.34-1-.99-1-1.73a2 2 0 0 1 2-2z"/></svg>`;

let 模型订阅者 = [];
let 当前模型信息 = { 供应商: null, 模型: null, 已配置: false };

/** 获取当前模型信息（供状态栏显示） */
export function 获取当前模型() {
  return { ...当前模型信息 };
}

/** 订阅模型切换（状态栏注册回调，切换时更新显示） */
export function 订阅模型切换(回调) {
  模型订阅者.push(回调);
}

function 通知模型切换() {
  for (const cb of 模型订阅者) {
    try { cb({ ...当前模型信息 }); } catch (e) { console.warn('模型订阅回调失败', e); }
  }
}

/** 挂载模型选择浮层到指定容器（状态栏点击时调用） */
export function 挂载模型浮层(容器) {
  容器.innerHTML = `
    <div class="mp-section">
      <h4>模型选择</h4>
      <div class="mp-row">
        <label>供应商</label>
        <select id="mp-provider"></select>
      </div>
      <div class="mp-row">
        <label>模型</label>
        <select id="mp-model"></select>
      </div>
      <div class="mp-hint" id="mp-hint">加载中…</div>
    </div>
    <div class="mp-section">
      <h4>接入新供应商</h4>
      <div class="mp-row">
        <label>模板</label>
        <select id="mp-tpl"></select>
        <button class="btn-sm" id="mp-tpl-fill">填入</button>
      </div>
      <div class="mp-row">
        <label>名称</label>
        <input id="mp-name" placeholder="供应商名称" />
      </div>
      <div class="mp-row">
        <label>官网</label>
        <input id="mp-base" placeholder="https://api.xxx.com/v1" />
      </div>
      <div class="mp-row">
        <label>密钥</label>
        <input id="mp-key" type="password" placeholder="密钥，或 env:变量名" />
      </div>
      <div class="mp-actions">
        <button class="btn-sm" id="mp-fetch">获取可用模型</button>
        <select id="mp-pick" style="flex:1;min-width:120px;"></select>
        <button class="btn-sm btn-primary" id="mp-connect">接入并选择</button>
      </div>
      <div class="mp-hint" id="mp-panel-hint"></div>
      <div id="mp-toml-wrap" style="display:none;">
        <div class="mp-toml-title">TOML 配置片段（可贴入 default.toml 永久配置）</div>
        <pre class="mp-toml" id="mp-toml"></pre>
      </div>
    </div>
  `;
  初始化浮层模型选择(容器);
  初始化浮层接入面板(容器);
}

/** 初始化浮层内的模型选择（供应商下拉→模型下拉→切换生效） */
async function 初始化浮层模型选择(容器) {
  const 供应商框 = 容器.querySelector('#mp-provider');
  const 模型框 = 容器.querySelector('#mp-model');
  const 提示 = 容器.querySelector('#mp-hint');
  if (!供应商框 || !模型框 || !提示) return;
  try {
    const 状态 = await fetch('/api/llm/status').then((响应) => 响应.json());
    if (!状态.配置 || !状态.供应商 || 状态.供应商.length === 0) {
      提示.textContent = '未配置 LLM 池（default.toml [llm] 或环境变量）';
      提示.className = 'mp-hint err';
      供应商框.disabled = true;
      模型框.disabled = true;
      当前模型信息 = { 供应商: null, 模型: null, 已配置: false };
      通知模型切换();
      return;
    }
    const 当前 = 状态.当前选择 || null;
    当前模型信息 = { 供应商: 当前?.供应商 || null, 模型: 当前?.模型 || null, 已配置: true };
    通知模型切换();
    for (const 供应商 of 状态.供应商) {
      const 选项 = document.createElement('option');
      选项.value = 供应商.名称;
      选项.textContent = `${供应商.名称}（默认 ${供应商.模型 || '—'}）`;
      供应商框.appendChild(选项);
    }
    if (当前 && 当前.供应商) 供应商框.value = 当前.供应商;
    供应商框.addEventListener('change', () => {
      提示.textContent = '';
      提示.className = 'mp-hint';
      刷新浮层模型列表(供应商框, 模型框, 提示, 当前);
    });
    模型框.addEventListener('change', () => 应用浮层模型选择(供应商框, 模型框, 提示));
    await 刷新浮层模型列表(供应商框, 模型框, 提示, 当前);
  } catch (错误) {
    console.warn('LLM 状态获取失败', 错误);
    提示.textContent = '无法获取 LLM 池状态（后端未就绪？）';
    提示.className = 'mp-hint err';
  }
}

/** 拉取当前供应商的可用模型列表并填充下拉 */
async function 刷新浮层模型列表(供应商框, 模型框, 提示, 当前) {
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
      提示.className = 'mp-hint ok';
    } else {
      提示.textContent = '模型列表不可用：' + (条目 ? 条目.错误 || '空列表' : '未知供应商');
      提示.className = 'mp-hint err';
    }
  } catch (错误) {
    console.warn('模型列表获取失败', 错误);
    提示.textContent = '模型列表拉取失败';
    提示.className = 'mp-hint err';
  }
}

/** 切换模型：POST /api/llm/select 运行时全局生效并落盘 */
async function 应用浮层模型选择(供应商框, 模型框, 提示) {
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
      提示.className = 'mp-hint ok';
      当前模型信息 = { 供应商, 模型, 已配置: true };
      通知模型切换();
      记日志('【模型】', 'act', `切换 LLM 选择：${供应商} / ${模型}`);
    } else {
      提示.textContent = '切换失败（非法供应商或模型？）';
      提示.className = 'mp-hint err';
    }
  } catch (错误) {
    console.warn('模型切换失败', 错误);
    提示.textContent = '切换失败（后端未就绪？）';
    提示.className = 'mp-hint err';
  }
}

/** 初始化浮层内的接入供应商面板（dsh「获取可用模型」式向导） */
async function 初始化浮层接入面板(容器) {
  const 模板框 = 容器.querySelector('#mp-tpl');
  const 名称框 = 容器.querySelector('#mp-name');
  const 地址框 = 容器.querySelector('#mp-base');
  const 密钥框 = 容器.querySelector('#mp-key');
  const 获取 = 容器.querySelector('#mp-fetch');
  const 选择框 = 容器.querySelector('#mp-pick');
  const 接入 = 容器.querySelector('#mp-connect');
  const 提示 = 容器.querySelector('#mp-panel-hint');
  const toml区 = 容器.querySelector('#mp-toml');
  const toml包裹 = 容器.querySelector('#mp-toml-wrap');
  if (!模板框 || !名称框 || !地址框 || !密钥框 || !获取 || !选择框 || !接入 || !提示 || !toml区) return;
  let 已获取模型 = [];
  let 探测来源 = '';
  // 模板下拉（内置目录，无密钥）
  try {
    const 响应 = await fetch('/api/llm/templates').then((r) => r.json());
    (响应.模板 || []).forEach((模板) => {
      const 选项 = document.createElement('option');
      选项.value = 模板.名称;
      选项.textContent = `${模板.显示名}（${模板.地址}）`;
      模板框.appendChild(选项);
    });
    sessionStorage.setItem('llm-模板缓存', JSON.stringify(响应.模板 || []));
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
      提示.className = 'mp-hint';
    }
  });
  // 获取可用模型：目录命中零网络；否则探测端点（密钥三级复用）
  获取.addEventListener('click', async () => {
    try {
      获取.disabled = true;
      提示.textContent = '获取模型中…';
      提示.className = 'mp-hint';
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
        提示.className = 'mp-hint err';
        return;
      }
      探测来源 = 结果.来源 || '';
      已获取模型 = 结果.模型 || [];
      if (已获取模型.length === 0) {
        提示.textContent = '未发现可用模型，可手输模型 ID 后接入';
        提示.className = 'mp-hint';
        return;
      }
      for (const 模型 of 已获取模型) {
        const 选项 = document.createElement('option');
        选项.value = 模型.id;
        选项.textContent = 模型.名称 ? `${模型.id}（${模型.名称}）` : 模型.id;
        选择框.appendChild(选项);
      }
      提示.textContent = `已从${探测来源 === '目录' ? '内置目录' : '端点'}获取 ${已获取模型.length} 个模型，选择后点击接入`;
      提示.className = 'mp-hint ok';
      if (!名称) 名称框.value = '自定义-' + (地址.match(/[a-z0-9]+/i) || ['svc'])[0];
      记日志('【模型】', 'act', `获取可用模型：${探测来源 === '目录' ? '目录' : 地址} 共 ${已获取模型.length} 个`);
    } catch (错误) {
      console.warn('获取可用模型失败', 错误);
      提示.textContent = '获取失败（后端未就绪？）';
      提示.className = 'mp-hint err';
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
      提示.className = 'mp-hint err';
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
        提示.className = 'mp-hint err';
        return;
      }
      const 结果 = await 响应.json();
      if (!响应.ok || !结果.成功) {
        提示.textContent = '接入失败：' + (结果.错误 || '后端拒绝');
        提示.className = 'mp-hint err';
        return;
      }
      提示.textContent = `已接入 ${结果.选择.供应商} / ${结果.选择.模型}。${密钥.startsWith('env:') ? 'env 引用已落盘，重启自动恢复。' : '明文密钥仅本次会话有效，重启后需重新接入。'}`;
      提示.className = 'mp-hint ok';
      if (结果.配置片段) {
        toml区.textContent = 结果.配置片段;
        toml包裹.style.display = 'block';
      }
      记日志('【模型】', 'act', `接入供应商：${结果.选择.供应商} / ${结果.选择.模型}`);
      // 更新当前模型信息并通知状态栏
      当前模型信息 = { 供应商: 结果.选择.供应商, 模型: 结果.选择.模型, 已配置: true };
      通知模型切换();
      // 刷新浮层内的供应商下拉（重新走初始化）
      const 供应商框 = 容器.querySelector('#mp-provider');
      const 模型框 = 容器.querySelector('#mp-model');
      const 顶部提示 = 容器.querySelector('#mp-hint');
      if (供应商框 && 模型框) {
        供应商框.innerHTML = '';
        模型框.innerHTML = '';
        初始化浮层模型选择(容器);
      }
    } catch (错误) {
      console.warn('接入失败', 错误);
      提示.textContent = '接入失败（后端未就绪？）';
      提示.className = 'mp-hint err';
    } finally {
      接入.disabled = false;
    }
  });
}
