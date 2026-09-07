import test from 'node:test';
import assert from 'node:assert/strict';
import { 看板存储, 加载看板数据, 发布任务, 选中任务, 驱动状态 } from '../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';

test('加载看板数据_成功更新存储', async () => {
  globalThis.fetch = async () => ({
    json: async () => [{ id: 1, title: '任务A', status: '待受理' }],
  });
  await 加载看板数据();
  const 状态 = 看板存储.取值();
  assert.equal(状态.任务.length, 1);
  assert.equal(状态.任务[0].title, '任务A');
  assert.equal(状态.任务[0].status, '待受理');
});

test('加载看板数据_失败不改存储', async () => {
  看板存储.更新({ 任务: [] });
  globalThis.fetch = async () => { throw new Error('后端不可达'); };
  await 加载看板数据();
  assert.deepEqual(看板存储.取值().任务, []);
});

test('发布任务_载荷含标题与优先级并回填看板', async () => {
  let 发布载荷 = null;
  globalThis.fetch = async (url, 选项) => {
    if (url === '/api/board' && 选项 && 选项.method === 'POST') {
      发布载荷 = JSON.parse(选项.body);
      return { ok: true, json: async () => ({ id: 1 }) };
    }
    return { json: async () => [{ id: 1, title: '任务A', status: '待受理' }] };
  };
  const 结果 = await 发布任务('任务A', '描述', '设计', 'P0');
  assert.equal(结果.id, 1);
  assert.equal(发布载荷.title, '任务A');
  assert.equal(发布载荷.description, '描述');
  assert.equal(发布载荷.scene, '设计');
  assert.equal(发布载荷.priority, 'P0');
  assert.equal(看板存储.取值().任务.length, 1);
});

test('发布任务_后端拒绝抛错', async () => {
  globalThis.fetch = async () => ({ ok: false, status: 500 });
  await assert.rejects(() => 发布任务('任务A', '描述', null, null), /发布失败: 500/);
});

test('选中任务_更新存储选中项', () => {
  看板存储.更新({ 任务: [{ id: 1, title: '任务A' }, { id: 2, title: '任务B' }], 选中: null });
  选中任务(2);
  assert.equal(看板存储.取值().选中, 2);
});

test('驱动状态_更新存储驱动字段', async () => {
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true, 最近结果: '空闲' }) });
  const 状态 = await 驱动状态();
  assert.equal(状态.就绪, true);
  assert.equal(状态.最近结果, '空闲');
  assert.equal(看板存储.取值().驱动.就绪, true);
});
