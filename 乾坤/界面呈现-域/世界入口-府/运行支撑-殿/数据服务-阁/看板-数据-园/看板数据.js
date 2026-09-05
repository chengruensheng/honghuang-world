// 看板数据.js —— 任务看板数据拉取与操作（GET/POST /api/board）

import { 创建存储 } from '../../状态管理-阁/响应式-核心-园/响应式核心.js';

export const 看板存储 = 创建存储({ 任务: [], 选中: null, 驱动: null });

const 取json = (响应) => 响应.json();

/** 从后端拉取看板全部任务 */
export async function 加载看板数据() {
  try {
    const 任务 = await fetch('/api/board').then(取json);
    看板存储.更新({ 任务 });
  } catch (错误) {
    console.warn('加载看板数据失败（后端未就绪？）', 错误);
  }
}

/** 发布任务 */
export async function 发布任务(标题, 描述, 场景, 优先级) {
  const body = { title: 标题, description: 描述 };
  if (场景) body.scene = 场景;
  if (优先级) body.priority = 优先级;
  const 响应 = await fetch('/api/board', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!响应.ok) throw new Error(`发布失败: ${响应.status}`);
  await 加载看板数据();
  return 响应.json();
}

/** 承接任务 */
export async function 承接任务(id, 角色) {
  const 响应 = await fetch(`/api/board/${id}/accept`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role: 角色 }),
  });
  if (!响应.ok) throw new Error(`承接失败: ${响应.status}`);
  await 加载看板数据();
}

/** 提交任务（流转到下一状态） */
export async function 提交任务(id, 角色, 下一状态) {
  const 响应 = await fetch(`/api/board/${id}/submit`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role: 角色, next_status: 下一状态 }),
  });
  if (!响应.ok) throw new Error(`提交失败: ${响应.status}`);
  await 加载看板数据();
}

/** 选中任务（用于属性面板展示详情） */
export function 选中任务(id) {
  看板存储.更新({ 选中: id });
}

/** 驱动看板一轮（POST /api/dev/pilot）—— AI 自主流转五层协作 */
export async function 驱动一轮() {
  const 响应 = await fetch('/api/dev/pilot', { method: 'POST' });
  if (!响应.ok) {
    const 体 = await 响应.json().catch(() => null);
    throw new Error((体 && 体.错误) || `驱动失败: ${响应.status}`);
  }
}

/** 查询看板驱动台状态（GET /api/dev/pilot/status） */
export async function 驱动状态() {
  const 状态 = await fetch('/api/dev/pilot/status').then(取json);
  看板存储.更新({ 驱动: 状态 });
  return 状态;
}