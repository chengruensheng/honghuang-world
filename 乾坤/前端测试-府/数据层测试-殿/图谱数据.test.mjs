import test from 'node:test';
import assert from 'node:assert/strict';
import { 加载图谱 } from '../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/图谱-数据-园/图谱数据.js';

test('加载图谱_成功返回图谱结构', async () => {
  globalThis.fetch = async () => ({
    json: async () => ({ 模块集: [{ 名称: 'hm-linkage' }], 符号集: [], 依赖集: [], 技术栈: ['serde'] }),
  });
  const 结果 = await 加载图谱();
  assert.equal(结果.模块集.length, 1);
  assert.equal(结果.模块集[0].名称, 'hm-linkage');
  assert.equal(结果.技术栈[0], 'serde');
});

test('加载图谱_网络失败返回空结构', async () => {
  globalThis.fetch = async () => { throw new Error('网络不可达'); };
  const 结果 = await 加载图谱();
  assert.deepEqual(结果, { 模块集: [], 符号集: [], 依赖集: [], 技术栈: [] });
});
