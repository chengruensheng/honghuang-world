// 工作台视图.js —— 可观测工作台：跨任务驱动过程全局时间线
//
// 聚合：顶部三指标（任务/会话/事件）+ 会话时间线（按创建时间倒序，点击按需展开事件流）。
// 复用既有 `GET /api/dev/sessions` + `GET /api/dev/sessions/{id}`，零后端契约变更。
// 事件关联任务显示标题，可跳转看板定位，实现「过程可见 → 任务可追溯」闭环。

import { 会话清单, 会话回放, 看板存储, 选中任务 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';
import { 切换视图 } from '../../../运行支撑-殿/路由导航-阁/导航-核心-园/导航核心.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';
import { 事件图标, 事件角色 } from '../事件-呈现-园/事件呈现.js';

const 图标 = `<svg viewBox="0 0 24 24"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>`;
const 角色名 = { 道祖: '道祖', 圣人: '圣人', A: '大罗金仙', 准圣: '准圣', 太乙金仙: '太乙金仙' };

function 转义(文本) {
  return String(文本).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

export const 工作台视图 = {
  键: '工作台',
  标题: '可观测工作台',
  副标题: '跨任务驱动过程 · 全局时间线',
  分组: '工作台',
  图标,
  挂载(容器) {
    const 任务们 = 看板存储.取值().任务 || [];
    容器.innerHTML = `
      <div class="view-head">
        <div><h2>可观测工作台</h2><p>跨任务驱动过程 · 全局时间线</p></div>
      </div>
      <div class="view-body wb-body">
        <div class="wb-metrics">
          <div class="wb-metric"><div class="wb-num" id="wb-task-num">${任务们.length}</div><div class="wb-label">任务总数</div></div>
          <div class="wb-metric"><div class="wb-num" id="wb-session-num">—</div><div class="wb-label">驱动会话</div></div>
          <div class="wb-metric"><div class="wb-num" id="wb-event-num">按需</div><div class="wb-label">驱动事件</div></div>
        </div>
        <div class="wb-timeline" id="wb-timeline"><div class="wb-empty">加载中…</div></div>
      </div>
    `;
    this.容器 = 容器;
    this.渲染时间线(容器.querySelector('#wb-timeline'));
  },
  属性(容器) {
    渲染属性面板(容器, '可观测工作台', '', '', `<div class="prop-group"><div class="kv"><b>全局驱动过程</b>每条会话展示 💭思考/🔧工具调用/✅工具结果/📝任务答复 阶段节拍，关联任务可跳转看板定位</div></div>`);
  },
  卸载() {
    // 清理未完成的异步请求标记，避免卸载后继续操作 DOM
    this.已卸载 = true;
  },
  async 渲染时间线(盒) {
    if (!盒) return;
    try {
      const 会话们 = await this.加载会话();
      if (this.已卸载 || !盒.isConnected) return;
      const 会话数元素 = this.容器 && this.容器.querySelector('#wb-session-num');
      if (会话数元素) 会话数元素.textContent = 会话们.length;
      if (会话们.length === 0) {
        盒.innerHTML = '<div class="wb-empty">暂无驱动会话</div>';
        return;
      }
      盒.innerHTML = '';
      const 排序 = [...会话们].sort((a, b) => (b.创建时间 || 0) - (a.创建时间 || 0));
      for (const 会话 of 排序) 盒.appendChild(this.会话折叠项(会话));
    } catch (错误) {
      if (this.已卸载 || !盒.isConnected) return;
      盒.innerHTML = `<div class="wb-empty">加载失败：${转义(错误.message || '未知错误')}（后端未就绪？）</div>`;
    }
  },
  async 加载会话() {
    const 会话们 = await 会话清单().catch(() => []);
    return 会话们 || [];
  },
  会话折叠项(会话) {
    const 任务数 = (会话.任务id列表 && 会话.任务id列表.length) ? 会话.任务id列表.length : 0;
    const 项 = document.createElement('div');
    项.className = 'wb-session';
    项.dataset.sessionId = 会话.会话id;
    项.innerHTML = `
      <div class="wb-session-head">
        <span class="wb-sess-icon">🎬</span>
        <span class="wb-sess-title">会话 #${转义(会话.会话id)}</span>
        <span class="wb-sess-meta">${转义(会话.发起方式 || '驱动')} · ${转义(会话.状态 || '—')} · ${任务数} 任务 · ${转义(时间文本(会话.创建时间))}</span>
        <button class="wb-expand" title="展开该会话驱动事件流">▶ 展开</button>
      </div>
      <div class="wb-session-flow" data-flow hidden></div>
    `;
    项.querySelector('.wb-expand').addEventListener('click', (e) => {
      e.stopPropagation();
      this.展开会话(项, e.currentTarget, 会话.会话id);
    });
    return 项;
  },
  async 展开会话(项, 按钮, 会话id) {
    const 流 = 项.querySelector('[data-flow]');
    if (!流) return;
    按钮.disabled = true;
    按钮.textContent = '加载…';
    流.hidden = false;
    流.innerHTML = '<div class="wb-empty">加载中…</div>';
    try {
      const 详情 = await 会话回放(会话id);
      if (this.已卸载 || !流.isConnected) return;
      const 事件们 = 详情.事件 || [];
      按钮.remove();
      if (事件们.length === 0) {
        流.innerHTML = '<div class="wb-empty">该会话暂无过程事件</div>';
        return;
      }
      流.innerHTML = '';
      const 头 = document.createElement('div');
      头.className = 'wb-flow-head';
      头.textContent = `共 ${事件们.length} 条驱动事件`;
      流.appendChild(头);
      for (const 某个事件 of 事件们) 流.appendChild(this.事件行(某个事件));
      // 更新指标：事件总数
      const 事件数元素 = this.容器 && this.容器.querySelector('#wb-event-num');
      if (事件数元素 && 事件数元素.textContent === '按需') 事件数元素.textContent = 事件们.length;
      else if (事件数元素) 事件数元素.textContent = Number(事件数元素.textContent) + 事件们.length;
    } catch (错误) {
      if (this.已卸载 || !流.isConnected) return;
      流.innerHTML = `<div class="wb-empty">加载失败：${转义(错误.message || '未知错误')}（会话可能已清理）</div>`;
      按钮.disabled = false;
      按钮.textContent = '▶ 重试';
    }
  },
  事件行(事件) {
    const 图标 = 事件图标(事件.类型);
    const 角色 = 事件角色(事件.角色, 角色名);
    const 轮次 = 事件.轮次 != null ? `R${事件.轮次}` : '';
    const 工具 = 事件.工具名 ? ` <span class="wb-tool">${转义(事件.工具名)}</span>` : '';
    const 行 = document.createElement('div');
    行.className = 'wb-event';
    行.innerHTML = `
      <div class="wb-event-meta">${图标} ${转义(角色)} ${轮次} ${转义(事件.类型)}${工具}${this.任务标签(事件.任务id)}</div>
      ${事件.内容 ? `<div class="wb-event-content">${转义(事件.内容)}</div>` : ''}
    `;
    return 行;
  },
  任务标签(任务id) {
    const 任务们 = 看板存储.取值().任务 || [];
    const 任务 = 任务们.find((t) => t.id === 任务id);
    if (!任务) return 任务id != null ? ` <span class="wb-task">#${转义(任务id)}（已清理/取消）</span>` : '';
    return ` <span class="wb-task">${转义(任务.title || `#${任务id}`)} <a class="wb-jump" data-jump="${任务id}" title="跳转看板定位该任务">跳转看板 →</a></span>`;
  },
};

// 事件 → 看板跳转（委托，作用域限定在时间线容器）
document.addEventListener('click', (e) => {
  const 跳 = e.target.closest('[data-jump]');
  if (!跳) return;
  const id = Number(跳.dataset.jump);
  看板存储.更新({ 选中: id });
  选中任务(id);
  切换视图('看板');
});

function 时间文本(时间) {
  if (!时间) return '—';
  const s = String(时间);
  if (/^\d+$/.test(s) && s.length >= 10) return new Date(Number(s)).toLocaleString('zh-CN');
  return s;
}
