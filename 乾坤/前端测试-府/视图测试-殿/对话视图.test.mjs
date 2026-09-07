import test from 'node:test';
import assert from 'node:assert/strict';

test('对话视图_挂载渲染输入框与欢迎气泡', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.fetch = async () => ({ ok: true, json: async () => ({}) });

  const { 对话视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/对话交互-阁/对话-视图-园/对话视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);

  对话视图.挂载(容器);

  assert.ok(容器.innerHTML.includes('对话'), '应渲染对话标题');
  assert.ok(容器.querySelector('#task'), '应有需求输入框');
  assert.ok(容器.querySelector('#go'), '应有发送按钮');
  assert.ok(容器.querySelector('#chat'), '应有聊天区');

  // 欢迎气泡由道祖接待开场渲染
  const 气泡区 = 容器.querySelector('#chat');
  assert.ok(气泡区.innerHTML.includes('道祖'), '欢迎气泡应有道祖角色');
  assert.ok(气泡区.innerHTML.includes('接待与任务澄清'), '欢迎气泡应含接待说明');
  assert.ok(容器.querySelector('#process-panel'), '应有实时过程面板');
  assert.ok(容器.querySelector('#history-header'), '应有历史会话面板');
});

test('对话视图_卸载后清理状态', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.fetch = async () => ({ ok: true, json: async () => ({}) });

  const { 对话视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/对话交互-阁/对话-视图-园/对话视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  对话视图.挂载(容器);
  对话视图.卸载();

  // 卸载不抛错即通过；再次挂载应仍可工作（幂等）
  对话视图.挂载(容器);
  assert.ok(容器.querySelector('#chat'), '重新挂载应恢复聊天区');
});
