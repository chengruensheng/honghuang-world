// 看板视图.js —— 任务看板（五层协作卡片视图，按状态分组展示 + 发布/承接/提交交互）

import { 看板存储, 加载看板数据, 发布任务, 承接任务, 提交任务, 清理任务, 定向回退, 扫尾任务, 澄清推进, 选中任务, 驱动一轮, 驱动到空闲, 驱动状态 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';
import { 渲染属性面板 } from '../../../框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';
import { 打开验收弹层 } from '../验收弹层-视图-园/验收弹层.js';

let 面板容器 = null;

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
  { 状态: '待清理', 色: '#22d3ee', 角色: '太乙金仙' },
  { 状态: '清理中', 色: '#22d3ee', 角色: '太乙金仙' },
  { 状态: '清理完成', 色: '#60a5fa', 角色: null },
  { 状态: '待道祖澄清', 色: '#a78bfa', 角色: '道祖' },
  { 状态: '道祖澄清中', 色: '#a78bfa', 角色: '道祖' },
  { 状态: '待重新设计', 色: '#4ade80', 角色: '圣人', 召回: true },
  { 状态: '待重新实现', 色: '#f87171', 角色: 'A', 召回: true },
  { 状态: '待重新验收', 色: '#fbbf24', 角色: '准圣', 召回: true },
  { 状态: '待重新清理', 色: '#22d3ee', 角色: '太乙金仙', 召回: true },
  { 状态: '已完成', 色: '#60a5fa', 角色: null },
  { 状态: '已取消', 色: '#64748b', 角色: null },
];

const 角色映射 = {
  '道祖': '道祖',
  '圣人': '圣人',
  'A': '大罗金仙',
  '准圣': '准圣',
  '太乙金仙': '太乙金仙',
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
  '道祖终审中': [{ 状态: '待清理', 角色: '道祖' }, { 状态: '待修复', 角色: '道祖' }],
  '待清理': [{ 状态: '清理中', 角色: '太乙金仙' }],
  '清理中': [{ 状态: '清理完成', 角色: '太乙金仙' }],
  '待道祖澄清': [{ 状态: '道祖澄清中', 角色: '道祖' }],
  '道祖澄清中': [{ 状态: '待圣人设计', 角色: '道祖' }, { 状态: '已取消', 角色: '道祖' }],
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
        <div style="display:flex;gap:10px;align-items:center">
          <span id="pilot-status" style="font-size:12px;color:#64748b">驱动：—</span>
          <button class="btn-pub" id="btn-pilot" title="AI 自主流转一轮五层协作（圣人→大罗金仙→准圣→道祖）">⚡ 驱动一轮</button>
          <button class="btn-pub" id="btn-pilot-drain" title="连续驱动到无可驱动任务（多任务一次自主流转，上限 10 轮）">⚡ 驱动到空闲</button>
          <button class="btn-pub" id="btn-pub">+ 发布任务</button>
        </div>
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
    看板存储.订阅(() => {
      渲染看板(容器.querySelector('#board'));
      渲染属性();
    });

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

    // 一键驱动：AI 自主流转（一轮 / 到空闲），轮询状态与看板直到完成
    const 驱动状态框 = 容器.querySelector('#pilot-status');
    const 驱动按钮 = 容器.querySelector('#btn-pilot');
    const 驱动到空闲按钮 = 容器.querySelector('#btn-pilot-drain');
    async function 刷新驱动状态() {
      try {
        const 状态 = await 驱动状态();
        if (状态.运行中) { 驱动状态框.textContent = '驱动：运行中…'; 驱动按钮.disabled = true; if (驱动到空闲按钮) 驱动到空闲按钮.disabled = true; return false; }
        驱动按钮.disabled = false;
        if (驱动到空闲按钮) 驱动到空闲按钮.disabled = false;
        let 说明 = '';
        if (状态.最近阶段) {
          const 阶段 = 状态.最近阶段;
          说明 = 阶段.类型 === '空闲' ? '空闲' : `${阶段.类型}${阶段.任务id ? ' #' + 阶段.任务id : ''}${阶段.新状态 ? ' → ' + 阶段.新状态 : ''}`;
        } else {
          说明 = 状态.就绪 ? '就绪' : '未就绪';
        }
        if (状态.最近结果 && !状态.运行中) 说明 += ' · ' + 状态.最近结果;
        驱动状态框.textContent = '驱动：' + 说明;
        return true;
      } catch { return true; }
    }
    function 启动驱动轮询(动作) {
      return async () => {
        try {
          await 动作();
          驱动状态框.textContent = '驱动：运行中…';
          驱动按钮.disabled = true;
          if (驱动到空闲按钮) 驱动到空闲按钮.disabled = true;
          await 加载看板数据();
          const 轮询 = setInterval(async () => {
            await 加载看板数据();
            const 完成 = await 刷新驱动状态();
            if (完成) clearInterval(轮询);
          }, 2000);
        } catch (e) { alert(e.message); 刷新驱动状态(); }
      };
    }
    刷新驱动状态();
    驱动按钮.addEventListener('click', 启动驱动轮询(驱动一轮));
    if (驱动到空闲按钮) 驱动到空闲按钮.addEventListener('click', 启动驱动轮询(() => 驱动到空闲(10)));
  },
  属性(容器) {
    面板容器 = 容器;
    渲染属性();
  },
};

