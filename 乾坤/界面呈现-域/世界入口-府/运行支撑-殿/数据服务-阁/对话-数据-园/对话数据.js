// 对话数据.js —— 道祖接待对话 API（POST /api/dev/chat + /confirm）

const 取json = (响应) => 响应.json();

/** 发送消息给道祖，返回 {阶段, 回复, 需求} */
export async function 道祖对话(消息) {
  const 响应 = await fetch('/api/dev/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ 消息 }),
  });
  if (!响应.ok) {
    const 错误 = await 响应.json().catch(() => null);
    throw new Error((错误 && 错误.错误) || `道祖未响应（HTTP ${响应.status}）`);
  }
  return 取json(响应);
}

/** 确认发布对齐需求，返回 {任务id} */
export async function 确认发布() {
  const 响应 = await fetch('/api/dev/chat/confirm', { method: 'POST' });
  if (!响应.ok) {
    const 错误 = await 响应.json().catch(() => null);
    throw new Error((错误 && 错误.错误) || `确认发布失败（HTTP ${响应.status}）`);
  }
  return 取json(响应);
}