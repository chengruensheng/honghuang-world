// 图谱视图.js —— 项目模块图谱（来自后端认知底座，空态显示占位）

import { 加载图谱 } from '../../../运行支撑-殿/数据服务-阁/图谱-数据-园/图谱数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><circle cx="5" cy="6" r="2.5"/><circle cx="19" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M7.5 6h9M7 7.5l3 8M17 7.5l-3 8"/></svg>`;

export const 图谱视图 = {
  键: '图谱',
  标题: '项目图谱',
  副标题: '模块 · 符号 · 依赖关系 · 来自认知底座',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>项目图谱</h2><p>模块 · 符号 · 依赖关系 · 来自认知底座</p></div></div><div class="view-body"><div class="tupu-wrap" id="tupu-body"></div></div>`;
    渲染图谱(容器.querySelector('#tupu-body'));
  },
  属性(容器) {
    容器.innerHTML = `<h3>图谱说明</h3><div class="prop-group"><div class="kv"><b>节点</b>认知底座图谱模块节点，点击查看职责 / 符号 / 依赖</div><div class="kv"><b>连线</b>表示依赖方向（依赖方 → 被依赖方）</div></div>`;
  },
};

async function 渲染图谱(容器) {
  if (!容器) return;
  const 图谱 = await 加载图谱();
  if (!图谱 || !图谱.模块集 || 图谱.模块集.length === 0) {
    容器.innerHTML = 空态();
    return;
  }
  容器.innerHTML = `<svg viewBox="0 0 720 420" id="tupu-svg"></svg>`;
  渲染节点(容器.querySelector('#tupu-svg'), 图谱);
}

function 空态() {
  return `<div class="kv" style="text-align:center;padding:48px 24px;"><b style="color:#8a8f98;">暂无数据</b><br>认知底座图谱尚未填充，等待未来构建项目真实模块结构</div>`;
}

function 渲染节点(svg, 图谱) {
  const 节点 = 图谱.模块集;
  const 坐标表 = {};
  const 数量 = Math.max(1, 节点.length);
  const 中心X = 360;
  const 中心Y = 210;
  const 半径X = 280;
  const 半径Y = 165;
  节点.forEach((模块, 下标) => {
    const 角度 = (下标 / 数量) * Math.PI * 2 - Math.PI / 2;
    坐标表[模块.名称] = {
      x: 中心X + 半径X * Math.cos(角度),
      y: 中心Y + 半径Y * Math.sin(角度),
    };
  });

  let 边html = '';
  (图谱.依赖集 || []).forEach((边) => {
    const 甲 = 坐标表[边.源];
    const 乙 = 坐标表[边.目标];
    if (!甲 || !乙) return;
    边html += `<line x1="${甲.x}" y1="${甲.y}" x2="${乙.x}" y2="${乙.y}" stroke="rgba(255,255,255,0.12)" stroke-width="1.2"/>`;
  });

  const 节点html = 节点.map((模块) => {
    const 位置 = 坐标表[模块.名称];
    return `<g style="cursor:pointer"><circle cx="${位置.x}" cy="${位置.y}" r="27" fill="rgba(16,16,18,0.94)" stroke="#6366f1" stroke-opacity="0.55" stroke-width="1.2"/><text x="${位置.x}" y="${位置.y-2}" text-anchor="middle" font-size="13" fill="#fff">${模块.名称}</text><text x="${位置.x}" y="${位置.y+13}" text-anchor="middle" font-size="9" fill="#8a8f98">${模块.路径}</text></g>`;
  }).join('');

  svg.innerHTML = 边html + 节点html;
}
