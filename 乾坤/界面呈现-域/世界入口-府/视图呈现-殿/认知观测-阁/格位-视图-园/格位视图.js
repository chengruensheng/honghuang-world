// 格位视图.js —— 格位地图 6×6（来自后端认知底座，空态显示占位）

import { 加载格位 } from '../../../运行支撑-殿/数据服务-阁/格位-数据-园/格位数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>`;

const 维度色 = {
  内部: '#4ade80',
  外在: '#60a5fa',
  规则: '#cbd5e1',
  执行: '#f87171',
  目标: '#fbbf24',
  经历: '#a855f7',
};

export const 格位视图 = {
  键: '格位',
  标题: '格位地图',
  副标题: '6 维度 × 6 格位 = 36 · 来自认知底座',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>格位地图</h2><p>6 维度 × 6 格位 = 36 · 来自认知底座</p></div></div><div class="view-body"><div class="matrix" id="matrix"></div></div>`;
    渲染矩阵(容器.querySelector('#matrix'));
  },
  属性(容器) {
    容器.innerHTML = `<h3>格位说明</h3><div class="prop-group"><div class="kv"><b>36 格位 · 6 维度</b>认知底座心智地图，含摘要 / 可信度 / 证据引用 / 目标内容</div></div>`;
  },
};

async function 渲染矩阵(容器) {
  if (!容器) return;
  const 地图 = await 加载格位();
  if (!地图 || !地图.格位集 || 地图.格位集.length === 0) {
    容器.innerHTML = 空态();
    return;
  }
  容器.innerHTML = 地图.格位集.map(格位卡).join('');
}

function 空态() {
  return `<div class="kv" style="text-align:center;padding:48px 24px;"><b style="color:#8a8f98;">暂无数据</b><br>认知底座格位尚未填充，等待未来构建 36 格位心智地图</div>`;
}

function 格位卡(格位) {
  const 色 = 维度色[格位.维度] || '#cbd5e1';
  const 摘要 = 格位.摘要 || '待补充';
  return `<div class="cell" style="--c:${色};"><div class="slot">${格位.格位名}</div><div class="dim">${格位.维度} · ${摘要}</div></div>`;
}
