// 格位数据.js —— 三态认知格位（来自后端认知底座：摘要 / 可信度 / 证据引用 / 维度载荷）

/** 从后端拉取心智地图格位 */
export async function 加载格位() {
  try {
    return await fetch('/api/cognition/cells').then((响应) => 响应.json());
  } catch (错误) {
    console.warn('加载格位失败（后端未就绪？）', 错误);
    return { 格位集: [] };
  }
}
