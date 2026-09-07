// 日志视图.js —— 运行日志（订阅日志共享存储，实时刷新）

import { 日志存储 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';

const 图标 = `<svg viewBox="0 0 24 24"><polyline points="4 17 10 11 14 15 20 7"/><path d="M14 7h6v6"/></svg>`;

export const 日志视图 = {
  键: '日志',
  标题: '运行日志',
  副标题: 'hm-agent · 实时流转记录',
  分组: '系统',
  图标,
  挂载(容器) {
    容器.innerHTML = `<div class="view-head"><div><h2>运行日志</h2><p>hm-agent · 实时流转记录</p></div></div><div class="view-body"><div class="log-body" id="log"></div></div>`;
    渲染日志(容器.querySelector('#log'));
    日志存储.订阅(() => 渲染日志(容器.querySelector('#log')));
  },
  属性(容器) {
    渲染属性面板(容器, '日志说明', '', '', `<div class="prop-group"><div class="kv"><b>相生闭环</b>每次任务触发一轮五行流转</div></div>`);
  },
};

function 渲染日志(容器) {
  if (!容器) return;
  容器.innerHTML = 日志存储.取值().记录.map((记录) =>
    `<div class="l"><span class="ts">${记录.时间}</span><span class="${记录.样式}">${记录.标签}</span> ${记录.内容}</div>`
  ).join('');
}