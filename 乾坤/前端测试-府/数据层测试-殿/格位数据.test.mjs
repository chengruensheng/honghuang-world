import test from 'node:test';
import assert from 'node:assert/strict';
import { 加载格位 } from '../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/格位-数据-园/格位数据.js';

test('加载格位_成功返回格位集', async () => {
  globalThis.fetch = async () => ({
    json: async () => ({ 格位集: [{ 维度: '外在', 名: '结构', 摘要: '5 个模块' }] }),
  });
  const 结果 = await 加载格位();
  assert.equal(结果.格位集.length, 1);
  assert.equal(结果.格位集[0].名, '结构');
});

test('加载格位_网络失败返回空格位集', async () => {
  globalThis.fetch = async () => { throw new Error('网络不可达'); };
  const 结果 = await 加载格位();
  assert.deepEqual(结果, { 格位集: [] });
});
