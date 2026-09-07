// 入口.js —— 世界入口府装配根（对应后端 crate 根模块.rs，层层装配）

import { 注册视图, 切换视图, 订阅切换, 监听地址变化, 从地址恢复, 全部视图, 当前视图键 } from './运行支撑-殿/路由导航-阁/导航-核心-园/导航核心.js';
import { 挂载侧栏 } from './框架布局-殿/导航侧栏-阁/侧栏-组件-园/侧栏组件.js';
import { 挂载面板 } from './框架布局-殿/属性面板-阁/面板-组件-园/面板组件.js';
import { 对话视图 } from './视图呈现-殿/对话交互-阁/对话-视图-园/对话视图.js';
import { 获取当前模型, 订阅模型切换, 挂载模型浮层 } from './视图呈现-殿/对话交互-阁/模型选择-园/模型选择.js';
import { 看板视图 } from './视图呈现-殿/看板观测-阁/看板-视图-园/看板视图.js';
import { 工作台视图 } from './视图呈现-殿/看板观测-阁/工作台-视图-园/工作台视图.js';
import { 五行视图 } from './状态监测-殿/引擎观测-阁/五行-视图-园/五行视图.js';
import { 图谱视图 } from './视图呈现-殿/认知观测-阁/图谱-视图-园/图谱视图.js';
import { 格位视图 } from './视图呈现-殿/认知观测-阁/格位-视图-园/格位视图.js';
import { 日志视图 } from './状态监测-殿/日志观测-阁/日志-视图-园/日志视图.js';
import { 记日志, 加载日志 } from './运行支撑-殿/数据服务-阁/日志-数据-园/日志数据.js';
import { 加载引擎数据 } from './运行支撑-殿/数据服务-阁/引擎-数据-园/引擎数据.js';
import { 加载看板数据 } from './运行支撑-殿/数据服务-阁/看板-数据-园/看板数据.js';

注册视图(对话视图);
注册视图(看板视图);
注册视图(工作台视图);
注册视图(五行视图);
注册视图(图谱视图);
注册视图(格位视图);
注册视图(日志视图);

let 当前视图对象 = null;
let 模型浮层 = null;
let 模型浮层打开 = false;

async function 启动() {
  const 侧栏容器 = document.getElementById('侧栏');
  const 主容器 = document.getElementById('主区');
  const 面板容器 = document.getElementById('面板');
  const 状态栏容器 = document.getElementById('状态栏');

  挂载侧栏(侧栏容器);
  挂载面板(面板容器);
  挂载状态栏(状态栏容器);

  订阅切换((视图) => {
    if (!视图) return;
    if (当前视图对象 && typeof 当前视图对象.卸载 === 'function') {
      当前视图对象.卸载();
    }
    当前视图对象 = 视图;
    主容器.innerHTML = '';
    视图.挂载(主容器);
    更新状态栏视图(视图.标题);
  });

  监听地址变化();

  const 恢复键 = 从地址恢复();
  const 初始键 = 恢复键 || 全部视图()[0].键;
  切换视图(初始键);

  await Promise.all([加载引擎数据(), 加载日志(), 加载看板数据()]);
  记日志('【就绪】', 'ok', '世界入口已启动');
}

/** 挂载底部状态栏：左侧视图名+状态灯，右侧模型指示器 */
function 挂载状态栏(容器) {
  const 初始视图 = 全部视图().find(v => v.键 === 当前视图键()) || 全部视图()[0];
  const 模型信息 = 获取当前模型();
  const 模型显示 = 模型信息.已配置 && 模型信息.模型
    ? `${模型信息.供应商 || ''} / ${模型信息.模型}`
    : '未配置 LLM';

  容器.innerHTML = `
    <div class="sb-left">
      <span class="sb-dot"></span>
      <span class="sb-view" id="sb-view">${初始视图 ? 初始视图.标题 : ''}</span>
      <span class="sb-sep"></span>
      <span style="color:var(--text-faint);">智能体运行中</span>
    </div>
    <div class="sb-right">
      <div class="model-indicator" id="sb-model" title="点击选择模型或接入供应商">
        <svg viewBox="0 0 24 24"><path d="M12 2a2 2 0 0 1 2 2c0 .74-.4 1.39-1 1.73V7h1a7 7 0 0 1 7 7h1a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-1v1a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-1H2a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h1a7 7 0 0 1 7-7h1V5.73c-.6-.34-1-.99-1-1.73a2 2 0 0 1 2-2z"/></svg>
        <span class="mi-provider" id="sb-model-provider">${模型信息.已配置 ? (模型信息.供应商 || '') : ''}</span>
        <span class="mi-model" id="sb-model-name">${模型显示}</span>
        <svg class="mi-chevron" viewBox="0 0 24 24"><path d="m6 9 6 6 6-6"/></svg>
      </div>
    </div>
  `;

  // 异步获取当前LLM状态并更新显示
  fetch('/api/llm/status')
    .then((响应) => 响应.json())
    .then((状态) => {
      if (状态.配置 && 状态.当前选择) {
        const 选择 = 状态.当前选择;
        const 名称元素 = 容器.querySelector('#sb-model-name');
        const 供应商元素 = 容器.querySelector('#sb-model-provider');
        if (名称元素) 名称元素.textContent = `${选择.供应商 || ''} / ${选择.模型 || ''}`;
        if (供应商元素) 供应商元素.textContent = 选择.供应商 || '';
      }
    })
    .catch(() => { /* LLM未配置时静默 */ });

  const 指示器 = 容器.querySelector('#sb-model');
  指示器.addEventListener('click', (e) => {
    e.stopPropagation();
    切换模型浮层(指示器);
  });

  // 点击浮层外关闭
  document.addEventListener('click', (e) => {
    if (模型浮层打开 && 模型浮层 && !模型浮层.contains(e.target) && !指示器.contains(e.target)) {
      关闭模型浮层(指示器);
    }
  });

  // 订阅模型切换，更新状态栏显示
  订阅模型切换((信息) => {
    const 名称元素 = 容器.querySelector('#sb-model-name');
    const 供应商元素 = 容器.querySelector('#sb-model-provider');
    if (名称元素) {
      名称元素.textContent = 信息.已配置 && 信息.模型
        ? `${信息.供应商 || ''} / ${信息.模型}`
        : '未配置 LLM';
    }
    if (供应商元素) {
      供应商元素.textContent = 信息.已配置 ? (信息.供应商 || '') : '';
    }
  });
}

/** 切换模型浮层显示/隐藏 */
function 切换模型浮层(指示器) {
  if (模型浮层打开) {
    关闭模型浮层(指示器);
  } else {
    打开模型浮层(指示器);
  }
}

function 打开模型浮层(指示器) {
  if (模型浮层) 模型浮层.remove();
  模型浮层 = document.createElement('div');
  模型浮层.className = 'model-popover';
  document.body.appendChild(模型浮层);
  挂载模型浮层(模型浮层);
  模型浮层打开 = true;
  指示器.classList.add('open');
}

function 关闭模型浮层(指示器) {
  if (模型浮层) {
    模型浮层.remove();
    模型浮层 = null;
  }
  模型浮层打开 = false;
  指示器.classList.remove('open');
}

/** 更新状态栏左侧视图名 */
function 更新状态栏视图(标题) {
  const 元素 = document.getElementById('sb-view');
  if (元素) 元素.textContent = 标题;
}

启动();
