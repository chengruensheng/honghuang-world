// 对话视图.js —— 对话界面（含模拟执行流：读文件 → 写文件 → 运行命令）

import { 记日志 } from '../../../运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 更新引擎数值 } from '../../../运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';

const 图标 = `<svg viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;

let 主容器 = null;
let 面板容器 = null;
let 定时器列表 = [];
let 运行中 = false;
let 轮次 = 0;
let 任务数 = 42;

export const 对话视图 = {
  键: '对话',
  标题: '对话',
  副标题: '与智能体对话，观察它的执行过程',
  分组: '工作台',
  图标,
  挂载(容器) {
    主容器 = 容器;
    容器.innerHTML = `<div class="view-head"><div><h2>对话</h2><p>与智能体对话，观察它的执行过程</p></div></div><div class="chat" id="chat"></div><div class="chat-input"><input id="task" placeholder="下达任务，如：修复登录超时 bug…" /><button class="btn" id="go">下达</button></div>`;
    const 输入框 = 容器.querySelector('#task');
    const 按钮 = 容器.querySelector('#go');
    按钮.addEventListener('click', () => 下达(输入框, 按钮));
    输入框.addEventListener('keydown', (事件) => {
      if (事件.key === 'Enter') 下达(输入框, 按钮);
    });
  },
  属性(容器) {
    面板容器 = 容器;
    渲染面板();
  },
  卸载() {
    定时器列表.forEach(clearTimeout);
    定时器列表 = [];
    主容器 = null;
    面板容器 = null;
  },
};

function 下达(输入框, 按钮) {
  const 文案 = 输入框.value.trim() || '演示任务';
  if (运行中) return;
  运行中 = true;
  按钮.disabled = true;
  输入框.value = '';
  轮次++;

  气泡('user', 文案);
  记日志('【下达】', 'act', 文案);
  设置过程('受理任务');

  定时器列表.push(setTimeout(() => { 气泡('ai', '我先查看相关代码，定位问题。'); 设置过程('读文件'); }, 400));
  定时器列表.push(setTimeout(() => { 工具调用('读文件', '路径 = 鸿蒙/基础能力-域/项目认知-府/格位数据.rs'); }, 1200));
  定时器列表.push(setTimeout(() => { 工具结果('已返回 144 行源码'); }, 2000));
  定时器列表.push(setTimeout(() => { 气泡('ai', '定位到了，开始修改实现。'); 设置过程('写文件'); }, 2800));
  定时器列表.push(setTimeout(() => { 工具调用('写文件', '路径 = 格位数据.rs · 更新格位结构'); }, 3600));
  定时器列表.push(setTimeout(() => { 工具结果('写入成功，共修改 3 处'); }, 4400));
  定时器列表.push(setTimeout(() => { 气泡('ai', '运行测试验证改动。'); 设置过程('运行命令'); }, 5200));
  定时器列表.push(setTimeout(() => { 工具调用('运行命令', 'cargo test'); }, 6000));
  定时器列表.push(setTimeout(() => { 工具结果('95 passed; 0 failed'); }, 6800));
  定时器列表.push(setTimeout(() => {
    气泡('ai', '✅ 修复完成，测试全部通过。');
    记日志('【完成】', 'ok', '任务完成 · 测试全绿');
    设置过程('任务完成');
    任务数 += 1;
    更新引擎数值(0, 任务数);
    渲染面板();
    运行中 = false;
    按钮.disabled = false;
  }, 7600));
}

function 气泡(角色, 文本) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ' + (角色 === 'user' ? 'user' : 'ai');
  行.innerHTML = `<div class="avatar ${角色 === 'user' ? 'u' : 'ai'}">${角色 === 'user' ? '我' : 'AI'}</div><div class="bubble"><span class="role">${角色 === 'user' ? '用户' : '智能体'}</span>${文本}</div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 工具调用(工具, 参数) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ai';
  行.innerHTML = `<div class="avatar ai">AI</div><div class="bubble" style="flex:1;max-width:100%;padding:0;background:none;border:none;"><div class="toolcall"><div class="tt"><svg viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"/></svg>调用工具 · <span class="cmd">${工具}</span></div><div class="tr">${参数}</div></div></div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 工具结果(文本) {
  if (!主容器) return;
  const 聊天区 = 主容器.querySelector('#chat');
  if (!聊天区) return;
  const 行 = document.createElement('div');
  行.className = 'msg-row ai';
  行.innerHTML = `<div class="avatar ai">AI</div><div class="bubble" style="flex:1;max-width:100%;padding:0;background:none;border:none;"><div class="toolcall"><div class="tt" style="color:var(--wood)">✓ 工具返回</div><div class="tr">${文本}</div></div></div>`;
  聊天区.appendChild(行);
  聊天区.scrollTop = 聊天区.scrollHeight;
}

function 设置过程(文本) {
  if (!面板容器) return;
  const 流 = 面板容器.querySelector('#p-flow');
  if (流) 流.innerHTML = `<div class="kv"><b>当前动作</b>${文本}</div>`;
}

function 渲染面板() {
  if (!面板容器) return;
  面板容器.innerHTML = `<h3>智能体状态</h3><div class="prop-group">
    <div class="prop-item"><span class="k">模型</span><span class="v">MiniMax</span></div>
    <div class="prop-item"><span class="k">当前轮次</span><span class="v">${轮次}</span></div>
    <div class="prop-item"><span class="k">工具</span><span class="v">读文件 / 写文件 / 运行命令</span></div>
    <div class="prop-item"><span class="k">工作区</span><span class="v">洪荒 - 世界</span></div>
  </div><h3>执行过程</h3><div id="p-flow"><div class="kv"><b>当前动作</b>待命</div></div>`;
}