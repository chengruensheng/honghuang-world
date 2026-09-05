// 看板视图.js —— 任务看板（五层协作卡片视图，按状态分组展示 + 发布/承接/提交交互）

import { 看板存储, 加载看板数据, 发布任务, 承接任务, 提交任务, 选中任务 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>`;

const 状态分组 = [
  { 状态: '待受理', 色: '#94a3b8', 角色: null },
  { 状态: '待圣人设计', 色: '#4ade80', 角色: '圣人' },
  { 状态: '圣人设计中', 色: '#4ade80', 角色: '圣人' },
  { 状态: '待大罗金仙实现', 色: '#f87171', 角色: 'A' },
  { 状态: '大罗金仙实现中', 色: '#f87171', 角色: 'A' },
  { 状态: '待准圣验收', 色: '#fbbf24', 角色: '准圣' },
  { 状态: '准圣验收中', 色: '#fbbf24', 角色: '准圣' },
  { 状态: '待修复', 色: '#f97316', 角色: 'A' },
  { 状态: '待道祖终审', 色: '#cbd5e1', 角色: '道祖' },
  { 状态: '道祖终审中', 色: '#cbd5e1', 角色: '道祖' },
  { 状态: '已完成', 色: '#60a5fa', 角色: null },
  { 状态: '已取消', 色: '#64748b', 角色: null },
];

const 角色映射 = {
  '道祖': '道祖',
  '圣人': '圣人',
  'A': '大罗金仙',
  '准圣': '准圣',
};

const 流转选项 = {
  '待受理': [{ 状态: '待圣人设计', 角色: null }],
  '待圣人设计': [{ 状态: '圣人设计中', 角色: '圣人' }],
  '圣人设计中': [{ 状态: '待大罗金仙实现', 角色: '圣人' }],
  '待大罗金仙实现': [{ 状态: '大罗金仙实现中', 角色: 'A' }],
  '大罗金仙实现中': [{ 状态: '待准圣验收', 角色: 'A' }],
  '待准圣验收': [{ 状态: '准圣验收中', 角色: '准圣' }],
  '准圣验收中': [{ 状态: '待道祖终审', 角色: '准圣' }, { 状态: '待修复', 角色: '准圣' }],
  '待修复': [{ 状态: '大罗金仙实现中', 角色: 'A' }],
  '待道祖终审': [{ 状态: '道祖终审中', 角色: '道祖' }],
  '道祖终审中': [{ 状态: '已完成', 角色: '道祖' }, { 状态: '待修复', 角色: '道祖' }],
};

export const 看板视图 = {
  键: '看板',
  标题: '任务看板',
  副标题: '洪荒五层协作 · 状态流转',
  分组: '工作台',
  图标,
  挂载(容器) {
    容器.innerHTML = `
      <div class="view-head">
        <div><h2>任务看板</h2><p>洪荒五层协作 · 状态流转</p></div>
        <button class="btn-pub" id="btn-pub">+ 发布任务</button>
      </div>
      <div class="view-body">
        <div class="board" id="board"></div>
      </div>
      <div class="modal" id="modal" style="display:none">
        <div class="modal-box">
          <h3>发布任务</h3>
          <input id="pub-title" placeholder="任务标题" class="modal-input"/>
          <textarea id="pub-desc" placeholder="任务描述" class="modal-input" rows="3"></textarea>
          <select id="pub-scene" class="modal-input">
            <option value="">场景（默认理解）</option>
            <option value="理解">理解</option>
            <option value="设计">设计</option>
            <option value="修改">修改</option>
            <option value="调试">调试</option>
            <option value="重构">重构</option>
          </select>
          <select id="pub-priority" class="modal-input">
            <option value="">优先级（默认P2）</option>
            <option value="P0">P0</option>
            <option value="P1">P1</option>
            <option value="P2">P2</option>
            <option value="P3">P3</option>
          </select>
          <div class="modal-actions">
            <button id="pub-cancel" class="btn-cancel">取消</button>
            <button id="pub-ok" class="btn-ok">发布</button>
          </div>
        </div>
      </div>
    `;
    渲染看板(容器.querySelector('#board'));
    看板存储.订阅(() => 渲染看板(容器.querySelector('#board')));

    const modal = 容器.querySelector('#modal');
    容器.querySelector('#btn-pub').addEventListener('click', () => { modal.style.display = 'flex'; });
    容器.querySelector('#pub-cancel').addEventListener('click', () => { modal.style.display = 'none'; });
    容器.querySelector('#pub-ok').addEventListener('click', async () => {
      const title = 容器.querySelector('#pub-title').value.trim();
      const desc = 容器.querySelector('#pub-desc').value.trim();
      const scene = 容器.querySelector('#pub-scene').value;
      const priority = 容器.querySelector('#pub-priority').value;
      if (!title) return;
      try {
        await 发布任务(title, desc, scene, priority);
        modal.style.display = 'none';
        容器.querySelector('#pub-title').value = '';
        容器.querySelector('#pub-desc').value = '';
      } catch (e) { alert(e.message); }
    });
  },
  属性(容器) {
    const 状态 = 看板存储.取值();
    const 任务 = 状态.任务.find((t) => t.id === 状态.选中);
    if (!任务) {
      容器.innerHTML = `<h3>任务详情</h3><div class="prop-group"><div class="kv">点击卡片查看详情</div></div>`;
      return;
    }
    容器.innerHTML = `
      <h3>${任务.title}</h3>
      <div class="prop-group">
        <div class="kv"><b>ID</b>${任务.id}</div>
        <div class="kv"><b>状态</b>${任务.status}</div>
        <div class="kv"><b>场景</b>${任务.场景 || '—'}</div>
        <div class="kv"><b>优先级</b>${任务.优先级 || '—'}</div>
        <div class="kv"><b>发起人</b>${角色映射[任务.发起人] || 任务.发起人 || '—'}</div>
        <div class="kv"><b>当前承接人</b>${角色映射[任务.当前承接人] || 任务.当前承接人 || '—'}</div>
        <div class="kv"><b>修复轮次</b>${任务.修复轮次 || 0}</div>
        <div class="kv"><b>描述</b>${任务.description || '—'}</div>
      </div>
      ${任务.承接历史 && 任务.承接历史.length ? `<div class="prop-group"><b>承接历史</b>${任务.承接历史.map((r) => 角色映射[r] || r).join(' → ')}</div>` : ''}
      ${任务.状态历史 && 任务.状态历史.length ? `<div class="prop-group"><b>状态历史</b>${任务.状态历史.map((h) => `${h.原状态}→${h.新状态}`).join(' · ')}</div>` : ''}
    `;
  },
};

function 渲染看板(容器) {
  if (!容器) return;
  const 状态 = 看板存储.取值();
  const 任务列表 = 状态.任务;
  let html = '';
  for (const 分组 of 状态分组) {
    const 任务们 = 任务列表.filter((t) => t.status === 分组.状态);
    if (任务们.length === 0 && !['待受理', '待圣人设计', '待大罗金仙实现', '待准圣验收', '待道祖终审', '已完成'].includes(分组.状态)) continue;
    html += `<div class="board-col" style="border-top:3px solid ${分组.色}">`;
    html += `<div class="board-col-head"><span class="board-col-title">${分组.状态}</span><span class="board-col-count">${任务们.length}</span></div>`;
    for (const 任务 of 任务们) {
      const 承接角色 = 分组.角色 ? 角色映射[分组.角色] : null;
      const 流转 = 流转选项[任务.status] || [];
      html += `<div class="board-card" data-id="${任务.id}">`;
      html += `<div class="board-card-title">${任务.title}</div>`;
      html += `<div class="board-card-meta">`;
      if (任务.优先级) html += `<span class="tag" style="background:${优先级色(任务.优先级)}">${任务.优先级}</span>`;
      if (任务.修复轮次) html += `<span class="tag" style="background:#f97316">修复${任务.修复轮次}</span>`;
      if (任务.当前承接人) html += `<span class="tag" style="background:#334155">${角色映射[任务.当前承接人] || 任务.当前承接人}</span>`;
      html += `</div>`;
      if (承接角色) {
        html += `<button class="btn-accept" data-id="${任务.id}" data-role="${承接角色}">承接</button>`;
      }
      for (const 流 of 流转) {
        const 流转角色 = 流.角色 ? 角色映射[流.角色] : null;
        html += `<button class="btn-submit" data-id="${任务.id}" data-role="${流转角色 || ''}" data-next="${流.状态}">→ ${流.状态}</button>`;
      }
      html += `</div>`;
    }
    html += `</div>`;
  }
  容器.innerHTML = html;

  容器.querySelectorAll('.board-card').forEach((卡片) => {
    卡片.addEventListener('click', (e) => {
      if (e.target.tagName === 'BUTTON') return;
      选中任务(Number(卡片.dataset.id));
    });
  });

  容器.querySelectorAll('.btn-accept').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      const role = 按钮.dataset.role;
      try { await 承接任务(id, role); } catch (err) { alert(err.message); }
    });
  });

  容器.querySelectorAll('.btn-submit').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      const role = 按钮.dataset.role;
      const next = 按钮.dataset.next;
      try { await 提交任务(id, role, next); } catch (err) { alert(err.message); }
    });
  });
}

function 优先级色(p) {
  if (p === 'P0') return '#ef4444';
  if (p === 'P1') return '#f97316';
  if (p === 'P2') return '#4ade80';
  return '#94a3b8';
}