// 图谱视图.js —— 项目模块图谱（节点可点击查看结构化解释）

import { 图谱节点, 图谱边 } from '../../../运行支撑-殿/数据服务-阁/图谱-数据-园/图谱数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><circle cx="5" cy="6" r="2.5"/><circle cx="19" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M7.5 6h9M7 7.5l3 8M17 7.5l-3 8"/></svg>`;

let 面板容器 = null;

export const 图谱视图 = {
  键: '图谱',
  标题: '项目图谱',
  副标题: '模块 · 符号 · 依赖关系 · 点击节点查看结构化解释',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>项目图谱</h2><p>模块 · 符号 · 依赖关系 · 点击节点查看结构化解释</p></div></div><div class="view-body"><div class="tupu-wrap"><svg viewBox="0 0 720 420" id="tupu-svg"></svg></div></div>`;
    渲染图谱(容器.querySelector('#tupu-svg'));
  },
  属性(容器) {
    面板容器 = 容器;
    容器.innerHTML = `<h3>图谱说明</h3><div class="prop-group"><div class="kv"><b>节点</b>点击任一模块节点，查看职责 / 符号 / 依赖 / 被依赖的结构化解释</div><div class="kv"><b>连线</b>表示依赖方向（依赖方 → 被依赖方）</div></div>`;
  },
};

function 渲染图谱(svg) {
  if (!svg) return;
  let 边html = '';
  图谱边.forEach(([起点, 终点]) => {
    const 甲 = 图谱节点.find((节点) => 节点.id === 起点);
    const 乙 = 图谱节点.find((节点) => 节点.id === 终点);
    if (!甲 || !乙) return;
    边html += `<line x1="${甲.x}" y1="${甲.y}" x2="${乙.x}" y2="${乙.y}" stroke="rgba(255,255,255,0.12)" stroke-width="1.2"/><circle cx="${(甲.x + 乙.x) / 2}" cy="${(甲.y + 乙.y) / 2}" r="2.4" fill="rgba(255,255,255,0.18)"/>`;
  });
  svg.innerHTML = 边html;
  图谱节点.forEach((节点) => {
    const 组 = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    组.innerHTML = `<circle cx="${节点.x}" cy="${节点.y}" r="27" fill="rgba(16,16,18,0.94)" stroke="${节点.色}" stroke-opacity="0.55" stroke-width="1.2"/><text x="${节点.x}" y="${节点.y-4}" text-anchor="middle" font-size="13" fill="#fff">${节点.名}</text><text x="${节点.x}" y="${节点.y+12}" text-anchor="middle" font-size="9" fill="#8a8f98">${节点.类}</text>`;
    组.style.cursor = 'pointer';
    组.addEventListener('click', () => 选中节点(节点));
    svg.appendChild(组);
  });
}

function 选中节点(节点) {
  if (!面板容器) return;
  面板容器.innerHTML = `<h3>模块详情</h3><div class="prop-group">
    <div class="prop-item"><span class="k">模块</span><span class="v" style="color:${节点.色}">${节点.名}</span></div>
    <div class="prop-item"><span class="k">类别</span><span class="v">${节点.类}</span></div>
    <div class="prop-item"><span class="k">所属</span><span class="v">${节点.层}</span></div>
    <div class="kv"><b>职责</b>${节点.职责}</div>
    <div class="kv"><b>符号</b>${节点.符号.join(' · ')}</div>
    <div class="kv"><b>依赖</b>${节点.依赖.join(' · ')}</div>
    <div class="kv"><b>被谁依赖</b>${节点.被依赖.length ? 节点.被依赖.join(' · ') : '—'}</div>
  </div>`;
}