/** 渲染属性面板：任务详情（选中任务后由存储订阅同步更新） */
function 渲染属性() {
  if (!面板容器) return;
  const 状态 = 看板存储.取值();
  const 任务 = 状态.任务.find((t) => t.id === 状态.选中);
  if (!任务) {
    渲染属性面板(面板容器, '任务详情', '', '', `<div class="prop-group"><div class="kv">点击卡片查看详情</div></div>`);
    return;
  }
  渲染属性面板(面板容器, 任务.title, 任务.status, '', `
    <div class="prop-group">
      <div class="kv"><b>ID</b>${任务.id}</div>
      <div class="kv"><b>状态</b>${转义(任务.status)}</div>
      <div class="kv"><b>场景</b>${转义(任务.场景 || '—')}</div>
      <div class="kv"><b>优先级</b>${转义(任务.优先级 || '—')}</div>
      <div class="kv"><b>发起人</b>${转义(角色映射[任务.发起人] || 任务.发起人 || '—')}</div>
      <div class="kv"><b>当前承接人</b>${转义(角色映射[任务.当前承接人] || 任务.当前承接人 || '—')}</div>
      <div class="kv"><b>修复轮次</b>${任务.修复轮次 || 0}</div>
      <div class="kv"><b>描述</b>${转义(任务.description || '—')}</div>
    </div>
    ${任务.承接历史 && 任务.承接历史.length ? `<div class="prop-group"><b>承接历史</b>${转义(任务.承接历史.map((r) => 角色映射[r] || r).join(' → '))}</div>` : ''}
    ${任务.状态历史 && 任务.状态历史.length ? `<div class="prop-group"><b>状态历史</b>${转义(任务.状态历史.map((h) => `${h.原状态}→${h.新状态}`).join(' · '))}</div>` : ''}
    ${验收包(任务)}
  `);
}

