// 格位视图.js —— 格位地图 6×6（格位可点击查看内部字段）

import { 维度表, 维度色, 可信度表, 格位详情, 目标内容表 } from '../../../运行支撑-殿/数据服务-阁/格位-数据-园/格位数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>`;

let 面板容器 = null;
let 矩阵容器 = null;

export const 格位视图 = {
  键: '格位',
  标题: '格位地图',
  副标题: '6 维度 × 6 格位 = 36',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>格位地图</h2><p>6 维度 × 6 格位 = 36</p></div></div><div class="view-body"><div class="matrix" id="matrix"></div></div>`;
    矩阵容器 = 容器.querySelector('#matrix');
    渲染矩阵();
  },
  属性(容器) {
    面板容器 = 容器;
    容器.innerHTML = `<h3>格位详情</h3><div class="prop-group"><div class="kv"><b>36 格位 · 6 维度</b>点击左侧任一格位，查看该格位的内部字段（摘要 / 可信度 / 证据引用 / 校验时间）。目标维度额外展示初心 / 现况 / 愿景 / 偏移。</div></div>`;
  },
};

function 渲染矩阵() {
  if (!矩阵容器) return;
  矩阵容器.innerHTML = '';
  Object.entries(维度表).forEach(([维度名, 格位列表]) => {
    格位列表.forEach((格位名) => {
      const 元素 = document.createElement('div');
      元素.className = 'cell';
      元素.style.setProperty('--c', 维度色[维度名]);
      元素.innerHTML = `<div class="slot">${格位名}</div><div class="dim">${维度名}</div>`;
      元素.addEventListener('click', () => 选中格位(维度名, 格位名, 元素));
      矩阵容器.appendChild(元素);
    });
  });
}

function 选中格位(维度名, 格位名, 元素) {
  if (矩阵容器) {
    矩阵容器.querySelectorAll('.cell').forEach((格) => 格.classList.remove('on'));
  }
  元素.classList.add('on');
  if (面板容器) {
    面板容器.innerHTML = 格位详情模板(维度名, 格位名);
  }
}

function 格位详情模板(维度名, 格位名) {
  let html = `<h3>格位详情</h3><div class="prop-group">
    <div class="prop-item"><span class="k">维度</span><span class="v" style="color:${维度色[维度名]}">${维度名}</span></div>
    <div class="prop-item"><span class="k">格位</span><span class="v">${格位名}</span></div>
    <div class="prop-item"><span class="k">可信度</span><span class="v">${可信度表[维度名]}</span></div>
    <div class="kv"><b>摘要</b>${格位详情[维度名][格位名] || '待补充'}</div>
    <div class="kv"><b>证据引用</b>${维度名}维 · 维护文档 v1.22</div>
    <div class="kv"><b>最后校验</b>2026-09-05 10:29</div>
  </div>`;
  if (维度名 === '目标') {
    const 目标 = 目标内容表[格位名] || ['', '', '', ''];
    html += `<h3>目标内容</h3><div class="prop-group">
      <div class="kv"><b>初心</b>${目标[0]}</div>
      <div class="kv"><b>现况</b>${目标[1]}</div>
      <div class="kv"><b>愿景</b>${目标[2]}</div>
      <div class="kv"><b>偏移</b>${目标[3]}</div>
    </div>`;
  }
  return html;
}