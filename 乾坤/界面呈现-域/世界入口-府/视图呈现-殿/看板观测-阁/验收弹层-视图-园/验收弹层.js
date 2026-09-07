// 验收弹层.js —— 交付验收视图弹层：卡片「📋 交付」展开独占式完整交付证据
//
// 聚合五块：基本信息 / 改了什么 / 怎么验证 / 卡在哪 / 谁确认 / 过程轨迹。
// 纯前端聚合 `GET /api/board` 已返回字段，零后端契约变更；扫尾/澄清为既有操作的内嵌入口。

import { 扫尾任务, 澄清推进, 会话清单, 会话回放 } from '../../../运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';
import { 事件图标 } from '../事件-呈现-园/事件呈现.js';

const 角色映射 = { 道祖: '道祖', 圣人: '圣人', A: '大罗金仙', 准圣: '准圣', 太乙金仙: '太乙金仙' };
const 五行层级色 = { 木: '#86efac', 火: '#fca5a5', 土: '#fcd34d', 金: '#fde68a', 水: '#67e8f9' };

function 转义(文本) {
  return String(文本).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function 角色名(角色) {
  return 角色映射[角色] || 角色 || '—';
}

/** 打开交付验收弹层（独占式查看，不含流转按钮） */
export function 打开验收弹层(任务) {
  if (!任务) return;
  const 已有 = document.getElementById('evidence-modal');
  if (已有) 已有.remove();

  const 弹层 = document.createElement('div');
  弹层.className = 'modal modal-wide';
  弹层.id = 'evidence-modal';
  弹层.innerHTML = `
    <div class="modal-box evidence-box">
      <div class="evidence-head">
        <h3>📋 交付验收 · #${任务.id} ${转义(任务.title || '')}</h3>
        <button class="btn-cancel evidence-close" title="关闭">✕</button>
      </div>
      <div class="evidence-grid">
        ${基本信息块(任务)}
        ${改什么块(任务)}
        ${怎么验证块(任务)}
        ${卡在哪块(任务)}
        ${谁确认块(任务)}
        ${过程轨迹块(任务)}
      </div>
    </div>
  `;

  // 关闭：✕ / 点击背景 / Esc
  弹层.querySelector('.evidence-close').addEventListener('click', () => 弹层.remove());
  弹层.addEventListener('mousedown', (e) => { if (e.target === 弹层) 弹层.remove(); });
  const 关 = () => 弹层.remove();
  document.addEventListener('keydown', 关, { once: true });

  // 内嵌操作：待道祖澄清 → 澄清；待清理无扫尾 → 扫尾
  弹层.querySelector('[data-evidence-clear]')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    const id = Number(e.target.dataset.evidenceClear);
    try { const { 记录 } = await 扫尾任务(id); 弹层.remove(); alert(`已扫尾：${记录.通过 ? '✅ 通过' : '❌ 未通过'}（兑现 ${记录.兑现数} / 未兑现 ${记录.未兑现数} / 多余 ${记录.多余数}）`); }
    catch (err) { alert(err.message); }
  });
  弹层.querySelector('[data-evidence-clarify]')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    const id = Number(e.target.dataset.evidenceClarify);
    const 结论 = prompt('请输入澄清结论（需求纠偏说明，作为重新设计的依据）：', '');
    if (结论 === null) return;
    const 继续 = confirm('澄清后是否继续设计？\n确定=继续进入待圣人设计；取消=终止本任务');
    try { await 澄清推进(id, 结论 || '（无结论）', 继续); 弹层.remove(); }
    catch (err) { alert(err.message); }
  });
  // ⑤驱动过程：按需加载该任务的驱动会话事件
  弹层.querySelector('.btn-evidence-process')?.addEventListener('click', (e) => {
    e.stopPropagation();
    渲染驱动过程(e.currentTarget, Number(e.currentTarget.dataset.taskId));
  });

  document.body.appendChild(弹层);
}

