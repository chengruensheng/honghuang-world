import test from 'node:test';
import assert from 'node:assert/strict';

const { 事件图标, 事件类型类, 事件角色 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/事件-呈现-园/事件呈现.js');

test('事件呈现_事件类型映射图标', () => {
  assert.equal(事件图标('思考'), '💭');
  assert.equal(事件图标('工具调用'), '🔧');
  assert.equal(事件图标('工具结果'), '✅');
  assert.equal(事件图标('任务答复'), '📝');
  assert.equal(事件图标('未知类型'), '•', '未知类型应兜底 •');
  assert.equal(事件图标(undefined), '•', '缺省类型应兜底 •');
});

test('事件呈现_事件类型映射CSS类', () => {
  assert.equal(事件类型类('思考'), 'think');
  assert.equal(事件类型类('工具调用'), 'tool-call');
  assert.equal(事件类型类('工具结果'), 'tool-result');
  assert.equal(事件类型类('任务答复'), 'reply');
  assert.equal(事件类型类('未知类型'), '', '未知类型应返回空类');
  assert.equal(事件类型类(undefined), '', '缺省类型应返回空类');
});

test('事件呈现_角色映射中文名与兜底', () => {
  const 映射 = { 道祖: '道祖', 圣人: '圣人', A: '大罗金仙', 准圣: '准圣', 太乙金仙: '太乙金仙' };
  assert.equal(事件角色('A', 映射), '大罗金仙', '命中映射应返回中文名');
  assert.equal(事件角色('圣人', 映射), '圣人');
  assert.equal(事件角色('未知角色', 映射), '未知角色', '未命中映射应返回原文');
  assert.equal(事件角色('A', null), 'A', '无映射应返回原文');
  assert.equal(事件角色(null, 映射), '?', '角色缺失应返回 ?');
  assert.equal(事件角色(undefined), '?', '全缺省应返回 ?');
});