/** 交付验收包：聚合「改了什么 / 怎么验证 / 卡在哪 / 谁确认」四块结构化视图 */
function 验收包(任务) {
  if (!任务) return '';
  const 实现 = 任务.实现文档;
  const 验收 = 任务.验收文档;
  const 终审 = 任务.终审文档;
  const 扫尾 = 任务.扫尾记录;
  const 回退 = 任务.回退来源;
  const 澄清 = 任务.澄清记录;

  // 1. 改了什么：实现文档代码变更清单
  const 变更 = (实现 && Array.isArray(实现.代码变更) && 实现.代码变更.length)
    ? 实现.代码变更.map((c) => `<div class="kv"><b>${转义(c.变更类型 || '变更')}</b>${转义(c.文件路径 || '')}${c.摘要 ? ` — ${转义(c.摘要)}` : ''}</div>`).join('')
    : `<div class="kv">待推进到实现阶段</div>`;

  // 2. 怎么验证：自检 + 验收轮次 + 扫尾通过
  const 自检 = 实现 && 实现.自检
    ? `${实现.自检.通过 ? '✅' : '❌'} 自检 ${实现.自检.通过 ? '通过' : '未通过'}${(实现.自检.问题 && 实现.自检.问题.length) ? '：' + 转义(实现.自检.问题.join('；')) : ''}`
    : '待自检';
  const 验收轮次 = (验收 && Array.isArray(验收.轮次) && 验收.轮次.length)
    ? 验收.轮次.map((r) => `<div class="kv">第${r.轮次}轮 ${r.通过 ? '✅' : '❌'}${(r.问题 && r.问题.length) ? ' 问题:' + 转义(r.问题.join('；')) : ''}${r.建议 ? ' 建议:' + 转义(r.建议) : ''}</div>`).join('')
    : `<div class="kv">待验收</div>`;
  const 扫尾态 = 扫尾
    ? (扫尾.通过 ? '✅' : '❌') + ` 扫尾：` + (扫尾.兑现数 ?? 0) + ' 兑现 / ' + (扫尾.未兑现数 ?? 0) + ' 未兑现 / ' + (扫尾.多余数 ?? 0) + ' 多余' + (扫尾.通过 ? '' : `（${转义(扫尾.说明 || '')}）`)
    : `待扫尾（交付核验）`;

  // 3. 卡在哪：当前状态 + 回退记录 + 澄清记录
  const 回退态 = 回退 ? `第${回退.回退次数}次回退 → ${转义(回退.原因 || '')}（${转义(回退.错误描述 || '')}）` : '无回退';
  const 澄清态 = 澄清 ? `澄清：${继续态(澄清.继续)} — ${转义(澄清.结论 || '')}（操作者 ${转义(澄清.操作者 || '—')}）` : '待道祖澄清';

  // 4. 谁确认：终审文档
  const 终审态 = 终审 && 终审.通过 !== undefined
    ? `${终审.通过 ? '✅' : '❌'} 终审 ${终审.通过 ? '通过' : '未通过'}${终审.评语 ? ` — ${转义(终审.评语)}` : ''}${终审.风险评估 ? `（风险：${转义(终审.风险评估)}）` : ''}`
    : '待道祖终审';

  return `
    <div class="prop-group 验收包">
      <div class="验收标题">📦 交付验收包</div>
      <div class="验收小节"><b>① 改了什么</b>${变更}</div>
      <div class="验收小节"><b>② 怎么验证</b><div class="kv">${自检}</div>${验收轮次}<div class="kv">${扫尾态}</div></div>
      <div class="验收小节"><b>③ 卡在哪</b><div class="kv"><b>当前状态</b>${转义(任务.status || '—')}</div><div class="kv"><b>回退</b>${回退态}</div><div class="kv"><b>澄清</b>${澄清态}</div></div>
      <div class="验收小节"><b>④ 谁确认</b><div class="kv">${终审态}</div></div>
    </div>`;
}

function 继续态(继续) {
  return 继续 ? '继续' : '终止';
}

/** 五行层级标签颜色（按 当前层级 显示小色块） */
const 五行层级色 = {
  木: '#86efac', 火: '#fca5a5', 土: '#fcd34d', 金: '#fde68a', 水: '#67e8f9',
};
/** 取任务五行层级（后端已序列化为中文枚举名 木/火/土/金/水）；缺失时留空 */
function 五行层级(任务) {
  const 层级 = 任务 && 任务.当前层级;
  return 五行层级色[层级] ? 层级 : '';
}
/** 五行标签 HTML：小色块 → 层级名（木/火/土/金/水） */
function 五行标签(任务) {
  const 层级 = 五行层级(任务);
  if (!层级) return '';
  const 色 = 五行层级色[层级] || '#94a3b8';
  return `<span class="wuxing-tag" style="background:${色}" title="五行层级：${层级}">${转义(层级)}</span> `;
}