function 基本信息块(任务) {
  const 层级 = 五行层级色[任务.当前层级];
  return `
    <div class="evidence 证据基本信息">
      <b>基本信息</b>
      <div class="kv"><span class="k">ID</span><span>${任务.id}</span></div>
      <div class="kv"><span class="k">状态</span><span>${转义(任务.status || '—')}</span></div>
      <div class="kv"><span class="k">场景</span><span>${转义(任务.场景 || '—')}</span></div>
      <div class="kv"><span class="k">优先级</span><span>${转义(任务.优先级 || '—')}</span></div>
      <div class="kv"><span class="k">五行</span><span>${层级 ? `<span style="color:${层级}">${转义(任务.当前层级)}</span>` : '—'}</span></div>
      <div class="kv"><span class="k">发起人</span><span>${角色名(任务.发起人)}</span></div>
      <div class="kv"><span class="k">当前承接</span><span>${角色名(任务.当前承接人)}</span></div>
      <div class="kv"><span class="k">修复轮次</span><span>${任务.修复轮次 || 0}</span></div>
      <div class="kv"><span class="k">描述</span><span>${转义(任务.description || '—')}</span></div>
    </div>`;
}

function 改什么块(任务) {
  const 实现 = 任务.实现文档;
  const 变更 = (实现 && Array.isArray(实现.代码变更) && 实现.代码变更.length)
    ? 实现.代码变更.map((c) => `<div class="kv row"><span class="k">${转义(c.变更类型 || '变更')}</span><span>${转义(c.文件路径 || '')}${c.摘要 ? ` — ${转义(c.摘要)}` : ''}</span></div>`).join('')
    : '<div class="kv">待推进到实现阶段</div>';
  return `<div class="evidence 证据改了什么"><b>① 改了什么</b>${变更}</div>`;
}

function 怎么验证块(任务) {
  const 实现 = 任务.实现文档;
  const 验收 = 任务.验收文档;
  const 扫尾 = 任务.扫尾记录;
  const 自检 = 实现 && 实现.自检
    ? `${实现.自检.通过 ? '✅' : '❌'} 自检 ${实现.自检.通过 ? '通过' : '未通过'}${(实现.自检.问题 && 实现.自检.问题.length) ? '：' + 转义(实现.自检.问题.join('；')) : ''}`
    : '待自检';
  const 轮次 = (验收 && Array.isArray(验收.轮次) && 验收.轮次.length)
    ? 验收.轮次.map((r) => `<div class="kv row"><span class="k">第${r.轮次}轮</span><span>${r.通过 ? '✅' : '❌'}${(r.问题 && r.问题.length) ? ' 问题:' + 转义(r.问题.join('；')) : ''}${r.建议 ? ' 建议:' + 转义(r.建议) : ''}</span></div>`).join('')
    : '<div class="kv">待验收</div>';
  let 扫尾态;
  if (扫尾) {
    const 漂移 = (扫尾.漂移项 && 扫尾.漂移项.length)
      ? 扫尾.漂移项.map((d) => `<div class="kv drift">· ${d.类型} ${d.路径}${d.说明 ? '（' + d.说明 + '）' : ''}</div>`).join('')
      : '<div class="kv drift">漂移项：无</div>';
    扫尾态 = `<div class="kv">${扫尾.通过 ? '✅' : '❌'} 扫尾：${扫尾.兑现数 ?? 0} 兑现 / ${扫尾.未兑现数 ?? 0} 未兑现 / ${扫尾.多余数 ?? 0} 多余${扫尾.说明 ? `（${转义(扫尾.说明)}）` : ''}</div>${漂移}`;
  } else {
    扫尾态 = `<div class="kv">待扫尾（交付核验）</div>${任务.status === '待清理' ? `<button class="btn-sweep" data-evidence-clear="${任务.id}">🧽 扫尾检查</button>` : ''}`;
  }
  return `<div class="evidence 证据怎么验证"><b>② 怎么验证</b><div class="kv">${自检}</div>${轮次}${扫尾态}</div>`;
}

function 卡在哪块(任务) {
  const 回退 = 任务.回退来源;
  const 澄清 = 任务.澄清记录;
  const 回退态 = 回退 ? `第${回退.回退次数}次回退 → ${转义(回退.原因 || '')}（${转义(回退.错误描述 || '')}）` : '无回退';
  let 澄清态;
  if (澄清) {
    澄清态 = `<div class="kv">澄清：${澄清.继续 ? '继续' : '终止'} — ${转义(澄清.结论 || '')}（操作者 ${转义(澄清.操作者 || '—')}）</div>`;
  } else if (任务.status === '待道祖澄清') {
    澄清态 = `<div class="kv">待道祖澄清</div><button class="btn-clarify" data-evidence-clarify="${任务.id}">💬 澄清</button>`;
  } else {
    澄清态 = '<div class="kv">待道祖澄清</div>';
  }
  return `<div class="evidence 证据卡在哪"><b>③ 卡在哪</b><div class="kv"><span class="k">当前状态</span><span>${转义(任务.status || '—')}</span></div><div class="kv"><span class="k">回退</span><span>${回退态}</span></div>${澄清态}<div class="kv"><span class="k">说明</span><span>${转义(任务.卡住说明 || '—')}</span></div></div>`;
}

