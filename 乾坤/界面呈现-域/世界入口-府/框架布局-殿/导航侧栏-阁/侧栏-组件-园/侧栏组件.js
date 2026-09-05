// 侧栏组件.js —— 左栏导航（依赖导航核心，按视图的「分组」字段自动聚合）

import { 全部视图, 切换视图, 当前视图键, 订阅切换 } from '../../../运行支撑-殿/路由导航-阁/导航-核心-园/导航核心.js';

/** 挂载侧边栏到容器，并随视图切换更新高亮 */
export function 挂载侧栏(容器) {
  渲染(容器);
  订阅切换(() => 更新高亮(容器));
}

function 渲染(容器) {
  let html = `<div class="logo"><span class="mark"></span><div>洪荒·世界<div class="sub">自主构建系统</div></div></div>`;
  const 分组映射 = 聚合分组(全部视图());
  for (const [组名, 列表] of Object.entries(分组映射)) {
    html += `<div class="group">${组名}</div><div class="nav">`;
    for (const 视图 of 列表) {
      html += `<div class="nav-item" data-v="${视图.键}">${视图.图标}${视图.标题}</div>`;
    }
    html += `</div>`;
  }
  html += `<div class="aside-foot"><span class="dot"></span>智能体运行中 · 初版</div>`;
  容器.innerHTML = html;
  容器.querySelectorAll('.nav-item').forEach((元素) => {
    元素.addEventListener('click', () => 切换视图(元素.dataset.v));
  });
}

function 聚合分组(视图们) {
  const 映射 = {};
  for (const 视图 of 视图们) {
    if (!映射[视图.分组]) 映射[视图.分组] = [];
    映射[视图.分组].push(视图);
  }
  return 映射;
}

function 更新高亮(容器) {
  const 键 = 当前视图键();
  容器.querySelectorAll('.nav-item').forEach((元素) => {
    元素.classList.toggle('active', 元素.dataset.v === 键);
  });
}