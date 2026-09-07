import test from 'node:test';
import assert from 'node:assert/strict';

test('看板视图_挂载渲染标题与操作按钮', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};

  // mock 驱动状态接口：空闲就绪，避免挂载时轮询报错
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true, 最近结果: null }) });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);

  看板视图.挂载(容器);

  assert.ok(容器.innerHTML.includes('任务看板'), '应渲染看板标题');
  assert.ok(容器.innerHTML.includes('洪荒五层协作'), '应渲染副标题');
  assert.ok(容器.querySelector('#btn-pub'), '应有发布任务按钮');
  assert.ok(容器.querySelector('#btn-pilot'), '应有驱动一轮按钮');
  assert.ok(容器.querySelector('#btn-pilot-drain'), '应有驱动到空闲按钮');
  assert.ok(容器.querySelector('#board'), '应有看板分组容器');
  assert.ok(容器.querySelector('#pub-title'), '发布弹窗应有标题输入框');
});

test('看板视图_按状态分组渲染任务卡片', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true }) });

  // 先写入存储再挂载，验证任务卡片渲染（待圣人设计：有承接按钮 + 流转按钮）
  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 1, title: '构建身份模块', status: '待圣人设计', 优先级: 'P0' }], 选中: null, 驱动: null });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  看板视图.挂载(容器);

  const 卡片 = 容器.querySelector('.board-card');
  assert.ok(卡片, '应有任务卡片');
  assert.ok(卡片.innerHTML.includes('构建身份模块'), '卡片应含任务标题');
  assert.ok(卡片.innerHTML.includes('P0'), '卡片应含优先级标签');
  assert.ok(容器.querySelector('.btn-accept'), '待圣人设计任务应有承接按钮');
  assert.ok(容器.querySelector('.btn-submit'), '待圣人设计任务应有流转按钮');
});

test('看板视图_待道祖澄清任务渲染澄清按钮', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 7, title: '需求偏差任务', status: '待道祖澄清' }], 选中: null, 驱动: null });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  看板视图.挂载(容器);

  const 澄清按钮 = 容器.querySelector('.btn-clarify');
  assert.ok(澄清按钮, '待道祖澄清任务应有澄清按钮');
  assert.equal(澄清按钮.dataset.id, '7');
});

test('看板视图_属性面板验收包聚合四块结构化信息', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({
    任务: [{
      id: 9,
      title: '验收包任务',
      status: '待清理',
      实现文档: { 代码变更: [{ 文件路径: 'src/主.rs', 变更类型: '新建', 摘要: '实现核心' }], 自检: { 通过: true, 问题: [] } },
      验收文档: { 轮次: [{ 轮次: 1, 通过: true, 问题: [], 建议: '' }] },
      终审文档: { 通过: true, 评语: '符合需求', 风险评估: '低' },
      扫尾记录: { 通过: true, 兑现数: 1, 未兑现数: 0, 多余数: 0, 说明: '' },
    }],
    选中: 9,
    驱动: null,
  });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 属性容器 = window.document.createElement('div');
  window.document.body.appendChild(属性容器);
  // 触发属性面板渲染（选中任务已就绪）
  看板视图.属性(属性容器);

  const 面板内容 = 属性容器.innerHTML;
  assert.ok(面板内容.includes('交付验收包'), '应有验收包标题');
  assert.ok(面板内容.includes('改了什么'), '应有「改了什么」块');
  assert.ok(面板内容.includes('怎么验证'), '应有「怎么验证」块');
  assert.ok(面板内容.includes('卡在哪'), '应有「卡在哪」块');
  assert.ok(面板内容.includes('谁确认'), '应有「谁确认」块');
  assert.ok(面板内容.includes('src/主.rs'), '应展示代码变更路径');
  assert.ok(面板内容.includes('符合需求'), '应展示终审评语');
});

test('看板视图_卡片有交付验收入口且弹层聚合五块', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.prompt = () => '继续';
  globalThis.confirm = () => true;
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({
    任务: [{
      id: 11, title: '证据弹层任务', status: '待清理', 当前层级: '土',
      实现文档: { 代码变更: [{ 文件路径: 'src/证据.rs', 变更类型: '修改', 摘要: '核心修复' }], 自检: { 通过: true, 问题: [] } },
      验收文档: { 轮次: [{ 轮次: 1, 通过: true, 问题: [], 建议: '无' }] },
      终审文档: { 通过: true, 评语: '验收通过', 风险评估: '低' },
      扫尾记录: { 通过: true, 兑现数: 1, 未兑现数: 0, 多余数: 0, 说明: '' },
      状态历史: [{ 原状态: '待圣人设计', 新状态: '圣人设计中' }, { 原状态: '待清理', 新状态: '清理完成' }],
      承接历史: ['道祖', '圣人', '大罗金仙'],
    }],
    选中: null,
    驱动: null,
  });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  看板视图.挂载(容器);

  const 入口 = 容器.querySelector('.btn-evidence');
  assert.ok(入口, '卡片应有交付验收入口按钮');
  assert.equal(入口.dataset.id, '11');
  入口.click();

  const 弹层 = window.document.getElementById('evidence-modal');
  assert.ok(弹层, '点击后应创建验收弹层');
  const 内容 = 弹层.innerHTML;
  assert.ok(内容.includes('交付验收'), '应有交付验收标题');
  assert.ok(内容.includes('改了什么'), '应有①改了什么');
  assert.ok(内容.includes('怎么验证'), '应有②怎么验证');
  assert.ok(内容.includes('卡在哪'), '应有③卡在哪');
  assert.ok(内容.includes('谁确认'), '应有④谁确认');
  assert.ok(内容.includes('过程轨迹'), '应有⑤过程轨迹');
  assert.ok(内容.includes('src/证据.rs'), '应展示代码变更路径');
  assert.ok(内容.includes('验收通过'), '应展示终审评语');
  assert.ok(内容.includes('待圣人设计 → 圣人设计中'), '应展示状态历史');

  // Esc 关闭
  window.document.dispatchEvent(new window.KeyboardEvent('keydown', { key: 'Escape' }));
  assert.equal(window.document.getElementById('evidence-modal'), null, 'Esc 应关闭并移除弹层');
});

