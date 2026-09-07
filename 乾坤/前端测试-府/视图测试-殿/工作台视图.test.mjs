import test from 'node:test';
import assert from 'node:assert/strict';

/** 创建 happy-dom 环境并全局注入，返回 window */
async function 造环境() {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  return window;
}

/** 等一小轮微任务，让异步渲染（会话清单/回放）settle */
const 刷新 = () => new Promise((r) => setTimeout(r, 0));

test('工作台_挂载渲染标题指标与空态', async () => {
  await 造环境();
  globalThis.fetch = async () => ({ json: async () => ({ 会话: [] }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [], 选中: null, 驱动: null });

  const { 工作台视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  工作台视图.挂载(容器);
  await 刷新();

  assert.ok(容器.innerHTML.includes('可观测工作台'), '应渲染工作台标题');
  assert.ok(容器.querySelector('#wb-task-num'), '应有任务总数指标');
  assert.ok(容器.querySelector('#wb-session-num'), '应有会话数指标');
  assert.ok(容器.querySelector('#wb-event-num'), '应有事件数指标');
  assert.ok(容器.querySelector('#wb-timeline'), '应有时间线容器');
  assert.equal(容器.querySelector('#wb-task-num').textContent, '0', '任务数应为 0');
  assert.ok(容器.querySelector('#wb-timeline').innerHTML.includes('暂无驱动会话'), '空清单应显示空态');
});

test('工作台_时间线按创建时间倒序渲染会话头', async () => {
  await 造环境();
  globalThis.fetch = async () => ({ json: async () => ({ 会话: [
    { 会话id: 1, 发起方式: '手动', 创建时间: 1000, 状态: '完成', 任务id列表: [1] },
    { 会话id: 2, 发起方式: '自动', 创建时间: 3000, 状态: '运行中', 任务id列表: [1, 2] },
  ] }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 1, title: '构建身份模块' }], 选中: null, 驱动: null });

  const { 工作台视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  工作台视图.挂载(容器);
  await 刷新();

  assert.equal(容器.querySelector('#wb-task-num').textContent, '1', '任务总数应为 1');
  assert.equal(容器.querySelector('#wb-session-num').textContent, '2', '会话总数应为 2');
  const 会话们 = 容器.querySelectorAll('.wb-session');
  assert.equal(会话们.length, 2, '应渲染 2 个会话');
  assert.ok(会话们[0].innerHTML.includes('会话 #2'), '创建时间大的会话应排最前（倒序）');
  assert.ok(会话们[0].innerHTML.includes('运行中'), '应为时间倒序第一项状态');
  assert.ok(会话们[1].innerHTML.includes('会话 #1'), '第二个会话为 #1');
  assert.ok(会话们[1].querySelector('.wb-expand'), '会话应有展开按钮');
});

test('工作台_点击展开按需回放并渲染事件流(转义/任务标题)', async () => {
  await 造环境();
  let 回放调用 = 0;
  globalThis.fetch = async (url) => {
    if (String(url).includes('/回放') || (typeof url === 'string' && /\/sessions\/\d+/.test(url))) {
      回放调用 += 1;
      return { json: async () => ({ 会话id: 2, 发起方式: '自动', 事件: [
        { 类型: '思考', 角色: '圣人', 轮次: 1, 内容: '设计 <bound> 模块', 任务id: 9 },
        { 类型: '工具调用', 角色: 'A', 轮次: 1, 工具名: '写文件', 内容: '写入 x.js', 任务id: 9 },
        { 类型: '任务答复', 角色: '道祖', 轮次: 2, 内容: '终审通过', 任务id: 9 },
      ] }) };
    }
    return { json: async () => ({ 会话: [{ 会话id: 2, 发起方式: '自动', 创建时间: 5000, 状态: '完成', 任务id列表: [9] }] }) };
  };

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 9, title: '设计模块' }], 选中: null, 驱动: null });

  const { 工作台视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  工作台视图.挂载(容器);
  await 刷新();

  const 会话 = 容器.querySelector('.wb-session');
  assert.equal(回放调用, 0, '未点击展开时不应请求会话回放');
  assert.ok(会话.querySelector('[data-flow]').hidden, '展开前事件流应隐藏');

  会话.querySelector('.wb-expand').click();
  await 刷新();

  assert.equal(回放调用, 1, '点击展开后应请求一次回放');
  const 流 = 会话.querySelector('[data-flow]');
  assert.ok(流.innerHTML.includes('设计模块'), '事件应显示关联任务标题');
  assert.ok(流.innerHTML.includes('写文件'), '应含工具名');
  assert.ok(流.innerHTML.includes('&lt;bound&gt;'), '事件内容应转义防注入');
  assert.ok(流.innerHTML.includes('💭'), '含思考图标');
  assert.ok(流.innerHTML.includes('📝'), '含答复图标');
  assert.ok(流.innerHTML.includes('跳转看板'), '关联任务应有跳转按钮');
});

test('工作台_空事件会话显示占位', async () => {
  await 造环境();
  globalThis.fetch = async (url) => {
    if (/\/sessions\/\d+/.test(String(url))) return { json: async () => ({ 会话id: 1, 事件: [] }) };
    return { json: async () => ({ 会话: [{ 会话id: 1, 发起方式: '自动', 创建时间: 5000, 状态: '完成', 任务id列表: [] }] }) };
  };

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [], 选中: null, 驱动: null });

  const { 工作台视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  工作台视图.挂载(容器);
  await 刷新();

  容器.querySelector('.wb-expand').click();
  await 刷新();

  assert.ok(容器.querySelector('.wb-session-flow').innerHTML.includes('该会话暂无过程事件'), '空事件应显示占位');
});

test('工作台_事件跳转看板定位任务', async () => {
  await 造环境();
  globalThis.fetch = async (url) => {
    if (/\/sessions\/\d+/.test(String(url))) return { json: async () => ({ 会话id: 2, 事件: [{ 类型: '思考', 角色: '圣人', 轮次: 1, 内容: 'x', 任务id: 9 }] }) };
    return { json: async () => ({ 会话: [{ 会话id: 2, 发起方式: '自动', 创建时间: 5000, 状态: '完成', 任务id列表: [9] }] }) };
  };

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 9, title: '设计模块' }], 选中: null, 驱动: null });

  // 强制重载模块实例，让全局点击委托绑定到本测试的 document
  const 视图模块 = await import(`../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js?t=${Date.now()}`);
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  视图模块.工作台视图.挂载(容器);
  await 刷新();

  容器.querySelector('.wb-expand').click();
  await 刷新();

  const 跳转 = 容器.querySelector('.wb-jump');
  assert.ok(跳转, '应有跳转看板链接');
  assert.equal(跳转.dataset.jump, '9', '跳转目标应为任务 9');

  跳转.dispatchEvent(new window.MouseEvent('click', { bubbles: true }));
  assert.equal(看板存储.取值().选中, 9, '点击跳转应选中任务 9');
});
