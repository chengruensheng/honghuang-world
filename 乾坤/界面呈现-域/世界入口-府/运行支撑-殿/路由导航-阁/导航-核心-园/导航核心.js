// 导航核心.js —— 视图注册与切换（参考 Vue Router 理念，hash 路由）

const 视图表 = new Map();
const 切换监听器 = new Set();
let 当前键 = null;

/** 注册一个视图组件（组件含 键/标题/挂载/属性 等字段） */
export function 注册视图(视图) {
  视图表.set(视图.键, 视图);
}

/** 返回所有已注册视图组件 */
export function 全部视图() {
  return [...视图表.values()];
}

/** 返回当前视图键 */
export function 当前视图键() {
  return 当前键;
}

/** 切换视图，同步浏览器 hash */
export function 切换视图(键) {
  if (!视图表.has(键)) return;
  当前键 = 键;
  if (window.location.hash !== '#' + 键) {
    window.location.hash = 键;
  }
  通知切换();
}

/** 订阅视图切换，回调收到新视图组件 */
export function 订阅切换(回调) {
  切换监听器.add(回调);
  return () => 切换监听器.delete(回调);
}

/** 监听浏览器地址 hash 变化（前进/后退） */
export function 监听地址变化() {
  window.addEventListener('hashchange', () => {
    const 键 = 从地址恢复();
    if (键 && 键 !== 当前键) {
      当前键 = 键;
      通知切换();
    }
  });
}

/** 从当前 hash 解析视图键，无效则返回 null */
export function 从地址恢复() {
  const 键 = decodeURIComponent(window.location.hash.slice(1));
  return 视图表.has(键) ? 键 : null;
}

function 通知切换() {
  const 视图 = 视图表.get(当前键);
  for (const 回调 of 切换监听器) 回调(视图);
}