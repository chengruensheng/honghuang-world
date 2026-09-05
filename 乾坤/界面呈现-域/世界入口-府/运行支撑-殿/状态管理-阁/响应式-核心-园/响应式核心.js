// 响应式核心.js —— 手写响应式状态存储（参考 Pinia 理念）

/**
 * 创建一个响应式存储。
 * 存储内部维护一份状态，订阅者在状态变化时收到通知。
 * @param {object} 初始状态 存储的初始状态
 * @returns {object} 含 取值 / 更新 / 订阅 的存储对象
 */
export function 创建存储(初始状态) {
  const 监听者 = new Set();
  let 状态 = { ...初始状态 };

  function 通知() {
    for (const 回调 of 监听者) 回调(状态);
  }

  return {
    取值: () => 状态,
    更新(补丁) {
      状态 = { ...状态, ...补丁 };
      通知();
    },
    订阅(回调) {
      监听者.add(回调);
      return () => 监听者.delete(回调);
    },
  };
}