test('看板视图_空任务与待澄清任务弹层占位与内嵌操作', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.prompt = () => '继续';
  globalThis.confirm = () => true;
  globalThis.fetch = async () => ({ json: async () => ({ 运行中: false, 就绪: true }) });

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({
    任务: [
      { id: 12, title: '空任务', status: '待圣人设计' },
      { id: 13, title: '澄清任务', status: '待道祖澄清' },
    ],
    选中: null,
    驱动: null,
  });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  看板视图.挂载(容器);

  // 空任务：弹层各块占位不报错
  const 空按钮 = [...容器.querySelectorAll('.btn-evidence')].find((b) => b.dataset.id === '12');
  空按钮.click();
  const 空弹层 = window.document.getElementById('evidence-modal');
  assert.ok(空弹层, '空任务也应打开弹层');
  assert.ok(空弹层.innerHTML.includes('待推进到实现阶段'), '空任务应显示变更占位');
  assert.ok(空弹层.innerHTML.includes('待自检'), '空任务应显示自检占位');
  空弹层.remove();

  // 待道祖澄清：内嵌澄清按钮
  const 澄清按钮 = [...容器.querySelectorAll('.btn-evidence')].find((b) => b.dataset.id === '13');
  澄清按钮.click();
  const 澄清弹层 = window.document.getElementById('evidence-modal');
  const 内嵌澄清 = 澄清弹层.querySelector('[data-evidence-clarify]');
  assert.ok(内嵌澄清, '待道祖澄清任务弹层应有内嵌澄清按钮');
  assert.equal(内嵌澄清.dataset.evidenceClarify, '13');
});

test('看板视图_验收弹层驱动过程按需加载并过滤目标任务', async () => {
  const { Window } = await import('happy-dom');
  const window = new Window({ url: 'http://localhost/' });
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.alert = () => {};
  globalThis.prompt = () => '继续';
  globalThis.confirm = () => true;

  // mock fetch：/api/dev/sessions 清单 + 回放详情（含任务 9 与任务 99 的事件）
  globalThis.fetch = async (url) => {
    const u = String(url);
    if (u === '/api/dev/sessions') {
      return { json: async () => ({ 会话: [{ 会话id: 200, 任务id列表: [9], 创建时间: 100, 状态: '已完成', 发起方式: '发布自动', 事件数: 3 }] }) };
    }
    if (u === '/api/dev/sessions/200') {
      return { json: async () => ({
        会话id: 200, 发起方式: '发布自动', 状态: '已完成',
        事件: [
          { 序号: 1, 任务id: 9, 角色: '圣人', 轮次: 0, 类型: '思考', 工具名: '', 内容: '设计模块 <bound> & 测试', 时间: 101 },
          { 序号: 2, 任务id: 9, 角色: '大罗金仙', 轮次: 1, 类型: '工具调用', 工具名: '写文件', 内容: '实现主逻辑', 时间: 102 },
          { 序号: 3, 任务id: 99, 角色: '圣人', 轮次: 0, 类型: '思考', 工具名: '', 内容: '另一个任务', 时间: 103 },
        ],
      }) };
    }
    return { json: async () => ({ 运行中: false, 就绪: true }) };
  };

  const { 看板存储 } = await import('../../界面呈现-域/世界入口-府/运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js');
  看板存储.更新({ 任务: [{ id: 9, title: '过程任务', status: '待清理' }], 选中: null, 驱动: null });

  const { 看板视图 } = await import('../../界面呈现-域/世界入口-府/视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js');
  const 容器 = window.document.createElement('div');
  window.document.body.appendChild(容器);
  看板视图.挂载(容器);

  const 入口 = 容器.querySelector('.btn-evidence');
  入口.click();
  const 弹层 = window.document.getElementById('evidence-modal');
  assert.ok(弹层, '应打开验收弹层');
  const 按钮 = 弹层.querySelector('.btn-evidence-process');
  assert.ok(按钮, '⑤块应有「查看驱动过程」按钮');
  // 按需加载：未点击时过程容器应为空
  const 初始盒 = 弹层.querySelector('[data-process-box]');
  assert.ok(初始盒, '应有驱动过程容器');
  assert.equal(初始盒.childElementCount, 0, '未点击驱动过程按钮不应请求');

  按钮.click();
  await new Promise((r) => setTimeout(r, 10));
  const 盒 = 弹层.querySelector('[data-process-box]');
  const 文本 = 盒.innerHTML;
  assert.ok(文本.includes('会话 #200'), '应展示会话摘要头');
  assert.ok(文本.includes('设计模块'), '应展示任务9的思考内容');
  assert.ok(文本.includes('写文件'), '应展示任务9的工具调用');
  assert.ok(文本.includes('&lt;bound&gt;'), '事件内容应被转义（＜ 应转义为 &lt;）');
  assert.ok(!文本.includes('另一个任务'), '不应渲染其他任务(99)的事件');
});

