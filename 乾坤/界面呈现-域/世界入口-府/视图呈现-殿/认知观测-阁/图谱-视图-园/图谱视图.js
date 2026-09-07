// 图谱视图.js —— 项目模块图谱（来自后端认知底座：模块 / 符号 / 依赖 / 技术栈，节点可点）

import { 加载图谱 } from '../../../运行支撑-殿/数据服务-阁/图谱-数据-园/图谱数据.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';

const 图标 = `<svg viewBox="0 0 24 24"><circle cx="5" cy="6" r="2.5"/><circle cx="19" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M7.5 6h9M7 7.5l3 8M17 7.5l-3 8"/></svg>`;

let 面板容器 = null;
let 当前图谱 = null;
let 选中模块 = null;

export const 图谱视图 = {
  键: '图谱',
  标题: '项目图谱',
  副标题: '模块 · 符号 · 依赖 · 技术栈 · 来自认知底座',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>项目图谱</h2><p>模块 · 符号 · 依赖 · 技术栈 · 来自认知底座</p></div></div><div class="view-body"><div class="tupu-wrap" id="tupu-body"><div class="loading">图谱加载中…</div></div></div>`;
    渲染图谱(容器.querySelector('#tupu-body'));
  },
  属性(容器) {
    面板容器 = 容器;
    渲染属性();
  },
};

async function 渲染图谱(容器) {
  if (!容器) return;
  const 图谱 = await 加载图谱();
  当前图谱 = 图谱;
  const 模块 = 图谱.模块集 || [];
  if (模块.length === 0) {
    容器.innerHTML = 空态();
    return;
  }
  容器.innerHTML = `<svg viewBox="0 0 760 440" id="tupu-svg"></svg>`;
  渲染节点(容器.querySelector('#tupu-svg'), 图谱);
  if (面板容器) 渲染属性();
}

function 空态() {
  return `<div class="kv" style="text-align:center;padding:48px 24px;"><b style="color:#8a8f98;">暂无数据</b><br>认知底座尚未扫描出项目结构，启动后会自动构建</div>`;
}

function 按模块聚合符号(符号集) {
  const 映射 = {};
  for (const 符号 of 符号集) {
    const 模块 = 符号.所属模块 || '未知';
    if (!映射[模块]) 映射[模块] = [];
    映射[模块].push(符号);
  }
  return 映射;
}

function 渲染节点(svg, 图谱) {
  const 节点 = 图谱.模块集;
  const 符号按模块 = 按模块聚合符号(图谱.符号集 || []);
  const 坐标表 = {};
  const 数量 = Math.max(1, 节点.length);
  const 中心X = 380, 中心Y = 220, 半径X = 300, 半径Y = 180;
  节点.forEach((模块, 下标) => {
    const 角度 = (下标 / 数量) * Math.PI * 2 - Math.PI / 2;
    坐标表[模块.名称] = { x: 中心X + 半径X * Math.cos(角度), y: 中心Y + 半径Y * Math.sin(角度) };
  });

  const 选中 = 选中模块;
  let 边html = '';
  (图谱.依赖集 || []).forEach((边) => {
    const 甲 = 坐标表[边.源];
    const 乙 = 坐标表[边.目标];
    if (!甲 || !乙) return;
    const 相关 = 选中 && (边.源 === 选中 || 边.目标 === 选中);
    边html += `<line x1="${甲.x}" y1="${甲.y}" x2="${乙.x}" y2="${乙.y}" stroke="${相关 ? 'rgba(99,102,241,0.65)' : 'rgba(255,255,255,0.08)'}" stroke-width="${相关 ? 1.6 : 1}"/>`;
  });

  const 节点html = 节点.map((模块) => {
    const 位置 = 坐标表[模块.名称];
    const 符号数 = (符号按模块[模块.名称] || []).length;
    const 高亮 = 模块.名称 === 选中;
    return `<g style="cursor:pointer" data-模块="${模块.名称}"><circle cx="${位置.x}" cy="${位置.y}" r="26" fill="rgba(16,16,18,0.94)" stroke="${高亮 ? '#a5b4fc' : '#6366f1'}" stroke-opacity="${高亮 ? '1' : '0.5'}" stroke-width="${高亮 ? 2 : 1.2}"/><text x="${位置.x}" y="${位置.y-4}" text-anchor="middle" font-size="12" fill="#fff">${模块.名称}</text><text x="${位置.x}" y="${位置.y+9}" text-anchor="middle" font-size="8" fill="#8a8f98">${符号数 ? 符号数 + ' 符号' : ''}</text></g>`;
  }).join('');

  svg.innerHTML = 边html + 节点html;
  svg.querySelectorAll('g[data-模块]').forEach((元素) => {
    元素.addEventListener('click', () => {
      选中模块 = 元素.dataset.模块;
      重绘高亮(svg);
      渲染属性();
    });
  });
}

function 重绘高亮(svg) {
  svg.querySelectorAll('g[data-模块]').forEach((g) => {
    const 高亮 = g.dataset.模块 === 选中模块;
    const 圆 = g.querySelector('circle');
    圆.setAttribute('stroke', 高亮 ? '#a5b4fc' : '#6366f1');
    圆.setAttribute('stroke-opacity', 高亮 ? '1' : '0.5');
    圆.setAttribute('stroke-width', 高亮 ? '2' : '1.2');
  });
}

function 渲染属性() {
  if (!面板容器) return;
  const 图谱 = 当前图谱;
  if (!图谱 || !图谱.模块集 || 图谱.模块集.length === 0) {
    渲染属性面板(面板容器, '图谱说明', '', '', `<div class="prop-group"><div class="kv">认知底座尚未扫描出项目结构</div></div>`);
    return;
  }
  const 符号按模块 = 按模块聚合符号(图谱.符号集 || []);
  const 模块 = 图谱.模块集.find((m) => m.名称 === 选中模块);

  if (!模块) {
    const 技术栈 = (图谱.技术栈 || []).join(' · ');
    渲染属性面板(面板容器, '图谱总览', `${图谱.模块集.length} 模块`, '', `<div class="prop-group">
      <div class="prop-item"><span class="k">模块</span><span class="v">${图谱.模块集.length}</span></div>
      <div class="prop-item"><span class="k">符号</span><span class="v">${(图谱.符号集 || []).length}</span></div>
      <div class="prop-item"><span class="k">依赖</span><span class="v">${(图谱.依赖集 || []).length}</span></div>
      <div class="kv"><b>技术栈</b>${转义(技术栈 || '—')}</div>
      <div class="kv"><b>操作</b>点击节点查看该模块的符号与依赖</div>
    </div>`);
    return;
  }

  const 符号们 = 符号按模块[模块.名称] || [];
  const 依赖们 = (图谱.依赖集 || []).filter((边) => 边.源 === 模块.名称 || 边.目标 === 模块.名称);
  const 符号上限 = 30;
  const 符号html = 符号们.length
    ? 符号们.slice(0, 符号上限).map((符号) => `<div class="kv"><b>${转义(符号.名称)} · ${转义(符号.种类)}</b>${转义(符号.签名 || '')}</div>`).join('') + (符号们.length > 符号上限 ? `<div class="kv" style="color:#8a8f98">…等共 ${符号们.length} 个符号</div>` : '')
    : '<div class="kv">无符号</div>';
  const 依赖html = 依赖们.length
    ? 依赖们.map((边) => `<div class="kv" style="font-family:Consolas,monospace;font-size:11px">${转义(边.源)} → ${转义(边.目标)}</div>`).join('')
    : '<div class="kv">无依赖</div>';

  渲染属性面板(面板容器, 模块.名称, `${符号们.length} 符号`, '', `<div class="prop-group">
    <div class="kv"><b>路径</b>${转义(模块.路径 || '—')}</div>
    <div class="prop-item"><span class="k">符号</span><span class="v">${符号们.length}</span></div>
    <div class="prop-item"><span class="k">依赖</span><span class="v">${依赖们.length}</span></div>
    <h3 style="margin-top:16px">符号列表</h3>${符号html}
    <h3 style="margin-top:16px">依赖关系</h3>${依赖html}
  </div>`);
}

function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
