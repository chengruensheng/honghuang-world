// 面板组件.js —— 右栏属性面板（统一折叠框架 + 委托当前视图渲染属性）

import { 订阅切换 } from '../../../运行支撑-殿/路由导航-阁/导航-核心-园/导航核心.js';

let 面板折叠 = true;

/** 挂载属性面板，随视图切换渲染对应视图的属性内容（切换视图时重置为收起） */
export function 挂载面板(容器) {
  订阅切换((视图) => {
    面板折叠 = true;
    if (视图 && typeof 视图.属性 === 'function') {
      视图.属性(容器);
    } else {
      容器.innerHTML = '';
    }
  });
}

/**
 * 渲染可折叠属性面板：标题栏（点击展开/收起）+ 内容区，默认收起。
 * 各视图的「属性」方法调用本函数，统一折叠体验；标题/徽标在此统一转义防注入。
 */
export function 渲染属性面板(容器, 标题, 徽标 = '', 徽标类 = '', 内容Html) {
  容器.innerHTML = `
    <button class="prop-toggle" id="prop-toggle" type="button">
      <span class="prop-chevron" id="prop-chevron">${面板折叠 ? '▸' : '▾'}</span>
      <span class="prop-title">${转义(标题)}</span>
      ${徽标 ? `<span class="prop-mini${徽标类 ? ' ' + 徽标类 : ''}">${转义(徽标)}</span>` : ''}
    </button>
    <div class="prop-content" id="prop-content" style="display:${面板折叠 ? 'none' : 'block'}">
      ${内容Html}
    </div>`;
  容器.classList.toggle('collapsed', 面板折叠);
  const 切换 = 容器.querySelector('#prop-toggle');
  if (切换) {
    切换.addEventListener('click', () => {
      面板折叠 = !面板折叠;
      应用折叠状态(容器);
    });
  }
}

function 应用折叠状态(容器) {
  const 内容 = 容器.querySelector('#prop-content');
  const 箭头 = 容器.querySelector('#prop-chevron');
  if (内容) 内容.style.display = 面板折叠 ? 'none' : 'block';
  if (箭头) 箭头.textContent = 面板折叠 ? '▸' : '▾';
  容器.classList.toggle('collapsed', 面板折叠);
}

/** HTML 转义，防止标题/徽标注入标记 */
function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
