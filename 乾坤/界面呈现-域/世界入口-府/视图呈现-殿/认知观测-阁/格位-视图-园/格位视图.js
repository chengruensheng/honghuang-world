// 格位视图.js —— 格位地图 6×6（来自后端认知底座，渲染可信度/证据，格位可点）

import { 加载格位 } from '../../../运行支撑-殿/数据服务-阁/格位-数据-园/格位数据.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';

const 图标 = `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>`;

const 维度色 = {
  内部: '#4ade80',
  外在: '#60a5fa',
  规则: '#cbd5e1',
  执行: '#f87171',
  目标: '#fbbf24',
  经历: '#a855f7',
};

let 面板容器 = null;
let 当前地图 = null;
let 选中下标 = null;

export const 格位视图 = {
  键: '格位',
  标题: '格位地图',
  副标题: '6 维度 × 6 格位 = 36 · 来自认知底座',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>格位地图</h2><p>6 维度 × 6 格位 = 36 · 来自认知底座</p></div></div><div class="view-body"><div class="matrix" id="matrix"><div class="loading">格位加载中…</div></div></div>`;
    渲染矩阵(容器.querySelector('#matrix'));
  },
  属性(容器) {
    面板容器 = 容器;
    渲染属性();
  },
};

async function 渲染矩阵(容器) {
  if (!容器) return;
  const 地图 = await 加载格位();
  当前地图 = 地图;
  const 格位集 = 地图.格位集 || [];
  if (格位集.length === 0) {
    容器.innerHTML = 空态();
    return;
  }
  容器.innerHTML = 格位集.map((格位, 下标) => 格位卡(格位, 下标)).join('');
  容器.querySelectorAll('.cell').forEach((元素) => {
    元素.addEventListener('click', () => {
      选中下标 = Number(元素.dataset.下标);
      容器.querySelectorAll('.cell').forEach((c) => c.classList.toggle('on', Number(c.dataset.下标) === 选中下标));
      渲染属性();
    });
  });
  if (面板容器) 渲染属性();
}

function 空态() {
  return `<div class="kv" style="text-align:center;padding:48px 24px;"><b style="color:#8a8f98;">暂无数据</b><br>认知底座格位尚未构建，启动后自动扫描填充</div>`;
}

function 格位卡(格位, 下标) {
  const 色 = 维度色[格位.维度] || '#cbd5e1';
  const 摘要 = 格位.摘要 || '待补充';
  const 可信度 = 格位.可信度 || 0;
  const 证据数 = (格位.证据引用 || []).length;
  const 可信度色 = 可信度 >= 0.7 ? '#4ade80' : 可信度 >= 0.4 ? '#fbbf24' : '#52525b';
  return `<div class="cell" data-下标="${下标}" style="--c:${色};">
    <div class="slot">${格位.格位名}${可信度 > 0 ? `<span class="conf" style="color:${可信度色}">${Math.round(可信度 * 100)}%</span>` : ''}</div>
    <div class="dim">${格位.维度}</div>
    <div class="cell-summary">${转义(摘要)}</div>
    ${证据数 ? `<div class="cell-evidence">${证据数} 证据</div>` : ''}
  </div>`;
}

function 渲染属性() {
  if (!面板容器) return;
  const 地图 = 当前地图;
  if (!地图 || !地图.格位集 || 地图.格位集.length === 0) {
    渲染属性面板(面板容器, '格位说明', '', '', `<div class="prop-group"><div class="kv">认知底座尚未构建格位</div></div>`);
    return;
  }
  const 格位集 = 地图.格位集;
  if (选中下标 == null) {
    const 已填 = 格位集.filter((g) => g.摘要).length;
    const 有证据 = 格位集.filter((g) => (g.证据引用 || []).length).length;
    渲染属性面板(面板容器, '格位总览', `${格位集.length} 格位`, '', `<div class="prop-group">
      <div class="prop-item"><span class="k">总格位</span><span class="v">${格位集.length}</span></div>
      <div class="prop-item"><span class="k">已填充</span><span class="v">${已填}</span></div>
      <div class="prop-item"><span class="k">有证据</span><span class="v">${有证据}</span></div>
      <div class="kv"><b>操作</b>点击格位查看摘要 / 可信度 / 证据引用 / 目标内容</div>
    </div>`);
    return;
  }
  const 格位 = 格位集[选中下标];
  if (!格位) return;
  const 证据 = 格位.证据引用 || [];
  const 证据html = 证据.length
    ? 证据.map((e) => `<div class="kv" style="font-family:Consolas,monospace;font-size:11px">${转义(e)}</div>`).join('')
    : '<div class="kv">无证据引用</div>';
  const 载荷 = 格位.维度载荷 && Object.keys(格位.维度载荷).length ? JSON.stringify(格位.维度载荷, null, 2) : '';
  const 时间 = 格位.最后校验时间 ? new Date(格位.最后校验时间 * 1000).toLocaleString('zh-CN') : '—';
  渲染属性面板(面板容器, 格位.格位名, `${Math.round((格位.可信度 || 0) * 100)}%`, '', `<div class="prop-group">
    <div class="prop-item"><span class="k">维度</span><span class="v">${转义(格位.维度)}</span></div>
    <div class="prop-item"><span class="k">可信度</span><span class="v">${Math.round((格位.可信度 || 0) * 100)}%</span></div>
    <div class="prop-item"><span class="k">证据数</span><span class="v">${证据.length}</span></div>
    <div class="prop-item"><span class="k">校验时间</span><span class="v">${时间}</span></div>
    <div class="kv"><b>摘要</b>${转义(格位.摘要 || '待补充')}</div>
    <h3 style="margin-top:16px">证据引用</h3>${证据html}
    ${载荷 ? `<h3 style="margin-top:16px">维度载荷</h3><pre class="payload">${转义(载荷)}</pre>` : ''}
  </div>`);
}

function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