function 谁确认块(任务) {
  const 终审 = 任务.终审文档;
  const 终审态 = 终审 && 终审.通过 !== undefined
    ? `${终审.通过 ? '✅' : '❌'} 终审 ${终审.通过 ? '通过' : '未通过'}${终审.评语 ? ` — ${转义(终审.评语)}` : ''}${终审.风险评估 ? `（风险：${转义(终审.风险评估)}）` : ''}`
    : '待道祖终审';
  return `<div class="evidence 证据谁确认"><b>④ 谁确认</b><div class="kv">${终审态}</div></div>`;
}

function 过程轨迹块(任务) {
  const 状态史 = (任务.状态历史 && 任务.状态历史.length)
    ? 任务.状态历史.map((h) => `<div class="kv">${转义(h.原状态)} → ${转义(h.新状态)}</div>`).join('')
    : '<div class="kv">—</div>';
  const 承接史 = (任务.承接历史 && 任务.承接历史.length)
    ? `<div class="kv">${任务.承接历史.map(角色名).join(' → ')}</div>`
    : '<div class="kv">—</div>';
  return `
    <div class="evidence 证据过程轨迹"><b>⑤ 过程轨迹</b>
      <div class="kv-title">状态历史</div>${状态史}
      <div class="kv-title">承接历史</div>${承接史}
      <div class="kv-title">驱动过程</div>
      <button class="btn-evidence-process" data-task-id="${任务.id}">▶ 查看驱动过程</button>
      <div class="evidence-process-box" data-process-box></div>
    </div>`;
}

/** 按需加载并渲染该任务的驱动过程事件（复用既有会话回放接口，仅渲染目标任务事件） */
async function 渲染驱动过程(按钮, 任务id) {
  const 弹层 = 按钮.closest('.modal');
  const 盒 = 弹层 && 弹层.querySelector(`[data-process-box]`);
  if (!盒) return;
  盒.innerHTML = '';
  盒.appendChild(过程状态行('加载中…'));
  try {
    const 会话们 = await 会话清单();
    const 匹配 = 会话们
      .filter((s) => s.任务id列表 && s.任务id列表.includes(任务id))
      .sort((a, b) => b.创建时间 - a.创建时间)[0];
    if (!匹配) {
      盒.innerHTML = '';
      盒.appendChild(过程状态行('暂无驱动过程记录'));
      return;
    }
    const 详情 = await 会话回放(匹配.会话id);
    const 事件 = (详情.事件 || []).filter((e) => e.任务id === 任务id);
    盒.innerHTML = '';
    if (事件.length === 0) {
      盒.appendChild(过程状态行('该会话无此任务的过程事件'));
      return;
    }
    const 头 = document.createElement('div');
    头.className = 'evidence-process-head';
    头.textContent = `会话 #${匹配.会话id} · ${详情.发起方式 || '驱动'} · ${详情.状态 || '—'} · ${事件.length} 条`;
    盒.appendChild(头);
    const 流 = document.createElement('div');
    流.className = 'evidence-process-stream';
    盒.appendChild(流);
    for (const e of 事件) 流.appendChild(过程事件行(e));
    流.scrollTop = 流.scrollHeight;
  } catch (错误) {
    盒.innerHTML = '';
    盒.appendChild(过程状态行(`加载驱动过程失败：${错误.message || '未知错误'}（会话可能已清理）`));
  }
}

function 过程状态行(文本) {
  const 行 = document.createElement('div');
  行.className = 'evidence-process-empty';
  行.textContent = 文本;
  return 行;
}

function 过程事件行(事件) {
  const 图标 = 事件图标(事件.类型);
  const 角色 = 事件.角色 || '?';
  const 轮次 = 事件.轮次 != null ? `R${事件.轮次}` : '';
  const 工具 = 事件.工具名 ? ` <span class="proc-tool">${转义(事件.工具名)}</span>` : '';
  const 行 = document.createElement('div');
  行.className = 'evidence-process-item';
  行.innerHTML = `<div class="ep-meta">${图标} ${转义(角色)} ${轮次} ${转义(事件.类型)}${工具}</div>${事件.内容 ? `<div class="ep-content">${转义(事件.内容)}</div>` : ''}`;
  return 行;
}
