// 日志数据.js —— 运行日志共享存储（前端乐观更新 + 后端持久记录）

import { 创建存储 } from '../../状态管理-阁/响应式-核心-园/响应式核心.js';

export const 日志存储 = 创建存储({ 记录: [] });

/** 追加一条运行日志：本地立即更新，异步上报后端 */
export function 记日志(标签, 样式, 内容) {
  const 记录 = { 标签, 样式, 内容, 时间: new Date().toTimeString().slice(0, 8) };
  日志存储.更新({ 记录: [...日志存储.取值().记录, 记录] });
  fetch('/api/logs', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ 标签, 样式, 内容 }),
  }).catch((错误) => console.warn('日志上报后端失败', 错误));
}

/** 从后端拉取历史日志 */
export async function 加载日志() {
  try {
    const 记录 = await fetch('/api/logs').then((响应) => 响应.json());
    日志存储.更新({ 记录: 记录.map(格式化) });
  } catch (错误) {
    console.warn('加载日志失败（后端未就绪？）', 错误);
  }
}

function 格式化(记录) {
  return { ...记录, 时间: new Date(记录.时间 * 1000).toTimeString().slice(0, 8) };
}