/** HTML 转义 */
function 转义(文本) {
  return String(文本)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function 渲染看板(容器) {
  if (!容器) return;
  const 状态 = 看板存储.取值();
  const 任务列表 = 状态.任务;
  let html = '';
  for (const 分组 of 状态分组) {
    const 任务们 = 任务列表.filter((t) => t.status === 分组.状态);
    if (任务们.length === 0 && !['待受理', '待圣人设计', '待大罗金仙实现', '待准圣验收', '待道祖终审', '待清理', '已完成'].includes(分组.状态)) continue;
    html += `<div class="board-col" style="border-top:3px solid ${分组.色}">`;
    html += `<div class="board-col-head"><span class="board-col-title">${分组.状态}</span><span class="board-col-count">${任务们.length}</span></div>`;
    for (const 任务 of 任务们) {
      const 承接角色 = 分组.角色 ? 角色映射[分组.角色] : null;
      const 流转 = 流转选项[任务.status] || [];
      html += `<div class="board-card" data-id="${任务.id}">`;
      html += `<div class="board-card-title">${五行标签(任务)}${转义(任务.title)}</div>`;
      html += `<button class="btn-evidence" data-id="${任务.id}" title="打开交付验收视图（改了什么/怎么验证/卡在哪/谁确认/过程轨迹）">📋 交付</button>`;
      html += `<div class="board-card-meta">`;
      if (分组.召回) html += `<span class="tag" style="background:#a78bfa">召回中</span>`;
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
      if (任务.status === '待清理') {
        html += `<button class="btn-clean" data-id="${任务.id}">🧹 一键清理</button>`;
        html += `<button class="btn-sweep" data-id="${任务.id}">🧽 扫尾检查</button>`;
      }
      if (任务.status === '待道祖澄清') {
        html += `<button class="btn-clarify" data-id="${任务.id}" title="道祖对回退到木层的任务给出澄清结论（继续设计或终止）">💬 澄清</button>`;
      }
      if (任务.status === '待修复' || 任务.status === '准圣验收中') {
        html += `<button class="btn-clean" data-id="${任务.id}" data-rollback="1" title="按任务标识追溯错误根源定向回退（土=实现/火=设计/木=需求）">⬅ 定向回退</button>`;
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

  容器.querySelectorAll('.btn-evidence').forEach((按钮) => {
    按钮.addEventListener('click', (e) => {
      e.stopPropagation();
      const 任务 = 看板存储.取值().任务.find((t) => t.id === Number(按钮.dataset.id));
      打开验收弹层(任务);
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

  容器.querySelectorAll('.btn-clean').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      try { await 清理任务(id); } catch (err) { alert(err.message); }
    });
  });

  容器.querySelectorAll('.btn-sweep').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      try {
        const { 记录 } = await 扫尾任务(id);
        const 状态 = 记录.通过 ? '✅ 通过' : '❌ 未通过';
        const 漂移 = (记录.漂移项 || []).map((d) => `  · ${d.类型} ${d.路径}${d.说明 ? '（' + d.说明 + '）' : ''}`).join('\n');
        alert(
          `${状态} 交付证据核验 (#${id})\n` +
          `声明变更 ${记录.变更总数} → 兑现 ${记录.兑现数} / 未兑现 ${记录.未兑现数}，多余新增 ${记录.多余数}\n` +
          (漂移 ? `漂移项：\n${漂移}` : `漂移项：无`) +
          (记录.说明 ? `\n${记录.说明}` : '')
        );
      } catch (err) { alert(err.message); }
    });
  });

    容器.querySelectorAll('.btn-clarify').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      const 结论 = prompt('请输入澄清结论（需求纠偏说明，作为重新设计的依据）：', '');
      if (结论 === null) return;
      const 继续 = confirm('澄清后是否继续设计？\n确定=继续进入待圣人设计；取消=终止本任务');
      try {
        const 结果 = await 澄清推进(id, 结论 || '（无结论）', 继续);
        alert(`任务 #${id} 已${结果.新状态 === '已取消' ? '终止（取消）' : '重新进入待圣人设计'}\n结论：${结果.澄清记录.结论}`);
      } catch (err) { alert(err.message); }
    });
  });

    容器.querySelectorAll('[data-rollback]').forEach((按钮) => {
    按钮.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = Number(按钮.dataset.id);
      const 错误描述 = prompt('请输入验收不通过的错误描述：', '');
      if (错误描述 === null) return;
      const 建议 = prompt('建议根源层级（可空：木/火/土，缺省由追溯器自动判断）：', '');
      try {
        const 结果 = await 定向回退(id, 错误描述 || '验收不通过', 建议 || undefined);
        const 影响 = 结果.影响任务数 ? `，连带召回 ${结果.影响任务数} 个依赖任务` : '';
        alert(`已定向回退到 ${结果.回退到}（累计第 ${结果.回退次数} 次回退${影响}）`);
      } catch (err) { alert(err.message); }
    });
  });
}

function 优先级色(p) {
  if (p === 'P0') return '#ef4444';
  if (p === 'P1') return '#f97316';
  if (p === 'P2') return '#4ade80';
  return '#94a3b8';
}