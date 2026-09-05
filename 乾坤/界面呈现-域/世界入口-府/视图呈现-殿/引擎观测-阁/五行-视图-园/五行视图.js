// 五行视图.js —— 五行引擎观测（相生环 + 引擎卡片）

import { 引擎存储 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3"/><path d="M12 3v6M12 15v6M3 12h6M15 12h6"/></svg>`;

const 顶点 = [
  { x: 160, y: 20 },
  { x: 293, y: 117 },
  { x: 242, y: 264 },
  { x: 78, y: 264 },
  { x: 27, y: 117 },
];

let 已订阅 = false;
let 面板容器 = null;

export const 五行视图 = {
  键: '五行',
  标题: '五行引擎',
  副标题: '太初 → 量劫 → 乾坤 → 道韵 → 混沌 · 相生闭环',
  分组: '观测',
  图标,
  挂载(容器) {
    容器.innerHTML = 主模板();
    渲染环(容器.querySelector('#ring-nodes'));
    渲染卡片(容器.querySelector('#engines'));
    if (!已订阅) {
      已订阅 = true;
      引擎存储.订阅(() => {
        const 卡片容器 = 容器.querySelector('#engines');
        渲染卡片(卡片容器);
      });
    }
  },
  属性(容器) {
    面板容器 = 容器;
    渲染属性面板();
  },
};

function 主模板() {
  return `<div class="view-head"><div><h2>五行引擎</h2><p>太初 → 量劫 → 乾坤 → 道韵 → 混沌 · 相生闭环</p></div></div>
    <div class="view-body"><div class="ring-wrap">
      <div class="ring">
        <svg viewBox="0 0 320 320">
          <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6366f1" stop-opacity="0.5"/><stop offset="1" stop-color="#a855f7" stop-opacity="0.5"/></linearGradient></defs>
          <circle cx="160" cy="160" r="140" fill="none" stroke="url(#g)" stroke-width="1" opacity="0.35"/>
          <polygon points="160,20 293,117 242,264 78,264 27,117" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="1"/>
          <g id="ring-nodes"></g>
        </svg>
        <div class="core"><b style="font-size:14px;">传承殿</b><br><i style="font-style:normal;font-size:11px;color:#8a8f98;">五行相生 · 闭合</i></div>
      </div>
      <div class="engines" id="engines"></div>
    </div></div>`;
}

function 渲染环(容器) {
  if (!容器) return;
  引擎存储.取值().列表.forEach((引擎, 下标) => {
    const 位置 = 顶点[下标];
    const 元素 = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    元素.innerHTML = `<circle cx="${位置.x}" cy="${位置.y}" r="21" fill="rgba(16,16,18,0.92)" stroke="${引擎.色}" stroke-opacity="0.55" stroke-width="1"/><circle cx="${位置.x}" cy="${位置.y}" r="25" fill="none" stroke="${引擎.色}" stroke-opacity="0.12" stroke-width="1"><animate attributeName="r" values="25;30;25" dur="3s" repeatCount="indefinite"/></circle><text x="${位置.x}" y="${位置.y-2}" text-anchor="middle" font-size="14" fill="#fff">${引擎.行}</text><text x="${位置.x}" y="${位置.y+36}" text-anchor="middle" font-size="9" fill="#8a8f98">${引擎.名}</text>`;
    容器.appendChild(元素);
  });
}

function 渲染卡片(容器) {
  if (!容器) return;
  容器.innerHTML = 引擎存储.取值().列表.map((引擎) => `
    <div class="engine" style="--c:${引擎.色};" data-名="${引擎.名}">
      <div class="head"><div><div class="nm">${引擎.名} · ${引擎.事}</div><div class="el">${引擎.行} · ${引擎.说明}</div></div><span class="badge">${引擎.行}</span></div>
      <div class="metric"><span class="num">${引擎.数值}</span><span class="unit">${引擎.单位}</span></div>
      <div class="bar"><i style="width:${引擎.占比}"></i></div>
      <div class="foot"><span>${引擎.副}</span></div>
    </div>`).join('');
  容器.querySelectorAll('.engine').forEach((元素) => {
    元素.style.cursor = 'pointer';
    元素.addEventListener('click', () => {
      const 名 = 元素.dataset.名;
      const 引擎 = 引擎存储.取值().列表.find((项) => 项.名 === 名);
      if (引擎) 选中引擎(引擎);
    });
  });
}

function 渲染属性面板() {
  if (!面板容器) return;
  const 列表 = 引擎存储.取值().列表;
  面板容器.innerHTML = `<h3>引擎列表</h3><div class="prop-group">${列表.map(详情卡).join('')}</div>`;
  面板容器.querySelectorAll('.kv').forEach((元素) => {
    元素.addEventListener('click', () => {
      const 名 = 元素.dataset.名;
      const 引擎 = 列表.find((项) => 项.名 === 名);
      if (引擎) 选中引擎(引擎);
    });
  });
}

function 详情卡(引擎) {
  return `<div class="kv" style="cursor:pointer" data-名="${引擎.名}"><b style="color:${引擎.色}">${引擎.名} · ${引擎.事}</b>${引擎.行} · ${引擎.说明} · ${引擎.副}</div>`;
}

function 选中引擎(引擎) {
  if (!面板容器) return;
  面板容器.innerHTML = 引擎详情模板(引擎);
}

function 引擎详情模板(引擎) {
  return `<h3>引擎详情</h3><div class="prop-group">
    <div class="prop-item"><span class="k">引擎</span><span class="v" style="color:${引擎.色}">${引擎.名}</span></div>
    <div class="prop-item"><span class="k">五行</span><span class="v">${引擎.行}</span></div>
    <div class="prop-item"><span class="k">职责</span><span class="v">${引擎.事}</span></div>
    <div class="prop-item"><span class="k">当前数值</span><span class="v">${引擎.数值} ${引擎.单位}</span></div>
    <div class="kv"><b>职责说明</b>${引擎.职责}</div>
    <div class="kv"><b>契约接口</b>${引擎.契约.join(' · ')}</div>
    <div class="kv"><b>相生（生谁）</b>${引擎.相生}</div>
    <div class="kv"><b>被生（谁生它）</b>${引擎.被生}</div>
  </div>`;
}